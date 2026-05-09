use crate::project::Project;
use crate::sim::{RunningSim, find_bridge_stm32, spawn_bridge_child, spawn_with_state};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET_FLAGS: &[&str] = &[
    "-mthumb", "-mcpu=cortex-m4", "-mfloat-abi=soft", "-Os",
    "-ffreestanding", "-nostartfiles", "-nostdlib", "-Wall", "-Wextra",
];

pub struct Stm32f405;

impl super::BoardImpl for Stm32f405 {
    fn board_name(&self) -> &'static str { "stm32" }
    fn voltage_mv(&self) -> u32 { 3300 }

    fn build(&self, root: &Path) -> Result<(PathBuf, String)> {
        let project = Project::load(&root.join("moxin.toml"))?;
        let src_rel = project.code.as_ref().map(|c| c.src.clone())
            .unwrap_or_else(|| "src/main.c".to_string());
        let src_abs = root.join(&src_rel);
        if !src_abs.exists() {
            bail!("source file not found: {}", src_abs.display());
        }

        ensure_arm_gcc()?;

        let support = find_support_dir()?;
        let startup = support.join("startup.s");
        let linker = support.join("linker.ld");
        if !startup.exists() || !linker.exists() {
            bail!("stm32 support files missing under {} — expected startup.s + linker.ld", support.display());
        }

        let build_dir = root.join("build");
        std::fs::create_dir_all(&build_dir).context("mkdir build")?;
        let target_name = format!("{}.elf", project.project.name);
        let target_elf = build_dir.join(&target_name);

        let mut cmd = Command::new("arm-none-eabi-gcc");
        cmd.args(TARGET_FLAGS)
            .arg(format!("-T{}", linker.display()))
            .arg(&startup).arg(&src_abs)
            .arg("-o").arg(&target_elf);
        let out = cmd.output().context("invoke arm-none-eabi-gcc")?;
        if !out.status.success() {
            bail!("arm-none-eabi-gcc compile failed:\n{}", String::from_utf8_lossy(&out.stderr).trim_end());
        }

        let size = std::fs::metadata(&target_elf).map(|m| m.len()).unwrap_or(0);
        let mut msg = String::new();
        for line in String::from_utf8_lossy(&out.stderr).lines() {
            let t = line.trim_end();
            if !t.is_empty() { msg.push_str(t); msg.push('\n'); }
        }
        msg.push_str(&format!("✓ arm-none-eabi-gcc compile OK → build/{} ({} bytes ELF)", target_name, size));
        Ok((target_elf, msg))
    }

    fn spawn_sim(&self, root: &Path, artifact: &Path) -> Result<RunningSim> {
        let bridge = find_bridge_stm32()?;
        if !bridge.exists() {
            bail!("stm32 bridge not found at {} — set $MOXIN_BRIDGE_STM32 or `make` in bridge/stm32/", bridge.display());
        }
        if !artifact.exists() {
            bail!("elf not found: {} — run `build` first", artifact.display());
        }
        let child = spawn_bridge_child(&bridge, &[artifact], root)?;
        spawn_with_state(child, self.voltage_mv(), Box::new(|port, bit| port == "GPIO" && bit == 13))
    }
}

fn ensure_arm_gcc() -> Result<()> {
    let out = Command::new("arm-none-eabi-gcc").arg("--version").output()
        .map_err(|e| anyhow::anyhow!(
            "arm-none-eabi-gcc not found in PATH: {} — try `brew install --cask gcc-arm-embedded`", e
        ))?;
    if !out.status.success() { bail!("arm-none-eabi-gcc --version exited non-zero"); }
    Ok(())
}

fn find_support_dir() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("MOXIN_STM32_SUPPORT") {
        return Ok(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("examples").join("stm32-blink").join("support");
            if candidate.exists() { return Ok(candidate); }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    Ok(PathBuf::from(home).join("projects/moxin-demo/examples/stm32-blink/support"))
}
