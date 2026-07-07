use super::util::{format_freq, pin_label_padded, pin_label_short, pin_level, pin_pwm};
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
        // tone() 走任意数字引脚,不限 spec.pwm_pins;有稳定方波就显示频率
        let st = match pin_pwm(pin, state) {
            Some(s) => format_freq(s.freq_hz),
            None => (if level == LedLevel::On { "ON" } else { "OFF" }).to_string(),
        };
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
        let (state_word, symbol, style) = match pin_pwm(pin, state) {
            Some(s) => (
                format_freq(s.freq_hz),
                "♪",
                Style::default().fg(Color::Rgb(255, 200, 40)),
            ),
            None if is_on => (
                "ON ".to_string(),
                "♪",
                Style::default().fg(Color::Rgb(255, 200, 40)),
            ),
            None => ("OFF".to_string(), "♪", Style::default().fg(Color::DarkGray)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectMeta;
    use crate::sim::PwmSample;

    fn render(state: &RunState) -> String {
        let comp = Buzzer.build("bz1".to_string(), &[]).unwrap();
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
        // D7 = PORTD bit 7:非 PWM 引脚,tone() 不受 pwm_pins 限制
        Buzzer.render_plain(&comp, &PinRef::BoardDigital(7), &project, state, spec)
    }

    #[test]
    fn buzzer_shows_tone_frequency_under_stable_wave() {
        let mut state = RunState::default();
        state.pwm.insert(
            "D:7".to_string(),
            PwmSample { duty: 128, freq_hz: 1000, stable: true, t_us: 5000 },
        );
        state.last_event_t_us = 5000;
        let out = render(&state);
        assert!(out.contains("1000Hz"), "稳定方波应显示频率,got: {out}");
    }

    #[test]
    fn buzzer_falls_back_to_on_off_without_wave() {
        let mut state = RunState::default();
        state.pin_states.insert("D:7".to_string(), 1);
        let out = render(&state);
        assert!(out.contains("BUZZ ON"), "无方波时回退电平显示,got: {out}");
    }
}
