//! TUI 主体。
//!
//! v2a 之后 (T8 / V1-V4) 的形态:
//! - alternate screen + raw mode、隐光标(光标在 set_cursor_position 时由
//!   ratatui 自行 Show)
//! - 多面板布局对齐 `docs/design/cli-vision.md`:左上板形 / 左下 Serial Monitor /
//!   右 AI Inspector / 底输入条 + toast
//! - 30 FPS 重绘
//! - 板形面板 title 右侧带状态角标(○ idle / ● run / ✗ error)
//! - 输入条 buffer / 光标 / 历史 / Backspace / Ctrl-C
//! - 窗口太窄时降级:<80 cols → 单列只渲染板形 + 输入条

use anyhow::{Context, Result};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::board::PinRef;
use crate::boards::BoardSpec;
use crate::llm::{self, LlmConfig};
use crate::sim::RunState;
use crate::inspector::{Inspector, InspectorLine, InspectorStatus, StubInspector};
use crate::project::Project;

const TOAST_TTL: Duration = Duration::from_secs(2);
const NARROW_WIDTH_THRESHOLD: u16 = 80;

/// AI Inspector 面板里 LLM 解读的实时状态(v3.2 M2)。后台 worker 线程写,渲染读。
enum LlmPanel {
    /// 未设 MOXIN_LLM_API_KEY —— LLM 功能关闭
    Disabled,
    /// 已启用,尚未问过
    Idle,
    /// 请求在途(渲染显示 analyzing…)
    Pending,
    /// 最近一次分析结果
    Ready(String),
    /// 最近一次错误(curl 缺失 / 网络 / API 报错)
    Error(String),
}

/// 触发一次 LLM 解读:置 Pending,后台线程跑 curl,回来写结果。**不阻塞渲染**。
/// 已在 Pending 时直接返回(不并发打多次)。
fn trigger_llm(shared: &Arc<Mutex<LlmPanel>>, cfg: LlmConfig, project: Project, snapshot: serde_json::Value) {
    {
        let mut p = match shared.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if matches!(*p, LlmPanel::Pending) {
            return;
        }
        *p = LlmPanel::Pending;
    }
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        let prompt = llm::build_prompt(&project, &snapshot);
        let body = llm::build_request_body(&cfg.model, &prompt);
        let result = llm::call_llm(&cfg, &body);
        if let Ok(mut p) = shared.lock() {
            *p = match result {
                Ok(ans) => LlmPanel::Ready(ans),
                Err(e) => LlmPanel::Error(e.to_string()),
            };
        }
    });
}

/// Ctrl+E 入口:从运行中的仿真取当前状态快照,触发 LLM 解读。返回给用户的 toast。
fn trigger_llm_from_tui(
    shell: &crate::shell::Shell,
    cfg: &LlmConfig,
    panel: &Arc<Mutex<LlmPanel>>,
) -> (String, Instant, Severity) {
    if !cfg.is_enabled() {
        return (
            "set MOXIN_LLM_API_KEY to enable AI explain".to_string(),
            Instant::now(),
            Severity::Error,
        );
    }
    let snapshot = shell
        .running
        .as_ref()
        .and_then(|sim| sim.state.lock().ok().map(|s| s.to_json()));
    match snapshot {
        Some(snap_json) => {
            trigger_llm(panel, cfg.clone(), shell.project.clone(), snap_json);
            ("asking LLM…".to_string(), Instant::now(), Severity::Success)
        }
        None => (
            "run the sim first, then Ctrl+E to explain".to_string(),
            Instant::now(),
            Severity::Error,
        ),
    }
}

/// 把 LLM 面板状态渲染成 AI Inspector 底部的若干行(owned,可在 draw 闭包外先算好)。
fn render_llm_panel(panel: &LlmPanel) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from("")];
    match panel {
        LlmPanel::Disabled => {
            lines.push(Line::from(Span::styled(
                "LLM: off (set MOXIN_LLM_API_KEY)".to_string(),
                Style::default().fg(Color::DarkGray),
            )));
        }
        LlmPanel::Idle => {
            lines.push(Line::from(Span::styled(
                "LLM: Ctrl+E to ask".to_string(),
                Style::default().fg(Color::Rgb(120, 200, 255)),
            )));
        }
        LlmPanel::Pending => {
            lines.push(Line::from(Span::styled(
                "LLM: analyzing…".to_string(),
                Style::default().fg(Color::Rgb(255, 200, 40)),
            )));
        }
        LlmPanel::Ready(answer) => {
            lines.push(Line::from(Span::styled(
                "LLM analysis (Ctrl+E refresh):".to_string(),
                Style::default().fg(Color::Rgb(40, 220, 80)),
            )));
            for l in answer.lines() {
                lines.push(Line::from(l.to_string()));
            }
        }
        LlmPanel::Error(e) => {
            lines.push(Line::from(Span::styled(
                "LLM error (Ctrl+E retry):".to_string(),
                Style::default().fg(Color::Rgb(255, 80, 80)),
            )));
            lines.push(Line::from(e.clone()));
        }
    }
    lines
}

struct TerminalGuard;

impl TerminalGuard {
    fn new() -> Self {
        if let Err(e) = enable_raw_mode() {
            eprintln!("enable_raw_mode failed: {}", e);
        }
        let mut out = io::stdout();
        if let Err(e) = execute!(out, EnterAlternateScreen, Hide) {
            eprintln!("enter alternate screen / hide cursor failed: {}", e);
        }
        TerminalGuard
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut out = io::stdout();
        if let Err(e) = execute!(out, Show, LeaveAlternateScreen) {
            eprintln!("show cursor / leave alternate screen failed: {}", e);
        }
        if let Err(e) = disable_raw_mode() {
            eprintln!("disable_raw_mode failed: {}", e);
        }
    }
}

#[derive(Clone, Copy)]
enum Severity {
    Success,
    Error,
}

struct InputState {
    buffer: Vec<char>,
    cursor: usize,
    history: Vec<String>,
    history_idx: Option<usize>,
}

impl InputState {
    fn new() -> Self {
        InputState {
            buffer: Vec::new(),
            cursor: 0,
            history: Vec::new(),
            history_idx: None,
        }
    }

    fn buffer_string(&self) -> String {
        self.buffer.iter().collect()
    }

    fn insert_char(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += 1;
        self.history_idx = None;
    }

    fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.buffer.remove(self.cursor);
            self.history_idx = None;
        }
    }

    fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.history_idx = None;
    }

    fn submit(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            return None;
        }
        let s: String = self.buffer.iter().collect();
        self.history.push(s.clone());
        self.buffer.clear();
        self.cursor = 0;
        self.history_idx = None;
        Some(s)
    }

    fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let new_idx = match self.history_idx {
            None => self.history.len() - 1,
            Some(i) if i > 0 => i - 1,
            Some(_) => 0,
        };
        self.history_idx = Some(new_idx);
        self.buffer = self.history[new_idx].chars().collect();
        self.cursor = self.buffer.len();
    }

    fn history_down(&mut self) {
        match self.history_idx {
            Some(i) if i + 1 < self.history.len() => {
                let next = i + 1;
                self.history_idx = Some(next);
                self.buffer = self.history[next].chars().collect();
                self.cursor = self.buffer.len();
            }
            Some(_) => {
                self.buffer.clear();
                self.cursor = 0;
                self.history_idx = None;
            }
            None => {}
        }
    }
}

/// 一帧渲染的入参快照。draw 闭包内只在最开头加锁一次取齐数据,
/// 然后整帧的渲染都不再碰 Mutex,避免 std::sync::Mutex 不可重入和
/// 锁占用时间过长。
struct FrameSnapshot {
    title: String,
    board_lines: Vec<Line<'static>>,
    serial_lines: Vec<String>,   // 最近 N 行原始 line(已截掉 t_us)
    inspector_lines: Vec<InspectorLine>,
    inspector_status: InspectorStatus,
    status_text: &'static str,
    status_color: Color,
}

fn build_snapshot(
    shell: &crate::shell::Shell,
    last_message: &Option<(String, Instant, Severity)>,
) -> FrameSnapshot {
    let project = &shell.project;
    let spec = shell.board.spec();
    let inspector = StubInspector;

    let (title, board_lines, serial_lines, inspector_lines, inspector_status, status_text, status_color) =
        match shell.running.as_ref() {
            Some(sim) => {
                let Ok(st) = sim.state.lock() else {
                    let title = format!("{} · {}", project.project.name, project.project.board);
                    let board_lines = crate::render::render_project_styled(project, spec);
                    let idle = RunState::default();
                    let (insp_lines, insp_status) = inspector.inspect(project, &idle);
                    return FrameSnapshot { title, board_lines, serial_lines: vec![], inspector_lines: insp_lines, inspector_status: insp_status, status_text: "(state unavailable)", status_color: Color::DarkGray };
                };
                let title = format!(
                    "{} · {} · t={:06.3}s",
                    project.project.name,
                    project.project.board,
                    st.started.elapsed().as_secs_f64()
                );
                let board_lines = crate::render::render_runtime_frame_styled(project, &st, shell.board.spec());
                let serial: Vec<String> =
                    st.serial_lines.iter().map(|(_, s)| s.clone()).collect();
                let (insp_lines, insp_status) = inspector.inspect(project, &st);

                // 状态角标:error(2s 内)优先于 run / idle
                let (text, color) = match (
                    last_message.as_ref().map(|(_, _, s)| *s),
                    true,
                ) {
                    (Some(Severity::Error), _) => ("✗ error", Color::Red),
                    _ => ("● run", Color::Green),
                };

                (title, board_lines, serial, insp_lines, insp_status, text, color)
            }
            None => {
                let title = format!(
                    "{} · {}",
                    project.project.name, project.project.board
                );
                let board_lines = crate::render::render_project_styled(project, spec);
                // idle 状态用 default RunState 喂 inspector,保持渲染对齐
                let idle = RunState::default();
                let (insp_lines, insp_status) = inspector.inspect(project, &idle);

                let (text, color) = match last_message.as_ref().map(|(_, _, s)| *s) {
                    Some(Severity::Error) => ("✗ error", Color::Red),
                    _ => ("○ idle", Color::DarkGray),
                };

                (title, board_lines, Vec::new(), insp_lines, insp_status, text, color)
            }
        };

    FrameSnapshot {
        title,
        board_lines,
        serial_lines,
        inspector_lines,
        inspector_status,
        status_text,
        status_color,
    }
}

fn render_serial_lines(snap: &FrameSnapshot, area: Rect) -> Vec<Line<'static>> {
    if snap.serial_lines.is_empty() {
        return vec![Line::from(vec![Span::styled(
            "(waiting for serial output)".to_string(),
            Style::default().fg(Color::DarkGray),
        )])];
    }
    // area.height - 2 留给 block border
    let cap = area.height.saturating_sub(2) as usize;
    let take = snap.serial_lines.len().min(cap.max(1));
    let start = snap.serial_lines.len() - take;
    snap.serial_lines[start..]
        .iter()
        .map(|s| {
            Line::from(vec![
                Span::styled("> ".to_string(), Style::default().fg(Color::DarkGray)),
                Span::raw(s.clone()),
            ])
        })
        .collect()
}

fn render_inspector(snap: &FrameSnapshot) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(snap.inspector_lines.len() + 3);
    for il in &snap.inspector_lines {
        let icon_color = if il.icon == '✓' {
            Color::Rgb(40, 220, 80)
        } else {
            Color::DarkGray
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", il.icon), Style::default().fg(icon_color)),
            Span::raw(format!("{}: ", il.label)),
            Span::styled(il.value.clone(), Style::default().fg(il.color)),
        ]));
        // 加一行空白拉开行距,接近 mockup 视觉
        lines.push(Line::from(""));
    }
    // Status 段
    lines.push(Line::from(vec![
        Span::raw("Status: ".to_string()),
        Span::styled(
            snap.inspector_status.label.clone(),
            Style::default()
                .fg(snap.inspector_status.color)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(snap.inspector_status.note.clone()));
    lines
}

/// 电位器 `comp_id` 接的 A 引脚 → MCU ADC 通道(扫 project.wires)。
fn pot_adc_channel(project: &Project, spec: &BoardSpec, comp_id: &str) -> Option<u8> {
    for w in &project.wires {
        let from = PinRef::parse(&w.from).ok();
        let to = PinRef::parse(&w.to).ok();
        let (pin, id) = match (from, to) {
            (Some(PinRef::Component { id, .. }), Some(p)) => (p, id),
            (Some(p), Some(PinRef::Component { id, .. })) => (p, id),
            _ => continue,
        };
        if id == comp_id {
            if let PinRef::BoardAnalog(n) = pin {
                return spec.adc_channel_for(n);
            }
        }
    }
    None
}

/// Tab 循环聚焦的候选:项目里所有 ADC 旋钮件(电位器/光敏,按 add 顺序)。
fn knob_candidates(project: &Project) -> Vec<String> {
    let reg = crate::components::registry();
    project
        .components
        .iter()
        .filter(|c| reg.resolve(&c.kind).is_some_and(|d| d.adc_knob()))
        .map(|c| c.id.clone())
        .collect()
}

/// 聚焦电位器时的方向键动作 → 注入新 ADC 值,返回 toast 文案。
fn adjust_knob(
    shell: &mut crate::shell::Shell,
    comp_id: &str,
    key: KeyCode,
) -> Result<String> {
    let spec = shell.board.spec();
    let Some(ch) = pot_adc_channel(&shell.project, spec, comp_id) else {
        anyhow::bail!("{} 没接到 A 引脚 — 先 wire A0 -> {}.wiper", comp_id, comp_id);
    };
    let Some(sim) = shell.running.as_mut() else {
        anyhow::bail!("simulator not running — try `run`");
    };
    let current = sim
        .state
        .lock()
        .ok()
        .and_then(|s| s.adc_values.get(&ch).copied())
        .unwrap_or(512);
    let next: u16 = match key {
        KeyCode::Left => current.saturating_sub(32),
        KeyCode::Right => (current + 32).min(1023),
        KeyCode::Home => 0,
        KeyCode::End => 1023,
        _ => current,
    };
    sim.set_adc(ch, next)?;
    Ok(format!("{} → ch{} = {}", comp_id, ch, next))
}

pub fn run(shell: &mut crate::shell::Shell) -> Result<()> {
    let _guard = TerminalGuard::new();
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).context("create ratatui terminal")?;
    let mut input = InputState::new();
    let mut last_message: Option<(String, Instant, Severity)> = None;
    // Tab 聚焦的电位器 id(None = 无聚焦,方向键不拦截)
    let mut knob_focus: Option<String> = None;
    // AI Inspector 的 LLM 面板(v3.2 M2):后台 worker 写、渲染读,非阻塞
    let llm_cfg = LlmConfig::from_process_env();
    let llm_panel = Arc::new(Mutex::new(if llm_cfg.is_enabled() {
        LlmPanel::Idle
    } else {
        LlmPanel::Disabled
    }));

    loop {
        if let Some((_, ts, _)) = last_message.as_ref() {
            if ts.elapsed() >= TOAST_TTL {
                last_message = None;
            }
        }

        let snap = build_snapshot(shell, &last_message);
        // 读一次 LLM 面板状态,渲染成 owned 行(draw 闭包外先算,闭包里只 clone)
        let llm_lines = match llm_panel.lock() {
            Ok(p) => render_llm_panel(&p),
            Err(_) => Vec::new(),
        };

        terminal
            .draw(|frame| {
                let area = frame.area();

                // outer 垂直分:主区 / toast / input
                let outer = Layout::vertical([
                    Constraint::Min(0),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(area);

                let main_area = outer[0];
                let narrow = main_area.width < NARROW_WIDTH_THRESHOLD;

                // 状态角标(右对齐 title)
                let status_line = Line::from(Span::styled(
                    snap.status_text.to_string(),
                    Style::default().fg(snap.status_color),
                ))
                .alignment(Alignment::Right);

                if narrow {
                    // 降级:单列只渲染板形 + 输入条 + 一行 hint
                    let board_block = Block::default()
                        .title(format!("[{}]", snap.title))
                        .title_top(status_line)
                        .borders(Borders::ALL);
                    frame.render_widget(
                        Paragraph::new(snap.board_lines.clone()).block(board_block),
                        main_area,
                    );
                } else {
                    // main 横向分:左 60% / 右 40%
                    let cols = Layout::horizontal([
                        Constraint::Percentage(60),
                        Constraint::Percentage(40),
                    ])
                    .split(main_area);
                    let left = cols[0];
                    let right = cols[1];

                    // 左侧再纵向分:板形 60% / Serial 40%
                    let left_rows = Layout::vertical([
                        Constraint::Percentage(60),
                        Constraint::Percentage(40),
                    ])
                    .split(left);
                    let board_area = left_rows[0];
                    let serial_area = left_rows[1];

                    // 板形 panel
                    let board_block = Block::default()
                        .title(format!("[{}]", snap.title))
                        .title_top(status_line)
                        .borders(Borders::ALL);
                    frame.render_widget(
                        Paragraph::new(snap.board_lines.clone()).block(board_block),
                        board_area,
                    );

                    // Serial Monitor panel
                    let serial_block = Block::default()
                        .title("[Serial Monitor]")
                        .borders(Borders::ALL);
                    let serial_render = render_serial_lines(&snap, serial_area);
                    frame.render_widget(
                        Paragraph::new(serial_render).block(serial_block),
                        serial_area,
                    );

                    // AI Inspector panel(右)
                    let insp_block = Block::default()
                        .title("[AI Inspector]")
                        .borders(Borders::ALL);
                    let mut insp_render = render_inspector(&snap);
                    insp_render.extend(llm_lines.iter().cloned());
                    frame.render_widget(
                        Paragraph::new(insp_render).block(insp_block),
                        right,
                    );
                }

                // toast
                if let Some((msg, _, sev)) = last_message.as_ref() {
                    let (prefix, color) = match sev {
                        Severity::Success => ("✓ ", Color::Green),
                        Severity::Error => ("✗ ", Color::Red),
                    };
                    let style = Style::default().fg(color);
                    let toast = Line::from(vec![
                        Span::styled(prefix.to_string(), style),
                        Span::styled(msg.clone(), style),
                    ]);
                    frame.render_widget(Paragraph::new(toast), outer[1]);
                }

                // input bar
                let buf_str = input.buffer_string();
                let prompt_line = format!("moxin > {}", buf_str);
                frame.render_widget(Paragraph::new(prompt_line), outer[2]);

                // 光标:`moxin > ` 占 8 列,然后 buffer 中 cursor 之前的字符数
                let cursor_x = outer[2].x + 8 + input.cursor as u16;
                let cursor_y = outer[2].y;
                frame.set_cursor_position(Position::new(cursor_x, cursor_y));
            })
            .context("draw frame")?;

        if event::poll(Duration::from_millis(33)).context("event poll")? {
            if let Event::Key(key) = event::read().context("event read")? {
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Tab => {
                        // Tab 在电位器之间循环聚焦:None → p1 → p2 → ... → None
                        let pots = knob_candidates(&shell.project);
                        if pots.is_empty() {
                            last_message = Some((
                                "no potentiometer in project".to_string(),
                                Instant::now(),
                                Severity::Error,
                            ));
                        } else {
                            let next = match knob_focus.as_deref() {
                                None => Some(pots[0].clone()),
                                Some(cur) => pots
                                    .iter()
                                    .position(|p| p == cur)
                                    .and_then(|i| pots.get(i + 1))
                                    .cloned(),
                            };
                            let msg = match &next {
                                Some(id) => format!("knob: {} (←/→ adjust, Home/End min/max, Tab next)", id),
                                None => "knob focus off".to_string(),
                            };
                            knob_focus = next;
                            last_message = Some((msg, Instant::now(), Severity::Success));
                        }
                    }
                    KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End
                        if knob_focus.is_some() =>
                    {
                        let id = knob_focus.clone().unwrap_or_default();
                        match adjust_knob(shell, &id, key.code) {
                            Ok(msg) => {
                                last_message = Some((msg, Instant::now(), Severity::Success));
                            }
                            Err(e) => {
                                last_message =
                                    Some((format!("{}", e), Instant::now(), Severity::Error));
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(cmd) = input.submit() {
                            match shell.dispatch(&cmd) {
                                Ok(msg) if !msg.is_empty() => {
                                    last_message =
                                        Some((msg, Instant::now(), Severity::Success));
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    last_message = Some((
                                        format!("{}", e),
                                        Instant::now(),
                                        Severity::Error,
                                    ));
                                }
                            }
                            // flush stale events accumulated during blocking dispatch (e.g. stop)
                            while event::poll(Duration::from_millis(0)).unwrap_or(false) {
                                let _ = event::read();
                            }
                        }
                    }
                    KeyCode::Backspace => input.backspace(),
                    KeyCode::Up => input.history_up(),
                    KeyCode::Down => input.history_down(),
                    KeyCode::Char(c) => {
                        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                        let alt = key.modifiers.contains(KeyModifiers::ALT);
                        if ctrl && (c == 'c' || c == 'C') {
                            input.clear();
                        } else if ctrl && (c == 'e' || c == 'E') {
                            // Ctrl+E:触发一次 LLM 解读(后台线程,非阻塞)
                            let msg = trigger_llm_from_tui(shell, &llm_cfg, &llm_panel);
                            last_message = Some(msg);
                        } else if !ctrl && !alt {
                            // 仿真运行 + 输入条为空时,按键作为串口 RX 注入固件
                            // (走 bridge `serial` 命令,老版本的裸字节写 stdin 会被当命令行丢弃)。
                            let injected = input.buffer.is_empty()
                                && shell
                                    .running
                                    .as_mut()
                                    .map(|sim| sim.send_serial(&c.to_string()).is_ok())
                                    .unwrap_or(false);
                            if !injected {
                                input.insert_char(c);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

// 抑制 unused_import 警告 if Project not actually used in this file.
// (Reserved for future direct project access in TUI; harmless.)
#[allow(dead_code)]
fn _project_marker(_p: &Project) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// 提取一行里所有 span 的文本(便于断言渲染内容)。
    fn line_text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn llm_panel_disabled_shows_hint() {
        let lines = render_llm_panel(&LlmPanel::Disabled);
        let joined: String = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");
        assert!(joined.contains("MOXIN_LLM_API_KEY"), "got: {joined}");
    }

    #[test]
    fn llm_panel_idle_and_pending_have_states() {
        assert!(render_llm_panel(&LlmPanel::Idle)
            .iter()
            .any(|l| line_text(l).contains("Ctrl+E")));
        assert!(render_llm_panel(&LlmPanel::Pending)
            .iter()
            .any(|l| line_text(l).contains("analyzing")));
    }

    #[test]
    fn llm_panel_ready_renders_each_answer_line() {
        let lines = render_llm_panel(&LlmPanel::Ready("first line\nsecond line".to_string()));
        let joined: String = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");
        assert!(joined.contains("first line"), "got: {joined}");
        assert!(joined.contains("second line"), "got: {joined}");
    }

    #[test]
    fn llm_panel_error_surfaces_message() {
        let lines = render_llm_panel(&LlmPanel::Error("curl not found".to_string()));
        let joined: String = lines.iter().map(line_text).collect::<Vec<_>>().join("\n");
        assert!(joined.contains("curl not found"), "got: {joined}");
    }
}
