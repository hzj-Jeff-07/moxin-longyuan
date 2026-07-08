use super::util::{pin_label_padded, pin_label_short};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// HC-SR04 超声波:trig / echo 两个信号端子。
/// 距离由 `dist` 命令注入(bridge 默认 50cm),固件 pulseIn 测回波脉宽。
pub struct Ultrasonic;

impl ComponentDef for Ultrasonic {
    fn kind(&self) -> &'static str {
        "ultrasonic"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["sr04", "hcsr04"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "ultrasonic".into(),
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
        match state.ultrasonic_cm {
            Some(cm) => format!(
                "{} ───●─── ⇢ {} [SR04 {}cm]",
                pin_label_short(pin),
                comp.id,
                cm
            ),
            None => format!(
                "{} ───●─── ⇢ {} [SR04 ?]",
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
        match state.ultrasonic_cm {
            Some(cm) => {
                // 距离越近波纹越亮
                let level = (255 - (cm.min(400) as u32 * 155 / 400)) as u8;
                Line::from(vec![
                    prefix,
                    Span::styled("⇢", Style::default().fg(Color::Rgb(40, level, 220))),
                    Span::raw(format!(" {} [SR04 {}cm]", comp.id, cm)),
                ])
            }
            None => Line::from(vec![
                prefix,
                Span::styled("⇢", Style::default().fg(Color::DarkGray)),
                Span::raw(format!(" {} [SR04 ?]", comp.id)),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectMeta;

    fn render(state: &RunState) -> String {
        let comp = Ultrasonic.build("us1".to_string(), &[]).unwrap();
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
        Ultrasonic.render_plain(&comp, &PinRef::BoardDigital(7), &project, state, spec)
    }

    #[test]
    fn sr04_shows_injected_distance() {
        let state = RunState {
            ultrasonic_cm: Some(120),
            ..Default::default()
        };
        assert!(render(&state).contains("120cm"));
    }

    #[test]
    fn sr04_unknown_before_configuration() {
        let state = RunState::default();
        assert!(render(&state).contains("SR04 ?"));
    }

    #[test]
    fn sr04_registered_with_aliases() {
        let r = crate::components::Registry::builtin();
        assert_eq!(r.resolve("sr04").unwrap().kind(), "ultrasonic");
        assert_eq!(r.resolve("hcsr04").unwrap().kind(), "ultrasonic");
    }
}
