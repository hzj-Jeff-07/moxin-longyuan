use super::util::{
    duty_percent, led_color, pin_is_pwm_capable, pin_label_padded, pin_label_short, pin_level,
    pin_pwm,
};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::{LedLevel, RunState};
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

pub struct Led;

/// LED 引脚上的有效调光信号 → 占空比百分比。
/// 仅对板子声明的 PWM 引脚开启,避免把慢速 blink 方波显示成百分比。
fn led_pwm(pin: &PinRef, state: &RunState, spec: &BoardSpec) -> Option<u32> {
    if !pin_is_pwm_capable(pin, spec) {
        return None;
    }
    pin_pwm(pin, state)
        .filter(|s| s.duty > 0)
        .map(|s| duty_percent(s.duty))
}

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
        let (label, marker) = match led_pwm(pin, state, spec) {
            // PWM 调光:显示占空比而不是 ON/OFF
            Some(pct) => (format!("{} {}%", color, pct), "~"),
            None => match level {
                LedLevel::On => (format!("{} ON", color), "#"),
                LedLevel::Off => (format!("{} OFF", color), "."),
            },
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
        let (state_word, marker, marker_style) = match led_pwm(pin, state, spec) {
            Some(pct) => (
                format!("{}%", pct),
                "●",
                Style::default().fg(led_color(color_name)),
            ),
            None => match level {
                LedLevel::On => (
                    "ON ".to_string(),
                    "●",
                    Style::default().fg(led_color(color_name)),
                ),
                LedLevel::Off => ("OFF".to_string(), "○", Style::default().fg(Color::DarkGray)),
            },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectMeta;
    use crate::sim::PwmSample;

    fn empty_project() -> Project {
        Project {
            project: ProjectMeta {
                name: "test".to_string(),
                board: "arduino-uno".to_string(),
                version: "0.2".to_string(),
            },
            components: vec![],
            wires: vec![],
            code: None,
        }
    }

    fn state_with_pwm(key: &str, duty: u8, freq_hz: u32, stable: bool) -> RunState {
        let mut state = RunState::default();
        state.pwm.insert(
            key.to_string(),
            PwmSample { duty, freq_hz, stable, t_us: 5000 },
        );
        state.last_event_t_us = 5000; // 样本新鲜
        state
    }

    fn render(pin: &PinRef, state: &RunState) -> String {
        let comp = Led.build("led1".to_string(), &["red".to_string()]).unwrap();
        let spec = crate::boards::board_from_str("arduino-uno").unwrap().spec();
        Led.render_plain(&comp, pin, &empty_project(), state, spec)
    }

    #[test]
    fn led_shows_duty_percent_under_stable_pwm() {
        // D9 = PORTB bit 1,是 Uno 的 PWM 引脚
        let state = state_with_pwm("B:1", 128, 980, true);
        let out = render(&PinRef::BoardDigital(9), &state);
        assert!(out.contains("50%"), "duty 128 应显示 50%,got: {out}");
        assert!(!out.contains("ON") && !out.contains("OFF"));
    }

    #[test]
    fn led_falls_back_to_on_off_without_pwm() {
        let mut state = RunState::default();
        state.pin_states.insert("B:1".to_string(), 1);
        let out = render(&PinRef::BoardDigital(9), &state);
        assert!(out.contains("red ON"), "无 PWM 采样时回退电平显示,got: {out}");
    }

    #[test]
    fn led_ignores_pwm_on_non_pwm_pin() {
        // D8 = PORTB bit 0,不是 PWM 引脚:即使有稳定方波也不显示百分比
        let state = state_with_pwm("B:0", 128, 980, true);
        let out = render(&PinRef::BoardDigital(8), &state);
        assert!(!out.contains('%'), "非 PWM 引脚不显示占空比,got: {out}");
    }

    #[test]
    fn led_ignores_unstable_or_slow_waves() {
        // 不稳定波形
        let state = state_with_pwm("B:1", 128, 980, false);
        assert!(!render(&PinRef::BoardDigital(9), &state).contains('%'));
        // 1Hz blink:低于 PWM_DISPLAY_MIN_FREQ_HZ
        let state = state_with_pwm("B:1", 128, 1, true);
        assert!(!render(&PinRef::BoardDigital(9), &state).contains('%'));
    }
}
