use anyhow::Result;
use crate::boards::board_from_str;
use crate::project::Project;

pub fn cmd_status(pin_name: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = Project::find_project_root(&cwd)?;
    let project = Project::load(&root.join("moxin.toml"))?;
    let board = board_from_str(&project.project.board)?;
    let spec = board.spec();

    let pin_spec = spec.find_pin(pin_name)
        .ok_or_else(|| anyhow::anyhow!("unknown pin `{}` on board {}", pin_name, spec.board_id))?;

    let bridge_key = if pin_spec.is_d13_led {
        Some(format!("{}:{}", spec.d13_bridge_port, spec.d13_bridge_bit))
    } else {
        None
    };

    let state_path = root.join("build").join(".moxin-state.json");
    if !state_path.exists() {
        println!("UNKNOWN");
        return Ok(());
    }
    let content = std::fs::read_to_string(&state_path)?;
    let map: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(key) = bridge_key {
        if let Some(pins) = map.get("pin_states").and_then(|v| v.as_object()) {
            if let Some(val) = pins.get(&key).and_then(|v| v.as_u64()) {
                println!("{}", if val != 0 { "HIGH" } else { "LOW" });
                return Ok(());
            }
        }
    }
    println!("UNKNOWN");
    Ok(())
}
