use anyhow::{Result, anyhow, bail};
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStderr, ChildStdout, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedLevel {
    Off,
    On,
}

pub const SERIAL_BUFFER_CAP: usize = 64;

#[derive(Debug)]
pub struct RunState {
    pub started: Instant,
    pub ready: bool,
    pub mcu: String,
    pub freq: u32,
    pub voltage_mv: u32,
    pub d13: LedLevel,
    pub last_pin_event_t_us: u64,
    pub prev_pin_event_t_us: u64,
    pub last_event_t_us: u64,
    pub serial_lines: VecDeque<(u64, String)>,
    pub bridge_exited: bool,
    pub bridge_exit_reason: Option<String>,
    pub button_pressed: bool,
    pub pin_states: HashMap<String, u8>,
}

impl Default for RunState {
    fn default() -> Self {
        RunState {
            started: Instant::now(),
            ready: false,
            mcu: String::new(),
            freq: 0,
            voltage_mv: 5000,
            d13: LedLevel::Off,
            last_pin_event_t_us: 0,
            prev_pin_event_t_us: 0,
            last_event_t_us: 0,
            serial_lines: VecDeque::with_capacity(SERIAL_BUFFER_CAP),
            bridge_exited: false,
            bridge_exit_reason: None,
            button_pressed: false,
            pin_states: HashMap::new(),
        }
    }
}

impl RunState {
    pub fn write_state_file(&self, path: &std::path::Path) {
        let mut map = serde_json::Map::new();
        map.insert("ready".into(), serde_json::Value::Bool(self.ready));
        map.insert("mcu".into(), serde_json::Value::String(self.mcu.clone()));
        map.insert("bridge_exited".into(), serde_json::Value::Bool(self.bridge_exited));
        let pins: HashMap<&str, u8> = self.pin_states.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        map.insert("pin_states".into(), serde_json::to_value(&pins).unwrap_or_default());
        let _ = std::fs::write(path, serde_json::to_string_pretty(&map).unwrap_or_default());
    }

    pub fn loop_time_us(&self) -> Option<u64> {
        if self.prev_pin_event_t_us == 0 || self.last_pin_event_t_us <= self.prev_pin_event_t_us {
            None
        } else {
            Some(self.last_pin_event_t_us - self.prev_pin_event_t_us)
        }
    }

    /// 查 (port, bit) 引脚状态 — 数据来源是 bridge 的 `pin` 事件,
    /// key 与 `apply_event` 写入格式保持一致(`"B:5"`)。
    /// 没收到过该引脚的事件 → `None`(对应 `moxin status` 的 `UNKNOWN`)。
    ///
    /// 注:Step 2 独立 commit,调用点在 Step 3(render.rs)/ Step 4(cmd_status.rs)。
    #[allow(dead_code)]
    pub fn get_pin(&self, port: char, bit: u8) -> Option<bool> {
        self.pin_states
            .get(&format!("{}:{}", port, bit))
            .map(|v| *v != 0)
    }

    /// 把 Arduino Uno 的数字引脚号 (D0-D13) 映射到 ATmega328P 的 (port, bit)。
    /// D0-D7   → PORTD bit 0-7
    /// D8-D13  → PORTB bit 0-5
    /// 越界返回 `None`(D14+ 不存在)。
    #[allow(dead_code)]
    pub fn arduino_digital_to_port_bit(d_pin: u8) -> Option<(char, u8)> {
        match d_pin {
            0..=7 => Some(('D', d_pin)),
            8..=13 => Some(('B', d_pin - 8)),
            _ => None,
        }
    }

    /// 把 Arduino Uno 的模拟引脚号 (A0-A5) 映射到 PORTC bit 0-5。
    /// 当前阶段 Phase 2-mini 把 Ax 也当数字引脚看(读 GPIO 电平),
    /// ADC 真采样推到 v0.5.0。
    #[allow(dead_code)]
    pub fn arduino_analog_to_port_bit(a_pin: u8) -> Option<(char, u8)> {
        match a_pin {
            0..=5 => Some(('C', a_pin)),
            _ => None,
        }
    }

    /// 查 D 引脚(D0-D13)电平。
    #[allow(dead_code)]
    pub fn get_arduino_digital(&self, d_pin: u8) -> Option<bool> {
        let (port, bit) = Self::arduino_digital_to_port_bit(d_pin)?;
        self.get_pin(port, bit)
    }

    /// 查 A 引脚(A0-A5)电平(数字视角)。
    #[allow(dead_code)]
    pub fn get_arduino_analog(&self, a_pin: u8) -> Option<bool> {
        let (port, bit) = Self::arduino_analog_to_port_bit(a_pin)?;
        self.get_pin(port, bit)
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "event")]
enum BridgeEvent {
    #[serde(rename = "ready")]
    Ready { mcu: String, freq: u32 },
    #[serde(rename = "pin")]
    Pin { t_us: u64, port: String, bit: u8, value: u8 },
    #[serde(rename = "serial")]
    Serial { t_us: u64, line: String },
    #[serde(rename = "exit")]
    Exit { state: i32 },
    #[serde(rename = "button")]
    Button { t_us: u64, pressed: bool },
}

pub struct RunningSim {
    pub state: Arc<Mutex<RunState>>,
    pub stdin: Option<ChildStdin>,
    child: Child,
    reader_handle: Option<thread::JoinHandle<()>>,
    stderr_reader_handle: Option<thread::JoinHandle<()>>,
}

impl RunningSim {
    pub fn stop(mut self) {
        let _ = self.child.kill();
        let reader = self.reader_handle.take();
        let stderr = self.stderr_reader_handle.take();
        let mut child = self.child;
        std::thread::spawn(move || {
            let _ = child.wait();
            if let Some(h) = reader { let _ = h.join(); }
            if let Some(h) = stderr { let _ = h.join(); }
        });
    }

    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

type IsD13Fn = Box<dyn Fn(&str, u32) -> bool + Send + 'static>;

/// Wrap an already-spawned bridge child into a RunningSim.
/// `is_d13` determines which (port, bit) pair maps to the board's D13 LED.
/// When `json_out` is set, each parsed event line is also echoed to stdout as
/// JSON Lines (for `moxin run --output json`).
pub fn spawn_with_state(
    mut child: Child,
    voltage_mv: u32,
    is_d13: IsD13Fn,
    json_out: bool,
) -> Result<RunningSim> {
    let stdin = child.stdin.take();
    let stdout = child.stdout.take().ok_or_else(|| anyhow!("bridge stdout not piped"))?;
    let stderr = child.stderr.take().ok_or_else(|| anyhow!("bridge stderr not piped"))?;

    let state = Arc::new(Mutex::new(RunState { voltage_mv, ..RunState::default() }));

    let state_bg = Arc::clone(&state);
    let handle = thread::spawn(move || reader_loop(stdout, state_bg, is_d13, json_out));

    let log_path = bridge_log_path();
    let stderr_handle = thread::spawn(move || stderr_reader_loop(stderr, log_path));

    Ok(RunningSim {
        state,
        stdin,
        child,
        reader_handle: Some(handle),
        stderr_reader_handle: Some(stderr_handle),
    })
}

fn reader_loop(
    stdout: ChildStdout,
    state: Arc<Mutex<RunState>>,
    is_d13: IsD13Fn,
    json_out: bool,
) {
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let Ok(ev) = serde_json::from_str::<BridgeEvent>(trimmed) else { continue };
        if json_out {
            emit_json_line(&mut std::io::stdout(), trimmed);
        }
        if std::env::var("MOXIN_DEBUG").is_ok() {
            eprintln!("[debug] {:?}", ev);
        }
        apply_event(&state, ev, &is_d13);
    }
    if let Ok(mut s) = state.lock() {
        s.bridge_exited = true;
        if s.bridge_exit_reason.is_none() {
            s.bridge_exit_reason = Some("stdout closed".to_string());
        }
    }
}

/// Forward one already-validated JSON Lines event to `out`, flushing so piped
/// consumers (`| jq`) see it immediately.
fn emit_json_line<W: Write>(out: &mut W, line: &str) {
    let _ = writeln!(out, "{}", line);
    let _ = out.flush();
}

fn apply_event(
    state: &Arc<Mutex<RunState>>,
    ev: BridgeEvent,
    is_d13: &dyn Fn(&str, u32) -> bool,
) {
    let Ok(mut s) = state.lock() else { return };
    match ev {
        BridgeEvent::Ready { mcu, freq } => {
            s.ready = true;
            s.mcu = mcu;
            s.freq = freq;
        }
        BridgeEvent::Pin { t_us, port, bit, value } => {
            s.last_event_t_us = t_us;
            s.prev_pin_event_t_us = s.last_pin_event_t_us;
            s.last_pin_event_t_us = t_us;
            s.pin_states.insert(format!("{}:{}", port, bit), value);
            if is_d13(&port, bit as u32) {
                s.d13 = if value != 0 { LedLevel::On } else { LedLevel::Off };
            }
        }
        BridgeEvent::Serial { t_us, line } => {
            s.last_event_t_us = t_us;
            if s.serial_lines.len() == SERIAL_BUFFER_CAP {
                s.serial_lines.pop_front();
            }
            s.serial_lines.push_back((t_us, line));
        }
        BridgeEvent::Exit { state: exit_state } => {
            s.bridge_exited = true;
            s.bridge_exit_reason = Some(format!("cpu state {}", exit_state));
        }
        BridgeEvent::Button { t_us, pressed } => {
            s.last_event_t_us = t_us;
            s.button_pressed = pressed;
        }
    }
}

pub(crate) fn stderr_reader_loop(stderr: ChildStderr, log_path: PathBuf) {
    use std::fs::OpenOptions;
    use std::io::Write;
    let mut log = OpenOptions::new().create(true).append(true).open(&log_path).ok();
    let reader = BufReader::new(stderr);
    for line in reader.lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        if let Some(f) = log.as_mut() { let _ = writeln!(f, "{}", line); }
    }
}

pub(crate) fn bridge_log_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .filter(|h| !h.is_empty());
    if let Some(home) = home {
        let dir = PathBuf::from(home).join(".cache").join("moxin");
        if std::fs::create_dir_all(&dir).is_ok() {
            return dir.join("bridge.log");
        }
    }
    PathBuf::from(".moxin-bridge.log")
}

pub fn find_bridge_avr() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("MOXIN_BRIDGE") {
        return Ok(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let name = if cfg!(windows) { "moxin-simavr-bridge.exe" } else { "moxin-simavr-bridge" };
            let candidate = dir.join(name);
            if candidate.exists() { return Ok(candidate); }
        }
    }
    bail!("simavr bridge not found — set $MOXIN_BRIDGE env var or place moxin-simavr-bridge next to the moxin binary")
}

const BRIDGE_STM32_SRC: &str = include_str!("../bridge/stm32/bridge-stm32.c");

pub fn find_bridge_stm32() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("MOXIN_BRIDGE_STM32") {
        return Ok(PathBuf::from(p));
    }
    let cache_dir = dirs_cache_dir()?;
    let bridge_name = if cfg!(windows) { "bridge-stm32.exe" } else { "bridge-stm32" };
    let bridge = cache_dir.join(bridge_name);
    if !bridge.exists() {
        let src = cache_dir.join("bridge-stm32.c");
        std::fs::write(&src, BRIDGE_STM32_SRC)?;
        let out = Command::new("cc")
            .args(["-O2", "-Wall", "-std=c11", "-D_POSIX_C_SOURCE=200809L"])
            .arg("-o").arg(&bridge)
            .arg(&src)
            .output()
            .map_err(|e| anyhow::anyhow!("cc not found: {}", e))?;
        if !out.status.success() {
            bail!("bridge-stm32 compile failed:\n{}", String::from_utf8_lossy(&out.stderr));
        }
    }
    Ok(bridge)
}

fn dirs_cache_dir() -> Result<PathBuf> {
    let base = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    let dir = base.join(".moxin");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Spawn a child process with piped stdio, ready for spawn_with_state.
pub fn spawn_bridge_child(bridge: &std::path::Path, args: &[&std::path::Path], root: &std::path::Path) -> Result<Child> {
    let mut cmd = Command::new(bridge);
    for a in args { cmd.arg(a); }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(root)
        .spawn()
        .map_err(|e| anyhow!("spawn {}: {}", bridge.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_bridge_avr_uses_env_var() {
        std::env::set_var("MOXIN_BRIDGE", "/tmp/fake-bridge");
        let p = find_bridge_avr().unwrap();
        assert_eq!(p, PathBuf::from("/tmp/fake-bridge"));
        std::env::remove_var("MOXIN_BRIDGE");
    }

    #[test]
    fn find_bridge_stm32_uses_env_var() {
        std::env::set_var("MOXIN_BRIDGE_STM32", "/tmp/fake-stm32-bridge");
        let p = find_bridge_stm32().unwrap();
        assert_eq!(p, PathBuf::from("/tmp/fake-stm32-bridge"));
        std::env::remove_var("MOXIN_BRIDGE_STM32");
    }

    #[test]
    fn apply_event_button_updates_state() {
        let state = Arc::new(Mutex::new(RunState::default()));
        let event_json = r#"{"event":"button","t_us":12345,"pressed":true}"#;
        let ev: BridgeEvent = serde_json::from_str(event_json)
            .expect("BridgeEvent::Button should deserialize from real bridge JSON");
        apply_event(&state, ev, &|_, _| false);
        let s = state.lock().unwrap();
        assert!(s.button_pressed);
        assert_eq!(s.last_event_t_us, 12345);
    }

    #[test]
    fn apply_event_button_release() {
        let state = Arc::new(Mutex::new(RunState::default()));
        {
            let mut s = state.lock().unwrap();
            s.button_pressed = true;
        }
        let ev: BridgeEvent = serde_json::from_str(
            r#"{"event":"button","t_us":99999,"pressed":false}"#,
        )
        .unwrap();
        apply_event(&state, ev, &|_, _| false);
        assert!(!state.lock().unwrap().button_pressed);
    }

    #[test]
    fn json_mode_forwards_core_events() {
        // --output json forwards exactly the lines that parse as a BridgeEvent;
        // ready / pin / serial are the three required by the Phase 1 DOD.
        for line in [
            r#"{"event":"ready","mcu":"atmega328p","freq":16000000}"#,
            r#"{"event":"pin","t_us":12345,"port":"B","bit":5,"value":1}"#,
            r#"{"event":"serial","t_us":12346,"line":"hello"}"#,
        ] {
            assert!(
                serde_json::from_str::<BridgeEvent>(line).is_ok(),
                "core event should parse (and thus be forwarded): {line}"
            );
        }
        // garbage is dropped so stdout stays strict JSON Lines
        assert!(serde_json::from_str::<BridgeEvent>("not json").is_err());
    }

    #[test]
    fn arduino_digital_mapping_covers_d0_to_d13() {
        // D0-D7 → PORTD bit 0-7
        for n in 0u8..=7 {
            assert_eq!(RunState::arduino_digital_to_port_bit(n), Some(('D', n)));
        }
        // D8-D13 → PORTB bit 0-5
        for n in 8u8..=13 {
            assert_eq!(RunState::arduino_digital_to_port_bit(n), Some(('B', n - 8)));
        }
        // D14+ 不存在
        assert_eq!(RunState::arduino_digital_to_port_bit(14), None);
        assert_eq!(RunState::arduino_digital_to_port_bit(255), None);
    }

    #[test]
    fn arduino_analog_mapping_covers_a0_to_a5() {
        for n in 0u8..=5 {
            assert_eq!(RunState::arduino_analog_to_port_bit(n), Some(('C', n)));
        }
        assert_eq!(RunState::arduino_analog_to_port_bit(6), None);
        assert_eq!(RunState::arduino_analog_to_port_bit(255), None);
    }

    #[test]
    fn get_arduino_digital_reads_from_pin_states() {
        let state = Arc::new(Mutex::new(RunState::default()));
        // 模拟 bridge 推送 D7(PORTD bit 7)拉高、D13(PORTB bit 5)拉低
        let ev_d7: BridgeEvent = serde_json::from_str(
            r#"{"event":"pin","t_us":100,"port":"D","bit":7,"value":1}"#,
        )
        .unwrap();
        let ev_d13: BridgeEvent = serde_json::from_str(
            r#"{"event":"pin","t_us":200,"port":"B","bit":5,"value":0}"#,
        )
        .unwrap();
        apply_event(&state, ev_d7, &|_, _| false);
        apply_event(&state, ev_d13, &|_, _| false);

        let s = state.lock().unwrap();
        assert_eq!(s.get_arduino_digital(7), Some(true));
        assert_eq!(s.get_arduino_digital(13), Some(false));
        // 没收到过事件的引脚 → None(UNKNOWN)
        assert_eq!(s.get_arduino_digital(2), None);
        // 越界
        assert_eq!(s.get_arduino_digital(14), None);
    }

    #[test]
    fn get_arduino_analog_reads_portc() {
        let state = Arc::new(Mutex::new(RunState::default()));
        let ev: BridgeEvent = serde_json::from_str(
            r#"{"event":"pin","t_us":50,"port":"C","bit":3,"value":1}"#,
        )
        .unwrap();
        apply_event(&state, ev, &|_, _| false);
        let s = state.lock().unwrap();
        assert_eq!(s.get_arduino_analog(3), Some(true));
        assert_eq!(s.get_arduino_analog(0), None);
    }

    #[test]
    fn emit_json_line_writes_line_with_trailing_newline() {
        let mut buf: Vec<u8> = Vec::new();
        let line = r#"{"event":"ready","mcu":"atmega328p","freq":16000000}"#;
        emit_json_line(&mut buf, line);
        assert_eq!(buf, format!("{line}\n").into_bytes());
    }
}
