use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 当前 schema 版本。v2a 引入板抽象,旧的 0.1 项目不再兼容。
pub const SCHEMA_VERSION: &str = "0.2";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Project {
    pub project: ProjectMeta,
    #[serde(rename = "component", default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    #[serde(rename = "wire", default, skip_serializing_if = "Vec::is_empty")]
    pub wires: Vec<Wire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeMeta>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProjectMeta {
    pub name: String,
    pub board: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Component {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<[i32; 2]>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Wire {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CodeMeta {
    pub src: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<String>,
}

impl Project {
    /// 默认 blink 项目 = arduino uno blink。保留为旧调用点兼容入口
    /// (T-1 测试 / cmd_new 默认路径)
    #[allow(dead_code)]
    pub fn new_blink(name: &str) -> Self {
        Self::new_blink_uno(name)
    }

    /// Arduino Uno blink 模板:`src/main.ino`
    pub fn new_blink_uno(name: &str) -> Self {
        Project {
            project: ProjectMeta {
                name: name.to_string(),
                board: "arduino-uno".to_string(),
                version: SCHEMA_VERSION.to_string(),
            },
            components: vec![],
            wires: vec![],
            code: Some(CodeMeta {
                src: "src/main.ino".to_string(),
                flags: vec![],
            }),
        }
    }

    /// STM32F405 (netduinoplus2) blink 模板:`src/main.c`
    pub fn new_blink_stm32(name: &str) -> Self {
        Project {
            project: ProjectMeta {
                name: name.to_string(),
                board: "stm32".to_string(),
                version: SCHEMA_VERSION.to_string(),
            },
            components: vec![],
            wires: vec![],
            code: Some(CodeMeta {
                src: "src/main.c".to_string(),
                flags: vec![],
            }),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        let p: Project =
            toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        if p.project.version != SCHEMA_VERSION {
            bail!(
                "moxin.toml schema version `{}` is not supported in v2a (expected `{}`).\n\
                 v2a 引入了板抽象,旧项目无法直接使用。请用 `moxin new <name> [--board=...]` 重新生成项目,\n\
                 然后把 src/ 下的代码手动迁过去。",
                p.project.version, SCHEMA_VERSION
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn full_project() -> Project {
        Project {
            project: ProjectMeta {
                name: "demo".to_string(),
                board: "arduino-uno".to_string(),
                version: SCHEMA_VERSION.to_string(),
            },
            components: vec![
                Component {
                    id: "led1".to_string(),
                    kind: "led".to_string(),
                    color: Some("red".to_string()),
                    pos: Some([10, 20]),
                },
                Component {
                    id: "btn1".to_string(),
                    kind: "button".to_string(),
                    color: None,
                    pos: None,
                },
            ],
            wires: vec![Wire {
                from: "board.D13".to_string(),
                to: "led1.anode".to_string(),
            }],
            code: Some(CodeMeta {
                src: "src/main.ino".to_string(),
                flags: vec!["-DDEBUG".to_string()],
            }),
        }
    }

    #[test]
    fn new_blink_has_correct_meta() {
        let p = Project::new_blink("blink");
        assert_eq!(p.project.name, "blink");
        assert_eq!(p.project.board, "arduino-uno");
        assert_eq!(p.project.version, SCHEMA_VERSION);
    }

    #[test]
    fn new_blink_has_default_code() {
        let p = Project::new_blink("blink");
        let code = p.code.expect("new_blink should set code");
        assert_eq!(code.src, "src/main.ino");
        assert!(code.flags.is_empty());
    }

    #[test]
    fn new_blink_has_no_components_or_wires() {
        let p = Project::new_blink("blink");
        assert!(p.components.is_empty());
        assert!(p.wires.is_empty());
    }

    #[test]
    fn new_blink_stm32_uses_c_template() {
        let p = Project::new_blink_stm32("blinks");
        assert_eq!(p.project.board, "stm32");
        assert_eq!(p.project.version, SCHEMA_VERSION);
        let code = p.code.expect("stm32 template has code");
        assert_eq!(code.src, "src/main.c");
    }

    #[test]
    fn add_component_accepts_distinct_ids() {
        let mut p = Project::new_blink("blink");
        let a = Component {
            id: "led1".to_string(),
            kind: "led".to_string(),
            color: Some("red".to_string()),
            pos: None,
        };
        let b = Component {
            id: "led2".to_string(),
            kind: "led".to_string(),
            color: Some("green".to_string()),
            pos: None,
        };
        assert!(p.add_component(a).is_ok());
        assert!(p.add_component(b).is_ok());
        assert_eq!(p.components.len(), 2);
    }

    #[test]
    fn add_component_rejects_duplicate_id() {
        let mut p = Project::new_blink("blink");
        let a = Component {
            id: "led1".to_string(),
            kind: "led".to_string(),
            color: None,
            pos: None,
        };
        let b_same_id = Component {
            id: "led1".to_string(),
            kind: "led".to_string(),
            color: Some("blue".to_string()),
            pos: None,
        };
        assert!(p.add_component(a).is_ok());
        let err = p.add_component(b_same_id);
        assert!(err.is_err(), "second push with duplicate id must fail");
        assert_eq!(p.components.len(), 1, "duplicate must not be appended");
    }

    #[test]
    fn save_load_roundtrip_preserves_all_fields() {
        let original = full_project();
        let tmp = NamedTempFile::new().expect("create tempfile");
        original
            .save(tmp.path())
            .expect("save full project to tempfile");
        let loaded = Project::load(tmp.path()).expect("load full project");
        assert_eq!(loaded, original, "round-trip must preserve all fields");
    }

    #[test]
    fn load_rejects_old_schema_version() {
        let toml_text = "[project]\nname = \"old\"\nboard = \"arduino-uno\"\nversion = \"0.1\"\n";
        let tmp = NamedTempFile::new().expect("create tempfile");
        std::fs::write(tmp.path(), toml_text).expect("write old-schema toml");
        let res = Project::load(tmp.path());
        assert!(res.is_err(), "0.1 should be rejected with executable msg");
        let msg = format!("{:#}", res.unwrap_err());
        assert!(msg.contains("moxin new"), "error must point user at fix: got {msg}");
    }
}
