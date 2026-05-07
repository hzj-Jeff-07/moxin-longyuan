//! TUI 主体。
//!
//! 当前职责(T2 + T4):进 alternate screen + raw mode、隐光标、按 30 FPS
//! 重绘 frame(电路 ASCII)、ESC 退出、Drop 收尾。
//!
//! 不在本阶段范围:染色(T5)、输入条(T6)、toast(T7)、调用 `Shell::dispatch`。

use anyhow::{Context, Result};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::{Constraint, Layout};
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

pub fn run(shell: &mut crate::shell::Shell) -> Result<()> {
    let _guard = TerminalGuard::new();
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).context("create ratatui terminal")?;

    loop {
        terminal
            .draw(|frame| {
                let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)])
                    .split(frame.area());

                let text = match shell.running.as_ref() {
                    Some(sim) => crate::render::render_runtime_frame(
                        &shell.project,
                        &sim.state.lock().unwrap(),
                    ),
                    None => crate::render::render_project(&shell.project),
                };
                let block = Block::default().title("moxin").borders(Borders::ALL);
                let p = Paragraph::new(text).block(block);
                frame.render_widget(p, chunks[0]);
                // chunks[1] 留空,T6 接管
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

