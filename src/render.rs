use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::components::registry;
use crate::project::{Component, Project};
use crate::sim::{LedLevel, RunState};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const FRAME_INNER_W: usize = 48; // 内容区宽度(不含两侧 │)

/// 渲染运行时 ASCII 一帧 (`show` 命令,piped / --no-tui 模式)
pub fn render_runtime_frame(project: &Project, state: &RunState, spec: &BoardSpec) -> String {
    let elapsed = state.started.elapsed().as_secs_f64();
    let title = format!(
        " moxin · {} · t={:06.3}s ",
        project.project.board, elapsed
    );
    let mut lines: Vec<String> = vec![header_line(&title), border_line()];

    let reg = registry();
    for w in &project.wires {
        let from = PinRef::parse(&w.from).ok();
        let to = PinRef::parse(&w.to).ok();
        let (pin_ref, comp_id) = match (&from, &to) {
            (Some(PinRef::Component { id, .. }), Some(p)) if !is_component(p) => (p.clone(), id.clone()),
            (Some(p), Some(PinRef::Component { id, .. })) if !is_component(p) => (p.clone(), id.clone()),
            _ => continue,
        };
        let comp = match project.components.iter().find(|c| c.id == comp_id) {
            Some(c) => c,
            None => continue,
        };
        let line = if let Some(def) = reg.resolve(&comp.kind) {
            def.render_plain(comp, &pin_ref, project, state, spec)
        } else {
            format!(
                "{} ───●─── {}:{}",
                pin_label_short(&pin_ref),
                comp.kind,
                comp.id
            )
        };
        lines.push(content_line(&line));
    }

    lines.push(footer_line());
    lines.join("\n")
}

fn header_line(title: &str) -> String {
    let dashes = FRAME_INNER_W
        .saturating_sub(title.chars().count());
    let mut s = String::from("┌─");
    s.push_str(title);
    for _ in 0..dashes.saturating_sub(2) {
        s.push('─');
    }
    s.push('┐');
    s
}

fn border_line() -> String {
    let mut s = String::from("│");
    for _ in 0..FRAME_INNER_W {
        s.push(' ');
    }
    s.push('│');
    s
}

fn footer_line() -> String {
    let mut s = String::from("└");
    for _ in 0..FRAME_INNER_W {
        s.push('─');
    }
    s.push('┘');
    s
}

fn content_line(inner: &str) -> String {
    let body = format!(" {}", inner);
    let pad = FRAME_INNER_W.saturating_sub(body.chars().count());
    let mut s = String::from("│");
    s.push_str(&body);
    for _ in 0..pad {
        s.push(' ');
    }
    s.push('│');
    s
}

fn pin_label_short(pin: &PinRef) -> String {
    match pin {
        PinRef::BoardDigital(n) => format!("D{}", n),
        PinRef::BoardAnalog(n) => format!("A{}", n),
        PinRef::BoardGnd => "GND".to_string(),
        PinRef::Board5V => "5V".to_string(),
        PinRef::BoardPort { port, pin } => format!("{}{}", port, pin),
        PinRef::Component { .. } => "?".to_string(),
    }
}

pub fn render_project(project: &Project) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "[project] name={}, board={}\n",
        project.project.name, project.project.board
    ));
    let comps = project
        .components
        .iter()
        .map(|c| {
            let extra = c
                .color
                .as_deref()
                .map(|x| format!("{} ", x))
                .unwrap_or_default();
            format!("{}({}{})", c.id, extra, c.kind)
        })
        .collect::<Vec<_>>()
        .join(", ");
    s.push_str(&format!(
        "[components] {}\n",
        if comps.is_empty() {
            "(none)".to_string()
        } else {
            comps
        }
    ));
    let wires = project
        .wires
        .iter()
        .map(|w| {
            let f = PinRef::parse(&w.from)
                .map(|p| p.render())
                .unwrap_or_else(|_| w.from.clone());
            let t = PinRef::parse(&w.to)
                .map(|p| p.render())
                .unwrap_or_else(|_| w.to.clone());
            format!("{} -> {}", f, t)
        })
        .collect::<Vec<_>>()
        .join(", ");
    s.push_str(&format!(
        "[wires] {}",
        if wires.is_empty() {
            "(none)".to_string()
        } else {
            wires
        }
    ));
    s
}

// ---- TUI 用的 styled 渲染(返回 Vec<Line<'static>>)----
//
// 旧 plain API(render_runtime_frame / render_project)继续给 piped/--no-tui 用。
// styled 版只产生**内容行**:Block 是唯一的框,这里不画 ┌─┐│└─┘。

/// 一条解析过的"接线行":一端是 board pin,另一端是 component。
/// 给 V1 通用接线渲染器用。
struct WireRow<'a> {
    pin: PinRef,
    component: &'a Component,
}

/// 扫描 project.wires,把每条 (board-pin, component) 类型的连线归一成 WireRow。
/// 同一组件被多根线连到不同 pin → 各自一行。
/// 顺序遵循 component add 顺序(spec 要求);忽略 component-component
/// / board-only / 解析失败的 wire。
fn collect_wire_rows<'a>(project: &'a Project) -> Vec<WireRow<'a>> {
    let mut rows: Vec<WireRow<'a>> = Vec::new();
    for w in &project.wires {
        let from = PinRef::parse(&w.from).ok();
        let to = PinRef::parse(&w.to).ok();
        let (pin_ref, comp_id) = match (from, to) {
            (Some(PinRef::Component { id, .. }), Some(p)) if !is_component(&p) => (p, id),
            (Some(p), Some(PinRef::Component { id, .. })) if !is_component(&p) => (p, id),
            _ => continue,
        };
        if let Some(c) = project.components.iter().find(|c| c.id == comp_id) {
            rows.push(WireRow {
                pin: pin_ref,
                component: c,
            });
        }
    }
    rows.sort_by_key(|r| {
        project
            .components
            .iter()
            .position(|c| c.id == r.component.id)
            .unwrap_or(usize::MAX)
    });
    rows
}

fn is_component(p: &PinRef) -> bool {
    matches!(p, PinRef::Component { .. })
}

fn pin_label_padded(pin: &PinRef) -> String {
    match pin {
        PinRef::BoardDigital(n) => format!("{:<4}", format!("D{}", n)),
        PinRef::BoardAnalog(n) => format!("{:<4}", format!("A{}", n)),
        PinRef::BoardGnd => "GND ".to_string(),
        PinRef::Board5V => "5V  ".to_string(),
        PinRef::BoardPort { port, pin } => format!("{:<4}", format!("{}{}", port, pin)),
        PinRef::Component { .. } => "?   ".to_string(),
    }
}

/// 把单根 wire 渲染成 mockup 风格的一行。
/// 调度走元件注册式 registry:`comp.kind` → `ComponentDef::render_styled`。
/// 未注册的 kind fallback 到通用 `kind:id` 格式。
fn wire_row_line<'a>(
    row: &WireRow<'a>,
    project: &Project,
    state: &RunState,
    spec: &BoardSpec,
) -> Line<'static> {
    let reg = registry();
    if let Some(def) = reg.resolve(&row.component.kind) {
        def.render_styled(row.component, &row.pin, project, state, spec)
    } else {
        Line::from(format!(
            " {} ━━━━━━━━━━━━━━━━━━━━━━ ● {}:{}",
            pin_label_padded(&row.pin),
            row.component.kind,
            row.component.id
        ))
    }
}

pub fn render_runtime_frame_styled(project: &Project, state: &RunState, spec: &'static BoardSpec) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let rows = collect_wire_rows(project);
    if rows.is_empty() {
        lines.push(Line::from(" (no wires yet — try `add led red --id led1` then `wire pin13 -> led1.a`)".to_string()));
    } else {
        for row in &rows {
            lines.push(wire_row_line(row, project, state, spec));
        }
    }

    // 板载 L LED:Arduino UNO / STM32 都跟 d13 联动(语义统一)
    let l_style = match state.d13 {
        LedLevel::On => Style::default().fg(Color::Rgb(40, 220, 80)),
        LedLevel::Off => Style::default().fg(Color::DarkGray),
    };
    lines.push(Line::from(vec![
        Span::raw("  L (built-in)  ".to_string()),
        Span::styled("●".to_string(), l_style),
    ]));

    lines
}

pub fn render_project_styled(project: &Project, spec: &'static BoardSpec) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let rows = collect_wire_rows(project);
    if rows.is_empty() {
        lines.push(Line::from(format!(
            "[project] name={}, board={}",
            project.project.name, project.project.board
        )));
        let comps = project
            .components
            .iter()
            .map(|c| {
                let extra = c
                    .color
                    .as_deref()
                    .map(|x| format!("{} ", x))
                    .unwrap_or_default();
                format!("{}({}{})", c.id, extra, c.kind)
            })
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(Line::from(format!(
            "[components] {}",
            if comps.is_empty() {
                "(none)".to_string()
            } else {
                comps
            }
        )));
        lines.push(Line::from(
            " (no wires yet — try `add led red --id led1` then `wire pin13 -> led1.a`)"
                .to_string(),
        ));
    } else {
        // idle 路径 → 没有 RunState,用 default 让 LED 走静态 OFF 渲染
        let idle_state = RunState::default();
        for row in &rows {
            lines.push(wire_row_line(row, project, &idle_state, spec));
        }
    }

    // 板载 L LED idle 状态 → 灰色 ○
    lines.push(Line::from(vec![
        Span::raw("  L (built-in)  ".to_string()),
        Span::styled("○".to_string(), Style::default().fg(Color::DarkGray)),
    ]));

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn project_with(components: Vec<Component>, wires: Vec<crate::project::Wire>) -> Project {
        Project {
            project: crate::project::ProjectMeta {
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
    fn render_runtime_frame_handles_phase2_components() {
        // Phase 2 新元件全部塞进一个项目,验证 render 不 panic 且都出现
        let mut r1 = make_comp("r1", "resistor");
        r1.ohms = Some(470);
        let bz = make_comp("bz1", "buzzer");
        let mut pot = make_comp("p1", "potentiometer");
        pot.max_ohms = Some(10_000);
        let seg = make_comp("s1", "seven_segment");
        let bb = make_comp("bb1", "breadboard");
        let mut dp = make_comp("w1", "dupont");
        dp.wire_color = Some("yellow".to_string());

        let wires = vec![
            crate::project::Wire { from: "board.D13".to_string(), to: "r1.a".to_string() },
            crate::project::Wire { from: "board.D13".to_string(), to: "bz1.a".to_string() },
            crate::project::Wire { from: "board.A0".to_string(), to: "p1.wiper".to_string() },
            crate::project::Wire { from: "board.D7".to_string(), to: "s1.a".to_string() },
            crate::project::Wire { from: "board.GND".to_string(), to: "bb1.a".to_string() },
            crate::project::Wire { from: "board.D2".to_string(), to: "w1.a".to_string() },
        ];
        let project = project_with(vec![r1, bz, pot, seg, bb, dp], wires);
        let state = RunState::default();
        let board = crate::boards::board_from_str("arduino-uno").unwrap();
        let out = render_runtime_frame(&project, &state, board.spec());

        assert!(out.contains("r1"), "resistor id missing: {}", out);
        assert!(out.contains("470Ω"), "resistance label missing");
        assert!(out.contains("BUZZ"), "buzzer label missing");
        assert!(out.contains("POT"), "potentiometer label missing");
        assert!(out.contains("7SEG"), "seven_segment label missing");
        assert!(out.contains("BREADBOARD"), "breadboard label missing");
        assert!(out.contains("WIRE"), "dupont label missing");
        assert!(out.contains("yellow"), "dupont color missing");
    }

    #[test]
    fn render_runtime_frame_phase2_with_d13_on_lights_buzzer() {
        let bz = make_comp("bz1", "buzzer");
        let wires = vec![crate::project::Wire {
            from: "board.D13".to_string(),
            to: "bz1.a".to_string(),
        }];
        let project = project_with(vec![bz], wires);
        let state = RunState { d13: LedLevel::On, ..Default::default() };
        let board = crate::boards::board_from_str("arduino-uno").unwrap();
        let out = render_runtime_frame(&project, &state, board.spec());
        assert!(out.contains("BUZZ ON"), "expected buzzer ON when d13 high: {}", out);
    }

    /// Phase 2-mini Step 3 关键回归:非 D13 引脚的 LED 渲染也要跟着真 GPIO 走,
    /// 不再静态 OFF。D7 高 → "red ON #";D2 没事件 → "red OFF ."。
    #[test]
    fn render_runtime_frame_non_d13_led_reflects_pin_state() {
        let mut led_d7 = make_comp("led7", "led");
        led_d7.color = Some("red".to_string());
        let mut led_d2 = make_comp("led2", "led");
        led_d2.color = Some("red".to_string());
        let wires = vec![
            crate::project::Wire { from: "board.D7".to_string(), to: "led7.a".to_string() },
            crate::project::Wire { from: "board.D2".to_string(), to: "led2.a".to_string() },
        ];
        let project = project_with(vec![led_d7, led_d2], wires);

        let mut state = RunState::default();
        state.pin_states.insert("D:7".to_string(), 1);

        let board = crate::boards::board_from_str("arduino-uno").unwrap();
        let out = render_runtime_frame(&project, &state, board.spec());

        assert!(
            out.contains("led7 red ON"),
            "D7 LED should show ON when pin_states has D:7=1: {}",
            out
        );
        assert!(
            out.contains("led2 red OFF"),
            "D2 LED should show OFF without pin event: {}",
            out
        );
    }

    /// Buzzer 也要跟非 D13 引脚的真状态走。D5=1 → BUZZ ON。
    #[test]
    fn render_runtime_frame_non_d13_buzzer_reflects_pin_state() {
        let bz = make_comp("bz5", "buzzer");
        let wires = vec![crate::project::Wire {
            from: "board.D5".to_string(),
            to: "bz5.a".to_string(),
        }];
        let project = project_with(vec![bz], wires);
        let mut state = RunState::default();
        state.pin_states.insert("D:5".to_string(), 1);
        let board = crate::boards::board_from_str("arduino-uno").unwrap();
        let out = render_runtime_frame(&project, &state, board.spec());
        assert!(out.contains("BUZZ ON"), "D5 buzzer should be ON: {}", out);
    }

    /// 集成回归:render_runtime_frame 在 D2/D3/D4/D5/D8 全亮时输出 "[3]" 而不是 "[8]"。
    /// 这是 Phase 2-mini Step 5 拔掉硬编码 "[8]" 的证据。
    #[test]
    fn render_runtime_frame_seven_seg_shows_real_digit() {
        let seg = make_comp("s1", "seven_segment");
        let wires = vec![
            crate::project::Wire { from: "board.D2".to_string(), to: "s1.seg_a".to_string() },
            crate::project::Wire { from: "board.D3".to_string(), to: "s1.seg_b".to_string() },
            crate::project::Wire { from: "board.D4".to_string(), to: "s1.seg_c".to_string() },
            crate::project::Wire { from: "board.D5".to_string(), to: "s1.seg_d".to_string() },
            crate::project::Wire { from: "board.D8".to_string(), to: "s1.seg_g".to_string() },
        ];
        let project = project_with(vec![seg], wires);
        let mut state = RunState::default();
        state.pin_states.insert("D:2".to_string(), 1);
        state.pin_states.insert("D:3".to_string(), 1);
        state.pin_states.insert("D:4".to_string(), 1);
        state.pin_states.insert("D:5".to_string(), 1);
        state.pin_states.insert("B:0".to_string(), 1);
        let board = crate::boards::board_from_str("arduino-uno").unwrap();
        let out = render_runtime_frame(&project, &state, board.spec());
        assert!(out.contains("[3] 7SEG s1"), "expected real '3' display: {}", out);
        assert!(!out.contains("[8] 7SEG"), "should no longer hardcode [8]: {}", out);
    }
}
