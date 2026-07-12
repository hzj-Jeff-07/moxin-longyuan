use super::util::{pin_label_padded, pin_label_short};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// LCD1602(PCF8574 I2C 背包 @0x27):SDA/SCL 接 A4/A5。
/// 内容来自 bridge 的 `lcd` 事件(TWI 从机解析 HD44780 4-bit 时序)。
pub struct Lcd1602;

impl ComponentDef for Lcd1602 {
    fn kind(&self) -> &'static str {
        "lcd1602"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["lcd"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "lcd1602".into(),
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
        match &state.lcd {
            Some((r0, r1)) => format!(
                "{} ───●─── ▤ {} [LCD |{}|{}|]",
                pin_label_short(pin),
                comp.id,
                r0,
                r1
            ),
            None => format!(
                "{} ───●─── ▤ {} [LCD (blank)]",
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
        // 经典蓝底白字配色
        let screen = Style::default()
            .fg(Color::Rgb(240, 248, 255))
            .bg(Color::Rgb(30, 60, 180));
        match &state.lcd {
            Some((r0, r1)) => Line::from(vec![
                prefix,
                Span::raw(format!("{} ", comp.id)),
                Span::styled(format!("▐{}▌", r0), screen),
                Span::raw(" "),
                Span::styled(format!("▐{}▌", r1), screen),
            ]),
            None => Line::from(vec![
                prefix,
                Span::raw(format!("{} ", comp.id)),
                Span::styled(
                    "▐                ▌".to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectMeta;

    fn render(state: &RunState) -> String {
        let comp = Lcd1602.build("lcd1".to_string(), &[]).unwrap();
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
        Lcd1602.render_plain(&comp, &PinRef::BoardAnalog(4), &project, state, spec)
    }

    #[test]
    fn lcd_shows_rows_from_bridge_event() {
        let state = RunState {
            lcd: Some(("Hello MoXin!    ".into(), "LCD1602 via I2C ".into())),
            ..Default::default()
        };
        let out = render(&state);
        assert!(out.contains("Hello MoXin!"), "got: {out}");
        assert!(out.contains("LCD1602 via I2C"), "got: {out}");
    }

    #[test]
    fn lcd_blank_before_any_event() {
        assert!(render(&RunState::default()).contains("(blank)"));
    }

    #[test]
    fn lcd_registered_with_alias() {
        let r = crate::components::Registry::builtin();
        assert_eq!(r.resolve("lcd").unwrap().kind(), "lcd1602");
    }
}
