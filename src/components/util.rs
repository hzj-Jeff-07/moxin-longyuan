use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::sim::{LedLevel, RunState};
use ratatui::style::Color;

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
