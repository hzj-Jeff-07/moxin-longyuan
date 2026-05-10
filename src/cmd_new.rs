use anyhow::{Context, Result, bail};
use std::path::Path;

pub fn cmd_new(name: &str, board: &str) -> Result<()> {
    let dir = Path::new(name);
    if dir.exists() {
        bail!("./{} already exists", name);
    }

    let board_impl = crate::boards::board_from_str(board)?;
    let project = board_impl.scaffold_project(name);
    std::fs::create_dir_all(dir.join("src"))
        .with_context(|| format!("create {}/src", name))?;
    project.save(&dir.join("moxin.toml"))?;

    let template = board_impl.source_template();
    let src_filename = project.code.as_ref()
        .map(|c| c.src.clone())
        .unwrap_or_else(|| "src/main.c".into());
    std::fs::write(dir.join(&src_filename), template)
        .with_context(|| format!("write {}", src_filename))?;

    println!("✓ created ./{} (board={})", name, board_impl.board_name());
    Ok(())
}
