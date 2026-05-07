use crate::board::{board_info, PinRef};
use crate::cmd_build::cmd_build;
use crate::cmd_run::{cmd_run, RunningSim};
use crate::project::{Component, Project, Wire};
use crate::render::{render_project, render_runtime_frame};
use anyhow::{Context, Result, anyhow, bail};
use rustyline::error::ReadlineError;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn cmd_shell(start_dir: &Path, no_tui: bool) -> Result<()> {
    let root = Project::find_project_root(start_dir)?;
    let project = Project::load(&root.join("moxin.toml"))?;

    let comp_count = project.components.len();
    let wire_count = project.wires.len();
    let summary = if comp_count == 0 && wire_count == 0 {
        "empty project".to_string()
    } else {
        format!("{} components, {} wires", comp_count, wire_count)
    };
    println!(
        "welcome to moxin shell · board={} · {}",
        project.project.board, summary
    );

    let mut shell = Shell {
        root,
        project,
        running: None,
    };

    let is_tty = io::stdin().is_terminal();
    if is_tty && !no_tui {
        crate::tui::run(&mut shell)?;
    } else if is_tty {
        repl_interactive(&mut shell)?;
    } else {
        repl_piped(&mut shell)?;
    }

    if let Some(sim) = shell.running.take() {
        sim.stop();
    }
    Ok(())
}

fn repl_interactive(shell: &mut Shell) -> Result<()> {
    let mut rl = rustyline::DefaultEditor::new().context("rustyline init")?;
    loop {
        let line = match rl.readline("moxin> ") {
            Ok(l) => l,
            Err(ReadlineError::Eof) | Err(ReadlineError::Interrupted) => {
                println!();
                break;
            }
            Err(e) => {
                eprintln!("readline error: {}", e);
                break;
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let _ = rl.add_history_entry(trimmed);
        if trimmed == "exit" || trimmed == "quit" {
            break;
        }
        match shell.dispatch(trimmed) {
            Ok(msg) if !msg.is_empty() => println!("{}", msg),
            Ok(_) => {}
            Err(e) => eprintln!("error: {}", e),
        }
    }
    Ok(())
}

fn repl_piped(shell: &mut Shell) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut handle = stdin.lock();
    let mut buf = String::new();
    loop {
        write!(out, "moxin> ")?;
        out.flush()?;
        buf.clear();
        match handle.read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let trimmed = buf.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        // 把命令回显一遍,便于从 transcript 中看清
        writeln!(out, "{}", trimmed)?;
        out.flush()?;
        if trimmed == "exit" || trimmed == "quit" {
            break;
        }
        match shell.dispatch(&trimmed) {
            Ok(msg) if !msg.is_empty() => writeln!(out, "{}", msg)?,
            Ok(_) => {}
            Err(e) => writeln!(out, "error: {}", e)?,
        }
        out.flush()?;
    }
    Ok(())
}

pub struct Shell {
    root: PathBuf,
    pub project: Project,
    pub running: Option<RunningSim>,
}

/// Panic 安全兜底:正常退出路径已经在 `cmd_shell` 末尾 `sim.stop()` 早释放,
/// Drop 只在 unwind / 异常返回时兜底,避免 sim 子进程变孤儿。
impl Drop for Shell {
    fn drop(&mut self) {
        if let Some(sim) = self.running.take() {
            sim.stop();
        }
    }
}

impl Shell {
    pub fn dispatch(&mut self, line: &str) -> Result<String> {
        // 特殊处理 `wire <from> -> <to>`,因为参数里有 `->`
        if let Some(rest) = line.strip_prefix("wire") {
            return self.cmd_wire(rest.trim());
        }
        let mut tokens = line.split_whitespace();
        let head = tokens.next().unwrap_or("");
        let rest: Vec<&str> = tokens.collect();
        match head {
            "board" => self.cmd_board(&rest),
            "add" => self.cmd_add(&rest),
            "show" => self.cmd_show(&rest),
            "edit" => self.cmd_edit(),
            "build" => self.cmd_build(),
            "run" => self.cmd_run_sim(),
            "stop" => self.cmd_stop(),
            "sleep" => self.cmd_sleep(&rest),
            "help" | "?" => Ok(help_text()),
            _ => bail!("unknown command: {} (try `help`)", head),
        }
    }

    fn cmd_board(&self, rest: &[&str]) -> Result<String> {
        match rest.first().copied() {
            Some("info") | None => Ok(board_info().to_string()),
            Some(other) => bail!("unknown board subcommand: {}", other),
        }
    }

    /// 添加元件: add <type> [args...] --id <id>
    /// 例: add led red --id led1
    /// 例: add button --id btn1
    fn cmd_add(&mut self, args: &[&str]) -> Result<String> {
        if args.is_empty() {
            bail!("usage: add <type> [color] --id <id>");
        }
        let kind = args[0].to_lowercase();
        let mut id: Option<String> = None;
        let mut positional: Vec<String> = Vec::new();
        let mut i = 1;
        while i < args.len() {
            let a = args[i];
            if a == "--id" {
                i += 1;
                if i >= args.len() {
                    bail!("--id requires a value");
                }
                id = Some(args[i].to_string());
            } else if let Some(v) = a.strip_prefix("--id=") {
                id = Some(v.to_string());
            } else {
                positional.push(a.to_string());
            }
            i += 1;
        }
        let id = id.ok_or_else(|| anyhow!("--id required"))?;

        let comp = match kind.as_str() {
            "led" => {
                let color = positional.first().cloned().unwrap_or_else(|| "red".into());
                Component {
                    id: id.clone(),
                    kind: "led".into(),
                    color: Some(color.clone()),
                    pos: None,
                }
            }
            "button" | "btn" => Component {
                id: id.clone(),
                kind: "button".into(),
                color: None,
                pos: None,
            },
            other => bail!("unknown component type: {}", other),
        };

        let display = match (&comp.color, comp.kind.as_str()) {
            (Some(c), "led") => format!("{} ({} led)", id, c),
            _ => id.clone(),
        };

        self.project.add_component(comp)?;
        self.save_project()?;
        Ok(format!("✓ added {}", display))
    }

    /// 连线: wire <from> -> <to>
    fn cmd_wire(&mut self, rest: &str) -> Result<String> {
        let parts: Vec<&str> = rest.split("->").map(|s| s.trim()).collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            bail!("usage: wire <from> -> <to>");
        }
        let from_ref = PinRef::parse(parts[0])?;
        let to_ref = PinRef::parse(parts[1])?;
        // 验证元件 id 存在
        if let PinRef::Component { id, .. } = &from_ref {
            if !self.project.components.iter().any(|c| &c.id == id) {
                bail!("unknown component id: {}", id);
            }
        }
        if let PinRef::Component { id, .. } = &to_ref {
            if !self.project.components.iter().any(|c| &c.id == id) {
                bail!("unknown component id: {}", id);
            }
        }
        let from_canon = from_ref.render_canonical();
        let to_canon = to_ref.render_canonical();
        self.project.add_wire(Wire {
            from: from_canon.clone(),
            to: to_canon.clone(),
        });
        self.save_project()?;
        Ok(format!("✓ wired {} -> {}", from_canon, to_canon))
    }

    fn cmd_show(&self, args: &[&str]) -> Result<String> {
        match args.first().copied() {
            Some("project") => Ok(render_project(&self.project)),
            None => {
                if let Some(sim) = self.running.as_ref() {
                    let st = sim.state.lock().unwrap();
                    Ok(render_runtime_frame(&self.project, &st))
                } else {
                    Ok(format!(
                        "{}\n(simulator not running — try `run`)",
                        render_project(&self.project)
                    ))
                }
            }
            Some(other) => bail!("unknown show subcommand: {}", other),
        }
    }

    fn cmd_edit(&self) -> Result<String> {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        let target = self.root.join(
            self.project
                .code
                .as_ref()
                .map(|c| c.src.clone())
                .unwrap_or_else(|| "src/main.ino".into()),
        );
        let status = Command::new(&editor)
            .arg(&target)
            .status()
            .with_context(|| format!("spawn $EDITOR ({})", editor))?;
        if !status.success() {
            bail!("editor exited non-zero");
        }
        Ok(String::new())
    }

    fn cmd_build(&self) -> Result<String> {
        let (_hex, msg) = cmd_build(&self.root)?;
        Ok(msg)
    }

    fn cmd_run_sim(&mut self) -> Result<String> {
        if let Some(s) = &mut self.running {
            if s.is_alive() {
                bail!("simulator already running (use `stop` first)");
            }
        }
        let hex = self
            .root
            .join("build")
            .join(format!("{}.hex", self.project.project.name));
        if !hex.exists() {
            bail!("hex not found at {} — run `build` first", hex.display());
        }
        let sim = cmd_run(&self.root, &hex)?;
        self.running = Some(sim);
        Ok("✓ simulator started (simavr)".to_string())
    }

    fn cmd_stop(&mut self) -> Result<String> {
        if let Some(sim) = self.running.take() {
            sim.stop();
            Ok("✓ simulator stopped".to_string())
        } else {
            Ok("(no simulator running)".to_string())
        }
    }

    /// `sleep <ms>` — 仅用于自动化测试,在剧本里观察 LED 翻转
    fn cmd_sleep(&self, args: &[&str]) -> Result<String> {
        let ms: u64 = args
            .first()
            .copied()
            .unwrap_or("1000")
            .parse()
            .map_err(|_| anyhow!("usage: sleep <milliseconds>"))?;
        std::thread::sleep(std::time::Duration::from_millis(ms));
        Ok(String::new())
    }

    fn save_project(&self) -> Result<()> {
        self.project.save(&self.root.join("moxin.toml"))
    }
}

fn help_text() -> String {
    String::from(
        "moxin shell commands:
  board info                       show board details
  add <type> [color] --id <id>     add component (led / button)
  wire <from> -> <to>              add wire (e.g. pin13 -> led1.anode)
  show project                     dump project state
  show                             render runtime ASCII frame
  edit                             open src/main.ino in $EDITOR
  build                            compile via arduino-cli
  run                              start simavr simulator
  stop                             stop simulator
  exit | quit                      leave shell",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// 构造一个**离线** Shell:tempdir 里建好 moxin.toml,running=None,
    /// 不开 simavr / arduino-cli,只测纯 dispatch 行为
    fn make_shell(tmp: &TempDir) -> Shell {
        let project = Project::new_blink("test");
        project
            .save(&tmp.path().join("moxin.toml"))
            .expect("save project");
        Shell {
            root: tmp.path().to_path_buf(),
            project,
            running: None,
        }
    }

    #[test]
    fn test_dispatch_add_led_returns_success() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let msg = shell
            .dispatch("add led red --id led1")
            .expect("add led should succeed");
        assert!(msg.contains("✓ added led1"), "got: {}", msg);
    }

    #[test]
    fn test_dispatch_add_without_id_returns_error() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        let res = shell.dispatch("add led");
        assert!(res.is_err(), "missing --id should error, got: {:?}", res);
    }

    #[test]
    fn test_dispatch_wire_after_add_returns_success() {
        let tmp = TempDir::new().expect("tempdir");
        let mut shell = make_shell(&tmp);
        shell
            .dispatch("add led red --id led1")
            .expect("add led prerequisite");
        let msg = shell
            .dispatch("wire pin13 -> led1.a")
            .expect("wire should succeed");
        assert!(msg.contains("✓ wired"), "got: {}", msg);
    }
}
