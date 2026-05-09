use crate::boards::spec::{ArtifactKind, BoardSpec, PinSpec};
use crate::project::{CodeMeta, Project, ProjectMeta, SCHEMA_VERSION};
use crate::sim::RunningSim;
use anyhow::{Result, bail};
use std::path::Path;

pub static GD32VF103_SPEC: BoardSpec = BoardSpec {
    board_id: "gd32vf103",
    display_name: "GD32VF103 (RISC-V)",
    mcu: "GD32VF103CBT6",
    clock_hz: 108_000_000,
    voltage_mv: 3300,
    artifact_kind: ArtifactKind::Elf,
    pins: &[
        PinSpec { name: "PA8", aliases: &["pa8"], is_d13_led: true },
        PinSpec { name: "GND", aliases: &["gnd"], is_d13_led: false },
        PinSpec { name: "3V3", aliases: &["3v3", "vcc"], is_d13_led: false },
    ],
    serial_count: 3,
    gpio_count: 37,
};

pub struct Gd32vf103;

impl super::BoardImpl for Gd32vf103 {
    fn spec(&self) -> &'static super::spec::BoardSpec { &GD32VF103_SPEC }
    fn scaffold_project(&self, name: &str) -> Project {
        Project {
            project: ProjectMeta { name: name.to_string(), board: "gd32vf103".to_string(), version: SCHEMA_VERSION.to_string() },
            components: vec![],
            wires: vec![],
            code: Some(CodeMeta { src: "src/main.c".to_string(), flags: vec![] }),
        }
    }
    fn source_template(&self) -> &'static str {
        "// GD32VF103 blink template\n// Toolchain not yet implemented — see docs/sprints/v2b.md\nint main(void) { for(;;) {} }\n"
    }
    fn build(&self, _root: &Path) -> Result<(std::path::PathBuf, String)> {
        bail!("GD32VF103 toolchain not yet implemented — riscv32-unknown-elf-gcc required (v2c sprint)")
    }
    fn spawn_sim(&self, _root: &Path, _artifact: &Path) -> Result<RunningSim> {
        bail!("GD32VF103 simulator not yet implemented (v2c sprint)")
    }
}
