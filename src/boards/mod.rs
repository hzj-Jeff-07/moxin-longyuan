mod arduino_uno;
mod stm32f405;

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

use crate::project::Project;
use crate::sim::RunningSim;

pub trait BoardImpl {
    fn board_name(&self) -> &'static str;
    fn voltage_mv(&self) -> u32;
    fn artifact_ext(&self) -> &'static str;
    fn scaffold_project(&self, name: &str) -> Project;
    fn source_template(&self) -> &'static str;
    fn build(&self, root: &Path) -> Result<(PathBuf, String)>;
    fn spawn_sim(&self, root: &Path, artifact: &Path) -> Result<RunningSim>;
}

pub fn board_from_str(s: &str) -> Result<Box<dyn BoardImpl>> {
    match s {
        "arduino-uno" | "uno" => Ok(Box::new(arduino_uno::ArduinoUno)),
        "stm32" | "stm32f405" => Ok(Box::new(stm32f405::Stm32f405)),
        other => bail!("unsupported board `{}` — supported: arduino-uno, stm32", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
