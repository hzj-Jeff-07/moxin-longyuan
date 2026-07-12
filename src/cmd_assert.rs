use anyhow::{Result, bail};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use crate::boards::{board_from_str, BoardSpec};
use crate::project::Project;
use crate::sim::RunState;

/// 把引脚名映射到 bridge 事件 key(`"D:7"` / `"B:5"` / `"GPIO:13"`)。
/// 返回 `None` = 该引脚在这块板上不可观测。
///
/// - 板载 D13 LED:用 spec 声明的 d13 bridge 映射(所有板通用)。
/// - Arduino 系(uno/nano):Phase 2-mini 起 bridge hook 全部 PORTB/C/D,
///   所以任意 D0-D13 / A0-A5 都可观测。
/// - STM32 / gd32:bridge 只上报 D13(GPIO:13),其它引脚不可观测。
fn bridge_key_for(spec: &BoardSpec, pin_name: &str) -> Option<String> {
    let p = spec.find_pin(pin_name)?; // 解析别名;未知引脚 → None
    if p.is_d13_led {
        return Some(format!("{}:{}", spec.d13_bridge_port, spec.d13_bridge_bit));
    }
    if spec.board_id.starts_with("arduino") {
        // 用规范名(D7 / A0)推导 port:bit,与 apply_event 写入的 key 一致
        if let Some(rest) = p.name.strip_prefix('D') {
            if let Ok(n) = rest.parse::<u8>() {
                if let Some((port, bit)) = RunState::arduino_digital_to_port_bit(n) {
                    return Some(format!("{}:{}", port, bit));
                }
            }
        } else if let Some(rest) = p.name.strip_prefix('A') {
            if let Ok(n) = rest.parse::<u8>() {
                if let Some((port, bit)) = RunState::arduino_analog_to_port_bit(n) {
                    return Some(format!("{}:{}", port, bit));
                }
            }
        }
    }
    None
}

/// 三种断言模式（A+B 轻量并集）：
///   moxin assert --pin D13 --eq HIGH --after 1s
///   moxin assert --pin D13 --toggles --within 3s
///   moxin assert --serial-contains "hello" --within 2s
///
/// 退出码：0=pass / 1=fail / 2=timeout（命令本身错误走 anyhow → 非 0/1/2）
pub fn cmd_assert(
    pin: Option<String>,
    eq: Option<String>,
    after: Option<String>,
    toggles: bool,
    within: Option<String>,
    serial_contains: Option<String>,
    send: Option<String>,
) -> Result<ExitCode> {
    let mode = AssertMode::resolve(&pin, &eq, &after, toggles, &within, &serial_contains)?;

    let cwd = std::env::current_dir()?;
    let root = Project::find_project_root(&cwd)?;
    let project = Project::load(&root.join("moxin.toml"))?;
    let board = board_from_str(&project.project.board)?;
    let spec = board.spec();
    let ext = board.artifact_ext();
    let artifact = root.join("build").join(format!("{}.{}", project.project.name, ext));
    if !artifact.exists() {
        bail!("artifact not found at {} — run `build` first", artifact.display());
    }

    // 全部断言模式都需要让 simulator 跑起来观测事件，与 Run 命令相同的启动姿势。
    let mut sim = board.spawn_sim(&root, &artifact, false)?;
    crate::sim::configure_peripherals(&mut sim, &project, spec)?;

    // --send:注入串口 RX 后再断言(测串口输入 → 输出的闭环)。给固件一点 setup 时间。
    if let Some(text) = send.as_deref() {
        std::thread::sleep(Duration::from_millis(300));
        sim.send_serial(text)?;
    }

    let result = match mode {
        AssertMode::PinEq { pin_name, expected, after_d } => {
            let bridge_key = bridge_key_for(spec, &pin_name)
                .ok_or_else(|| anyhow::anyhow!(
                    "pin `{}` is not observable on board {} (Arduino: any D0-D13/A0-A5; STM32/gd32: only the D13 LED)",
                    pin_name, spec.board_id
                ))?;
            // 等 after_d，然后读一次 pin 状态做相等判断
            std::thread::sleep(after_d);
            let level = read_pin(&sim, &bridge_key);
            match (level, expected) {
                (Some(true), PinLevel::High) | (Some(false), PinLevel::Low) => AssertResult::Pass,
                (Some(_), _) => AssertResult::Fail,
                (None, _) => AssertResult::Timeout,
            }
        }
        AssertMode::PinToggles { pin_name, within: w } => {
            let bridge_key = bridge_key_for(spec, &pin_name)
                .ok_or_else(|| anyhow::anyhow!(
                    "pin `{}` is not observable on board {} (Arduino: any D0-D13/A0-A5; STM32/gd32: only the D13 LED)",
                    pin_name, spec.board_id
                ))?;
            wait_toggle(&mut sim, &bridge_key, w)
        }
        AssertMode::SerialContains { needle, within: w } => {
            wait_serial(&mut sim, &needle, w)
        }
    };

    sim.stop();

    Ok(match result {
        AssertResult::Pass => {
            println!("PASS");
            ExitCode::from(0)
        }
        AssertResult::Fail => {
            println!("FAIL");
            ExitCode::from(1)
        }
        AssertResult::Timeout => {
            println!("TIMEOUT");
            ExitCode::from(2)
        }
    })
}

#[derive(Debug, PartialEq, Eq)]
enum AssertResult { Pass, Fail, Timeout }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PinLevel { High, Low }

#[derive(Debug)]
enum AssertMode {
    PinEq { pin_name: String, expected: PinLevel, after_d: Duration },
    PinToggles { pin_name: String, within: Duration },
    SerialContains { needle: String, within: Duration },
}

impl AssertMode {
    fn resolve(
        pin: &Option<String>,
        eq: &Option<String>,
        after: &Option<String>,
        toggles: bool,
        within: &Option<String>,
        serial_contains: &Option<String>,
    ) -> Result<AssertMode> {
        // 互斥：--serial-contains 不能和 --pin 同时出现
        if serial_contains.is_some() && pin.is_some() {
            bail!("--serial-contains 与 --pin 不能同时使用");
        }
        if let Some(needle) = serial_contains.clone() {
            if toggles || eq.is_some() || after.is_some() {
                bail!("--serial-contains 不接受 --eq / --after / --toggles");
            }
            let w = parse_duration(within.as_deref().unwrap_or("2s"))?;
            return Ok(AssertMode::SerialContains { needle, within: w });
        }

        let pin_name = pin.clone()
            .ok_or_else(|| anyhow::anyhow!("缺少 --pin 或 --serial-contains"))?;

        if toggles {
            if eq.is_some() || after.is_some() {
                bail!("--toggles 不接受 --eq / --after");
            }
            let w = parse_duration(within.as_deref().unwrap_or("3s"))?;
            return Ok(AssertMode::PinToggles { pin_name, within: w });
        }

        let expected_str = eq.clone()
            .ok_or_else(|| anyhow::anyhow!("缺少 --eq <HIGH|LOW> 或 --toggles"))?;
        let expected = match expected_str.to_uppercase().as_str() {
            "HIGH" | "1" => PinLevel::High,
            "LOW" | "0" => PinLevel::Low,
            other => bail!("--eq 仅接受 HIGH/LOW（收到 `{}`）", other),
        };
        let after_d = parse_duration(after.as_deref().unwrap_or("1s"))?;
        Ok(AssertMode::PinEq { pin_name, expected, after_d })
    }
}

/// 解析 "500ms" / "2s" / "1m" 这种紧凑时长。
fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() { bail!("空时长字符串"); }
    let (num_part, unit) = if let Some(rest) = s.strip_suffix("ms") {
        (rest, "ms")
    } else if let Some(rest) = s.strip_suffix('s') {
        (rest, "s")
    } else if let Some(rest) = s.strip_suffix('m') {
        (rest, "m")
    } else {
        bail!("不支持的时长单位 `{}`（仅支持 ms/s/m）", s);
    };
    let n: u64 = num_part.parse()
        .map_err(|_| anyhow::anyhow!("时长数值无法解析 `{}`", s))?;
    Ok(match unit {
        "ms" => Duration::from_millis(n),
        "s" => Duration::from_secs(n),
        "m" => Duration::from_secs(n * 60),
        _ => unreachable!(),
    })
}

fn read_pin(sim: &crate::sim::RunningSim, bridge_key: &str) -> Option<bool> {
    let s = sim.state.lock().ok()?;
    s.pin_states.get(bridge_key).map(|v| *v != 0)
}

/// 在 `within` 时间内观察到至少一次电平翻转（值从 v 变为 !v）即 PASS。
fn wait_toggle(sim: &mut crate::sim::RunningSim, bridge_key: &str, within: Duration) -> AssertResult {
    let deadline = Instant::now() + within;
    // 初始读一次（可能尚未 ready）
    let mut last: Option<u8> = sim.state.lock().ok()
        .and_then(|s| s.pin_states.get(bridge_key).copied());
    while Instant::now() < deadline {
        if !sim.is_alive() { break; }
        if let Ok(s) = sim.state.lock() {
            if let Some(&cur) = s.pin_states.get(bridge_key) {
                match last {
                    Some(prev) if prev != cur => return AssertResult::Pass,
                    None => { last = Some(cur); }
                    _ => {}
                }
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    AssertResult::Timeout
}

/// 在 `within` 时间内 serial_lines 中出现包含 `needle` 的一行即 PASS。
fn wait_serial(sim: &mut crate::sim::RunningSim, needle: &str, within: Duration) -> AssertResult {
    let deadline = Instant::now() + within;
    while Instant::now() < deadline {
        if !sim.is_alive() { break; }
        if let Ok(s) = sim.state.lock() {
            if s.serial_lines.iter().any(|(_, line)| line.contains(needle)) {
                return AssertResult::Pass;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    AssertResult::Timeout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_ms() {
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
    }

    #[test]
    fn bridge_key_arduino_all_gpio() {
        let spec = board_from_str("arduino-uno").unwrap();
        let s = spec.spec();
        // D13 走 spec 的 d13 映射(PORTB bit 5)
        assert_eq!(bridge_key_for(s, "D13").as_deref(), Some("B:5"));
        // 任意数字引脚:D0-D7 → PORTD,D8-D12 → PORTB
        assert_eq!(bridge_key_for(s, "D7").as_deref(), Some("D:7"));
        assert_eq!(bridge_key_for(s, "D2").as_deref(), Some("D:2"));
        assert_eq!(bridge_key_for(s, "D8").as_deref(), Some("B:0"));
        // 模拟引脚按 PORTC 数字视角
        assert_eq!(bridge_key_for(s, "A0").as_deref(), Some("C:0"));
        assert_eq!(bridge_key_for(s, "A5").as_deref(), Some("C:5"));
        // 别名也解析
        assert_eq!(bridge_key_for(s, "pin7").as_deref(), Some("D:7"));
        // 电源轨不可观测
        assert!(bridge_key_for(s, "GND").is_none());
        // 未知引脚
        assert!(bridge_key_for(s, "D99").is_none());
    }

    #[test]
    fn bridge_key_nano_matches_uno() {
        let spec = board_from_str("arduino-nano").unwrap();
        assert_eq!(bridge_key_for(spec.spec(), "D7").as_deref(), Some("D:7"));
    }

    #[test]
    fn bridge_key_stm32_only_d13() {
        let spec = board_from_str("stm32").unwrap();
        let s = spec.spec();
        // PA13 是板载 LED → GPIO:13
        assert_eq!(bridge_key_for(s, "PA13").as_deref(), Some("GPIO:13"));
        // 其它 STM32 引脚不可观测(bridge 只上报 D13)
        assert!(bridge_key_for(s, "PA0").is_none());
    }

    #[test]
    fn parse_duration_s() {
        assert_eq!(parse_duration("3s").unwrap(), Duration::from_secs(3));
    }

    #[test]
    fn parse_duration_m() {
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
    }

    #[test]
    fn parse_duration_rejects_bad_unit() {
        assert!(parse_duration("3h").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn parse_duration_rejects_non_numeric() {
        assert!(parse_duration("abcs").is_err());
    }

    #[test]
    fn resolve_pin_eq_high_with_defaults() {
        let m = AssertMode::resolve(
            &Some("D13".into()), &Some("HIGH".into()), &None,
            false, &None, &None,
        ).unwrap();
        match m {
            AssertMode::PinEq { pin_name, expected, after_d } => {
                assert_eq!(pin_name, "D13");
                assert_eq!(expected, PinLevel::High);
                assert_eq!(after_d, Duration::from_secs(1));
            }
            _ => panic!("expected PinEq"),
        }
    }

    #[test]
    fn resolve_pin_eq_low_lowercase() {
        let m = AssertMode::resolve(
            &Some("D13".into()), &Some("low".into()), &Some("250ms".into()),
            false, &None, &None,
        ).unwrap();
        match m {
            AssertMode::PinEq { expected, after_d, .. } => {
                assert_eq!(expected, PinLevel::Low);
                assert_eq!(after_d, Duration::from_millis(250));
            }
            _ => panic!("expected PinEq"),
        }
    }

    #[test]
    fn resolve_pin_toggles_default_within_3s() {
        let m = AssertMode::resolve(
            &Some("D13".into()), &None, &None,
            true, &None, &None,
        ).unwrap();
        match m {
            AssertMode::PinToggles { pin_name, within } => {
                assert_eq!(pin_name, "D13");
                assert_eq!(within, Duration::from_secs(3));
            }
            _ => panic!("expected PinToggles"),
        }
    }

    #[test]
    fn resolve_serial_contains_default_within_2s() {
        let m = AssertMode::resolve(
            &None, &None, &None,
            false, &None, &Some("hello".into()),
        ).unwrap();
        match m {
            AssertMode::SerialContains { needle, within } => {
                assert_eq!(needle, "hello");
                assert_eq!(within, Duration::from_secs(2));
            }
            _ => panic!("expected SerialContains"),
        }
    }

    #[test]
    fn resolve_rejects_pin_and_serial_together() {
        let r = AssertMode::resolve(
            &Some("D13".into()), &None, &None,
            false, &None, &Some("hi".into()),
        );
        assert!(r.is_err());
    }

    #[test]
    fn resolve_rejects_toggles_with_eq() {
        let r = AssertMode::resolve(
            &Some("D13".into()), &Some("HIGH".into()), &None,
            true, &None, &None,
        );
        assert!(r.is_err());
    }

    #[test]
    fn resolve_rejects_eq_garbage() {
        let r = AssertMode::resolve(
            &Some("D13".into()), &Some("MAYBE".into()), &None,
            false, &None, &None,
        );
        assert!(r.is_err());
    }

    #[test]
    fn resolve_rejects_missing_eq_and_toggles() {
        let r = AssertMode::resolve(
            &Some("D13".into()), &None, &None,
            false, &None, &None,
        );
        assert!(r.is_err());
    }

    #[test]
    fn resolve_rejects_no_pin_no_serial() {
        let r = AssertMode::resolve(
            &None, &None, &None,
            false, &None, &None,
        );
        assert!(r.is_err());
    }
}
