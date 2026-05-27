use anyhow::{Result, anyhow};
use crate::boards::{BoardSpec, PinSpec, board_from_str};
use crate::project::Project;
use crate::sim::RunState;

pub fn cmd_status(pin_name: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = Project::find_project_root(&cwd)?;
    let project = Project::load(&root.join("moxin.toml"))?;
    let board = board_from_str(&project.project.board)?;
    let spec = board.spec();

    let state_path = root.join("build").join(".moxin-state.json");
    let state_value: Option<serde_json::Value> = if state_path.exists() {
        let content = std::fs::read_to_string(&state_path)?;
        Some(serde_json::from_str(&content)?)
    } else {
        None
    };
    let pin_states = state_value
        .as_ref()
        .and_then(|v| v.get("pin_states"))
        .and_then(|v| v.as_object());

    let status = resolve_pin_status(pin_name, spec, pin_states)?;
    println!("{}", status);
    Ok(())
}

/// Resolve a pin name to "HIGH" / "LOW" / "UNKNOWN" against persisted state.
///
/// Phase 2-mini Step 4:全 D0-D13 / A0-A5 都能查,不再只懂 D13。
/// 解析顺序:
/// 1. `find_pin` 校验存在(不存在 → error)
/// 2. GND → "LOW",5V → "HIGH"(电源轨常量,永远不是 UNKNOWN)
/// 3. 映射到 bridge key 后查 `pin_states`(没有事件 / 没有 state 文件 → UNKNOWN)
/// 4. 映射不到 bridge key(STM32 BoardPort 等 → UNKNOWN,不退化)
fn resolve_pin_status(
    pin_name: &str,
    spec: &BoardSpec,
    pin_states: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Result<&'static str> {
    let pin_spec = spec.find_pin(pin_name).ok_or_else(|| {
        anyhow!("unknown pin `{}` on board {}", pin_name, spec.board_id)
    })?;

    if pin_spec.name == "GND" {
        return Ok("LOW");
    }
    if pin_spec.name == "5V" {
        return Ok("HIGH");
    }

    let Some(bridge_key) = pin_bridge_key(pin_spec, spec) else {
        return Ok("UNKNOWN");
    };
    let Some(pins) = pin_states else {
        return Ok("UNKNOWN");
    };
    let Some(val) = pins.get(&bridge_key).and_then(|v| v.as_u64()) else {
        return Ok("UNKNOWN");
    };
    Ok(if val != 0 { "HIGH" } else { "LOW" })
}

/// 把 PinSpec 映射到 bridge `pin_states` 的 key(形如 `"B:5"`)。
///
/// - D13 LED:走 `spec.d13_bridge_port`/`bit`(stm32 是 `"GPIO:13"`,arduino 是 `"B:5"`)
/// - Arduino D0-D12:Step 2 的 `arduino_digital_to_port_bit`
/// - Arduino A0-A5:Step 2 的 `arduino_analog_to_port_bit`
/// - STM32 BoardPort / 未来扩展:`None`(调用方 → UNKNOWN,不退化)
fn pin_bridge_key(pin_spec: &PinSpec, spec: &BoardSpec) -> Option<String> {
    if pin_spec.is_d13_led {
        return Some(format!("{}:{}", spec.d13_bridge_port, spec.d13_bridge_bit));
    }
    if let Some(rest) = pin_spec.name.strip_prefix('D') {
        if let Ok(n) = rest.parse::<u8>() {
            return RunState::arduino_digital_to_port_bit(n)
                .map(|(p, b)| format!("{}:{}", p, b));
        }
    }
    if let Some(rest) = pin_spec.name.strip_prefix('A') {
        if let Ok(n) = rest.parse::<u8>() {
            return RunState::arduino_analog_to_port_bit(n)
                .map(|(p, b)| format!("{}:{}", p, b));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn uno_spec() -> &'static BoardSpec {
        // BoardImpl::spec() 返回 &'static BoardSpec(每块板子是 static),
        // 跟 Box 生命周期解耦,所以临时 Box drop 后引用仍然有效。
        board_from_str("arduino-uno").unwrap().spec()
    }

    fn pins(json: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        json.as_object().unwrap().clone()
    }

    #[test]
    fn d13_high_when_b5_is_one() {
        let p = pins(json!({"B:5": 1}));
        assert_eq!(resolve_pin_status("D13", uno_spec(), Some(&p)).unwrap(), "HIGH");
        // alias 也认
        assert_eq!(resolve_pin_status("pin13", uno_spec(), Some(&p)).unwrap(), "HIGH");
    }

    #[test]
    fn d13_low_when_b5_is_zero() {
        let p = pins(json!({"B:5": 0}));
        assert_eq!(resolve_pin_status("D13", uno_spec(), Some(&p)).unwrap(), "LOW");
    }

    #[test]
    fn d7_high_when_d_port_bit7_set() {
        // Step 4 关键回归:非 D13 引脚也能查,不再永远 UNKNOWN
        let p = pins(json!({"D:7": 1}));
        assert_eq!(resolve_pin_status("D7", uno_spec(), Some(&p)).unwrap(), "HIGH");
    }

    #[test]
    fn d2_low_when_d_port_bit2_zero() {
        let p = pins(json!({"D:2": 0}));
        assert_eq!(resolve_pin_status("D2", uno_spec(), Some(&p)).unwrap(), "LOW");
    }

    #[test]
    fn a3_reads_from_port_c() {
        let p = pins(json!({"C:3": 1}));
        assert_eq!(resolve_pin_status("A3", uno_spec(), Some(&p)).unwrap(), "HIGH");
    }

    #[test]
    fn a0_unknown_when_no_event() {
        let p = pins(json!({"D:7": 1}));
        assert_eq!(resolve_pin_status("A0", uno_spec(), Some(&p)).unwrap(), "UNKNOWN");
    }

    #[test]
    fn gnd_is_always_low_even_without_state() {
        assert_eq!(resolve_pin_status("GND", uno_spec(), None).unwrap(), "LOW");
    }

    #[test]
    fn five_v_is_always_high_even_without_state() {
        assert_eq!(resolve_pin_status("5V", uno_spec(), None).unwrap(), "HIGH");
        assert_eq!(resolve_pin_status("vcc", uno_spec(), None).unwrap(), "HIGH");
    }

    #[test]
    fn unknown_pin_errors() {
        let p = pins(json!({}));
        assert!(resolve_pin_status("D99", uno_spec(), Some(&p)).is_err());
        assert!(resolve_pin_status("XYZ", uno_spec(), Some(&p)).is_err());
    }

    #[test]
    fn no_state_file_returns_unknown_for_gpio() {
        // build/.moxin-state.json 不存在 → 所有 GPIO 都是 UNKNOWN(电源轨除外)
        assert_eq!(resolve_pin_status("D7", uno_spec(), None).unwrap(), "UNKNOWN");
        assert_eq!(resolve_pin_status("D13", uno_spec(), None).unwrap(), "UNKNOWN");
        assert_eq!(resolve_pin_status("A0", uno_spec(), None).unwrap(), "UNKNOWN");
    }
}
