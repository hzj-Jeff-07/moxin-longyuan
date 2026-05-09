mod arduino_uno;
mod stm32f405;

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

use crate::sim::RunningSim;

pub trait BoardImpl {
    fn board_name(&self) -> &'static str;
    fn voltage_mv(&self) -> u32;
    fn build(&self, root: &Path) -> Result<(PathBuf, String)>;
    fn spawn_sim(&self, root: &Path, artifact: &Path) -> Result<RunningSim>;
}

pub fn board_from_str(s: &str) -> Result<Box<dyn BoardImpl>> {
    match s {
        "arduino-uno" => Ok(Box::new(arduino_uno::ArduinoUno)),
        "stm32" => Ok(Box::new(stm32f405::Stm32f405)),
        other => bail!("unsupported board `{}` — supported: arduino-uno, stm32", other),
    }
}
