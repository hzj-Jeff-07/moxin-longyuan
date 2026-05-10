use anyhow::{bail, Result};
use std::path::PathBuf;

pub fn cmd_install() -> Result<()> {
    let src = std::env::current_exe()?;

    let dest_dir = if let Ok(home) = std::env::var("USERPROFILE") {
        PathBuf::from(home).join(".cargo").join("bin")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cargo").join("bin")
    } else {
        bail!("cannot determine home directory (USERPROFILE / HOME not set)");
    };

    if !dest_dir.exists() {
        bail!("destination {} does not exist — is Rust installed?", dest_dir.display());
    }

    let exe_name = if cfg!(windows) { "moxin.exe" } else { "moxin" };
    let dest = dest_dir.join(exe_name);

    std::fs::copy(&src, &dest)?;
    println!("✓ installed to {}", dest.display());
    println!("  open a new terminal and run: moxin --help");
    Ok(())
}
