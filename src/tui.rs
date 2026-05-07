//! TUI 主体。
//!
//! 当前职责(T2 + T4 + T5 + T6 + T7):
//! - 进 alternate screen + raw mode、隐光标(光标在 set_cursor_position 时由
//!   ratatui 自行 Show)
//! - 上区块按 30 FPS 重绘 styled frame(电路 ASCII + LED truecolor)
//! - 中间一行 toast(成功 ✓ 绿 / 失败 ✗ 红,2 秒淡出)
//! - 底部一行输入条 ▶ buffer,支持 buffer / 光标 / 历史 / Backspace / Ctrl-C 清空
//! - Enter 调 `Shell::dispatch`,结果进 toast
//! - ESC 退出、Drop 收尾

use anyhow::{Context, Result};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::{Alignment, Constraint, Layout, Position};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::io;
use std::time::{Duration, Instant};

const TOAST_TTL: Duration = Duration::from_secs(2);

/// RAII 终端守卫:`new()` 时初始化(raw mode + alt screen + hide cursor),
/// `drop()` 时反向收尾。每一步失败仅 `eprintln!` 到 stderr,绝不 panic / unwrap。
/// 这样即使初始化部分失败,Drop 仍会尽力还原终端,避免用户终端坏掉。
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

/// 输入条状态:用 `Vec<char>` 维护以避开 UTF-8 字节边界陷阱。
/// `cursor` 是 char 索引;`history_idx == None` 表示当前是新输入态。
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

    /// 清空当前行(Ctrl-C),不退出 TUI
    fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.history_idx = None;
    }

    /// 提交当前 buffer:返回提交内容,清空 buffer + 写入 history。
    /// 空 buffer 返回 None,不入 history。
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
                // 越过最新一条 → 回到新输入态
                self.buffer.clear();
                self.cursor = 0;
                self.history_idx = None;
            }
            None => {}
        }
    }
}

pub fn run(shell: &mut crate::shell::Shell) -> Result<()> {
    let _guard = TerminalGuard::new();
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).context("create ratatui terminal")?;
    let mut input = InputState::new();
    let mut last_message: Option<(String, Instant, Severity)> = None;

    loop {
        // 过期 toast 主动清掉,避免渲染时刷一次空白
        if let Some((_, ts, _)) = last_message.as_ref() {
            if ts.elapsed() >= TOAST_TTL {
                last_message = None;
            }
        }

        terminal
            .draw(|frame| {
                let chunks = Layout::vertical([
                    Constraint::Min(0),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(frame.area());

                // std::sync::Mutex 不可重入,这里**单次加锁**同时取 elapsed 和 lines
                let (title, lines) = match shell.running.as_ref() {
                    Some(sim) => {
                        let st = sim.state.lock().unwrap();
                        let title = format!(
                            "moxin · {} · t={:06.3}s",
                            shell.project.project.board,
                            st.started.elapsed().as_secs_f64()
                        );
                        let lines = crate::render::render_runtime_frame_styled(
                            &shell.project,
                            &st,
                        );
                        (title, lines)
                    }
                    None => {
                        let title = format!("moxin · {}", shell.project.project.board);
                        let lines = crate::render::render_project_styled(&shell.project);
                        (title, lines)
                    }
                };

                // 状态角标:error(2s 内)优先于 run / idle。
                // last_message 在循环开头已经按 TOAST_TTL=2s 主动清掉,所以只要它还在
                // 且 severity 是 Error,就是"最近 2s 内有错"。
                let (status_text, status_color) = match (
                    last_message.as_ref().map(|(_, _, s)| *s),
                    shell.running.is_some(),
                ) {
                    (Some(Severity::Error), _) => ("✗ error", Color::Red),
                    (_, true) => ("● run", Color::Green),
                    _ => ("○ idle", Color::DarkGray),
                };
                let status_line = Line::from(Span::styled(
                    status_text.to_string(),
                    Style::default().fg(status_color),
                ))
                .alignment(Alignment::Right);

                let block = Block::default()
                    .title(title)
                    .title_top(status_line)
                    .borders(Borders::ALL);
                frame.render_widget(Paragraph::new(lines).block(block), chunks[0]);

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
                    frame.render_widget(Paragraph::new(toast), chunks[1]);
                }

                let buf_str = input.buffer_string();
                let prompt_line = format!("▶ {}", buf_str);
                frame.render_widget(Paragraph::new(prompt_line), chunks[2]);

                // 光标:`▶` (1 列) + 空格 (1 列) + buffer 中 cursor 之前的字符数
                let cursor_x = chunks[2].x + 2 + input.cursor as u16;
                let cursor_y = chunks[2].y;
                frame.set_cursor_position(Position::new(cursor_x, cursor_y));
            })
            .context("draw frame")?;

        if event::poll(Duration::from_millis(33)).context("event poll")? {
            if let Event::Key(key) = event::read().context("event read")? {
                match key.code {
                    KeyCode::Esc => break,
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
                        } else if !ctrl && !alt {
                            input.insert_char(c);
                        }
                        // 其它组合键忽略
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}



