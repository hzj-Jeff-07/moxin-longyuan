use super::util::{led_color, pin_label_padded, pin_label_short};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

pub struct Dupont;

impl ComponentDef for Dupont {
    fn kind(&self) -> &'static str {
        "dupont"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["wire", "jumper"]
    }

    fn build(&self, id: String, args: &[String]) -> Result<Component> {
        let wire_color = args.first().cloned();
        Ok(Component {
            id,
            kind: "dupont".into(),
            color: None,
            pos: None,
            ohms: None,
            max_ohms: None,
            wire_color,
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
        let color = comp.wire_color.as_deref().unwrap_or("red");
        format!(
            "{} ━━━━━━━━━━━ WIRE {} [{}]",
            pin_label_short(pin),
            comp.id,
            color
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
        let color_name = comp.wire_color.as_deref().unwrap_or("red");
        let wire_style = Style::default().fg(led_color(color_name));
        Line::from(vec![
            Span::raw(format!(" {} ", pin_label_padded(pin))),
            Span::styled("━━━━━━━━━━━━━━━━━━━━━━━━", wire_style),
            Span::raw(format!(" WIRE {}", comp.id)),
        ])
    }
}
