use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedLevel {
    Off,
    On,
}

/// Serial Monitor 环形缓冲容量。
/// 64 行够 1 屏左右,bridge 比 TUI 出 line 快也不会无界增长。
pub const SERIAL_BUFFER_CAP: usize = 64;

/// 共享状态,后台读 bridge stdout 的线程写,主线程 / TUI / Inspector 读。
#[derive(Debug)]
pub struct RunState {
    pub started: Instant,
    pub ready: bool,
    pub mcu: String,
    pub freq: u32,
    /// 板子标称工作电压(mV)。Inspector 渲染那行 `Voltage: 5.02V` 的源。
    /// arduino-uno 5000,stm32 3300。
    pub voltage_mv: u32,
    /// D13 = 当前演示用 LED 的电平。AVR 上跟 PORTB bit 5 对齐;
    /// STM32 上跟我们规定的 GPIO bit=13 对齐(firmware 端 toggle PA13)。
    pub d13: LedLevel,
    /// 最近一次 pin event 的 t_us(MCU 时间轴)。
    pub last_pin_event_t_us: u64,
    /// 上一次 pin event 的 t_us。Inspector 用 (last - prev) 算 loop time。
    pub prev_pin_event_t_us: u64,
    /// 最近事件的 t_us(任何类型),T7.5 时段进度用。
    pub last_event_t_us: u64,
    /// Serial Monitor 行环形缓冲。bridge 每收到一行非 PIN 模式的 UART 输出
    /// 就 push (t_us, line) 进来。容量超 SERIAL_BUFFER_CAP 时从前丢。
    pub serial_lines: VecDeque<(u64, String)>,
    pub bridge_exited: bool,
    pub bridge_exit_reason: Option<String>,
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
        }
    }
}

impl RunState {
    /// Inspector 用:两次连续 pin event 之间的微秒差。
    /// 不到两次 pin event 时返回 None,UI 上渲染 "—"。
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
    Pin {
        t_us: u64,
        port: String,
        bit: u8,
        value: u8,
    },
    #[serde(rename = "serial")]
    Serial { t_us: u64, line: String },
    #[serde(rename = "exit")]
    Exit { state: i32 },
}

/// 一个正在运行的 bridge 子进程 + 后台读取线程的句柄
pub struct RunningSim {
    pub state: Arc<Mutex<RunState>>,
    child: Child,
    reader_handle: Option<thread::JoinHandle<()>>,
    stderr_reader_handle: Option<thread::JoinHandle<()>>,
}

impl RunningSim {
    pub fn stop(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(h) = self.reader_handle.take() {
            let _ = h.join();
        }
        if let Some(h) = self.stderr_reader_handle.take() {
            let _ = h.join();
        }
    }

    pub fn is_alive(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,
            _ => false,
        }
    }
}

pub fn cmd_run(root: &Path, hex: &Path) -> Result<RunningSim> {
    let bridge = find_bridge()?;
    if !bridge.exists() {
        bail!(
            "simavr bridge not found at {} — set $MOXIN_BRIDGE or `make` in bridge/",
            bridge.display()
        );
    }
    if !hex.exists() {
        bail!("hex not found: {} — run `build` first", hex.display());
    }

    let child = Command::new(&bridge)
        .arg(hex)
        .arg("atmega328p")
        .arg("16000000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(root)
        .spawn()
        .with_context(|| format!("spawn {}", bridge.display()))?;

    // arduino-uno 标称 5V
    spawn_with_state(child, 5000)
}

/// 通用入口:已经 spawn 出 bridge 子进程后,把它包成 RunningSim。
/// arduino 路径(cmd_run)和 stm32 路径(cmd_run_stm32)共享。
pub(crate) fn spawn_with_state(mut child: Child, voltage_mv: u32) -> Result<RunningSim> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("bridge stdout not piped"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("bridge stderr not piped"))?;

    let mut initial = RunState::default();
    initial.voltage_mv = voltage_mv;
    let state = Arc::new(Mutex::new(initial));

    let state_bg = Arc::clone(&state);
    let handle = thread::spawn(move || reader_loop(stdout, state_bg));

    let log_path = bridge_log_path();
    let stderr_handle = thread::spawn(move || stderr_reader_loop(stderr, log_path));

    Ok(RunningSim {
        state,
        child,
        reader_handle: Some(handle),
        stderr_reader_handle: Some(stderr_handle),
    })
}

fn reader_loop(stdout: ChildStdout, state: Arc<Mutex<RunState>>) {
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<BridgeEvent>(trimmed) {
            Ok(ev) => {
                if std::env::var("MOXIN_DEBUG").is_ok() {
                    eprintln!("[debug] {:?}", ev);
                }
                apply_event(&state, ev);
            }
            Err(_) => {
                // 可能是非 JSON 调试输出,忽略
            }
        }
    }
    // 流结束 → 子进程退出了
    if let Ok(mut s) = state.lock() {
        s.bridge_exited = true;
        if s.bridge_exit_reason.is_none() {
            s.bridge_exit_reason = Some("stdout closed".to_string());
        }
    }
}

fn apply_event(state: &Arc<Mutex<RunState>>, ev: BridgeEvent) {
    let mut s = state.lock().unwrap();
    match ev {
        BridgeEvent::Ready { mcu, freq } => {
            s.ready = true;
            s.mcu = mcu;
            s.freq = freq;
        }
        BridgeEvent::Pin {
            t_us,
            port,
            bit,
            value,
        } => {
            s.last_event_t_us = t_us;
            s.prev_pin_event_t_us = s.last_pin_event_t_us;
            s.last_pin_event_t_us = t_us;
            // 简化映射:AVR 上 PORTB bit5 = D13;STM32 bridge 用 port="GPIO" + bit=13。
            // 任一种命中都更新 d13。
            let is_d13 =
                (port == "B" && bit == 5) || (port == "GPIO" && bit == 13);
            if is_d13 {
                s.d13 = if value != 0 {
                    LedLevel::On
                } else {
                    LedLevel::Off
                };
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
    }
}

fn find_bridge() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("MOXIN_BRIDGE") {
        return Ok(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("moxin-simavr-bridge");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    Ok(PathBuf::from(home).join("projects/moxin-demo/bridge/moxin-simavr-bridge"))
}

/// 把 bridge stderr 行追加到日志文件,绝不影响主进程。
/// 文件打不开就静默丢弃 —— bridge stderr 是诊断信息,不致命。
pub(crate) fn stderr_reader_loop(stderr: ChildStderr, log_path: PathBuf) {
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok();

    let reader = BufReader::new(stderr);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if let Some(f) = log.as_mut() {
            let _ = writeln!(f, "{}", line);
        }
    }
}

/// 决定 bridge 日志文件路径:
///   首选 `~/.cache/moxin/bridge.log`(目录自动建)
///   HOME 缺失或目录建不出 → fallback 到 `./.moxin-bridge.log`
pub(crate) fn bridge_log_path() -> PathBuf {
    if let Some(home) = std::env::var("HOME").ok().filter(|h| !h.is_empty()) {
        let dir = PathBuf::from(home).join(".cache").join("moxin");
        if std::fs::create_dir_all(&dir).is_ok() {
            return dir.join("bridge.log");
        }
    }
    PathBuf::from(".moxin-bridge.log")
}
