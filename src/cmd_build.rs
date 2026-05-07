use crate::project::Project;
use anyhow::{Context, Result, anyhow, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

const FQBN: &str = "arduino:avr:uno";

pub fn cmd_build(root: &Path) -> Result<(PathBuf, String)> {
    let project_path = root.join("moxin.toml");
    let project = Project::load(&project_path)?;

    let src_rel = project
        .code
        .as_ref()
        .map(|c| c.src.clone())
        .unwrap_or_else(|| "src/main.ino".to_string());
    let src_abs = root.join(&src_rel);
    if !src_abs.exists() {
        bail!("source file not found: {}", src_abs.display());
    }

    ensure_arduino_cli()?;
    ensure_avr_core()?;

    // arduino-cli 要求 sketch 目录名 == .ino 文件名,这里搭一个临时 sketch
    // build/sketch/sketch.ino,编译输出落到 build/sketch/build,再把 hex 拷成 build/blink.hex
    let build_dir = root.join("build");
    std::fs::create_dir_all(&build_dir).context("mkdir build")?;
    let sketch_dir = build_dir.join("sketch");
    if sketch_dir.exists() {
        std::fs::remove_dir_all(&sketch_dir).ok();
    }
    std::fs::create_dir_all(&sketch_dir).context("mkdir build/sketch")?;
    std::fs::copy(&src_abs, sketch_dir.join("sketch.ino"))
        .context("copy main.ino → build/sketch/sketch.ino")?;

    let out_dir = sketch_dir.join("out");
    let out = Command::new("arduino-cli")
        .arg("compile")
        .arg("--fqbn")
        .arg(FQBN)
        .arg("--output-dir")
        .arg(&out_dir)
        .arg(&sketch_dir)
        .output()
        .context("invoke arduino-cli compile")?;
    if !out.status.success() {
        // 把 arduino-cli stderr 也带回来给上层显示,便于诊断
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("arduino-cli compile failed:\n{}", stderr.trim_end());
    }

    let produced_hex = out_dir.join("sketch.ino.hex");
    if !produced_hex.exists() {
        bail!(
            "expected hex not produced at {}",
            produced_hex.display()
        );
    }
    let target_name = format!("{}.hex", project.project.name);
    let target_hex = build_dir.join(&target_name);
    std::fs::copy(&produced_hex, &target_hex).context("copy hex to build/")?;
    let prog_bytes = ihex_program_size(&target_hex).unwrap_or_else(|_| {
        std::fs::metadata(&target_hex).map(|m| m.len()).unwrap_or(0)
    });

    // 累积返回字符串:arduino-cli stdout 的实质行 + ✓ 摘要
    let mut msg = String::new();
    let stdout_text = String::from_utf8_lossy(&out.stdout);
    for line in stdout_text.lines() {
        let trimmed = line.trim_end();
        if !trimmed.is_empty() {
            msg.push_str(trimmed);
            msg.push('\n');
        }
    }
    msg.push_str(&format!(
        "✓ arduino-cli compile OK → build/{} ({} bytes)",
        target_name, prog_bytes
    ));

    Ok((target_hex, msg))
}

/// 解析 Intel HEX 文件,把所有 data record (type 00) 的字节数加起来
/// 这是真实的程序大小,与 arduino-cli 输出的 "Sketch uses N bytes" 一致
fn ihex_program_size(path: &Path) -> Result<u64> {
    let text = std::fs::read_to_string(path)?;
    let mut total: u64 = 0;
    for line in text.lines() {
        if !line.starts_with(':') || line.len() < 11 {
            continue;
        }
        let count = u8::from_str_radix(&line[1..3], 16).unwrap_or(0);
        let rtype = u8::from_str_radix(&line[7..9], 16).unwrap_or(0xFF);
        if rtype == 0x00 {
            total += count as u64;
        }
    }
    Ok(total)
}

fn ensure_arduino_cli() -> Result<()> {
    let out = Command::new("arduino-cli")
        .arg("version")
        .output()
        .map_err(|e| anyhow!("arduino-cli not found in PATH: {}", e))?;
    if !out.status.success() {
        bail!("arduino-cli version check failed");
    }
    Ok(())
}

fn ensure_avr_core() -> Result<()> {
    // 缓存标记:在用户 home 下放一个 marker,避免每次都列 core
    let marker = dirs_home().join(".moxin_avr_core_ok");
    if marker.exists() {
        return Ok(());
    }
    // 检查已装的 cores
    let out = Command::new("arduino-cli")
        .args(["core", "list"])
        .output()
        .context("arduino-cli core list")?;
    let listed = String::from_utf8_lossy(&out.stdout);
    if listed.contains("arduino:avr") {
        let _ = std::fs::write(&marker, b"ok");
        return Ok(());
    }

    eprintln!("(installing arduino:avr core, first run only — this may take a minute)");
    let s1 = Command::new("arduino-cli")
        .args(["core", "update-index"])
        .status()
        .context("core update-index")?;
    if !s1.success() {
        bail!("arduino-cli core update-index failed");
    }
    let s2 = Command::new("arduino-cli")
        .args(["core", "install", "arduino:avr"])
        .status()
        .context("core install arduino:avr")?;
    if !s2.success() {
        bail!("arduino-cli core install arduino:avr failed");
    }
    let _ = std::fs::write(&marker, b"ok");
    Ok(())
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
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
        // 两条 data record (type=00),16B + 12B;一条 EOF (type=01) 应被忽略。
        // checksum 字段实现里没校验,这里随便填。
        let hex = ":10010000214601360121470136007EFE09D2190140\n\
                   :0C0010006162636465666768696071624A\n\
                   :00000001FF\n";
        let mut tmp = NamedTempFile::new().expect("create tempfile");
        tmp.write_all(hex.as_bytes()).expect("write hex bytes");
        let n = ihex_program_size(tmp.path()).expect("parse hex");
        assert_eq!(n, 16 + 12);
    }
}
