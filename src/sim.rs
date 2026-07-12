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

/// 连续多少个周期彼此偏差 ≤5% 才认定为稳定 PWM 波形。
pub const PWM_STABLE_PERIODS: u32 = 3;

/// 一次 PWM 推导结果。由 `PwmTracker` 从 `pin` 边沿事件算出,
/// bridge 侧不参与(phase-2-full RFC Step 3 选定的纯 Rust 方案)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PwmSample {
    /// 占空比 0..=255,对齐 Arduino `analogWrite` 值域(128 = 50%)
    pub duty: u8,
    pub freq_hz: u32,
    /// 连续 `PWM_STABLE_PERIODS` 个周期频率稳定才为 true;随机翻转保持 false
    pub stable: bool,
    /// 本样本对应上升沿的仿真时间,配合 `RunState::get_pwm` 做过期判定
    pub t_us: u64,
}

/// 基于 pin 边沿时间差推导 duty / freq。
/// 一个完整周期 = 上升沿 → 下降沿 → 下一个上升沿;每个上升沿收口一次采样。
#[derive(Debug, Default)]
pub struct PwmTracker {
    last_rise: Option<u64>,
    last_fall: Option<u64>,
    last_period_us: Option<u64>,
    last_level: Option<u8>,
    stable_periods: u32,
}

impl PwmTracker {
    /// 喂入一条 pin 事件。返回 `Some` 表示刚收口一个完整周期。
    pub fn observe(&mut self, value: u8, t_us: u64) -> Option<PwmSample> {
        let level = u8::from(value != 0);
        if self.last_level == Some(level) {
            return None; // 电平重复,不是边沿
        }
        self.last_level = Some(level);
        if level == 0 {
            self.last_fall = Some(t_us);
            return None;
        }
        let prev_rise = self.last_rise.replace(t_us)?;
        let period = t_us.saturating_sub(prev_rise);
        if period == 0 {
            return None;
        }
        if let Some(prev_period) = self.last_period_us {
            // |period - prev| ≤ 5% × prev(整数运算:diff × 20 ≤ prev)
            if period.abs_diff(prev_period) * 20 <= prev_period {
                self.stable_periods += 1;
            } else {
                self.stable_periods = 0;
            }
        }
        self.last_period_us = Some(period);
        let fall = self.last_fall.filter(|f| (prev_rise..t_us).contains(f))?;
        let high_us = fall - prev_rise;
        Some(PwmSample {
            duty: ((high_us * 255 + period / 2) / period) as u8,
            freq_hz: ((1_000_000 + period / 2) / period) as u32,
            stable: self.stable_periods + 1 >= PWM_STABLE_PERIODS,
            t_us,
        })
    }
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
    /// 每引脚最新 PWM 采样,key 与 `pin_states` 同格式(`"B:1"`)。读取走 `get_pwm`(带过期判定)。
    pub pwm: HashMap<String, PwmSample>,
    /// 每 ADC 通道最新注入值(0..=1023),来自 bridge 的 `adc` 事件回显。
    pub adc_values: HashMap<u8, u16>,
    /// 超声波注入距离(cm)。`configure_ultrasonics` 时置 bridge 默认值 50,
    /// `set_distance` 后跟随注入值;没有超声波元件时保持 None。
    pub ultrasonic_cm: Option<u16>,
    /// DHT11 注入环境 (temp°C, hum%)。configure 时置 bridge 默认 (25, 60)。
    pub dht_env: Option<(u8, u8)>,
    /// 最近发送的红外 NEC 码(bridge `ir` 事件回显)。
    pub ir_code: Option<u32>,
    /// bridge `hello` 事件宣告的能力(如 "adc" / "serial");老 bridge 不发 hello → 空。
    pub bridge_capabilities: Vec<String>,
    /// bridge 协议版本,来自 `hello`;老 bridge → None。
    pub bridge_protocol: Option<String>,
    /// 每引脚边沿追踪器,只在 `apply_event` 内喂数据。
    pub pwm_trackers: HashMap<String, PwmTracker>,
    /// 自上次写 `.moxin-state.json` 后状态是否变过。
    /// 初值 true 保证启动后立即落一次初始快照;写盘方(main.rs run --output json)
    /// 写完负责清零,避免 50ms 轮询在状态不变时反复写盘。
    pub dirty: bool,
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
            pwm: HashMap::new(),
            adc_values: HashMap::new(),
            ultrasonic_cm: None,
            dht_env: None,
            ir_code: None,
            bridge_capabilities: Vec::new(),
            bridge_protocol: None,
            pwm_trackers: HashMap::new(),
            dirty: true,
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
    pub fn get_pin(&self, port: char, bit: u8) -> Option<bool> {
        self.pin_states
            .get(&format!("{}:{}", port, bit))
            .map(|v| *v != 0)
    }

    /// 把 Arduino Uno 的数字引脚号 (D0-D13) 映射到 ATmega328P 的 (port, bit)。
    /// D0-D7   → PORTD bit 0-7
    /// D8-D13  → PORTB bit 0-5
    /// 越界返回 `None`(D14+ 不存在)。
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
    pub fn arduino_analog_to_port_bit(a_pin: u8) -> Option<(char, u8)> {
        match a_pin {
            0..=5 => Some(('C', a_pin)),
            _ => None,
        }
    }

    /// 查 (port, bit) 引脚的 PWM 采样。样本对应的波形停止后不能一直挂着旧值:
    /// 距最新事件超过 3 个周期没有新边沿 → 视为过期,返回 `None`(渲染回退 ON/OFF)。
    pub fn get_pwm(&self, port: char, bit: u8) -> Option<PwmSample> {
        let sample = self.pwm.get(&format!("{}:{}", port, bit))?;
        if sample.freq_hz == 0 {
            return None;
        }
        let period_us = 1_000_000 / sample.freq_hz as u64;
        if self.last_event_t_us.saturating_sub(sample.t_us) > period_us * 3 {
            return None;
        }
        Some(*sample)
    }

    /// 查 D 引脚(D0-D13)电平。
    pub fn get_arduino_digital(&self, d_pin: u8) -> Option<bool> {
        let (port, bit) = Self::arduino_digital_to_port_bit(d_pin)?;
        self.get_pin(port, bit)
    }

    /// 查 A 引脚(A0-A5)电平(数字视角)。
    pub fn get_arduino_analog(&self, a_pin: u8) -> Option<bool> {
        let (port, bit) = Self::arduino_analog_to_port_bit(a_pin)?;
        self.get_pin(port, bit)
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "event")]
enum BridgeEvent {
    #[serde(rename = "hello")]
    Hello { protocol: String, capabilities: Vec<String> },
    #[serde(rename = "ready")]
    Ready { mcu: String, freq: u32 },
    #[serde(rename = "pin")]
    Pin { t_us: u64, port: String, bit: u8, value: u8 },
    #[serde(rename = "serial")]
    Serial { t_us: u64, line: String },
    #[serde(rename = "adc")]
    Adc { t_us: u64, channel: u8, value: u16 },
    #[serde(rename = "dht")]
    Dht { t_us: u64, temp: u8, hum: u8 },
    #[serde(rename = "ir")]
    Ir { t_us: u64, code: u32 },
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

    /// 通过 bridge stdin 命令通道注入 ADC 值(0..=1023,超界截断)。
    /// bridge 处理后会回显 `adc` 事件,`RunState::adc_values` 由事件流更新。
    pub fn set_adc(&mut self, channel: u8, value: u16) -> Result<()> {
        if let Ok(s) = self.state.lock() {
            // 只对宣告了 adc 能力的 bridge 发命令;老 bridge 不发 hello → 拒绝并说明
            if !s.bridge_capabilities.iter().any(|c| c == "adc") {
                bail!(
                    "bridge does not support adc injection (capabilities: {:?}) — rebuild bridge/ (make -C bridge)",
                    s.bridge_capabilities
                );
            }
        }
        let Some(stdin) = self.stdin.as_mut() else {
            bail!("bridge stdin not available — cannot inject adc value");
        };
        writeln!(stdin, "adc {} {}", channel, value.min(1023))
            .and_then(|_| stdin.flush())
            .map_err(|e| anyhow!("write adc command to bridge: {}", e))?;
        Ok(())
    }

    /// 向 bridge 声明超声波 trigger/echo 引脚((port, bit) 对)。
    /// 老 bridge 不认识该命令,写了会被忽略 — 声明本身无副作用,不做能力检查。
    pub fn configure_sr04(&mut self, trig: (char, u8), echo: (char, u8)) -> Result<()> {
        let Some(stdin) = self.stdin.as_mut() else {
            bail!("bridge stdin not available — cannot configure sr04");
        };
        writeln!(stdin, "sr04 {} {} {} {}", trig.0, trig.1, echo.0, echo.1)
            .and_then(|_| stdin.flush())
            .map_err(|e| anyhow!("write sr04 command to bridge: {}", e))?;
        if let Ok(mut s) = self.state.lock() {
            s.ultrasonic_cm = Some(50); // bridge 侧默认距离
        }
        Ok(())
    }

    /// 向 bridge 声明 DHT11 data 引脚。同 configure_sr04,老 bridge 忽略。
    pub fn configure_dht(&mut self, data: (char, u8)) -> Result<()> {
        let Some(stdin) = self.stdin.as_mut() else {
            bail!("bridge stdin not available — cannot configure dht");
        };
        writeln!(stdin, "dht {} {}", data.0, data.1)
            .and_then(|_| stdin.flush())
            .map_err(|e| anyhow!("write dht command to bridge: {}", e))?;
        if let Ok(mut s) = self.state.lock() {
            s.dht_env = Some((25, 60)); // bridge 侧默认环境
        }
        Ok(())
    }

    /// 注入 DHT11 环境:温度 0..=50°C,湿度 20..=90%(超界截断)。
    pub fn set_env(&mut self, temp: u8, hum: u8) -> Result<()> {
        if let Ok(s) = self.state.lock() {
            if !s.bridge_capabilities.iter().any(|c| c == "dht") {
                bail!(
                    "bridge does not support dht env injection (capabilities: {:?}) — rebuild bridge/ (make -C bridge)",
                    s.bridge_capabilities
                );
            }
        }
        let temp = temp.min(50);
        let hum = hum.clamp(20, 90);
        let Some(stdin) = self.stdin.as_mut() else {
            bail!("bridge stdin not available — cannot inject env");
        };
        writeln!(stdin, "env {} {}", temp, hum)
            .and_then(|_| stdin.flush())
            .map_err(|e| anyhow!("write env command to bridge: {}", e))?;
        // dht_env 由 bridge 回显的 dht 事件更新,这里不重复写
        Ok(())
    }

    /// 向 bridge 声明红外接收头 out 引脚。声明后 bridge 500ms 自发一帧自检码。
    pub fn configure_ir(&mut self, out: (char, u8)) -> Result<()> {
        let Some(stdin) = self.stdin.as_mut() else {
            bail!("bridge stdin not available — cannot configure ir");
        };
        writeln!(stdin, "ir {} {}", out.0, out.1)
            .and_then(|_| stdin.flush())
            .map_err(|e| anyhow!("write ir command to bridge: {}", e))?;
        Ok(())
    }

    /// 发送一帧 NEC 红外码(32 bit)。
    pub fn send_ir(&mut self, code: u32) -> Result<()> {
        if let Ok(s) = self.state.lock() {
            if !s.bridge_capabilities.iter().any(|c| c == "ir") {
                bail!(
                    "bridge does not support ir injection (capabilities: {:?}) — rebuild bridge/ (make -C bridge)",
                    s.bridge_capabilities
                );
            }
        }
        let Some(stdin) = self.stdin.as_mut() else {
            bail!("bridge stdin not available — cannot send ir frame");
        };
        writeln!(stdin, "irtx {:08X}", code)
            .and_then(|_| stdin.flush())
            .map_err(|e| anyhow!("write irtx command to bridge: {}", e))?;
        Ok(())
    }

    /// 注入超声波距离(2..=400cm,超界截断)。
    pub fn set_distance(&mut self, cm: u16) -> Result<()> {
        if let Ok(s) = self.state.lock() {
            if !s.bridge_capabilities.iter().any(|c| c == "sr04") {
                bail!(
                    "bridge does not support sr04 distance injection (capabilities: {:?}) — rebuild bridge/ (make -C bridge)",
                    s.bridge_capabilities
                );
            }
        }
        let cm = cm.clamp(2, 400);
        let Some(stdin) = self.stdin.as_mut() else {
            bail!("bridge stdin not available — cannot inject distance");
        };
        writeln!(stdin, "dist {}", cm)
            .and_then(|_| stdin.flush())
            .map_err(|e| anyhow!("write dist command to bridge: {}", e))?;
        if let Ok(mut s) = self.state.lock() {
            s.ultrasonic_cm = Some(cm);
            s.dirty = true;
        }
        Ok(())
    }
}

/// spawn_sim 之后调用一次的外设自动配置总入口
/// (shell run / run --output json / assert 三个入口共用)。
pub fn configure_peripherals(
    sim: &mut RunningSim,
    project: &crate::project::Project,
    spec: &crate::boards::BoardSpec,
) -> Result<()> {
    configure_ultrasonics(sim, project, spec)?;
    configure_dhts(sim, project, spec)?;
    configure_irs(sim, project, spec)?;
    Ok(())
}

/// 扫 project 里的红外接收头,把 out 引脚经 stdin 下发给 bridge。
fn configure_irs(
    sim: &mut RunningSim,
    project: &crate::project::Project,
    _spec: &crate::boards::BoardSpec,
) -> Result<()> {
    for comp in project.components.iter().filter(|c| c.kind == "ir_receiver") {
        for (terminal, pin) in
            crate::components::util::component_terminal_pins(&comp.id, project)
        {
            if !matches!(terminal.as_str(), "out" | "data" | "signal") {
                continue;
            }
            let port_bit = match pin {
                crate::board::PinRef::BoardDigital(n) => {
                    RunState::arduino_digital_to_port_bit(n)
                }
                crate::board::PinRef::BoardAnalog(n) => {
                    RunState::arduino_analog_to_port_bit(n)
                }
                _ => None,
            };
            if let Some(p) = port_bit {
                sim.configure_ir(p)?;
            }
        }
    }
    Ok(())
}

/// 扫 project 里的 DHT11 元件,把 data 引脚经 stdin 下发给 bridge。
fn configure_dhts(
    sim: &mut RunningSim,
    project: &crate::project::Project,
    _spec: &crate::boards::BoardSpec,
) -> Result<()> {
    for comp in project.components.iter().filter(|c| c.kind == "dht11") {
        for (terminal, pin) in
            crate::components::util::component_terminal_pins(&comp.id, project)
        {
            if !matches!(terminal.as_str(), "data" | "out" | "signal") {
                continue;
            }
            let port_bit = match pin {
                crate::board::PinRef::BoardDigital(n) => {
                    RunState::arduino_digital_to_port_bit(n)
                }
                crate::board::PinRef::BoardAnalog(n) => {
                    RunState::arduino_analog_to_port_bit(n)
                }
                _ => None,
            };
            if let Some(d) = port_bit {
                sim.configure_dht(d)?;
            }
        }
    }
    Ok(())
}

/// 扫 project 里的超声波元件,把 trigger/echo 引脚经 stdin 下发给 bridge。
/// 没有超声波元件时是 no-op;bridge 老版本会忽略该命令。
fn configure_ultrasonics(
    sim: &mut RunningSim,
    project: &crate::project::Project,
    _spec: &crate::boards::BoardSpec,
) -> Result<()> {
    for comp in project.components.iter().filter(|c| c.kind == "ultrasonic") {
        let mut trig: Option<(char, u8)> = None;
        let mut echo: Option<(char, u8)> = None;
        for (terminal, pin) in
            crate::components::util::component_terminal_pins(&comp.id, project)
        {
            let port_bit = match pin {
                crate::board::PinRef::BoardDigital(n) => {
                    RunState::arduino_digital_to_port_bit(n)
                }
                crate::board::PinRef::BoardAnalog(n) => {
                    RunState::arduino_analog_to_port_bit(n)
                }
                _ => None,
            };
            match terminal.as_str() {
                "trig" | "trigger" => trig = port_bit,
                "echo" => echo = port_bit,
                _ => {}
            }
        }
        if let (Some(t), Some(e)) = (trig, echo) {
            sim.configure_sr04(t, e)?;
        }
    }
    Ok(())
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
        s.dirty = true;
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
    s.dirty = true;
    match ev {
        BridgeEvent::Hello { protocol, capabilities } => {
            s.bridge_protocol = Some(protocol);
            s.bridge_capabilities = capabilities;
        }
        BridgeEvent::Ready { mcu, freq } => {
            s.ready = true;
            s.mcu = mcu;
            s.freq = freq;
        }
        BridgeEvent::Pin { t_us, port, bit, value } => {
            s.last_event_t_us = t_us;
            s.prev_pin_event_t_us = s.last_pin_event_t_us;
            s.last_pin_event_t_us = t_us;
            let key = format!("{}:{}", port, bit);
            if let Some(sample) = s.pwm_trackers.entry(key.clone()).or_default().observe(value, t_us) {
                s.pwm.insert(key.clone(), sample);
            }
            s.pin_states.insert(key, value);
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
        BridgeEvent::Adc { t_us, channel, value } => {
            s.last_event_t_us = t_us;
            s.adc_values.insert(channel, value.min(1023));
        }
        BridgeEvent::Dht { t_us, temp, hum } => {
            s.last_event_t_us = t_us;
            s.dht_env = Some((temp, hum));
        }
        BridgeEvent::Ir { t_us, code } => {
            s.last_event_t_us = t_us;
            s.ir_code = Some(code);
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
    find_bridge_avr_impl(std::env::var("MOXIN_BRIDGE").ok())
}

/// `env_override` = `$MOXIN_BRIDGE` 的值。单独拆出来是为了让单测注入,
/// 避免 `std::env::set_var` 在并行测试线程里与 `getenv` 竞态。
fn find_bridge_avr_impl(env_override: Option<String>) -> Result<PathBuf> {
    if let Some(p) = env_override {
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
    find_bridge_stm32_impl(std::env::var("MOXIN_BRIDGE_STM32").ok())
}

/// `env_override` = `$MOXIN_BRIDGE_STM32` 的值,拆分理由同 `find_bridge_avr_impl`。
fn find_bridge_stm32_impl(env_override: Option<String>) -> Result<PathBuf> {
    if let Some(p) = env_override {
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
        let p = find_bridge_avr_impl(Some("/tmp/fake-bridge".to_string())).unwrap();
        assert_eq!(p, PathBuf::from("/tmp/fake-bridge"));
    }

    #[test]
    fn find_bridge_stm32_uses_env_var() {
        let p = find_bridge_stm32_impl(Some("/tmp/fake-stm32-bridge".to_string())).unwrap();
        assert_eq!(p, PathBuf::from("/tmp/fake-stm32-bridge"));
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

    /// 喂入 duty_us/period_us 的方波,返回最后一个上升沿收口的采样。
    fn feed_square_wave(
        tracker: &mut PwmTracker,
        period_us: u64,
        high_us: u64,
        cycles: u64,
    ) -> Option<PwmSample> {
        let mut last = None;
        for n in 0..cycles {
            let rise = n * period_us;
            if let Some(s) = tracker.observe(1, rise) {
                last = Some(s);
            }
            tracker.observe(0, rise + high_us);
        }
        last
    }

    #[test]
    fn pwm_tracker_detects_stable_1khz_50pct() {
        let mut t = PwmTracker::default();
        let sample = feed_square_wave(&mut t, 1000, 500, 5).expect("should yield a sample");
        assert_eq!(sample.duty, 128, "500/1000 us → analogWrite 值域下 50% = 128");
        assert_eq!(sample.freq_hz, 1000);
        assert!(sample.stable, "5 个等周期后应判定 stable");
    }

    #[test]
    fn pwm_tracker_single_toggle_is_not_stable() {
        let mut t = PwmTracker::default();
        // 只有一个上升沿 + 一个下降沿:凑不出完整周期,无采样
        assert!(t.observe(1, 0).is_none());
        assert!(t.observe(0, 500).is_none());
        // 第二个上升沿收口第一个周期:有采样但数据不足,不能算稳定
        let s = t.observe(1, 1000).expect("first full period yields a sample");
        assert!(!s.stable);
    }

    #[test]
    fn pwm_tracker_irregular_intervals_not_recognized() {
        let mut t = PwmTracker::default();
        // 周期 1000 → 3000 → 700 → 2100 us,相邻偏差远超 5%
        let mut last = None;
        for (rise, fall) in [(0u64, 500), (1000, 2500), (4000, 4300), (4700, 6000)] {
            if let Some(s) = t.observe(1, rise) {
                last = Some(s);
            }
            t.observe(0, fall);
        }
        if let Some(s) = t.observe(1, 6800) {
            last = Some(s);
        }
        let s = last.expect("samples are produced per period");
        assert!(!s.stable, "不规则间隔不能判定为 PWM");
    }

    #[test]
    fn pwm_tracker_ignores_repeated_levels() {
        let mut t = PwmTracker::default();
        assert!(t.observe(1, 0).is_none());
        assert!(t.observe(1, 100).is_none(), "重复电平不是边沿");
        t.observe(0, 500);
        let s = t.observe(1, 1000).unwrap();
        assert_eq!(s.freq_hz, 1000, "重复电平不得干扰周期计算");
    }

    #[test]
    fn apply_event_pin_feeds_pwm_and_get_pwm_expires() {
        let state = Arc::new(Mutex::new(RunState::default()));
        // 4 个完整周期的 1kHz 50% 方波(D9 = PORTB bit 1)
        for n in 0u64..4 {
            let rise = format!(
                r#"{{"event":"pin","t_us":{},"port":"B","bit":1,"value":1}}"#,
                n * 1000
            );
            let fall = format!(
                r#"{{"event":"pin","t_us":{},"port":"B","bit":1,"value":0}}"#,
                n * 1000 + 500
            );
            apply_event(&state, serde_json::from_str(&rise).unwrap(), &|_, _| false);
            apply_event(&state, serde_json::from_str(&fall).unwrap(), &|_, _| false);
        }
        {
            let s = state.lock().unwrap();
            let sample = s.get_pwm('B', 1).expect("fresh sample available");
            assert_eq!(sample.duty, 128);
            assert!(sample.stable);
            assert!(s.get_pwm('B', 2).is_none(), "没有事件的引脚无 PWM");
        }
        // 波形停止:另一引脚把仿真时间推进 3 个周期以上 → 样本过期
        let ev = r#"{"event":"pin","t_us":60000,"port":"D","bit":2,"value":1}"#;
        apply_event(&state, serde_json::from_str(ev).unwrap(), &|_, _| false);
        assert!(
            state.lock().unwrap().get_pwm('B', 1).is_none(),
            "超过 3 个周期无新边沿的样本应过期"
        );
    }

    #[test]
    fn apply_event_hello_records_protocol_and_capabilities() {
        let state = Arc::new(Mutex::new(RunState::default()));
        let ev: BridgeEvent = serde_json::from_str(
            r#"{"event":"hello","protocol":"1","capabilities":["adc","serial"]}"#,
        )
        .expect("hello event should deserialize from real bridge JSON");
        apply_event(&state, ev, &|_, _| false);
        let s = state.lock().unwrap();
        assert_eq!(s.bridge_protocol.as_deref(), Some("1"));
        assert_eq!(s.bridge_capabilities, vec!["adc", "serial"]);
    }

    #[test]
    fn apply_event_adc_updates_channel_value() {
        let state = Arc::new(Mutex::new(RunState::default()));
        let ev: BridgeEvent = serde_json::from_str(
            r#"{"event":"adc","t_us":777,"channel":0,"value":512}"#,
        )
        .expect("adc event should deserialize from real bridge JSON");
        apply_event(&state, ev, &|_, _| false);
        let s = state.lock().unwrap();
        assert_eq!(s.adc_values.get(&0), Some(&512));
        assert_eq!(s.last_event_t_us, 777);
        assert!(!s.adc_values.contains_key(&1));
    }

    #[test]
    fn apply_event_adc_clamps_to_10bit() {
        let state = Arc::new(Mutex::new(RunState::default()));
        let ev: BridgeEvent = serde_json::from_str(
            r#"{"event":"adc","t_us":1,"channel":3,"value":40000}"#,
        )
        .unwrap();
        apply_event(&state, ev, &|_, _| false);
        assert_eq!(state.lock().unwrap().adc_values.get(&3), Some(&1023));
    }

    #[test]
    fn old_bridge_without_hello_has_no_capabilities() {
        // 老 bridge 只发 ready:capabilities 保持空 → set_adc 会拒绝
        let state = Arc::new(Mutex::new(RunState::default()));
        let ev: BridgeEvent = serde_json::from_str(
            r#"{"event":"ready","mcu":"atmega328p","freq":16000000}"#,
        )
        .unwrap();
        apply_event(&state, ev, &|_, _| false);
        let s = state.lock().unwrap();
        assert!(s.ready);
        assert!(s.bridge_protocol.is_none());
        assert!(s.bridge_capabilities.is_empty());
    }

    #[test]
    fn apply_event_dht_updates_env() {
        let state = Arc::new(Mutex::new(RunState::default()));
        let ev: BridgeEvent = serde_json::from_str(
            r#"{"event":"dht","t_us":42,"temp":31,"hum":75}"#,
        )
        .expect("dht event should deserialize from real bridge JSON");
        apply_event(&state, ev, &|_, _| false);
        let s = state.lock().unwrap();
        assert_eq!(s.dht_env, Some((31, 75)));
        assert_eq!(s.last_event_t_us, 42);
    }

    #[test]
    fn dirty_flag_starts_set_and_reset_after_event() {
        // 初值 true:启动后第一轮轮询要落初始快照
        let state = Arc::new(Mutex::new(RunState::default()));
        assert!(state.lock().unwrap().dirty);

        // 写盘方清零后,新事件必须重新置脏
        state.lock().unwrap().dirty = false;
        let ev: BridgeEvent = serde_json::from_str(
            r#"{"event":"pin","t_us":100,"port":"B","bit":5,"value":1}"#,
        )
        .unwrap();
        apply_event(&state, ev, &|_, _| false);
        assert!(state.lock().unwrap().dirty);
    }

    #[test]
    fn emit_json_line_writes_line_with_trailing_newline() {
        let mut buf: Vec<u8> = Vec::new();
        let line = r#"{"event":"ready","mcu":"atmega328p","freq":16000000}"#;
        emit_json_line(&mut buf, line);
        assert_eq!(buf, format!("{line}\n").into_bytes());
    }
}
