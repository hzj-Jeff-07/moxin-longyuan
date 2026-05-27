use super::util::{pin_label_padded, pin_label_short};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::text::Line;

pub struct Button;

impl ComponentDef for Button {
    fn kind(&self) -> &'static str {
        "button"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["btn"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "button".into(),
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
        state: &RunState,
        _spec: &BoardSpec,
    ) -> String {
        let st = if state.button_pressed { "DOWN" } else { "UP" };
        format!(
            "{} ───●─── [Button:{} {}]",
            pin_label_short(pin),
            comp.id,
            st
        )
    }

    fn render_styled(
        &self,
        comp: &Component,
        pin: &PinRef,
        _project: &Project,
        state: &RunState,
        _spec: &BoardSpec,
    ) -> Line<'static> {
        Line::from(format!(
            " {} ━━━━━━━━━━━━━━━━━━━━━━ ● {} [BTN {}]",
            pin_label_padded(pin),
            comp.id,
            if state.button_pressed { "DOWN" } else { "UP" }
        ))
    }
}
