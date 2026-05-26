use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::sim::{LedLevel, RunState};
use crate::project::{Component, Project};
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

    // 遍历所有 wire 中涉及 component 的连接
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
        let pin_label = match &pin_ref {
            PinRef::BoardDigital(n) => format!("D{}", n),
            PinRef::BoardAnalog(n) => format!("A{}", n),
            PinRef::BoardGnd => "GND".to_string(),
            PinRef::Board5V => "5V".to_string(),
            PinRef::BoardPort { port, pin } => format!("{}{}", port, pin),
            PinRef::Component { .. } => "?".to_string(),
        };
        let level = pin_level(&pin_ref, state, spec);
        let line = match comp.kind.as_str() {
            "led" => {
                let (label, marker) = format_led(comp, level);
                format!("{} ───●─── [LED:{} {} {}]", pin_label, comp.id, label, marker)
            }
            "button" => {
                let st = if state.button_pressed { "DOWN" } else { "UP" };
                format!("{} ───●─── [Button:{} {}]", pin_label, comp.id, st)
            }
            "resistor" => {
                let label = format_resistance(comp.ohms.unwrap_or(0));
                format!("{} ───┤▮▮▮▮├─── {} {}", pin_label, label, comp.id)
            }
            "buzzer" => {
                let st = if level == LedLevel::On { "ON" } else { "OFF" };
                format!("{} ───●─── ♪ {} [BUZZ {}]", pin_label, comp.id, st)
            }
            "potentiometer" => {
                let max = comp.max_ohms.map(format_resistance).unwrap_or_else(|| "10kΩ".to_string());
                format!("{} ───●─── ◎ {} [POT {}]", pin_label, comp.id, max)
            }
            "seven_segment" => {
                format!("{} ───●─── [8] 7SEG {}", pin_label, comp.id)
            }
            "breadboard" => {
                format!("{} ───●─── ▦ BREADBOARD {}", pin_label, comp.id)
            }
            "dupont" => {
                let color = comp.wire_color.as_deref().unwrap_or("red");
                format!("{} ━━━━━━━━━━━ WIRE {} [{}]", pin_label, comp.id, color)
            }
            other => {
                format!("{} ───●─── {}:{}", pin_label, other, comp.id)
            }
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

fn format_led(led: &Component, level: LedLevel) -> (String, &'static str) {
    let color = led.color.as_deref().unwrap_or("red").to_string();
    match level {
        LedLevel::On => (format!("{} ON", color), "#"),
        LedLevel::Off => (format!("{} OFF", color), "."),
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

/// 4-band 色环:digit1, digit2, multiplier, tolerance(金色 ±5%)。
/// 返回 4 个 ratatui Color。
fn resistance_color_rings(ohms: u32) -> [Color; 4] {
    let digit_color = |d: u8| -> Color {
        match d {
            0 => Color::Black,
            1 => Color::Rgb(139, 69, 19),   // brown
            2 => Color::Rgb(255, 40, 40),    // red
            3 => Color::Rgb(255, 165, 0),    // orange
            4 => Color::Rgb(255, 200, 40),   // yellow
            5 => Color::Rgb(40, 220, 80),    // green
            6 => Color::Rgb(60, 120, 255),   // blue
            7 => Color::Rgb(148, 0, 211),    // violet
            8 => Color::Rgb(128, 128, 128),  // gray
            9 => Color::Rgb(240, 240, 240),  // white
            _ => Color::Black,
        }
    };
    // 把 ohms 分解为 2位有效数字 + 10^n 乘数
    if ohms == 0 {
        return [Color::Black, Color::Black, Color::Black, Color::Rgb(218, 165, 32)];
    }
    let mut val = ohms;
    let mut mult: u8 = 0;
    while val >= 100 {
        val /= 10;
        mult += 1;
    }
    // val 现在是 10..99 的两位数
    let d1 = (val / 10) as u8;
    let d2 = (val % 10) as u8;
    [digit_color(d1), digit_color(d2), digit_color(mult), Color::Rgb(218, 165, 32)] // gold tolerance
}

/// 把 ohms 格式化为人类可读字符串:470Ω / 10kΩ / 1MΩ / 4.7kΩ
fn format_resistance(ohms: u32) -> String {
    if ohms >= 1_000_000 {
        let m = ohms as f64 / 1_000_000.0;
        if (m - m.round()).abs() < 0.01 {
            format!("{}MΩ", m as u32)
        } else {
            format!("{:.1}MΩ", m)
        }
    } else if ohms >= 1_000 {
        let k = ohms as f64 / 1_000.0;
        if (k - k.round()).abs() < 0.01 {
            format!("{}kΩ", k as u32)
        } else {
            format!("{:.1}kΩ", k)
        }
    } else {
        format!("{}Ω", ohms)
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

/// 把 (PinRef, RunState, BoardSpec) → LedLevel(渲染层用)。
///
/// 优先级:
/// 1. `spec.is_d13_pin(pin)` → 走 `state.d13`(保 stm32 D13 兼容,bridge 协议端口名不同)
/// 2. Arduino 数字引脚 D0-D13 → Step 2 的 `arduino_digital_to_port_bit` + `state.get_pin`
/// 3. Arduino 模拟引脚 A0-A5 → 同上(GPIO 视角,ADC 真采样推到 v0.5.0)
/// 4. 其它(stm32 BoardPort 非 D13 / GND / 5V / Component / 没事件)→ `Off`
///
/// 没收到过事件 = `Off`(保持老 UX:静态灰)。下一阶段如果要区分
/// `UNKNOWN`,改返回 `Option<LedLevel>` 即可,签名独立可演进。
fn pin_level(pin: &PinRef, state: &RunState, spec: &BoardSpec) -> LedLevel {
    if spec.is_d13_pin(pin) {
        return state.d13;
    }
    let level = match pin {
        PinRef::BoardDigital(n) => state.get_arduino_digital(*n),
        PinRef::BoardAnalog(n) => state.get_arduino_analog(*n),
        _ => None,
    };
    match level {
        Some(true) => LedLevel::On,
        _ => LedLevel::Off,
    }
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
            let level = pin_level(&row.pin, state, spec);
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
        "resistor" => {
            let ohms = row.component.ohms.unwrap_or(0);
            let rings = resistance_color_rings(ohms);
            let label = format_resistance(ohms);
            Line::from(vec![
                Span::raw(format!(" {} ━━━┤", pin_label)),
                Span::styled("▮", Style::default().fg(rings[0])),
                Span::styled("▮", Style::default().fg(rings[1])),
                Span::styled("▮", Style::default().fg(rings[2])),
                Span::styled("▮", Style::default().fg(rings[3])),
                Span::raw(format!("├━━━ {} {}", label, row.component.id)),
            ])
        }
        "buzzer" => {
            let is_on = pin_level(&row.pin, state, spec) == LedLevel::On;
            let (state_word, symbol, style) = if is_on {
                ("ON ", "♪", Style::default().fg(Color::Rgb(255, 200, 40)))
            } else {
                ("OFF", "♪", Style::default().fg(Color::DarkGray))
            };
            Line::from(vec![
                Span::raw(format!(" {} ━━━━━━━━━━━━━━━━━━━━━━ ", pin_label)),
                Span::styled(symbol.to_string(), style),
                Span::raw(format!(" {} [BUZZ {}]", row.component.id, state_word)),
            ])
        }
        "potentiometer" => Line::from(vec![
            Span::raw(format!(" {} ━━━━━━━━━━━━━━━━━━━━━━ ", pin_label)),
            Span::styled("◎", Style::default().fg(Color::Rgb(60, 120, 255))),
            Span::raw(format!(" {} [POT {}]",
                row.component.id,
                row.component.max_ohms.map(format_resistance).unwrap_or_else(|| "10kΩ".to_string()),
            )),
        ]),
        "seven_segment" => Line::from(vec![
            Span::raw(format!(" {} ━━━━━━━━━━━━━━━━━━━━━━ ", pin_label)),
            Span::styled("[8]", Style::default().fg(Color::Rgb(255, 40, 40))),
            Span::raw(format!(" 7SEG {}", row.component.id)),
        ]),
        "breadboard" => Line::from(format!(
            " {} ━━━━━━━━━━━━━━━━━━━━━━ ▦ BREADBOARD {}",
            pin_label, row.component.id
        )),
        "dupont" => {
            let color_name = row.component.wire_color.as_deref().unwrap_or("red");
            let wire_style = Style::default().fg(led_color(color_name));
            Line::from(vec![
                Span::raw(format!(" {} ", pin_label)),
                Span::styled("━━━━━━━━━━━━━━━━━━━━━━━━", wire_style),
                Span::raw(format!(" WIRE {}", row.component.id)),
            ])
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resistance_color_rings_470() {
        // 470 → val=47, mult=1 → digits 4,7 → multiplier brown(1)
        let rings = resistance_color_rings(470);
        assert_eq!(rings[0], Color::Rgb(255, 200, 40));  // yellow (4)
        assert_eq!(rings[1], Color::Rgb(148, 0, 211));   // violet (7)
        assert_eq!(rings[2], Color::Rgb(139, 69, 19));   // brown (1)
        assert_eq!(rings[3], Color::Rgb(218, 165, 32));  // gold tolerance
    }

    #[test]
    fn resistance_color_rings_10k() {
        // 10_000 → 10000/10=1000/10=100/10=10 → val=10, mult=3
        let rings = resistance_color_rings(10_000);
        assert_eq!(rings[0], Color::Rgb(139, 69, 19));  // brown (1)
        assert_eq!(rings[1], Color::Black);               // black (0)
        assert_eq!(rings[2], Color::Rgb(255, 165, 0));   // orange (3)
    }

    #[test]
    fn resistance_color_rings_1m() {
        // 1_000_000 → val=10, mult=5 → digits 1,0 → multiplier green(5)
        let rings = resistance_color_rings(1_000_000);
        assert_eq!(rings[0], Color::Rgb(139, 69, 19));  // brown (1)
        assert_eq!(rings[1], Color::Black);               // black (0)
        assert_eq!(rings[2], Color::Rgb(40, 220, 80));   // green (5)
    }

    #[test]
    fn format_resistance_values() {
        assert_eq!(format_resistance(470), "470Ω");
        assert_eq!(format_resistance(1_000), "1kΩ");
        assert_eq!(format_resistance(10_000), "10kΩ");
        assert_eq!(format_resistance(4_700), "4.7kΩ");
        assert_eq!(format_resistance(1_000_000), "1MΩ");
        assert_eq!(format_resistance(2_200_000), "2.2MΩ");
    }

    #[test]
    fn resistance_color_rings_zero() {
        let rings = resistance_color_rings(0);
        assert_eq!(rings[0], Color::Black);
        assert_eq!(rings[1], Color::Black);
        assert_eq!(rings[2], Color::Black);
    }

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
    /// 这条测试就是任务书红线"只有 D13 真实仿真"被拔掉的证据。
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

        // 模拟 bridge 推送 D7(PORTD bit 7)拉高;D2 始终静默 → 没事件
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
}
