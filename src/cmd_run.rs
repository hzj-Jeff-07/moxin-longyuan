use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedLevel {
    Off,
    On,
}

/// 共享状态,后台读 bridge stdout 的线程写,主线程的 `show` 命令读
#[derive(Debug)]
pub struct RunState {
    pub started: Instant,
    pub ready: bool,
    pub mcu: String,
    pub freq: u32,
    /// D13 = PORTB bit 5 的当前电平
    pub d13: LedLevel,
    pub last_event_t_us: u64,
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
            d13: LedLevel::Off,
            last_event_t_us: 0,
            bridge_exited: false,
            bridge_exit_reason: None,
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
    #[serde(rename = "exit")]
    Exit { state: i32 },
}

/// 一个正在运行的 simavr 子进程 + 后台读取线程的句柄
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

    let mut child = Command::new(&bridge)
        .arg(hex)
        .arg("atmega328p")
        .arg("16000000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(root)
        .spawn()
        .with_context(|| format!("spawn {}", bridge.display()))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("bridge stdout not piped"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow!("bridge stderr not piped"))?;

    let state = Arc::new(Mutex::new(RunState::default()));
    let state_bg = Arc::clone(&state);
    let handle = thread::spawn(move || reader_loop(stdout, state_bg));

    let log_path = bridge_log_path();
    let stderr_handle = thread::spawn(move || stderr_reader_loop(stderr, log_path));

    println!("✓ simulator started (simavr)");

    Ok(RunningSim {
        state,
        child,
        reader_handle: Some(handle),
        stderr_reader_handle: Some(stderr_handle),
    })
}

fn reader_loop(stdout: std::process::ChildStdout, state: Arc<Mutex<RunState>>) {
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
            if port == "B" && bit == 5 {
                s.d13 = if value != 0 {
                    LedLevel::On
                } else {
                    LedLevel::Off
                };
            }
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
    // 尝试在 moxin 可执行文件附近找
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("moxin-simavr-bridge");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }
    // 开发默认路径
    let home = std::env::var("HOME").unwrap_or_default();
    Ok(PathBuf::from(home).join("projects/moxin-demo/bridge/moxin-simavr-bridge"))
}

/// 把 bridge stderr 行追加到日志文件,绝不影响主进程。
/// 文件打不开就静默丢弃 —— bridge stderr 是诊断信息,不致命。
fn stderr_reader_loop(stderr: std::process::ChildStderr, log_path: PathBuf) {
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
fn bridge_log_path() -> PathBuf {
    if let Some(home) = std::env::var("HOME").ok().filter(|h| !h.is_empty()) {
        let dir = PathBuf::from(home).join(".cache").join("moxin");
        if std::fs::create_dir_all(&dir).is_ok() {
            return dir.join("bridge.log");
        }
    }
    PathBuf::from(".moxin-bridge.log")
}
