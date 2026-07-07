use crate::boards::spec::{ArtifactKind, BoardSpec, PinSpec};
use crate::project::{CodeMeta, Project, ProjectMeta, SCHEMA_VERSION};
use crate::sim::{RunningSim, find_bridge_avr, spawn_bridge_child, spawn_with_state};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

const FQBN: &str = "arduino:avr:uno";

pub static ARDUINO_UNO_SPEC: BoardSpec = BoardSpec {
    board_id: "arduino-uno",
    display_name: "Arduino Uno",
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
        PinSpec { name: "GND", aliases: &["gnd"],                is_d13_led: false },
        PinSpec { name: "5V",  aliases: &["5v", "vcc"],          is_d13_led: false },
    ],
    serial_count: 1,
    gpio_count: 14,
    d13_bridge_port: "B",
    d13_bridge_bit: 5,
    pwm_pins: &[3, 5, 6, 9, 10, 11],
};

pub const BLINK_INO_TEMPLATE: &str = r#"// 由 moxin new 自动生成
// 经典 blink:D13 每 1000 ms 翻转一次
const int LED_PIN = 13;

void setup() {
  pinMode(LED_PIN, OUTPUT);
}

void loop() {
  digitalWrite(LED_PIN, HIGH);
  delay(1000);
  digitalWrite(LED_PIN, LOW);
  delay(1000);
}
"#;

pub struct ArduinoUno;

impl super::BoardImpl for ArduinoUno {
    fn spec(&self) -> &'static super::spec::BoardSpec { &ARDUINO_UNO_SPEC }
    fn scaffold_project(&self, name: &str) -> Project {
        Project {
            project: ProjectMeta { name: name.to_string(), board: "arduino-uno".to_string(), version: SCHEMA_VERSION.to_string() },
            components: vec![],
            wires: vec![],
            code: Some(CodeMeta { src: "src/main.ino".to_string(), flags: vec![] }),
        }
    }
    fn source_template(&self) -> &'static str { BLINK_INO_TEMPLATE }

    fn build(&self, root: &Path) -> Result<(PathBuf, String)> {
        let project = Project::load(&root.join("moxin.toml"))?;
        let src_rel = project.code.as_ref().map(|c| c.src.clone())
            .unwrap_or_else(|| "src/main.ino".to_string());
        let src_abs = root.join(&src_rel);
        if !src_abs.exists() {
            bail!("source file not found: {}", src_abs.display());
        }

        ensure_arduino_cli()?;
        ensure_avr_core()?;

        let build_dir = root.join("build");
        std::fs::create_dir_all(&build_dir).context("mkdir build")?;
        let sketch_dir = build_dir.join("sketch");
        if sketch_dir.exists() { std::fs::remove_dir_all(&sketch_dir).ok(); }
        std::fs::create_dir_all(&sketch_dir).context("mkdir build/sketch")?;
        std::fs::copy(&src_abs, sketch_dir.join("sketch.ino"))
            .context("copy main.ino → build/sketch/sketch.ino")?;

        let out_dir = sketch_dir.join("out");
        let out = Command::new("arduino-cli")
            .arg("compile").arg("--fqbn").arg(FQBN)
            .arg("--output-dir").arg(&out_dir).arg(&sketch_dir)
            .output().context("invoke arduino-cli compile")?;
        if !out.status.success() {
            bail!("arduino-cli compile failed:\n{}", String::from_utf8_lossy(&out.stderr).trim_end());
        }

        let produced_hex = out_dir.join("sketch.ino.hex");
        if !produced_hex.exists() {
            bail!("expected hex not produced at {}", produced_hex.display());
        }
        let target_name = format!("{}.hex", project.project.name);
        let target_hex = build_dir.join(&target_name);
        std::fs::copy(&produced_hex, &target_hex).context("copy hex to build/")?;
        let prog_bytes = ihex_program_size(&target_hex)
            .unwrap_or_else(|_| std::fs::metadata(&target_hex).map(|m| m.len()).unwrap_or(0));

        let mut msg = String::new();
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let t = line.trim_end();
            if !t.is_empty() { msg.push_str(t); msg.push('\n'); }
        }
        msg.push_str(&format!("✓ arduino-cli compile OK → build/{} ({} bytes)", target_name, prog_bytes));
        Ok((target_hex, msg))
    }

    fn spawn_sim(&self, root: &Path, artifact: &Path, json_out: bool) -> Result<RunningSim> {
        let bridge = find_bridge_avr()?;
        if !bridge.exists() {
            bail!("simavr bridge not found at {} — set $MOXIN_BRIDGE or `make` in bridge/", bridge.display());
        }
        if !artifact.exists() {
            bail!("hex not found: {} — run `build` first", artifact.display());
        }
        let child = spawn_bridge_child(&bridge, &[artifact, Path::new("atmega328p"), Path::new("16000000")], root)?;
        spawn_with_state(child, self.voltage_mv(), self.spec().make_is_d13(), json_out)
    }
}

fn ihex_program_size(path: &Path) -> Result<u64> {
    let text = std::fs::read_to_string(path)?;
    let mut total: u64 = 0;
    for line in text.lines() {
        if !line.starts_with(':') || line.len() < 11 { continue; }
        let count = u8::from_str_radix(&line[1..3], 16)
            .with_context(|| format!("invalid ihex byte count in line: {}", line))?;
        let rtype = u8::from_str_radix(&line[7..9], 16)
            .with_context(|| format!("invalid ihex record type in line: {}", line))?;
        if rtype == 0x00 { total += count as u64; }
    }
    Ok(total)
}

fn ensure_arduino_cli() -> Result<()> {
    let out = Command::new("arduino-cli").arg("version").output()
        .map_err(|e| anyhow::anyhow!("arduino-cli not found in PATH: {}", e))?;
    if !out.status.success() { bail!("arduino-cli version check failed"); }
    Ok(())
}

fn ensure_avr_core() -> Result<()> {
    let marker = dirs_home().join(".moxin_avr_core_ok");
    if marker.exists() { return Ok(()); }
    let out = Command::new("arduino-cli").args(["core", "list"]).output()
        .context("arduino-cli core list")?;
    if String::from_utf8_lossy(&out.stdout).contains("arduino:avr") {
        let _ = std::fs::write(&marker, b"ok");
        return Ok(());
    }
    eprintln!("(installing arduino:avr core, first run only — this may take a minute)");
    let s1 = Command::new("arduino-cli").args(["core", "update-index"]).status()
        .context("core update-index")?;
    if !s1.success() { bail!("arduino-cli core update-index failed"); }
    let s2 = Command::new("arduino-cli").args(["core", "install", "arduino:avr"]).status()
        .context("core install arduino:avr")?;
    if !s2.success() { bail!("arduino-cli core install arduino:avr failed"); }
    let _ = std::fs::write(&marker, b"ok");
    Ok(())
}

fn dirs_home() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_ihex_program_size_counts_data_records() {
        let hex = ":10010000214601360121470136007EFE09D2190140\n\
                   :0C0010006162636465666768696071624A\n\
                   :00000001FF\n";
        let mut tmp = NamedTempFile::new().expect("create tempfile");
        tmp.write_all(hex.as_bytes()).expect("write hex bytes");
        let n = ihex_program_size(tmp.path()).expect("parse hex");
        assert_eq!(n, 16 + 12);
    }

    #[test]
    fn uno_pwm_pins_match_atmega328p_timers() {
        // analogWrite 可用引脚 = Timer0/1/2 的 OC 输出
        assert_eq!(ARDUINO_UNO_SPEC.pwm_pins, &[3, 5, 6, 9, 10, 11]);
        // 每个 pwm 引脚必须真实存在于 pins 表里
        for n in ARDUINO_UNO_SPEC.pwm_pins {
            assert!(ARDUINO_UNO_SPEC.find_pin(&format!("D{}", n)).is_some());
        }
        // D13(板载 LED / blink)不是 PWM 引脚,慢速 blink 不能被当成调光
        assert!(!ARDUINO_UNO_SPEC.pwm_pins.contains(&13));
    }
}
