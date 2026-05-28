use super::util::{pin_label_padded, pin_label_short, pin_level};
use super::ComponentDef;
use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::project::{Component, Project};
use crate::sim::{LedLevel, RunState};
use anyhow::Result;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

pub struct SevenSegment;

impl ComponentDef for SevenSegment {
    fn kind(&self) -> &'static str {
        "seven_segment"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["7seg"]
    }

    fn build(&self, id: String, _args: &[String]) -> Result<Component> {
        Ok(Component {
            id,
            kind: "seven_segment".into(),
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
        let segs = seven_seg_segments(&comp.id, project, state, spec);
        let disp = seven_seg_display(&segs);
        format!(
            "{} ───●─── [{}] 7SEG {}",
            pin_label_short(pin),
            disp,
            comp.id
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
        let segs = seven_seg_segments(&comp.id, project, state, spec);
        let disp = seven_seg_display(&segs);
        let label = format!("[{}]", disp);
        Line::from(vec![
            Span::raw(format!(
                " {} ━━━━━━━━━━━━━━━━━━━━━━ ",
                pin_label_padded(pin)
            )),
            Span::styled(label, Style::default().fg(Color::Rgb(255, 40, 40))),
            Span::raw(format!(" 7SEG {}", comp.id)),
        ])
    }
}

/// 共阴 7 段查表。bits = `[a, b, c, d, e, f, g, dp]`,1=点亮。
/// 全灭 → `" "`;命中 9 种标准段位 → `"0"`..`"9"`;非法 → `"-"`;dp 不参与识别。
pub(crate) fn segments_to_glyph(lit: &[bool; 8]) -> &'static str {
    let key: u8 = (lit[0] as u8) << 6
        | (lit[1] as u8) << 5
        | (lit[2] as u8) << 4
        | (lit[3] as u8) << 3
        | (lit[4] as u8) << 2
        | (lit[5] as u8) << 1
        | (lit[6] as u8);
    match key {
        0b0000000 => " ",
        0b1111110 => "0",
        0b0110000 => "1",
        0b1101101 => "2",
        0b1111001 => "3",
        0b0110011 => "4",
        0b1011011 => "5",
        0b1011111 => "6",
        0b1110000 => "7",
        0b1111111 => "8",
        0b1111011 => "9",
        _ => "-",
    }
}

/// 扫 wires 把所有连到 `<comp_id>.seg_X` 的板 pin 找出来,经 `pin_level` 取电平,
/// 装回 8 元数组。terminal 识别 `seg_a..seg_g`/`seg_dp` 主名 + `a..g`/`dp`/`dot` 别名。
pub(crate) fn seven_seg_segments(
    comp_id: &str,
    project: &Project,
    state: &RunState,
    spec: &BoardSpec,
) -> [bool; 8] {
    let mut seg = [false; 8];
    for w in &project.wires {
        let from = PinRef::parse(&w.from).ok();
        let to = PinRef::parse(&w.to).ok();
        let (pin_ref, terminal) = match (from, to) {
            (Some(PinRef::Component { id, terminal }), Some(p))
                if id == comp_id && !matches!(p, PinRef::Component { .. }) =>
            {
                (p, terminal)
            }
            (Some(p), Some(PinRef::Component { id, terminal }))
                if id == comp_id && !matches!(p, PinRef::Component { .. }) =>
            {
                (p, terminal)
            }
            _ => continue,
        };
        let idx = match terminal.as_str() {
            "seg_a" | "a" => 0,
            "seg_b" | "b" => 1,
            "seg_c" | "c" => 2,
            "seg_d" | "d" => 3,
            "seg_e" | "e" => 4,
            "seg_f" | "f" => 5,
            "seg_g" | "g" => 6,
            "seg_dp" | "dp" | "dot" => 7,
            _ => continue,
        };
        seg[idx] = pin_level(&pin_ref, state, spec) == LedLevel::On;
    }
    seg
}

/// 命中数字且 dp 亮 → `"3."`;其它情况(空白/破折号)dp 忽略。
pub(crate) fn seven_seg_display(segs: &[bool; 8]) -> String {
    let g = segments_to_glyph(segs);
    if segs[7] && g != " " && g != "-" {
        format!("{}.", g)
    } else {
        g.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Component, Project, ProjectMeta, Wire};

    fn make_comp(id: &str, kind: &str) -> Component {
        Component {
            id: id.to_string(),
            kind: kind.to_string(),
            color: None,
            pos: None,
            ohms: None,
            max_ohms: None,
            wire_color: None,
        }
    }

    fn project_with(components: Vec<Component>, wires: Vec<Wire>) -> Project {
        Project {
            project: ProjectMeta {
                name: "test".to_string(),
                board: "arduino-uno".to_string(),
                version: "0.2".to_string(),
            },
            components,
            wires,
            code: None,
        }
    }

    #[test]
    fn segments_to_glyph_covers_0_through_9() {
        let cases: &[(&[bool; 8], &str)] = &[
            (&[true, true, true, true, true, true, false, false], "0"),
            (&[false, true, true, false, false, false, false, false], "1"),
            (&[true, true, false, true, true, false, true, false], "2"),
            (&[true, true, true, true, false, false, true, false], "3"),
            (&[false, true, true, false, false, true, true, false], "4"),
            (&[true, false, true, true, false, true, true, false], "5"),
            (&[true, false, true, true, true, true, true, false], "6"),
            (&[true, true, true, false, false, false, false, false], "7"),
            (&[true, true, true, true, true, true, true, false], "8"),
            (&[true, true, true, true, false, true, true, false], "9"),
        ];
        for (segs, expect) in cases {
            assert_eq!(
                segments_to_glyph(segs),
                *expect,
                "segments {:?} should map to {}",
                segs,
                expect
            );
        }
    }

    #[test]
    fn segments_to_glyph_blank_when_all_off() {
        assert_eq!(segments_to_glyph(&[false; 8]), " ");
    }

    #[test]
    fn segments_to_glyph_dash_for_illegal_pattern() {
        let mut segs = [false; 8];
        segs[0] = true;
        segs[6] = true;
        assert_eq!(segments_to_glyph(&segs), "-");
    }

    #[test]
    fn segments_to_glyph_ignores_dp() {
        let mut segs = [false; 8];
        segs[7] = true;
        assert_eq!(segments_to_glyph(&segs), " ");
    }

    #[test]
    fn seven_seg_display_appends_dp_when_digit_present() {
        let segs = [true, true, true, true, false, false, true, true];
        assert_eq!(seven_seg_display(&segs), "3.");
    }

    #[test]
    fn seven_seg_display_skips_dp_for_dash() {
        let mut segs = [false; 8];
        segs[0] = true;
        segs[6] = true;
        segs[7] = true;
        assert_eq!(seven_seg_display(&segs), "-");
    }

    #[test]
    fn seven_seg_segments_reads_wired_pins() {
        let seg = make_comp("s1", "seven_segment");
        let wires = vec![
            Wire { from: "board.D2".to_string(), to: "s1.seg_a".to_string() },
            Wire { from: "board.D3".to_string(), to: "s1.seg_b".to_string() },
            Wire { from: "board.D4".to_string(), to: "s1.seg_c".to_string() },
            Wire { from: "board.D5".to_string(), to: "s1.seg_d".to_string() },
            Wire { from: "board.D6".to_string(), to: "s1.seg_e".to_string() },
            Wire { from: "board.D7".to_string(), to: "s1.seg_f".to_string() },
            Wire { from: "board.D8".to_string(), to: "s1.seg_g".to_string() },
            Wire { from: "board.D9".to_string(), to: "s1.dp".to_string() },
        ];
        let project = project_with(vec![seg], wires);

        let mut state = RunState::default();
        state.pin_states.insert("D:2".to_string(), 1);
        state.pin_states.insert("D:3".to_string(), 1);
        state.pin_states.insert("D:4".to_string(), 1);
        state.pin_states.insert("D:5".to_string(), 1);
        state.pin_states.insert("B:0".to_string(), 1);

        let board = crate::boards::board_from_str("arduino-uno").unwrap();
        let segs = seven_seg_segments("s1", &project, &state, board.spec());
        assert_eq!(
            segs,
            [true, true, true, true, false, false, true, false],
            "expected '3' pattern segments"
        );
        assert_eq!(segments_to_glyph(&segs), "3");
    }

    #[test]
    fn seven_seg_segments_unwired_defaults_off() {
        let seg = make_comp("s1", "seven_segment");
        let project = project_with(vec![seg], vec![]);
        let state = RunState::default();
        let board = crate::boards::board_from_str("arduino-uno").unwrap();
        let segs = seven_seg_segments("s1", &project, &state, board.spec());
        assert_eq!(segs, [false; 8]);
        assert_eq!(segments_to_glyph(&segs), " ");
    }
}
