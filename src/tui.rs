//! TUI 最小骨架(T2)。
//!
//! 当前职责仅:进 alternate screen + raw mode、隐光标、画一个含
//! "ESC to quit" 字样的 Block、30 FPS draw + 事件双轮询、ESC 退出、Drop 收尾。
//!
//! 不在本阶段范围:渲染电路 frame、LED 状态、输入条、toast、染色、调用
//! `Shell::dispatch`。这些留给 T4–T7。
//! `shell` 参数已按 spec 定下,T3 起会真正用到。

use anyhow::{Context, Result};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
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

pub fn run(_shell: &mut crate::shell::Shell) -> Result<()> {
    let _guard = TerminalGuard::new();
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).context("create ratatui terminal")?;

    loop {
        terminal
            .draw(|frame| {
                let block = Block::default()
                    .title("moxin · TUI scaffold")
                    .borders(Borders::ALL);
                let p = Paragraph::new("ESC to quit").block(block);
                frame.render_widget(p, frame.area());
            })
            .context("draw frame")?;

        if event::poll(Duration::from_millis(33)).context("event poll")? {
            if let Event::Key(key) = event::read().context("event read")? {
                if key.code == KeyCode::Esc {
                    break;
                }
            }
        }
    }
    Ok(())
}
