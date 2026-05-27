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
                let segs = seven_seg_segments(&comp.id, project, state, spec);
                let disp = seven_seg_display(&segs);
                format!("{} ───●─── [{}] 7SEG {}", pin_label, disp, comp.id)
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

/// 共阴 7 段查表。bits = `[a, b, c, d, e, f, g, dp]`,1=点亮。
///
/// - 全灭 → `" "`(待机空白)
/// - 命中 9 种标准段位组合 → `"0"`..`"9"`
/// - 任何非法组合(段亮了但拼不出十进制数字)→ `"-"`(防止把噪声当数字)
/// - `dp` 不参与字符识别,由 caller 决定是否拼成 `"3."`
fn segments_to_glyph(lit: &[bool; 8]) -> &'static str {
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

/// 扫 `project.wires`,把所有连到 `<comp_id>.seg_X` 的 board pin 找出来,
/// 经 [`pin_level`] 取电平,装回 `[a, b, c, d, e, f, g, dp]` 8 元数组。
///
/// terminal 名识别:`seg_a`..`seg_g`/`seg_dp` 主名 + `a`..`g`/`dp`/`dot` 别名
/// (与 `components/seven_segment.toml` 的 `aliases` 对齐)。
/// 未接线 / 接的不是 board pin 的段位 → 默认 `false`(灭)。
fn seven_seg_segments(
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
                if id == comp_id && !is_component(&p) =>
            {
                (p, terminal)
            }
            (Some(p), Some(PinRef::Component { id, terminal }))
                if id == comp_id && !is_component(&p) =>
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

/// 把段位数组渲染成屏显字符串:命中数字时 dp 亮 → `"3."`;其它情况(空白/破折号)dp 忽略。
fn seven_seg_display(segs: &[bool; 8]) -> String {
    let g = segments_to_glyph(segs);
    if segs[7] && g != " " && g != "-" {
        format!("{}.", g)
    } else {
        g.to_string()
    }
}

/// 把单根 wire 渲染成 mockup 风格的一行:
///   `PIN13 ●——[LED:led1 red ON #]`
///   `PIN02 ●——[Button:btn1 UP]`
///   `GND   ●——[Button:btn1 GND]`
///
/// LED 状态字符的颜色随 RunState.d13(只对 board pin 13 起效;其它 pin
/// 在 v2a 阶段 fall back 成静态 OFF 灰)。
fn wire_row_line<'a>(
    row: &WireRow<'a>,
    project: &Project,
    state: &RunState,
    spec: &BoardSpec,
) -> Line<'static> {
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
        "seven_segment" => {
            let segs = seven_seg_segments(&row.component.id, project, state, spec);
            let disp = seven_seg_display(&segs);
            let label = format!("[{}]", disp);
            Line::from(vec![
                Span::raw(format!(" {} ━━━━━━━━━━━━━━━━━━━━━━ ", pin_label)),
                Span::styled(label, Style::default().fg(Color::Rgb(255, 40, 40))),
                Span::raw(format!(" 7SEG {}", row.component.id)),
            ])
        }
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

    // ---- Step 5: 数码管 7 段真驱动 ----

    /// 段位查表必须把 0-9 全部命中。bit 顺序 = [a,b,c,d,e,f,g,dp]。
    /// 标准共阴段位码(dp 始终留 0):
    ///   0=abcdef  1=bc      2=abdeg   3=abcdg
    ///   4=bcfg    5=acdfg   6=acdefg  7=abc
    ///   8=abcdefg 9=abcdfg
    #[test]
    fn segments_to_glyph_covers_0_through_9() {
        let cases: &[(&[bool; 8], &str)] = &[
            // a    b    c    d    e    f    g    dp
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

    /// 任意段亮但拼不出 0-9 → "-"。
    /// 例:只点亮 a + g(顶杠 + 中杠)不是任何数字。
    #[test]
    fn segments_to_glyph_dash_for_illegal_pattern() {
        let mut segs = [false; 8];
        segs[0] = true; // a
        segs[6] = true; // g
        assert_eq!(segments_to_glyph(&segs), "-");
    }

    /// dp 不影响字符识别:dp 单独亮(其它段全灭)仍是 " ",
    /// 因为 seven_seg_display 显式跳过 " " 不拼小数点。
    #[test]
    fn segments_to_glyph_ignores_dp() {
        let mut segs = [false; 8];
        segs[7] = true; // dp only
        assert_eq!(segments_to_glyph(&segs), " ");
    }

    /// dp 与有效数字结合 → "3."
    #[test]
    fn seven_seg_display_appends_dp_when_digit_present() {
        // 3 = abcdg + dp
        let segs = [true, true, true, true, false, false, true, true];
        assert_eq!(seven_seg_display(&segs), "3.");
    }

    /// dp 与 "-" 一起不拼小数点(避免出 "-." 这种 garbage)
    #[test]
    fn seven_seg_display_skips_dp_for_dash() {
        let mut segs = [false; 8];
        segs[0] = true; // a — 拼不出数字
        segs[6] = true; // g
        segs[7] = true; // dp
        assert_eq!(seven_seg_display(&segs), "-");
    }

    /// 扫 wires 取 8 段电平:wire 用 "seg_a".."seg_g"/"seg_dp" 主名 + a..g/dp 别名
    /// 都能识别;接的不是 board pin 的段位保持 false。
    #[test]
    fn seven_seg_segments_reads_wired_pins() {
        let seg = make_comp("s1", "seven_segment");
        // 拼数字 "3"(a,b,c,d,g)用主名;dp 用别名 "dp" 走 alias 分支
        let wires = vec![
            crate::project::Wire { from: "board.D2".to_string(), to: "s1.seg_a".to_string() },
            crate::project::Wire { from: "board.D3".to_string(), to: "s1.seg_b".to_string() },
            crate::project::Wire { from: "board.D4".to_string(), to: "s1.seg_c".to_string() },
            crate::project::Wire { from: "board.D5".to_string(), to: "s1.seg_d".to_string() },
            crate::project::Wire { from: "board.D6".to_string(), to: "s1.seg_e".to_string() }, // 接但不点
            crate::project::Wire { from: "board.D7".to_string(), to: "s1.seg_f".to_string() }, // 接但不点
            crate::project::Wire { from: "board.D8".to_string(), to: "s1.seg_g".to_string() },
            crate::project::Wire { from: "board.D9".to_string(), to: "s1.dp".to_string() },
        ];
        let project = project_with(vec![seg], wires);

        let mut state = RunState::default();
        // a=D2/PD2, b=D3/PD3, c=D4/PD4, d=D5/PD5, g=D8/PB0
        state.pin_states.insert("D:2".to_string(), 1);
        state.pin_states.insert("D:3".to_string(), 1);
        state.pin_states.insert("D:4".to_string(), 1);
        state.pin_states.insert("D:5".to_string(), 1);
        state.pin_states.insert("B:0".to_string(), 1);

        let board = crate::boards::board_from_str("arduino-uno").unwrap();
        let segs = seven_seg_segments("s1", &project, &state, board.spec());
        assert_eq!(segs, [true, true, true, true, false, false, true, false],
            "expected '3' pattern segments");
        assert_eq!(segments_to_glyph(&segs), "3");
    }

    /// 没接线的段位默认灭 → 整体仍是空白。
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
