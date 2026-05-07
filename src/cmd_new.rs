use crate::project::Project;
use anyhow::{Context, Result, bail};
use std::path::Path;

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

pub fn cmd_new(name: &str) -> Result<()> {
    let dir = Path::new(name);
    if dir.exists() {
        bail!("./{} already exists", name);
    }
    std::fs::create_dir_all(dir.join("src"))
        .with_context(|| format!("create {}/src", name))?;

    let project = Project::new_blink(name);
    project.save(&dir.join("moxin.toml"))?;

    std::fs::write(dir.join("src").join("main.ino"), BLINK_INO_TEMPLATE)
        .context("write src/main.ino")?;

    println!("✓ created ./{}", name);
    Ok(())
}
