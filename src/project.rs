use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub project: ProjectMeta,
    #[serde(rename = "component", default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    #[serde(rename = "wire", default, skip_serializing_if = "Vec::is_empty")]
    pub wires: Vec<Wire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeMeta>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectMeta {
    pub name: String,
    pub board: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Component {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<[i32; 2]>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Wire {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeMeta {
    pub src: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<String>,
}

impl Project {
    pub fn new_blink(name: &str) -> Self {
        Project {
            project: ProjectMeta {
                name: name.to_string(),
                board: "arduino-uno".to_string(),
                version: "0.1".to_string(),
            },
            components: vec![],
            wires: vec![],
            code: Some(CodeMeta {
                src: "src/main.ino".to_string(),
                flags: vec![],
            }),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        let p: Project =
            toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        Ok(p)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).context("serialize project")?;
        std::fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }

    pub fn find_project_root(start: &Path) -> Result<PathBuf> {
        let mut cur = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
        loop {
            if cur.join("moxin.toml").exists() {
                return Ok(cur);
            }
            match cur.parent() {
                Some(p) => cur = p.to_path_buf(),
                None => bail!("not inside a moxin project (no moxin.toml found)"),
            }
        }
    }

    pub fn add_component(&mut self, c: Component) -> Result<()> {
        if self.components.iter().any(|x| x.id == c.id) {
            bail!("component id already exists: {}", c.id);
        }
        self.components.push(c);
        Ok(())
    }

    pub fn add_wire(&mut self, w: Wire) {
        self.wires.push(w);
    }
}
