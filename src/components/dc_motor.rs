use super::util::{
    component_terminal_pins, duty_percent, pin_drive_level, pin_label_padded, pin_label_short,
    pin_level,
};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::{LedLevel, RunState};
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// 直流电机(经 L298N 驱动):`ena` 调速(PWM),`in1`/`in2` 定向。
/// in1=1,in2=0 → 正转;反之反转;同电平 → 停(刹车/滑行不细分)。
pub struct DcMotor;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum MotorRun {
    Forward(u32),
    Reverse(u32),
    Stop,
}

fn motor_state(
    comp_id: &str,
    project: &Project,
    state: &RunState,
    spec: &BoardSpec,
) -> MotorRun {
    let mut ena: Option<u8> = None;
    let mut in1 = false;
    let mut in2 = false;
    for (terminal, pin) in component_terminal_pins(comp_id, project) {
        match terminal.as_str() {
            "ena" | "en" | "speed" => ena = Some(pin_drive_level(&pin, state, spec)),
            "in1" => in1 = pin_level(&pin, state, spec) == LedLevel::On,
            "in2" => in2 = pin_level(&pin, state, spec) == LedLevel::On,
            _ => {}
        }
    }
    // ena 未接线 = 常使能(跳线帽直连 5V 的常见接法)
    let speed = duty_percent(ena.unwrap_or(255));
    match (in1, in2) {
        _ if speed == 0 => MotorRun::Stop,
        (true, false) => MotorRun::Forward(speed),
        (false, true) => MotorRun::Reverse(speed),
        _ => MotorRun::Stop,
    }
}

fn motor_label(run: MotorRun) -> String {
    match run {
        MotorRun::Forward(s) => format!("▶ {}%", s),
        MotorRun::Reverse(s) => format!("◀ {}%", s),
        MotorRun::Stop => "■ 0%".to_string(),
    }
}

impl ComponentDef for DcMotor {
    fn kind(&self) -> &'static str {
        "dc_motor"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["motor"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "dc_motor".into(),
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
        project: &Project,
        state: &RunState,
        spec: &BoardSpec,
    ) -> String {
        let run = motor_state(&comp.id, project, state, spec);
        format!(
            "{} ───●─── ⚙ {} [MOTOR {}]",
            pin_label_short(pin),
            comp.id,
            motor_label(run)
        )
    }

    fn render_styled(
        &self,
        comp: &Component,
        pin: &PinRef,
        project: &Project,
        state: &RunState,
        spec: &BoardSpec,
    ) -> Line<'static> {
        let run = motor_state(&comp.id, project, state, spec);
        let style = match run {
            MotorRun::Stop => Style::default().fg(Color::DarkGray),
            _ => Style::default().fg(Color::Rgb(40, 220, 80)),
        };
        Line::from(vec![
            Span::raw(format!(
                " {} ━━━━━━━━━━━━━━━━━━━━━━ ",
                pin_label_padded(pin)
            )),
            Span::styled("⚙", style),
            Span::raw(format!(" {} [MOTOR {}]", comp.id, motor_label(run))),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{ProjectMeta, Wire};
    use crate::sim::PwmSample;

    fn project_with_motor() -> Project {
        Project {
            project: ProjectMeta {
                name: "test".to_string(),
                board: "arduino-uno".to_string(),
                version: "0.2".to_string(),
            },
            components: vec![DcMotor.build("m1".to_string(), &[]).unwrap()],
            wires: vec![
                Wire { from: "board.D9".to_string(), to: "m1.ena".to_string() },
                Wire { from: "board.D7".to_string(), to: "m1.in1".to_string() },
                Wire { from: "board.D8".to_string(), to: "m1.in2".to_string() },
            ],
            code: None,
        }
    }

    fn run_of(state: &RunState) -> MotorRun {
        let project = project_with_motor();
        let spec = crate::boards::board_from_str("arduino-uno").unwrap().spec();
        motor_state("m1", &project, state, spec)
    }

    #[test]
    fn motor_forward_at_pwm_speed() {
        let mut state = RunState::default();
        state.pwm.insert(
            "B:1".to_string(), // D9 ena
            PwmSample { duty: 191, freq_hz: 490, stable: true, t_us: 5000 },
        );
        state.last_event_t_us = 5000;
        state.pin_states.insert("D:7".to_string(), 1); // in1 高
        assert_eq!(run_of(&state), MotorRun::Forward(75));
    }

    #[test]
    fn motor_reverse_when_in2_high() {
        let mut state = RunState::default();
        state.pin_states.insert("B:1".to_string(), 1); // ena 数字高 = 100%
        state.pin_states.insert("B:0".to_string(), 1); // D8 in2 高
        assert_eq!(run_of(&state), MotorRun::Reverse(100));
    }

    #[test]
    fn motor_stops_when_directions_agree_or_ena_low() {
        // in1 = in2 = 0 → 停
        let mut state = RunState::default();
        state.pin_states.insert("B:1".to_string(), 1);
        assert_eq!(run_of(&state), MotorRun::Stop);
        // ena 低电平 → 停(即使 in1 高)
        let mut state = RunState::default();
        state.pin_states.insert("D:7".to_string(), 1);
        state.pin_states.insert("B:1".to_string(), 0);
        assert_eq!(run_of(&state), MotorRun::Stop);
    }

    #[test]
    fn motor_registered_with_alias() {
        let r = crate::components::Registry::builtin();
        assert_eq!(r.resolve("motor").unwrap().kind(), "dc_motor");
    }
}
