use crate::boards::spec::{ArtifactKind, BoardSpec, PinSpec};
use crate::project::{CodeMeta, Project, ProjectMeta, SCHEMA_VERSION};
use crate::sim::RunningSim;
use anyhow::Result;
use std::path::{Path, PathBuf};

use super::arduino_uno::{avr_build, avr_spawn_sim, BLINK_INO_TEMPLATE};

const FQBN: &str = "arduino:avr:nano";

/// Arduino Nano:与 Uno 同 ATmega328P/16MHz,复用同一个 simavr bridge。
/// 差异:板型丝印、A6/A7 两个 ADC-only 引脚(无数字功能,GPIO 事件不覆盖)。
pub static ARDUINO_NANO_SPEC: BoardSpec = BoardSpec {
    board_id: "arduino-nano",
    display_name: "Arduino Nano",
    mcu: "ATmega328P",
    clock_hz: 16_000_000,
    voltage_mv: 5000,
    artifact_kind: ArtifactKind::Hex,
    pins: &[
        PinSpec { name: "D0",  aliases: &["pin0",  "0",  "d0"],  is_d13_led: false },
        PinSpec { name: "D1",  aliases: &["pin1",  "1",  "d1"],  is_d13_led: false },
        PinSpec { name: "D2",  aliases: &["pin2",  "2",  "d2"],  is_d13_led: false },
        PinSpec { name: "D3",  aliases: &["pin3",  "3",  "d3"],  is_d13_led: false },
        PinSpec { name: "D4",  aliases: &["pin4",  "4",  "d4"],  is_d13_led: false },
        PinSpec { name: "D5",  aliases: &["pin5",  "5",  "d5"],  is_d13_led: false },
        PinSpec { name: "D6",  aliases: &["pin6",  "6",  "d6"],  is_d13_led: false },
        PinSpec { name: "D7",  aliases: &["pin7",  "7",  "d7"],  is_d13_led: false },
        PinSpec { name: "D8",  aliases: &["pin8",  "8",  "d8"],  is_d13_led: false },
        PinSpec { name: "D9",  aliases: &["pin9",  "9",  "d9"],  is_d13_led: false },
        PinSpec { name: "D10", aliases: &["pin10", "10", "d10"], is_d13_led: false },
        PinSpec { name: "D11", aliases: &["pin11", "11", "d11"], is_d13_led: false },
        PinSpec { name: "D12", aliases: &["pin12", "12", "d12"], is_d13_led: false },
        PinSpec { name: "D13", aliases: &["pin13", "13", "d13"], is_d13_led: true  },
        PinSpec { name: "A0",  aliases: &["a0"],                 is_d13_led: false },
        PinSpec { name: "A1",  aliases: &["a1"],                 is_d13_led: false },
        PinSpec { name: "A2",  aliases: &["a2"],                 is_d13_led: false },
        PinSpec { name: "A3",  aliases: &["a3"],                 is_d13_led: false },
        PinSpec { name: "A4",  aliases: &["a4"],                 is_d13_led: false },
        PinSpec { name: "A5",  aliases: &["a5"],                 is_d13_led: false },
        // Nano 独有:A6/A7 是 ADC-only(无 GPIO),digitalRead/pin 事件不覆盖
        PinSpec { name: "A6",  aliases: &["a6"],                 is_d13_led: false },
        PinSpec { name: "A7",  aliases: &["a7"],                 is_d13_led: false },
        PinSpec { name: "GND", aliases: &["gnd"],                is_d13_led: false },
        PinSpec { name: "5V",  aliases: &["5v", "vcc"],          is_d13_led: false },
    ],
    serial_count: 1,
    gpio_count: 14,
    d13_bridge_port: "B",
    d13_bridge_bit: 5,
    pwm_pins: &[3, 5, 6, 9, 10, 11],
    adc_channels: &[
        (0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5),
        (6, 6), (7, 7), // A6/A7 = ADC6/ADC7
    ],
};

pub struct ArduinoNano;

impl super::BoardImpl for ArduinoNano {
    fn spec(&self) -> &'static super::spec::BoardSpec { &ARDUINO_NANO_SPEC }

    fn scaffold_project(&self, name: &str) -> Project {
        Project {
            project: ProjectMeta { name: name.to_string(), board: "arduino-nano".to_string(), version: SCHEMA_VERSION.to_string() },
            components: vec![],
            wires: vec![],
            code: Some(CodeMeta { src: "src/main.ino".to_string(), flags: vec![] }),
        }
    }

    fn source_template(&self) -> &'static str { BLINK_INO_TEMPLATE }

    fn build(&self, root: &Path) -> Result<(PathBuf, String)> {
        avr_build(root, FQBN)
    }

    fn spawn_sim(&self, root: &Path, artifact: &Path, json_out: bool) -> Result<RunningSim> {
        avr_spawn_sim(root, artifact, self.spec(), json_out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nano_spec_metadata() {
        assert_eq!(ARDUINO_NANO_SPEC.board_id, "arduino-nano");
        assert_eq!(ARDUINO_NANO_SPEC.mcu, "ATmega328P");
        assert_eq!(ARDUINO_NANO_SPEC.clock_hz, 16_000_000);
        assert_eq!(ARDUINO_NANO_SPEC.d13_bridge_port, "B");
        assert_eq!(ARDUINO_NANO_SPEC.d13_bridge_bit, 5);
    }

    #[test]
    fn nano_has_a6_a7_adc_only_pins() {
        assert!(ARDUINO_NANO_SPEC.find_pin("A6").is_some());
        assert!(ARDUINO_NANO_SPEC.find_pin("A7").is_some());
        assert_eq!(ARDUINO_NANO_SPEC.adc_channel_for(6), Some(6));
        assert_eq!(ARDUINO_NANO_SPEC.adc_channel_for(7), Some(7));
        // Uno 没有 A6/A7
        let uno = &super::super::arduino_uno::ARDUINO_UNO_SPEC;
        assert!(uno.find_pin("A6").is_none());
        assert_eq!(uno.adc_channel_for(6), None);
    }

    #[test]
    fn nano_pwm_pins_match_uno() {
        assert_eq!(
            ARDUINO_NANO_SPEC.pwm_pins,
            super::super::arduino_uno::ARDUINO_UNO_SPEC.pwm_pins
        );
    }
}
