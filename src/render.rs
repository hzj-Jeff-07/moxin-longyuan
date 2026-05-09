use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::sim::{LedLevel, RunState};
use crate::project::{Component, Project};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const FRAME_INNER_W: usize = 48; // 内容区宽度(不含两侧 │)

/// 渲染运行时 ASCII 一帧 (`show` 命令,piped / --no-tui 模式)
pub fn render_runtime_frame(project: &Project, state: &RunState) -> String {
    let elapsed = state.started.elapsed().as_secs_f64();
    let title = format!(
        " moxin · {} · t={:06.3}s ",
        project.project.board, elapsed
    );
    let mut lines: Vec<String> = vec![header_line(&title), border_line()];

    let leds_on_d13 = leds_connected_to_pin(project, 13);
    let pin13_line = if !leds_on_d13.is_empty() {
        let led = leds_on_d13[0];
        let (label, marker) = format_led(led, state.d13);
        format!(
            "PIN13 ───●─── [LED:{} {} {}]",
            led.id, label, marker
        )
    } else {
        "PIN13 ───●─── (no LED wired)".to_string()
    };
    lines.push(content_line(&pin13_line));

    let buttons: Vec<&Component> = project
        .components
        .iter()
        .filter(|c| c.kind == "button")
        .collect();
    if let Some(btn) = buttons.first() {
        if let Some((pin_n, _)) = find_button_pin(project, &btn.id) {
            lines.push(content_line(&format!("PIN{:02} ───┐", pin_n)));
            lines.push(content_line(&format!("          ├── [Button:{} UP]", btn.id)));
            lines.push(content_line("GND  ────┘"));
        } else {
            lines.push(content_line(&format!(
                "       [Button:{} UP (unwired)]",
                btn.id
            )));
        }
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

fn format_led(led: &Component, level: LedLevel) -> (String, &'static str) {
    let color = led.color.as_deref().unwrap_or("red").to_string();
    match level {
        LedLevel::On => (format!("{} ON", color), "#"),
        LedLevel::Off => (format!("{} OFF", color), "."),
    }
}

fn leds_connected_to_pin(project: &Project, pin_n: u8) -> Vec<&Component> {
    let mut found = Vec::new();
    for w in &project.wires {
        let from = PinRef::parse(&w.from).ok();
        let to = PinRef::parse(&w.to).ok();
        let touches_pin = matches!(&from, Some(PinRef::BoardDigital(n)) if *n == pin_n)
            || matches!(&to, Some(PinRef::BoardDigital(n)) if *n == pin_n);
        if !touches_pin {
            continue;
        }
        let comp_ref = match (&from, &to) {
            (Some(PinRef::Component { id, .. }), _) => Some(id.clone()),
            (_, Some(PinRef::Component { id, .. })) => Some(id.clone()),
            _ => None,
        };
        if let Some(id) = comp_ref {
            if let Some(c) = project
                .components
                .iter()
                .find(|c| c.id == id && c.kind == "led")
            {
                if !found.iter().any(|x: &&Component| x.id == c.id) {
                    found.push(c);
                }
            }
        }
    }
    found
}

fn find_button_pin(project: &Project, btn_id: &str) -> Option<(u8, String)> {
    for w in &project.wires {
        let from = PinRef::parse(&w.from).ok();
        let to = PinRef::parse(&w.to).ok();
        let comp_term = |p: &Option<PinRef>| -> Option<String> {
            if let Some(PinRef::Component { id, terminal }) = p {
                if id == btn_id {
                    return Some(terminal.clone());
                }
            }
            None
        };
        let board_pin = |p: &Option<PinRef>| -> Option<u8> {
            if let Some(PinRef::BoardDigital(n)) = p {
                Some(*n)
            } else {
                None
            }
        };
        if let (Some(t), Some(n)) = (comp_term(&from), board_pin(&to)) {
            return Some((n, t));
        }
        if let (Some(t), Some(n)) = (comp_term(&to), board_pin(&from)) {
            return Some((n, t));
        }
    }
    None
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

/// 把 LED 颜色名字符串映射到 Color。未知颜色 fallback 到 white。
fn led_color(name: &str) -> Color {
    match name {
        "red" => Color::Rgb(255, 40, 40),
        "green" => Color::Rgb(40, 220, 80),
        "blue" => Color::Rgb(60, 120, 255),
        "yellow" => Color::Rgb(255, 200, 40),
        "white" => Color::Rgb(240, 240, 240),
        _ => Color::Rgb(240, 240, 240),
    }
}

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
    // 按 component add 顺序排序:rows 已按 wire 顺序遍历,这里再按 components
    // 在 project.components 里的索引重排,demo 单组件场景不会变,多组件多线
    // 场景下也保持稳定。
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

/// 把单根 wire 渲染成 mockup 风格的一行:
///   `PIN13 ●——[LED:led1 red ON #]`
///   `PIN02 ●——[Button:btn1 UP]`
///   `GND   ●——[Button:btn1 GND]`
///
/// LED 状态字符的颜色随 RunState.d13(只对 board pin 13 起效;其它 pin
/// 在 v2a 阶段 fall back 成静态 OFF 灰)。
fn wire_row_line<'a>(row: &WireRow<'a>, state: &RunState, spec: &BoardSpec) -> Line<'static> {
    let pin_label = match &row.pin {
        PinRef::BoardDigital(n) => format!("{:<4}", format!("D{}", n)),
        PinRef::BoardAnalog(n) => format!("{:<4}", format!("A{}", n)),
        PinRef::BoardGnd => "GND ".to_string(),
        PinRef::Board5V => "5V  ".to_string(),
        PinRef::BoardPort { port, pin } => format!("{:<4}", format!("{}{}", port, pin)),
        PinRef::Component { .. } => "?   ".to_string(),
    };

    match row.component.kind.as_str() {
        "led" => {
            let color_name = row.component.color.as_deref().unwrap_or("red");
            let level = if spec.is_d13_pin(&row.pin) { state.d13 } else { LedLevel::Off };
            let (state_word, marker, marker_style) = match level {
                LedLevel::On  => ("ON ", "●", Style::default().fg(led_color(color_name))),
                LedLevel::Off => ("OFF", "○", Style::default().fg(Color::DarkGray)),
            };
            let abbr: String = color_name.to_uppercase().chars().take(3).collect();
            Line::from(vec![
                Span::raw(format!(" {} ━━━━━━━━━━━━━━━━━━━━━━ ", pin_label)),
                Span::styled(marker.to_string(), marker_style),
                Span::raw(format!(" {} [{} {}]", row.component.id, abbr, state_word)),
            ])
        }
        "button" => Line::from(format!(
            " {} ━━━━━━━━━━━━━━━━━━━━━━ ● {} [BTN {}]",
            pin_label, row.component.id,
            if state.button_pressed { "DOWN" } else { "UP" }
        )),
        other => Line::from(format!(
            " {} ━━━━━━━━━━━━━━━━━━━━━━ ● {}:{}",
            pin_label, other, row.component.id
        )),
    }
}

pub fn render_runtime_frame_styled(project: &Project, state: &RunState, spec: &'static BoardSpec) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let rows = collect_wire_rows(project);
    if rows.is_empty() {
        lines.push(Line::from(" (no wires yet — try `add led red --id led1` then `wire pin13 -> led1.a`)".to_string()));
    } else {
        for row in &rows {
            lines.push(wire_row_line(row, state, spec));
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
            lines.push(wire_row_line(row, &idle_state, spec));
        }
    }

    // 板载 L LED idle 状态 → 灰色 ○
    lines.push(Line::from(vec![
        Span::raw("  L (built-in)  ".to_string()),
        Span::styled("○".to_string(), Style::default().fg(Color::DarkGray)),
    ]));

    lines
}
