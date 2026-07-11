use super::util::{pin_label_padded, pin_label_short};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// 红外接收头(VS1838 类):out 端子输出解调后的 NEC 波形。
/// 码值经 shell `ir <hex>` 注入;声明后 bridge 自发一帧自检码。
pub struct IrReceiver;

impl ComponentDef for IrReceiver {
    fn kind(&self) -> &'static str {
        "ir_receiver"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["ir", "vs1838"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "ir_receiver".into(),
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
        match state.ir_code {
            Some(code) => format!(
                "{} ───●─── ◉ {} [IR {:08X}]",
                pin_label_short(pin),
                comp.id,
                code
            ),
            None => format!(
                "{} ───●─── ◉ {} [IR ?]",
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
        match state.ir_code {
            Some(code) => Line::from(vec![
                prefix,
                Span::styled("◉", Style::default().fg(Color::Rgb(180, 60, 220))),
                Span::raw(format!(" {} [IR {:08X}]", comp.id, code)),
            ]),
            None => Line::from(vec![
                prefix,
                Span::styled("◉", Style::default().fg(Color::DarkGray)),
                Span::raw(format!(" {} [IR ?]", comp.id)),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectMeta;

    fn render(state: &RunState) -> String {
        let comp = IrReceiver.build("ir1".to_string(), &[]).unwrap();
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
        IrReceiver.render_plain(&comp, &PinRef::BoardDigital(2), &project, state, spec)
    }

    #[test]
    fn ir_shows_last_code_in_hex() {
        let state = RunState {
            ir_code: Some(0x20DF10EF),
            ..Default::default()
        };
        assert!(render(&state).contains("20DF10EF"));
    }

    #[test]
    fn ir_unknown_before_any_frame() {
        assert!(render(&RunState::default()).contains("IR ?"));
    }

    #[test]
    fn ir_registered_with_aliases() {
        let r = crate::components::Registry::builtin();
        assert_eq!(r.resolve("ir").unwrap().kind(), "ir_receiver");
        assert_eq!(r.resolve("vs1838").unwrap().kind(), "ir_receiver");
    }
}
