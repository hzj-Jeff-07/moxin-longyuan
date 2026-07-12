use super::util::{pin_label_padded, pin_label_short};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::RunState;
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// OLED SSD1306(128×64,I2C @0x3C):SDA/SCL 接 A4/A5。
/// 帧缓冲经 bridge 的 `oled` 事件降采样成盲文行传来(16 行 × 64 字符)。
/// 板面板一行装不下 64×16 盲文,这里显示亮像素统计 + 最密一行的预览片段;
/// 完整 16 行存在 `RunState.oled` 里,供将来专用面板渲染。
pub struct OledSsd1306;

/// 一个盲文字符(U+2800..U+28FF)点亮几个点。空格 / 非盲文按 0 算。
fn braille_dots(c: char) -> u32 {
    let cp = c as u32;
    if (0x2800..=0x28FF).contains(&cp) {
        (cp - 0x2800).count_ones()
    } else {
        0
    }
}

/// 统计全帧点亮像素;并返回最密行的索引。
fn frame_stats(rows: &[String]) -> (u32, usize) {
    let mut total = 0u32;
    let mut best = 0usize;
    let mut best_lit = 0u32;
    for (i, row) in rows.iter().enumerate() {
        let lit: u32 = row.chars().map(braille_dots).sum();
        total += lit;
        if lit > best_lit {
            best_lit = lit;
            best = i;
        }
    }
    (total, best)
}

impl ComponentDef for OledSsd1306 {
    fn kind(&self) -> &'static str {
        "oled_ssd1306"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["oled", "ssd1306"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "oled_ssd1306".into(),
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
        match &state.oled {
            Some(rows) if !rows.is_empty() => {
                let (total, best) = frame_stats(rows);
                let preview: String = rows[best].chars().take(24).collect();
                format!(
                    "{} ───●─── ▦ {} [OLED 128x64 {}px {}]",
                    pin_label_short(pin),
                    comp.id,
                    total,
                    preview
                )
            }
            _ => format!(
                "{} ───●─── ▦ {} [OLED 128x64 (blank)]",
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
        match &state.oled {
            Some(rows) if !rows.is_empty() => {
                let (total, best) = frame_stats(rows);
                let preview: String = rows[best].chars().take(28).collect();
                Line::from(vec![
                    prefix,
                    Span::raw(format!("{} [OLED ", comp.id)),
                    Span::styled(preview, Style::default().fg(Color::Rgb(120, 200, 255))),
                    Span::raw(format!(" {}px]", total)),
                ])
            }
            _ => Line::from(vec![
                prefix,
                Span::styled(
                    format!("{} [OLED 128x64 blank]", comp.id),
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
        let comp = OledSsd1306.build("oled1".to_string(), &[]).unwrap();
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
        OledSsd1306.render_plain(&comp, &PinRef::BoardAnalog(4), &project, state, spec)
    }

    #[test]
    fn oled_reports_pixel_count() {
        // 一行含两个满点盲文字符(⣿ = 8 点)= 16 亮像素
        let mut rows: Vec<String> = vec![String::new(); 16];
        rows[5] = "⣿⣿".to_string();
        let state = RunState {
            oled: Some(rows),
            ..Default::default()
        };
        let out = render(&state);
        assert!(out.contains("16px"), "两个满点盲文 = 16 像素,got: {out}");
        assert!(out.contains('⣿'), "预览应取最密行,got: {out}");
    }

    #[test]
    fn oled_blank_before_any_frame() {
        assert!(render(&RunState::default()).contains("(blank)"));
        // 空帧数组也算 blank
        let state = RunState {
            oled: Some(vec![String::new(); 16]),
            ..Default::default()
        };
        assert!(render(&state).contains("0px"));
    }

    #[test]
    fn braille_dot_count() {
        assert_eq!(braille_dots('⣿'), 8); // U+28FF 全 8 点
        assert_eq!(braille_dots('⠀'), 0); // U+2800 空盲文
        assert_eq!(braille_dots(' '), 0); // 普通空格
    }

    #[test]
    fn oled_registered_with_aliases() {
        let r = crate::components::Registry::builtin();
        assert_eq!(r.resolve("oled").unwrap().kind(), "oled_ssd1306");
        assert_eq!(r.resolve("ssd1306").unwrap().kind(), "oled_ssd1306");
    }
}
