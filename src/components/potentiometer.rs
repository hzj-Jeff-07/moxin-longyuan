use super::util::{format_resistance, parse_resistance, pin_label_padded, pin_label_short};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

pub struct Potentiometer;

impl ComponentDef for Potentiometer {
    fn kind(&self) -> &'static str {
        "potentiometer"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["pot"]
    }

    fn build(&self, id: String, args: &[String]) -> Result<Component> {
        let max_ohms = args
            .first()
            .map(|r| parse_resistance(r))
            .transpose()?
            .unwrap_or(10_000);
        Ok(Component {
            id,
            kind: "potentiometer".into(),
            color: None,
            pos: None,
            ohms: None,
            max_ohms: Some(max_ohms),
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
        let max = comp
            .max_ohms
            .map(format_resistance)
            .unwrap_or_else(|| "10kΩ".to_string());
        format!(
            "{} ───●─── ◎ {} [POT {}]",
            pin_label_short(pin),
            comp.id,
            max
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
        Line::from(vec![
            Span::raw(format!(
                " {} ━━━━━━━━━━━━━━━━━━━━━━ ",
                pin_label_padded(pin)
            )),
            Span::styled("◎", Style::default().fg(Color::Rgb(60, 120, 255))),
            Span::raw(format!(
                " {} [POT {}]",
                comp.id,
                comp.max_ohms
                    .map(format_resistance)
                    .unwrap_or_else(|| "10kΩ".to_string())
            )),
        ])
    }
}
