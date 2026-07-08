use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::Project;
use crate::sim::{LedLevel, PwmSample, RunState};
use ratatui::style::Color;

/// 低于这个频率的方波按普通 GPIO 翻转(blink)对待,不显示为 PWM。
/// analogWrite 最低 490Hz、tone() 最低 31Hz,20Hz 以下只可能是 delay 循环。
pub const PWM_DISPLAY_MIN_FREQ_HZ: u32 = 20;

pub fn pin_label_padded(pin: &PinRef) -> String {
    match pin {
        PinRef::BoardDigital(n) => format!("{:<4}", format!("D{}", n)),
        PinRef::BoardAnalog(n) => format!("{:<4}", format!("A{}", n)),
        PinRef::BoardGnd => "GND ".to_string(),
        PinRef::Board5V => "5V  ".to_string(),
        PinRef::BoardPort { port, pin } => format!("{:<4}", format!("{}{}", port, pin)),
        PinRef::Component { .. } => "?   ".to_string(),
    }
}

pub fn pin_label_short(pin: &PinRef) -> String {
    match pin {
        PinRef::BoardDigital(n) => format!("D{}", n),
        PinRef::BoardAnalog(n) => format!("A{}", n),
        PinRef::BoardGnd => "GND".to_string(),
        PinRef::Board5V => "5V".to_string(),
        PinRef::BoardPort { port, pin } => format!("{}{}", port, pin),
        PinRef::Component { .. } => "?".to_string(),
    }
}

pub fn pin_level(pin: &PinRef, state: &RunState, spec: &BoardSpec) -> LedLevel {
    if spec.is_d13_pin(pin) {
        return state.d13;
    }
    let level = match pin {
        PinRef::BoardDigital(n) => state.get_arduino_digital(*n),
        PinRef::BoardAnalog(n) => state.get_arduino_analog(*n),
        _ => None,
    };
    match level {
        Some(true) => LedLevel::On,
        _ => LedLevel::Off,
    }
}

/// 引脚上仍在持续的、稳定且频率达标的 PWM 采样;
/// 不稳定 / 已停止(过期)/ 慢速 blink → `None`,渲染回退 ON/OFF。
pub fn pin_pwm(pin: &PinRef, state: &RunState) -> Option<PwmSample> {
    let (port, bit) = match pin {
        PinRef::BoardDigital(n) => RunState::arduino_digital_to_port_bit(*n)?,
        PinRef::BoardAnalog(n) => RunState::arduino_analog_to_port_bit(*n)?,
        _ => return None,
    };
    state
        .get_pwm(port, bit)
        .filter(|s| s.stable && s.freq_hz >= PWM_DISPLAY_MIN_FREQ_HZ)
}

/// 引脚是否支持硬件 PWM(analogWrite)。LED 调光显示只对这些引脚开启。
pub fn pin_is_pwm_capable(pin: &PinRef, spec: &BoardSpec) -> bool {
    matches!(pin, PinRef::BoardDigital(n) if spec.pwm_pins.contains(n))
}

/// duty(0..=255,analogWrite 值域)→ 百分比 0..=100,四舍五入。
pub fn duty_percent(duty: u8) -> u32 {
    (duty as u32 * 100 + 127) / 255
}

/// PWM 频率显示:`"980Hz"` / `"31.4kHz"`。
pub fn format_freq(freq_hz: u32) -> String {
    if freq_hz >= 10_000 {
        format!("{:.1}kHz", freq_hz as f64 / 1000.0)
    } else {
        format!("{}Hz", freq_hz)
    }
}

/// 扫 wires,返回连到 `<comp_id>.<terminal>` 的所有 (terminal, 板侧 pin)。
/// 多端子元件(RGB LED / 电机 / 七段)用它按端子名取各自的信号。
pub fn component_terminal_pins(comp_id: &str, project: &Project) -> Vec<(String, PinRef)> {
    let mut out = Vec::new();
    for w in &project.wires {
        let from = PinRef::parse(&w.from).ok();
        let to = PinRef::parse(&w.to).ok();
        let (pin, terminal) = match (from, to) {
            (Some(PinRef::Component { id, terminal }), Some(p))
                if id == comp_id && !matches!(p, PinRef::Component { .. }) =>
            {
                (p, terminal)
            }
            (Some(p), Some(PinRef::Component { id, terminal }))
                if id == comp_id && !matches!(p, PinRef::Component { .. }) =>
            {
                (p, terminal)
            }
            _ => continue,
        };
        out.push((terminal, pin));
    }
    out
}

/// 数字/PWM 混合驱动电平:PWM 引脚上有稳定波形 → duty(0..=255);
/// 否则按数字电平折算 0 / 255。RGB 调色、电机调速共用。
pub fn pin_drive_level(pin: &PinRef, state: &RunState, spec: &BoardSpec) -> u8 {
    if pin_is_pwm_capable(pin, spec) {
        if let Some(s) = pin_pwm(pin, state) {
            return s.duty;
        }
    }
    match pin_level(pin, state, spec) {
        LedLevel::On => 255,
        LedLevel::Off => 0,
    }
}

pub fn led_color(name: &str) -> Color {
    match name {
        "red" => Color::Rgb(255, 40, 40),
        "green" => Color::Rgb(40, 220, 80),
        "blue" => Color::Rgb(60, 120, 255),
        "yellow" => Color::Rgb(255, 200, 40),
        "white" => Color::Rgb(240, 240, 240),
        _ => Color::Rgb(240, 240, 240),
    }
}

pub fn format_resistance(ohms: u32) -> String {
    if ohms >= 1_000_000 {
        let m = ohms as f64 / 1_000_000.0;
        if (m - m.round()).abs() < 0.01 {
            format!("{}MΩ", m as u32)
        } else {
            format!("{:.1}MΩ", m)
        }
    } else if ohms >= 1_000 {
        let k = ohms as f64 / 1_000.0;
        if (k - k.round()).abs() < 0.01 {
            format!("{}kΩ", k as u32)
        } else {
            format!("{:.1}kΩ", k)
        }
    } else {
        format!("{}Ω", ohms)
    }
}

pub fn resistance_color_rings(ohms: u32) -> [Color; 4] {
    let digit_color = |d: u8| -> Color {
        match d {
            0 => Color::Black,
            1 => Color::Rgb(139, 69, 19),
            2 => Color::Rgb(255, 40, 40),
            3 => Color::Rgb(255, 165, 0),
            4 => Color::Rgb(255, 200, 40),
            5 => Color::Rgb(40, 220, 80),
            6 => Color::Rgb(60, 120, 255),
            7 => Color::Rgb(148, 0, 211),
            8 => Color::Rgb(128, 128, 128),
            9 => Color::Rgb(240, 240, 240),
            _ => Color::Black,
        }
    };
    if ohms == 0 {
        return [Color::Black, Color::Black, Color::Black, Color::Rgb(218, 165, 32)];
    }
    let mut val = ohms;
    let mut mult: u8 = 0;
    while val >= 100 {
        val /= 10;
        mult += 1;
    }
    let d1 = (val / 10) as u8;
    let d2 = (val % 10) as u8;
    [digit_color(d1), digit_color(d2), digit_color(mult), Color::Rgb(218, 165, 32)]
}

/// 解析电阻值字符串:`"470"` / `"10k"` / `"4.7k"` / `"1m"`。
/// 零或负值 → error(电阻必须 >0)。从 shell.rs 迁移过来。
pub fn parse_resistance(s: &str) -> anyhow::Result<u32> {
    let s = s.trim().to_lowercase();
    let (num_part, multiplier) = if let Some(n) = s.strip_suffix('m') {
        (n, 1_000_000u64)
    } else if let Some(n) = s.strip_suffix('k') {
        (n, 1_000u64)
    } else {
        (s.as_str(), 1u64)
    };
    let val: f64 = num_part
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid resistance value: {}", s))?;
    let ohms = (val * multiplier as f64) as u32;
    if ohms == 0 {
        anyhow::bail!("resistance must be > 0");
    }
    Ok(ohms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resistance_color_rings_470() {
        let rings = resistance_color_rings(470);
        assert_eq!(rings[0], Color::Rgb(255, 200, 40));
        assert_eq!(rings[1], Color::Rgb(148, 0, 211));
        assert_eq!(rings[2], Color::Rgb(139, 69, 19));
        assert_eq!(rings[3], Color::Rgb(218, 165, 32));
    }

    #[test]
    fn resistance_color_rings_10k() {
        let rings = resistance_color_rings(10_000);
        assert_eq!(rings[0], Color::Rgb(139, 69, 19));
        assert_eq!(rings[1], Color::Black);
        assert_eq!(rings[2], Color::Rgb(255, 165, 0));
    }

    #[test]
    fn resistance_color_rings_1m() {
        let rings = resistance_color_rings(1_000_000);
        assert_eq!(rings[0], Color::Rgb(139, 69, 19));
        assert_eq!(rings[1], Color::Black);
        assert_eq!(rings[2], Color::Rgb(40, 220, 80));
    }

    #[test]
    fn resistance_color_rings_zero() {
        let rings = resistance_color_rings(0);
        assert_eq!(rings[0], Color::Black);
        assert_eq!(rings[1], Color::Black);
        assert_eq!(rings[2], Color::Black);
    }

    #[test]
    fn format_resistance_values() {
        assert_eq!(format_resistance(470), "470Ω");
        assert_eq!(format_resistance(1_000), "1kΩ");
        assert_eq!(format_resistance(10_000), "10kΩ");
        assert_eq!(format_resistance(4_700), "4.7kΩ");
        assert_eq!(format_resistance(1_000_000), "1MΩ");
        assert_eq!(format_resistance(2_200_000), "2.2MΩ");
    }
}
