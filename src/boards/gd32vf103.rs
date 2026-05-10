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
        PinSpec { name: "PA0",  aliases: &["pa0"],  is_d13_led: false },
        PinSpec { name: "PA1",  aliases: &["pa1"],  is_d13_led: false },
        PinSpec { name: "PA2",  aliases: &["pa2"],  is_d13_led: false },
        PinSpec { name: "PA3",  aliases: &["pa3"],  is_d13_led: false },
        PinSpec { name: "PA4",  aliases: &["pa4"],  is_d13_led: false },
        PinSpec { name: "PA5",  aliases: &["pa5"],  is_d13_led: false },
        PinSpec { name: "PA6",  aliases: &["pa6"],  is_d13_led: false },
        PinSpec { name: "PA7",  aliases: &["pa7"],  is_d13_led: false },
        PinSpec { name: "PA8",  aliases: &["pa8"],  is_d13_led: true  },
        PinSpec { name: "PA9",  aliases: &["pa9"],  is_d13_led: false },
        PinSpec { name: "PA10", aliases: &["pa10"], is_d13_led: false },
        PinSpec { name: "PA11", aliases: &["pa11"], is_d13_led: false },
        PinSpec { name: "PA12", aliases: &["pa12"], is_d13_led: false },
        PinSpec { name: "PA13", aliases: &["pa13"], is_d13_led: false },
        PinSpec { name: "PA14", aliases: &["pa14"], is_d13_led: false },
        PinSpec { name: "PA15", aliases: &["pa15"], is_d13_led: false },
        PinSpec { name: "PB0",  aliases: &["pb0"],  is_d13_led: false },
        PinSpec { name: "PB1",  aliases: &["pb1"],  is_d13_led: false },
        PinSpec { name: "PB2",  aliases: &["pb2"],  is_d13_led: false },
        PinSpec { name: "PB3",  aliases: &["pb3"],  is_d13_led: false },
        PinSpec { name: "PB4",  aliases: &["pb4"],  is_d13_led: false },
        PinSpec { name: "PB5",  aliases: &["pb5"],  is_d13_led: false },
        PinSpec { name: "PB6",  aliases: &["pb6"],  is_d13_led: false },
        PinSpec { name: "PB7",  aliases: &["pb7"],  is_d13_led: false },
        PinSpec { name: "PB8",  aliases: &["pb8"],  is_d13_led: false },
        PinSpec { name: "PB9",  aliases: &["pb9"],  is_d13_led: false },
        PinSpec { name: "PB10", aliases: &["pb10"], is_d13_led: false },
        PinSpec { name: "PB11", aliases: &["pb11"], is_d13_led: false },
        PinSpec { name: "PB12", aliases: &["pb12"], is_d13_led: false },
        PinSpec { name: "PB13", aliases: &["pb13"], is_d13_led: false },
        PinSpec { name: "PB14", aliases: &["pb14"], is_d13_led: false },
        PinSpec { name: "PB15", aliases: &["pb15"], is_d13_led: false },
        PinSpec { name: "PC0",  aliases: &["pc0"],  is_d13_led: false },
        PinSpec { name: "PC1",  aliases: &["pc1"],  is_d13_led: false },
        PinSpec { name: "PC2",  aliases: &["pc2"],  is_d13_led: false },
        PinSpec { name: "PC3",  aliases: &["pc3"],  is_d13_led: false },
        PinSpec { name: "PC4",  aliases: &["pc4"],  is_d13_led: false },
        PinSpec { name: "PC5",  aliases: &["pc5"],  is_d13_led: false },
        PinSpec { name: "PC6",  aliases: &["pc6"],  is_d13_led: false },
        PinSpec { name: "PC7",  aliases: &["pc7"],  is_d13_led: false },
        PinSpec { name: "PC8",  aliases: &["pc8"],  is_d13_led: false },
        PinSpec { name: "PC9",  aliases: &["pc9"],  is_d13_led: false },
        PinSpec { name: "PC10", aliases: &["pc10"], is_d13_led: false },
        PinSpec { name: "PC11", aliases: &["pc11"], is_d13_led: false },
        PinSpec { name: "PC12", aliases: &["pc12"], is_d13_led: false },
        PinSpec { name: "PC13", aliases: &["pc13"], is_d13_led: false },
        PinSpec { name: "GND",  aliases: &["gnd"],  is_d13_led: false },
        PinSpec { name: "3V3",  aliases: &["3v3", "vcc"], is_d13_led: false },
    ],
    serial_count: 3,
    gpio_count: 37,
    d13_bridge_port: "GPIO",
    d13_bridge_bit: 8,
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
