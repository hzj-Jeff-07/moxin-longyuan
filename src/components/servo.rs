use super::util::{pin_label_padded, pin_label_short, pin_pwm};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// SG90 舵机:50Hz PWM,脉宽 500-2500us 线性映射 0-180°。
/// 角度从 `PwmSample` 推导:pulse_us = duty/255 × period_us。
pub struct Servo;

/// 舵机信号的合法频率窗口(标称 50Hz,留余量给 Servo 库的定时误差)。
const SERVO_FREQ_MIN: u32 = 40;
const SERVO_FREQ_MAX: u32 = 100;

/// 从引脚 PWM 采样推导舵机角度(0-180°)。非舵机频段 / 无信号 → None。
fn servo_angle(pin: &PinRef, state: &RunState) -> Option<u32> {
    let s = pin_pwm(pin, state)?;
    if !(SERVO_FREQ_MIN..=SERVO_FREQ_MAX).contains(&s.freq_hz) {
        return None;
    }
    let period_us = 1_000_000 / s.freq_hz as u64;
    let pulse_us = s.duty as u64 * period_us / 255;
    // 500-2500us → 0-180°,窗口外截断
    let clamped = pulse_us.clamp(500, 2500);
    Some(((clamped - 500) * 180 / 2000) as u32)
}

/// 角度的方向指针字符(粗分 5 档)。
fn angle_glyph(angle: u32) -> &'static str {
    match angle {
        0..=22 => "←",
        23..=67 => "↖",
        68..=112 => "↑",
        113..=157 => "↗",
        _ => "→",
    }
}

impl ComponentDef for Servo {
    fn kind(&self) -> &'static str {
        "servo"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["sg90"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "servo".into(),
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
        match servo_angle(pin, state) {
            Some(a) => format!(
                "{} ───●─── {} {} [SERVO {}°]",
                pin_label_short(pin),
                angle_glyph(a),
                comp.id,
                a
            ),
            None => format!(
                "{} ───●─── ↑ {} [SERVO ?]",
                pin_label_short(pin),
                comp.id
            ),
        }
    }

    fn render_styled(
        &self,
        comp: &Component,
        pin: &PinRef,
        _project: &Project,
        state: &RunState,
        _spec: &BoardSpec,
    ) -> Line<'static> {
        let prefix = Span::raw(format!(
            " {} ━━━━━━━━━━━━━━━━━━━━━━ ",
            pin_label_padded(pin)
        ));
        match servo_angle(pin, state) {
            Some(a) => Line::from(vec![
                prefix,
                Span::styled(
                    angle_glyph(a).to_string(),
                    Style::default().fg(Color::Rgb(60, 200, 255)),
                ),
                Span::raw(format!(" {} [SERVO {}°]", comp.id, a)),
            ]),
            None => Line::from(vec![
                prefix,
                Span::styled("↑", Style::default().fg(Color::DarkGray)),
                Span::raw(format!(" {} [SERVO ?]", comp.id)),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::PwmSample;

    /// 50Hz + 给定脉宽(us)构造 PwmSample:duty = pulse/period × 255
    fn sample_50hz(pulse_us: u64) -> PwmSample {
        let period_us = 20_000u64;
        PwmSample {
            duty: ((pulse_us * 255 + period_us / 2) / period_us) as u8,
            freq_hz: 50,
            stable: true,
            t_us: 5000,
        }
    }

    fn state_with(sample: PwmSample) -> RunState {
        let mut state = RunState::default();
        state.pwm.insert("B:1".to_string(), sample); // D9
        state.last_event_t_us = 5000;
        state
    }

    #[test]
    fn servo_center_pulse_reads_near_90_deg() {
        // 1500us = 90°;duty 量化(255 档)带来 ±3° 误差
        let state = state_with(sample_50hz(1500));
        let a = servo_angle(&PinRef::BoardDigital(9), &state).unwrap();
        assert!((85..=95).contains(&a), "1500us 应接近 90°,got {a}");
    }

    #[test]
    fn servo_extremes_clamp_to_range() {
        let state = state_with(sample_50hz(500));
        assert_eq!(servo_angle(&PinRef::BoardDigital(9), &state), Some(0));
        let state = state_with(sample_50hz(2500));
        assert_eq!(servo_angle(&PinRef::BoardDigital(9), &state), Some(180));
    }

    #[test]
    fn servo_rejects_non_servo_frequency() {
        // 490Hz analogWrite 波形不是舵机信号
        let mut s = sample_50hz(1500);
        s.freq_hz = 490;
        let state = state_with(s);
        assert_eq!(servo_angle(&PinRef::BoardDigital(9), &state), None);
    }

    #[test]
    fn servo_renders_question_mark_without_signal() {
        let comp = Servo.build("sv1".to_string(), &[]).unwrap();
        let spec = crate::boards::board_from_str("arduino-uno").unwrap().spec();
        let project = crate::project::Project {
            project: crate::project::ProjectMeta {
                name: "t".into(),
                board: "arduino-uno".into(),
                version: "0.2".into(),
            },
            components: vec![],
            wires: vec![],
            code: None,
        };
        let out = Servo.render_plain(
            &comp,
            &PinRef::BoardDigital(9),
            &project,
            &RunState::default(),
            spec,
        );
        assert!(out.contains("SERVO ?"), "got: {out}");
    }
}
