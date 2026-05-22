use anyhow::{Result, anyhow, bail};
use serde::Deserialize;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
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
        }
    }
}

impl RunState {
    pub fn loop_time_us(&self) -> Option<u64> {
        if self.prev_pin_event_t_us == 0 || self.last_pin_event_t_us <= self.prev_pin_event_t_us {
            None
        } else {
            Some(self.last_pin_event_t_us - self.prev_pin_event_t_us)
        }
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
pub fn spawn_with_state(
    mut child: Child,
    voltage_mv: u32,
    is_d13: IsD13Fn,
) -> Result<RunningSim> {
    let stdin = child.stdin.take();
    let stdout = child.stdout.take().ok_or_else(|| anyhow!("bridge stdout not piped"))?;
    let stderr = child.stderr.take().ok_or_else(|| anyhow!("bridge stderr not piped"))?;

    let state = Arc::new(Mutex::new(RunState { voltage_mv, ..RunState::default() }));

    let state_bg = Arc::clone(&state);
    let handle = thread::spawn(move || reader_loop(stdout, state_bg, is_d13));

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
) {
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        if let Ok(ev) = serde_json::from_str::<BridgeEvent>(trimmed) {
            if std::env::var("MOXIN_DEBUG").is_ok() {
                eprintln!("[debug] {:?}", ev);
            }
            apply_event(&state, ev, &is_d13);
        }
    }
    if let Ok(mut s) = state.lock() {
        s.bridge_exited = true;
        if s.bridge_exit_reason.is_none() {
            s.bridge_exit_reason = Some("stdout closed".to_string());
        }
    }
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
    if let Some(home) = std::env::var("HOME").ok().filter(|h| !h.is_empty()) {
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
            let candidate = dir.join("moxin-simavr-bridge");
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
    let bridge = cache_dir.join("bridge-stm32");
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
    let base = std::env::var("HOME")
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
}
