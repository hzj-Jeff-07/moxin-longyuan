//! STM32F405 (Cortex-M4) 的 build 路径。
//!
//! 跟 `cmd_build`(arduino-cli / avr)是平行 sibling。v2a 阶段不抽 trait,
//! 直接调 `arm-none-eabi-gcc` + 项目里 `src/main.c` + 仓库自带的 startup.s /
//! linker.ld(在 `examples/stm32-blink/support/`)。
//!
//! 产出:`<root>/build/<name>.elf`

use crate::project::Project;
use anyhow::{Context, Result, anyhow, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET_FLAGS: &[&str] = &[
    "-mthumb",
    "-mcpu=cortex-m4",
    "-mfloat-abi=soft",
    "-Os",
    "-ffreestanding",
    "-nostartfiles",
    "-nostdlib",
    "-Wall",
    "-Wextra",
];

pub fn cmd_build_stm32(root: &Path) -> Result<(PathBuf, String)> {
    let project_path = root.join("moxin.toml");
    let project = Project::load(&project_path)?;

    let src_rel = project
        .code
        .as_ref()
        .map(|c| c.src.clone())
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
        bail!(
            "stm32 support files missing under {} — expected startup.s + linker.ld",
            support.display()
        );
    }

    let build_dir = root.join("build");
    std::fs::create_dir_all(&build_dir).context("mkdir build")?;
    let target_name = format!("{}.elf", project.project.name);
    let target_elf = build_dir.join(&target_name);

    let mut cmd = Command::new("arm-none-eabi-gcc");
    cmd.args(TARGET_FLAGS)
        .arg(format!("-T{}", linker.display()))
        .arg(&startup)
        .arg(&src_abs)
        .arg("-o")
        .arg(&target_elf);
    let out = cmd.output().context("invoke arm-none-eabi-gcc")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!(
            "arm-none-eabi-gcc compile failed:\n{}",
            stderr.trim_end()
        );
    }

    let size = std::fs::metadata(&target_elf)
        .map(|m| m.len())
        .unwrap_or(0);

    let mut msg = String::new();
    let stderr_text = String::from_utf8_lossy(&out.stderr);
    for line in stderr_text.lines() {
        let trimmed = line.trim_end();
        if !trimmed.is_empty() {
            msg.push_str(trimmed);
            msg.push('\n');
        }
    }
    msg.push_str(&format!(
        "✓ arm-none-eabi-gcc compile OK → build/{} ({} bytes ELF)",
        target_name, size
    ));

    Ok((target_elf, msg))
}

fn ensure_arm_gcc() -> Result<()> {
    let out = Command::new("arm-none-eabi-gcc")
        .arg("--version")
        .output()
        .map_err(|e| {
            anyhow!(
                "arm-none-eabi-gcc not found in PATH: {} — try `brew install --cask gcc-arm-embedded`",
                e
            )
        })?;
    if !out.status.success() {
        bail!("arm-none-eabi-gcc --version exited non-zero");
    }
    Ok(())
}

/// 找 startup.s / linker.ld 的位置。优先级:
/// 1. $MOXIN_STM32_SUPPORT
/// 2. <exe>/../examples/stm32-blink/support/  (release 安装路径)
/// 3. ~/projects/moxin-demo/examples/stm32-blink/support/  (开发机器约定)
fn find_support_dir() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("MOXIN_STM32_SUPPORT") {
        return Ok(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("examples").join("stm32-blink").join("support");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    Ok(PathBuf::from(home)
        .join("projects/moxin-demo/examples/stm32-blink/support"))
}
