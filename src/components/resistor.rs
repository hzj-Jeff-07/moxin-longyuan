use super::util::{
    format_resistance, parse_resistance, pin_label_padded, pin_label_short, resistance_color_rings,
};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::{anyhow, Result};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

pub struct Resistor;

impl ComponentDef for Resistor {
    fn kind(&self) -> &'static str {
        "resistor"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["res"]
    }

    fn build(&self, id: String, args: &[String]) -> Result<Component> {
        let raw = args.first().ok_or_else(|| {
            anyhow!("resistor requires ohms value (e.g. `add resistor 10k --id r1`)")
        })?;
        let ohms = parse_resistance(raw)?;
        Ok(Component {
            id,
            kind: "resistor".into(),
            color: None,
            pos: None,
            ohms: Some(ohms),
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
        let label = format_resistance(comp.ohms.unwrap_or(0));
        format!(
            "{} ───┤▮▮▮▮├─── {} {}",
            pin_label_short(pin),
            label,
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
        let ohms = comp.ohms.unwrap_or(0);
        let rings = resistance_color_rings(ohms);
        let label = format_resistance(ohms);
        Line::from(vec![
            Span::raw(format!(" {} ━━━┤", pin_label_padded(pin))),
            Span::styled("▮", Style::default().fg(rings[0])),
            Span::styled("▮", Style::default().fg(rings[1])),
            Span::styled("▮", Style::default().fg(rings[2])),
            Span::styled("▮", Style::default().fg(rings[3])),
            Span::raw(format!("├━━━ {} {}", label, comp.id)),
        ])
    }

    fn inspector_extra(&self, comp: &Component) -> Option<String> {
        comp.ohms.map(format_resistance)
    }
}
