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

/// 电位器接的 A 引脚上的 ADC 读数 → (原始值, 百分比)。
/// 与 PWM 不同,ADC 值是"旋钮位置",不做过期判定 — 没转它就停在原地。
fn adc_reading(pin: &PinRef, state: &RunState, spec: &BoardSpec) -> Option<(u16, u32)> {
    let PinRef::BoardAnalog(n) = pin else {
        return None;
    };
    let ch = spec.adc_channel_for(*n)?;
    let v = *state.adc_values.get(&ch)?;
    Some((v, (v as u32 * 100 + 511) / 1023))
}

/// 10 格进度条:`▮▮▮▮▮▯▯▯▯▯`
fn knob_bar(percent: u32) -> String {
    let filled = ((percent + 5) / 10).min(10) as usize;
    format!("{}{}", "▮".repeat(filled), "▯".repeat(10 - filled))
}

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
        state: &RunState,
        spec: &BoardSpec,
    ) -> String {
        let max = comp
            .max_ohms
            .map(format_resistance)
            .unwrap_or_else(|| "10kΩ".to_string());
        match adc_reading(pin, state, spec) {
            Some((raw, pct)) => format!(
                "{} ───●─── ◎ {} [POT {}% ({}) {}]",
                pin_label_short(pin),
                comp.id,
                pct,
                raw,
                max
            ),
            None => format!(
                "{} ───●─── ◎ {} [POT {}]",
                pin_label_short(pin),
                comp.id,
                max
            ),
        }
    }

    fn render_styled(
        &self,
        comp: &Component,
        pin: &PinRef,
        _project: &Project,
        state: &RunState,
        spec: &BoardSpec,
    ) -> Line<'static> {
        let max = comp
            .max_ohms
            .map(format_resistance)
            .unwrap_or_else(|| "10kΩ".to_string());
        let mut spans = vec![
            Span::raw(format!(
                " {} ━━━━━━━━━━━━━━━━━━━━━━ ",
                pin_label_padded(pin)
            )),
            Span::styled("◎", Style::default().fg(Color::Rgb(60, 120, 255))),
        ];
        match adc_reading(pin, state, spec) {
            Some((raw, pct)) => {
                spans.push(Span::raw(format!(" {} [POT ", comp.id)));
                spans.push(Span::styled(
                    knob_bar(pct),
                    Style::default().fg(Color::Rgb(60, 120, 255)),
                ));
                spans.push(Span::raw(format!(" {}% ({}) {}]", pct, raw, max)));
            }
            None => {
                spans.push(Span::raw(format!(" {} [POT {}]", comp.id, max)));
            }
        }
        Line::from(spans)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectMeta;

    fn render(pin: &PinRef, state: &RunState) -> String {
        let comp = Potentiometer.build("p1".to_string(), &[]).unwrap();
        let spec = crate::boards::board_from_str("arduino-uno").unwrap().spec();
        let project = Project {
            project: ProjectMeta {
                name: "test".to_string(),
                board: "arduino-uno".to_string(),
                version: "0.2".to_string(),
            },
            components: vec![],
            wires: vec![],
            code: None,
        };
        Potentiometer.render_plain(&comp, pin, &project, state, spec)
    }

    #[test]
    fn pot_shows_adc_percent_when_value_present() {
        let mut state = RunState::default();
        state.adc_values.insert(0, 512); // A0 = ADC ch0
        let out = render(&PinRef::BoardAnalog(0), &state);
        assert!(out.contains("50%"), "512/1023 应显示 50%,got: {out}");
        assert!(out.contains("(512)"), "应显示原始值,got: {out}");
    }

    #[test]
    fn pot_falls_back_to_static_max_ohms() {
        let state = RunState::default();
        let out = render(&PinRef::BoardAnalog(0), &state);
        assert!(out.contains("10kΩ"), "无 ADC 值时回退静态阻值,got: {out}");
        assert!(!out.contains('%'), "无 ADC 值不显示百分比,got: {out}");
    }

    #[test]
    fn pot_on_digital_pin_stays_static() {
        // 接错到数字引脚:不查 ADC,静态显示
        let mut state = RunState::default();
        state.adc_values.insert(0, 512);
        let out = render(&PinRef::BoardDigital(9), &state);
        assert!(!out.contains('%'), "数字引脚不显示 ADC 百分比,got: {out}");
    }

    #[test]
    fn knob_bar_fills_by_percent() {
        assert_eq!(knob_bar(0), "▯▯▯▯▯▯▯▯▯▯");
        assert_eq!(knob_bar(50), "▮▮▮▮▮▯▯▯▯▯");
        assert_eq!(knob_bar(100), "▮▮▮▮▮▮▮▮▮▮");
    }
}
