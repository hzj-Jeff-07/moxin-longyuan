//! 元件注册式抽象层(Phase 2-full Step 1)。
//!
//! 替换 v0.4.0 的多处 `match comp.kind` 死写(render plain/styled 双分派,
//! inspector summarize,shell cmd_add)。新增元件 = 新增一个 [`ComponentDef`]
//! 实现 + 在 [`Registry::builtin`] 加一行,主路径不动。
//!
//! 不引入新 crate 依赖(per CLAUDE.md):用 stdlib `HashMap` + `Arc<dyn Trait>`。

use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::text::Line;
use std::collections::HashMap;
use std::sync::Arc;

pub mod util;

mod breadboard;
mod button;
mod buzzer;
mod dupont;
mod led;
mod potentiometer;
mod resistor;
mod seven_segment;

/// 元件类型定义 trait。一个实现 = 一种元件类型(led / button / ...)。
pub trait ComponentDef: Send + Sync {
    /// 元件主名,与 `moxin.toml` 的 `type` 字段对齐。
    fn kind(&self) -> &'static str;

    /// shell `add` 命令的别名(`btn` → button 等)。空则只接受主名。
    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    /// 从 shell `add <kind> [args] --id <id>` 的 positional 参数构造 Component。
    /// 各实现自行决定 args 含义(LED 第一个是 color,resistor 第一个是 ohms,等等)。
    fn build(&self, id: String, args: &[String]) -> Result<Component>;

    /// `add` 成功后的友好显示文本(默认 "<id>")。LED 会覆写为 "<id> (<color> led)"。
    fn display_after_add(&self, comp: &Component) -> String {
        comp.id.clone()
    }

    /// 渲染为一行 plain ASCII(`render_runtime_frame` 用,piped/--no-tui)。
    /// `pin` 是从 wire 里解析出的板侧端子(此元件这条 wire 的板侧那端)。
    fn render_plain(
        &self,
        comp: &Component,
        pin: &crate::board::PinRef,
        project: &Project,
        state: &RunState,
        spec: &BoardSpec,
    ) -> String;

    /// 渲染为 ratatui 富样式行(TUI 用)。
    fn render_styled(
        &self,
        comp: &Component,
        pin: &crate::board::PinRef,
        project: &Project,
        state: &RunState,
        spec: &BoardSpec,
    ) -> Line<'static>;

    /// AI Inspector 的"Components"摘要扩展信息(可选)。
    /// 比如 resistor 返回 `"470Ω,10kΩ"`,inspector 拼成 `"resistor×2(470Ω,10kΩ)"`。
    /// 默认无扩展信息。
    fn inspector_extra(&self, _comp: &Component) -> Option<String> {
        None
    }
}

pub struct Registry {
    by_kind: HashMap<&'static str, Arc<dyn ComponentDef>>,
    by_alias: HashMap<&'static str, &'static str>,
}

impl Registry {
    pub fn empty() -> Self {
        Registry {
            by_kind: HashMap::new(),
            by_alias: HashMap::new(),
        }
    }

    pub fn register(&mut self, def: Arc<dyn ComponentDef>) {
        let kind = def.kind();
        for &alias in def.aliases() {
            self.by_alias.insert(alias, kind);
        }
        self.by_kind.insert(kind, def);
    }

    /// 通过主名或别名查找元件定义。
    pub fn resolve(&self, kind_or_alias: &str) -> Option<&dyn ComponentDef> {
        let canonical = self
            .by_alias
            .get(kind_or_alias)
            .copied()
            .unwrap_or(kind_or_alias);
        self.by_kind.get(canonical).map(|a| a.as_ref())
    }

    /// 内置 8 件元件的标准注册。
    pub fn builtin() -> Self {
        let mut r = Self::empty();
        r.register(Arc::new(led::Led));
        r.register(Arc::new(button::Button));
        r.register(Arc::new(resistor::Resistor));
        r.register(Arc::new(buzzer::Buzzer));
        r.register(Arc::new(potentiometer::Potentiometer));
        r.register(Arc::new(seven_segment::SevenSegment));
        r.register(Arc::new(breadboard::Breadboard));
        r.register(Arc::new(dupont::Dupont));
        r
    }
}

/// 全局只读注册表(builtin)。第一次访问时初始化。
pub fn registry() -> &'static Registry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(Registry::builtin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_resolves_main_kinds() {
        let r = Registry::builtin();
        assert!(r.resolve("led").is_some());
        assert!(r.resolve("button").is_some());
        assert!(r.resolve("resistor").is_some());
        assert!(r.resolve("buzzer").is_some());
        assert!(r.resolve("potentiometer").is_some());
        assert!(r.resolve("seven_segment").is_some());
        assert!(r.resolve("breadboard").is_some());
        assert!(r.resolve("dupont").is_some());
    }

    #[test]
    fn registry_resolves_aliases() {
        let r = Registry::builtin();
        assert_eq!(r.resolve("btn").unwrap().kind(), "button");
        assert_eq!(r.resolve("res").unwrap().kind(), "resistor");
        assert_eq!(r.resolve("buzz").unwrap().kind(), "buzzer");
        assert_eq!(r.resolve("pot").unwrap().kind(), "potentiometer");
        assert_eq!(r.resolve("7seg").unwrap().kind(), "seven_segment");
        assert_eq!(r.resolve("bb").unwrap().kind(), "breadboard");
        assert_eq!(r.resolve("wire").unwrap().kind(), "dupont");
        assert_eq!(r.resolve("jumper").unwrap().kind(), "dupont");
    }

    #[test]
    fn registry_returns_none_for_unknown() {
        let r = Registry::builtin();
        assert!(r.resolve("nonsense").is_none());
        assert!(r.resolve("").is_none());
    }
}
