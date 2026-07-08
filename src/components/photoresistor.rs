use super::util::{pin_label_padded, pin_label_short};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// 光敏电阻 LDR:接 A 引脚,环境光经 `adc` 命令 / TUI 旋钮注入。
/// 与电位器同为 ADC 输入件,值越大 = 光越强。
pub struct Photoresistor;

/// A 引脚上的注入值 → (原始值, 百分比)。同 potentiometer,不做过期判定。
fn adc_reading(pin: &PinRef, state: &RunState, spec: &BoardSpec) -> Option<(u16, u32)> {
    let PinRef::BoardAnalog(n) = pin else {
        return None;
    };
    let ch = spec.adc_channel_for(*n)?;
    let v = *state.adc_values.get(&ch)?;
    Some((v, (v as u32 * 100 + 511) / 1023))
}

/// 光强图标:暗 → 亮
fn light_icon(percent: u32) -> &'static str {
    match percent {
        0..=25 => "☾",
        26..=60 => "☁",
        _ => "☀",
    }
}

impl ComponentDef for Photoresistor {
    fn kind(&self) -> &'static str {
        "photoresistor"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["ldr"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "photoresistor".into(),
            color: None,
            pos: None,
            ohms: None,
            max_ohms: None,
            wire_color: None,
        })
    }

    fn adc_knob(&self) -> bool {
        true
    }

    fn render_plain(
        &self,
        comp: &Component,
        pin: &PinRef,
        _project: &Project,
        state: &RunState,
        spec: &BoardSpec,
    ) -> String {
        match adc_reading(pin, state, spec) {
            Some((raw, pct)) => format!(
                "{} ───●─── {} {} [LDR {}% ({})]",
                pin_label_short(pin),
                light_icon(pct),
                comp.id,
                pct,
                raw
            ),
            None => format!(
                "{} ───●─── ☁ {} [LDR ?]",
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
        spec: &BoardSpec,
    ) -> Line<'static> {
        let prefix = Span::raw(format!(
            " {} ━━━━━━━━━━━━━━━━━━━━━━ ",
            pin_label_padded(pin)
        ));
        match adc_reading(pin, state, spec) {
            Some((raw, pct)) => {
                // 光强映射亮度:暗灰 → 亮黄
                let level = (105 + pct * 150 / 100) as u8;
                Line::from(vec![
                    prefix,
                    Span::styled(
                        light_icon(pct).to_string(),
                        Style::default().fg(Color::Rgb(level, level, 40)),
                    ),
                    Span::raw(format!(" {} [LDR {}% ({})]", comp.id, pct, raw)),
                ])
            }
            None => Line::from(vec![
                prefix,
                Span::styled("☁", Style::default().fg(Color::DarkGray)),
                Span::raw(format!(" {} [LDR ?]", comp.id)),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectMeta;

    fn render(pin: &PinRef, state: &RunState) -> String {
        let comp = Photoresistor.build("ldr1".to_string(), &[]).unwrap();
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
        Photoresistor.render_plain(&comp, pin, &project, state, spec)
    }

    #[test]
    fn ldr_shows_percent_with_injected_value() {
        let mut state = RunState::default();
        state.adc_values.insert(0, 1023);
        let out = render(&PinRef::BoardAnalog(0), &state);
        assert!(out.contains("100%"), "满量程应显示 100%,got: {out}");
        assert!(out.contains('☀'), "强光应显示太阳图标,got: {out}");
    }

    #[test]
    fn ldr_falls_back_without_value() {
        let state = RunState::default();
        let out = render(&PinRef::BoardAnalog(0), &state);
        assert!(out.contains("LDR ?"), "无注入值时显示未知,got: {out}");
        assert!(!out.contains('%'));
    }

    #[test]
    fn ldr_dark_icon_at_low_light() {
        let mut state = RunState::default();
        state.adc_values.insert(0, 100); // ~10%
        let out = render(&PinRef::BoardAnalog(0), &state);
        assert!(out.contains('☾'), "弱光应显示月亮图标,got: {out}");
    }

    #[test]
    fn ldr_registered_with_alias_and_knob() {
        let r = crate::components::Registry::builtin();
        let def = r.resolve("ldr").expect("alias ldr resolves");
        assert_eq!(def.kind(), "photoresistor");
        assert!(def.adc_knob(), "光敏是 ADC 旋钮件");
        assert!(!r.resolve("led").unwrap().adc_knob(), "LED 不是旋钮件");
    }
}
