use super::util::{pin_label_padded, pin_label_short};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// DHT11 温湿度传感器:单总线 data 端子。
/// 环境经 `env` 命令注入(bridge 默认 25°C/60%),固件按 DHT11 时序 bit-bang 读。
pub struct Dht11;

impl ComponentDef for Dht11 {
    fn kind(&self) -> &'static str {
        "dht11"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["dht"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "dht11".into(),
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
        match state.dht_env {
            Some((t, h)) => format!(
                "{} ───●─── ≋ {} [DHT11 {}°C {}%]",
                pin_label_short(pin),
                comp.id,
                t,
                h
            ),
            None => format!(
                "{} ───●─── ≋ {} [DHT11 ?]",
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
        match state.dht_env {
            Some((t, h)) => {
                // 温度映射颜色:冷蓝 → 热红
                let warm = (t.min(50) as u32 * 255 / 50) as u8;
                Line::from(vec![
                    prefix,
                    Span::styled(
                        "≋",
                        Style::default().fg(Color::Rgb(warm, 80, 255 - warm)),
                    ),
                    Span::raw(format!(" {} [DHT11 {}°C {}%]", comp.id, t, h)),
                ])
            }
            None => Line::from(vec![
                prefix,
                Span::styled("≋", Style::default().fg(Color::DarkGray)),
                Span::raw(format!(" {} [DHT11 ?]", comp.id)),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectMeta;

    fn render(state: &RunState) -> String {
        let comp = Dht11.build("dht1".to_string(), &[]).unwrap();
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
        Dht11.render_plain(&comp, &PinRef::BoardDigital(2), &project, state, spec)
    }

    #[test]
    fn dht_shows_injected_env() {
        let state = RunState {
            dht_env: Some((31, 75)),
            ..Default::default()
        };
        let out = render(&state);
        assert!(out.contains("31°C") && out.contains("75%"), "got: {out}");
    }

    #[test]
    fn dht_unknown_before_configuration() {
        assert!(render(&RunState::default()).contains("DHT11 ?"));
    }

    #[test]
    fn dht_registered_with_alias() {
        let r = crate::components::Registry::builtin();
        assert_eq!(r.resolve("dht").unwrap().kind(), "dht11");
    }
}
