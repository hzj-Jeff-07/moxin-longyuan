mod arduino_uno;
mod gd32vf103;
mod spec;
mod stm32f405;

pub use spec::{BoardSpec, PinSpec};

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

use crate::project::Project;
use crate::sim::RunningSim;

pub trait BoardImpl {
    fn spec(&self) -> &'static BoardSpec;
    fn board_name(&self) -> &'static str { self.spec().board_id }
    fn voltage_mv(&self) -> u32 { self.spec().voltage_mv }
    fn artifact_ext(&self) -> &'static str { self.spec().artifact_ext() }
    fn scaffold_project(&self, name: &str) -> Project;
    fn source_template(&self) -> &'static str;
    fn build(&self, root: &Path) -> Result<(PathBuf, String)>;
    fn spawn_sim(&self, root: &Path, artifact: &Path, json_out: bool) -> Result<RunningSim>;
}

pub fn board_from_str(s: &str) -> Result<Box<dyn BoardImpl>> {
    match s {
        "arduino-uno" | "uno" => Ok(Box::new(arduino_uno::ArduinoUno)),
        "stm32" | "stm32f405" => Ok(Box::new(stm32f405::Stm32f405)),
        "gd32vf103" | "gd32" => Ok(Box::new(gd32vf103::Gd32vf103)),
        other => bail!("unsupported board `{}` — supported: arduino-uno, stm32, gd32vf103", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::PinRef;
    use std::path::Path;

    #[test]
    fn board_from_str_arduino_uno() {
        assert!(board_from_str("arduino-uno").is_ok());
        assert!(board_from_str("uno").is_ok());
    }

    #[test]
    fn board_from_str_stm32() {
        assert!(board_from_str("stm32").is_ok());
        assert!(board_from_str("stm32f405").is_ok());
    }

    #[test]
    fn board_from_str_gd32() {
        assert!(board_from_str("gd32vf103").is_ok());
        assert!(board_from_str("gd32").is_ok());
    }

    #[test]
    fn board_from_str_unknown_errors() {
        assert!(board_from_str("esp32").is_err());
    }

    #[test]
    fn arduino_uno_artifact_ext_is_hex() {
        let b = board_from_str("arduino-uno").unwrap();
        assert_eq!(b.artifact_ext(), "hex");
    }

    #[test]
    fn stm32_artifact_ext_is_elf() {
        let b = board_from_str("stm32").unwrap();
        assert_eq!(b.artifact_ext(), "elf");
    }

    #[test]
    fn artifact_ext_from_spec() {
        let uno = board_from_str("uno").unwrap();
        assert_eq!(uno.spec().artifact_ext(), uno.artifact_ext());
        let stm = board_from_str("stm32").unwrap();
        assert_eq!(stm.spec().artifact_ext(), stm.artifact_ext());
    }

    #[test]
    fn arduino_uno_spec_metadata() {
        let b = board_from_str("arduino-uno").unwrap();
        let s = b.spec();
        assert_eq!(s.board_id, "arduino-uno");
        assert_eq!(s.mcu, "ATmega328P");
        assert_eq!(s.voltage_mv, 5000);
    }

    #[test]
    fn stm32f405_spec_metadata() {
        let b = board_from_str("stm32").unwrap();
        let s = b.spec();
        assert_eq!(s.board_id, "stm32");
        assert_eq!(s.mcu, "STM32F405RG");
        assert_eq!(s.voltage_mv, 3300);
    }

    #[test]
    fn gd32vf103_spec_metadata() {
        let b = board_from_str("gd32vf103").unwrap();
        let s = b.spec();
        assert_eq!(s.board_id, "gd32vf103");
        assert_eq!(s.mcu, "GD32VF103CBT6");
        assert_eq!(s.clock_hz, 108_000_000);
    }

    #[test]
    fn gd32_build_returns_friendly_error() {
        let b = board_from_str("gd32vf103").unwrap();
        let err = b.build(Path::new("/tmp")).unwrap_err();
        assert!(err.to_string().contains("not yet implemented"));
    }

    // --- v2b new tests ---

    #[test]
    fn arduino_d13_spec_is_d13_led() {
        let spec = board_from_str("arduino-uno").unwrap();
        let pin = spec.spec().find_pin("D13").unwrap();
        assert!(pin.is_d13_led);
    }

    #[test]
    fn arduino_d12_spec_not_d13_led() {
        let spec = board_from_str("arduino-uno").unwrap();
        let pin = spec.spec().find_pin("D12").unwrap();
        assert!(!pin.is_d13_led);
    }

    #[test]
    fn stm32_pa13_spec_is_d13_led() {
        let spec = board_from_str("stm32").unwrap();
        let pin = spec.spec().find_pin("PA13").unwrap();
        assert!(pin.is_d13_led);
    }

    #[test]
    fn arduino_spec_has_all_digital_pins() {
        let spec = board_from_str("arduino-uno").unwrap();
        let s = spec.spec();
        for n in 0u8..=13 {
            assert!(s.find_pin(&format!("D{}", n)).is_some(), "D{} missing", n);
        }
    }

    #[test]
    fn arduino_spec_has_all_analog_pins() {
        let spec = board_from_str("arduino-uno").unwrap();
        let s = spec.spec();
        for n in 0u8..=5 {
            assert!(s.find_pin(&format!("A{}", n)).is_some(), "A{} missing", n);
        }
    }

    #[test]
    fn pin_ref_valid_rejects_d99_arduino() {
        let spec = board_from_str("arduino-uno").unwrap();
        assert!(!spec.spec().pin_ref_valid(&PinRef::BoardDigital(99)));
    }

    #[test]
    fn pin_ref_valid_rejects_d99_stm32() {
        let spec = board_from_str("stm32").unwrap();
        assert!(!spec.spec().pin_ref_valid(&PinRef::BoardDigital(99)));
    }

    #[test]
    fn board_info_arduino_contains_display_name() {
        let b = board_from_str("arduino-uno").unwrap();
        assert!(b.spec().board_info_string().contains("Arduino Uno"));
    }

    #[test]
    fn board_info_stm32_contains_display_name() {
        let b = board_from_str("stm32").unwrap();
        assert!(b.spec().board_info_string().contains("STM32F405"));
    }

    #[test]
    fn board_info_gd32_contains_display_name() {
        let b = board_from_str("gd32vf103").unwrap();
        assert!(b.spec().board_info_string().contains("GD32VF103"));
    }

    #[test]
    fn make_is_d13_arduino_matches_bridge_port_bit() {
        let b = board_from_str("arduino-uno").unwrap();
        let f = b.spec().make_is_d13();
        assert!(f("B", 5));
        assert!(!f("B", 4));
        assert!(!f("C", 5));
    }

    #[test]
    fn make_is_d13_stm32_matches_bridge_port_bit() {
        let b = board_from_str("stm32").unwrap();
        let f = b.spec().make_is_d13();
        assert!(f("GPIO", 13));
        assert!(!f("GPIO", 5));
        assert!(!f("PA", 13));
    }
}
