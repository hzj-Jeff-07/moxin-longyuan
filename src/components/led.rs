use super::util::{led_color, pin_label_padded, pin_label_short, pin_level};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::{LedLevel, RunState};
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

pub struct Led;

impl ComponentDef for Led {
    fn kind(&self) -> &'static str {
        "led"
    }

    fn build(&self, id: String, args: &[String]) -> Result<Component> {
        let color = args.first().cloned().unwrap_or_else(|| "red".into());
        Ok(Component {
            id,
            kind: "led".into(),
            color: Some(color),
            pos: None,
            ohms: None,
            max_ohms: None,
            wire_color: None,
        })
    }

    fn display_after_add(&self, comp: &Component) -> String {
        match comp.color.as_deref() {
            Some(c) => format!("{} ({} led)", comp.id, c),
            None => comp.id.clone(),
        }
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
        let color = comp.color.as_deref().unwrap_or("red").to_string();
        let (label, marker) = match level {
            LedLevel::On => (format!("{} ON", color), "#"),
            LedLevel::Off => (format!("{} OFF", color), "."),
        };
        format!(
            "{} ───●─── [LED:{} {} {}]",
            pin_label_short(pin),
            comp.id,
            label,
            marker
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
        let color_name = comp.color.as_deref().unwrap_or("red");
        let level = pin_level(pin, state, spec);
        let (state_word, marker, marker_style) = match level {
            LedLevel::On => ("ON ", "●", Style::default().fg(led_color(color_name))),
            LedLevel::Off => ("OFF", "○", Style::default().fg(Color::DarkGray)),
        };
        let abbr: String = color_name.to_uppercase().chars().take(3).collect();
        Line::from(vec![
            Span::raw(format!(
                " {} ━━━━━━━━━━━━━━━━━━━━━━ ",
                pin_label_padded(pin)
            )),
            Span::styled(marker.to_string(), marker_style),
            Span::raw(format!(" {} [{} {}]", comp.id, abbr, state_word)),
        ])
    }
}
