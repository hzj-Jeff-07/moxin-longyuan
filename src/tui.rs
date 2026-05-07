//! TUI 主体。
//!
//! 当前职责(T2 + T4 + T5 + T6):
//! - 进 alternate screen + raw mode、隐光标(光标在 set_cursor_position 时由
//!   ratatui 自行 Show)
//! - 上区块按 30 FPS 重绘 styled frame(电路 ASCII + LED truecolor)
//! - 底部一行输入条 ▶ buffer,支持 buffer / 光标 / 历史 / Backspace / Ctrl-C 清空
//! - ESC 退出、Drop 收尾
//!
//! T6 阶段 Enter 仅 push 到 history,**不调用 Shell::dispatch**(那是 T7)。

use anyhow::{Context, Result};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::io;
use std::time::Duration;

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

    loop {
        terminal
            .draw(|frame| {
                let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)])
                    .split(frame.area());

                let lines = match shell.running.as_ref() {
                    Some(sim) => crate::render::render_runtime_frame_styled(
                        &shell.project,
                        &sim.state.lock().unwrap(),
                    ),
                    None => crate::render::render_project_styled(&shell.project),
                };
                let block = Block::default().title("moxin").borders(Borders::ALL);
                frame.render_widget(Paragraph::new(lines).block(block), chunks[0]);

                let buf_str = input.buffer_string();
                let prompt_line = format!("▶ {}", buf_str);
                frame.render_widget(Paragraph::new(prompt_line), chunks[1]);

                // 光标:`▶` (1 列) + 空格 (1 列) + buffer 中 cursor 之前的字符数
                let cursor_x = chunks[1].x + 2 + input.cursor as u16;
                let cursor_y = chunks[1].y;
                frame.set_cursor_position(Position::new(cursor_x, cursor_y));
            })
            .context("draw frame")?;

        if event::poll(Duration::from_millis(33)).context("event poll")? {
            if let Event::Key(key) = event::read().context("event read")? {
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        // T6: 仅 push 到 history,T7 才接 dispatch
                        let _ = input.submit();
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


