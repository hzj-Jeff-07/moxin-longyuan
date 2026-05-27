use super::util::{pin_label_padded, pin_label_short};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::text::Line;

pub struct Breadboard;

impl ComponentDef for Breadboard {
    fn kind(&self) -> &'static str {
        "breadboard"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["bb"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "breadboard".into(),
            color: None,
            pos: None,
            ohms: None,
            max_ohms: None,
            wire_color: None,
        })
    }

    fn render_plain(
        &self,
        comp: &Component,
        pin: &PinRef,
        _project: &Project,
        _state: &RunState,
        _spec: &BoardSpec,
    ) -> String {
        format!(
            "{} ───●─── ▦ BREADBOARD {}",
            pin_label_short(pin),
            comp.id
        )
    }

    fn render_styled(
        &self,
        comp: &Component,
        pin: &PinRef,
        _project: &Project,
        _state: &RunState,
        _spec: &BoardSpec,
    ) -> Line<'static> {
        Line::from(format!(
            " {} ━━━━━━━━━━━━━━━━━━━━━━ ▦ BREADBOARD {}",
            pin_label_padded(pin),
            comp.id
        ))
    }
}
