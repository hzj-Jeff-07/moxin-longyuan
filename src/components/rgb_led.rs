use super::util::{component_terminal_pins, pin_drive_level, pin_label_padded, pin_label_short};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// 共阴 RGB LED:r / g / b 三个端子分别接三个(最好是 PWM)引脚,
/// 各通道 duty 混出颜色。接普通数字引脚的通道按 0/255 处理。
pub struct RgbLed;

/// 读三通道驱动电平 (r, g, b),未接线的通道 = 0。
fn rgb_channels(
    comp_id: &str,
    project: &Project,
    state: &RunState,
    spec: &BoardSpec,
) -> (u8, u8, u8) {
    let mut rgb = (0u8, 0u8, 0u8);
    for (terminal, pin) in component_terminal_pins(comp_id, project) {
        let level = pin_drive_level(&pin, state, spec);
        match terminal.as_str() {
            "r" | "red" => rgb.0 = level,
            "g" | "green" => rgb.1 = level,
            "b" | "blue" => rgb.2 = level,
            _ => {}
        }
    }
    rgb
}

impl ComponentDef for RgbLed {
    fn kind(&self) -> &'static str {
        "rgb_led"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["rgb"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "rgb_led".into(),
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
        let (r, g, b) = rgb_channels(&comp.id, project, state, spec);
        format!(
            "{} ───●─── ▣ {} [RGB r:{} g:{} b:{} #{:02X}{:02X}{:02X}]",
            pin_label_short(pin),
            comp.id,
            r,
            g,
            b,
            r,
            g,
            b
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
        let (r, g, b) = rgb_channels(&comp.id, project, state, spec);
        let swatch_style = if (r, g, b) == (0, 0, 0) {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Rgb(r, g, b))
        };
        Line::from(vec![
            Span::raw(format!(
                " {} ━━━━━━━━━━━━━━━━━━━━━━ ",
                pin_label_padded(pin)
            )),
            Span::styled("██", swatch_style),
            Span::raw(format!(
                " {} [RGB #{:02X}{:02X}{:02X}]",
                comp.id, r, g, b
            )),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{ProjectMeta, Wire};
    use crate::sim::PwmSample;

    fn project_with_rgb() -> Project {
        Project {
            project: ProjectMeta {
                name: "test".to_string(),
                board: "arduino-uno".to_string(),
                version: "0.2".to_string(),
            },
            components: vec![RgbLed.build("rgb1".to_string(), &[]).unwrap()],
            wires: vec![
                Wire { from: "board.D9".to_string(), to: "rgb1.r".to_string() },
                Wire { from: "board.D10".to_string(), to: "rgb1.g".to_string() },
                Wire { from: "board.D11".to_string(), to: "rgb1.b".to_string() },
            ],
            code: None,
        }
    }

    fn render(project: &Project, state: &RunState) -> String {
        let spec = crate::boards::board_from_str("arduino-uno").unwrap().spec();
        let comp = &project.components[0];
        RgbLed.render_plain(comp, &PinRef::BoardDigital(9), project, state, spec)
    }

    #[test]
    fn rgb_mixes_pwm_duties_into_hex() {
        let mut state = RunState::default();
        // D9 = B:1(r=255 全亮),D10 = B:2(g=128 半亮),D11 = B:3 无信号(b=0)
        state.pwm.insert(
            "B:1".to_string(),
            PwmSample { duty: 255, freq_hz: 490, stable: true, t_us: 5000 },
        );
        state.pwm.insert(
            "B:2".to_string(),
            PwmSample { duty: 128, freq_hz: 490, stable: true, t_us: 5000 },
        );
        state.last_event_t_us = 5000;
        let out = render(&project_with_rgb(), &state);
        assert!(out.contains("#FF8000"), "255/128/0 应混出 #FF8000,got: {out}");
    }

    #[test]
    fn rgb_digital_high_counts_as_full_channel() {
        let mut state = RunState::default();
        state.pin_states.insert("B:1".to_string(), 1); // D9 数字高
        let out = render(&project_with_rgb(), &state);
        assert!(out.contains("r:255"), "数字高电平通道应为 255,got: {out}");
        assert!(out.contains("#FF0000"), "got: {out}");
    }

    #[test]
    fn rgb_all_off_without_signal() {
        let state = RunState::default();
        let out = render(&project_with_rgb(), &state);
        assert!(out.contains("#000000"), "无信号应全灭,got: {out}");
    }

    #[test]
    fn rgb_registered_with_alias() {
        let r = crate::components::Registry::builtin();
        assert_eq!(r.resolve("rgb").unwrap().kind(), "rgb_led");
    }
}
