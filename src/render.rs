use crate::board::PinRef;
use crate::cmd_run::{LedLevel, RunState};
use crate::project::{Component, Project};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const FRAME_INNER_W: usize = 48; // 内容区宽度(不含两侧 │)

/// 渲染运行时 ASCII 一帧 (`show` 命令)
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

    // button:三行 ── 简化但保留剧本视觉
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
    // 单空格内边距 + 右侧填充到 FRAME_INNER_W
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

fn leds_connected_to_pin<'a>(project: &'a Project, pin_n: u8) -> Vec<&'a Component> {
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

// ---- T5: TUI 用的 styled 渲染(返回 Vec<Line<'static>>)----
//
// 旧 API(render_runtime_frame / render_project)继续给 piped/--no-tui 用,纯字符串。
// styled 版仅在 LED 状态字符上染色,其它行 1:1 复用 plain 版的字符布局,
// 保证视觉宽度跟 plain 版一致(便于人眼对齐)。

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

pub fn render_runtime_frame_styled(project: &Project, state: &RunState) -> Vec<Line<'static>> {
    let elapsed = state.started.elapsed().as_secs_f64();
    let title = format!(" moxin · {} · t={:06.3}s ", project.project.board, elapsed);
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(header_line(&title)));
    lines.push(Line::from(border_line()));

    let leds_on_d13 = leds_connected_to_pin(project, 13);
    if !leds_on_d13.is_empty() {
        let led = leds_on_d13[0];
        let label_color = led.color.as_deref().unwrap_or("red");
        // label_word 含尾空格,等同 plain 版 format_led 的 "{label} " 之后再接 marker
        let (label_word, marker, marker_style) = match state.d13 {
            LedLevel::On => (
                format!("{} ON ", label_color),
                "#",
                Style::default().fg(led_color(label_color)),
            ),
            LedLevel::Off => (
                format!("{} OFF ", label_color),
                ".",
                Style::default().fg(Color::DarkGray),
            ),
        };
        // 视觉布局严格复刻 content_line(plain LED 行):│ + " " + 内容 + 填充 + │
        let prefix = format!(" PIN13 ───●─── [LED:{} {}", led.id, label_word);
        let suffix = "]";
        let inner_w = prefix.chars().count() + 1 /* marker */ + suffix.chars().count();
        let pad = FRAME_INNER_W.saturating_sub(inner_w);
        lines.push(Line::from(vec![
            Span::raw("│"),
            Span::raw(prefix),
            Span::styled(marker.to_string(), marker_style),
            Span::raw(suffix.to_string()),
            Span::raw(" ".repeat(pad)),
            Span::raw("│"),
        ]));
    } else {
        lines.push(Line::from(content_line("PIN13 ───●─── (no LED wired)")));
    }

    let buttons: Vec<&Component> = project
        .components
        .iter()
        .filter(|c| c.kind == "button")
        .collect();
    if let Some(btn) = buttons.first() {
        if let Some((pin_n, _)) = find_button_pin(project, &btn.id) {
            lines.push(Line::from(content_line(&format!("PIN{:02} ───┐", pin_n))));
            lines.push(Line::from(content_line(&format!(
                "          ├── [Button:{} UP]",
                btn.id
            ))));
            lines.push(Line::from(content_line("GND  ────┘")));
        } else {
            lines.push(Line::from(content_line(&format!(
                "       [Button:{} UP (unwired)]",
                btn.id
            ))));
        }
    }

    lines.push(Line::from(footer_line()));
    lines
}

pub fn render_project_styled(project: &Project) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
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
    lines.push(Line::from(format!(
        "[wires] {}",
        if wires.is_empty() {
            "(none)".to_string()
        } else {
            wires
        }
    )));
    lines
}
