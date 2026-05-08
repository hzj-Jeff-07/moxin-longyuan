//! STM32F405 (netduinoplus2 via QEMU) 的 run 路径。
//!
//! 跟 `cmd_run`(simavr / arduino-uno)的关系:平行 sibling,共享 `RunState` /
//! `RunningSim` / reader 线程。差异只在于:启动哪个 bridge 二进制 + 标称电压。
//!
//! v2a 阶段刻意**不抽 trait**,先把两块板的真实 cmd 实现都写出来,trait 设计
//! 留到 Phase 2(T4–T7)再归纳。

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::cmd_run::{RunningSim, spawn_with_state};

pub fn cmd_run_stm32(root: &Path, elf: &Path) -> Result<RunningSim> {
    let bridge = find_bridge_stm32()?;
    if !bridge.exists() {
        bail!(
            "stm32 bridge not found at {} — set $MOXIN_BRIDGE_STM32 or `make` in bridge/stm32/",
            bridge.display()
        );
    }
    if !elf.exists() {
        bail!("elf not found: {} — run `build` first", elf.display());
    }

    let child = Command::new(&bridge)
        .arg(elf)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(root)
        .spawn()
        .with_context(|| format!("spawn {}", bridge.display()))?;

    // STM32F405 标称 3.3V
    spawn_with_state(child, 3300)
}

fn find_bridge_stm32() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("MOXIN_BRIDGE_STM32") {
        return Ok(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("bridge-stm32");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    Ok(PathBuf::from(home).join("projects/moxin-demo/bridge/stm32/bridge-stm32"))
}
