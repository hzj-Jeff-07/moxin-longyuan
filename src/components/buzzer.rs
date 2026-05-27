use super::util::{pin_label_padded, pin_label_short, pin_level};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::{LedLevel, RunState};
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

pub struct Buzzer;

impl ComponentDef for Buzzer {
    fn kind(&self) -> &'static str {
        "buzzer"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["buzz"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "buzzer".into(),
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
        spec: &BoardSpec,
    ) -> String {
        let level = pin_level(pin, state, spec);
        let st = if level == LedLevel::On { "ON" } else { "OFF" };
        format!(
            "{} ───●─── ♪ {} [BUZZ {}]",
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
        spec: &BoardSpec,
    ) -> Line<'static> {
        let is_on = pin_level(pin, state, spec) == LedLevel::On;
        let (state_word, symbol, style) = if is_on {
            ("ON ", "♪", Style::default().fg(Color::Rgb(255, 200, 40)))
        } else {
            ("OFF", "♪", Style::default().fg(Color::DarkGray))
        };
        Line::from(vec![
            Span::raw(format!(
                " {} ━━━━━━━━━━━━━━━━━━━━━━ ",
                pin_label_padded(pin)
            )),
            Span::styled(symbol.to_string(), style),
            Span::raw(format!(" {} [BUZZ {}]", comp.id, state_word)),
        ])
    }
}
