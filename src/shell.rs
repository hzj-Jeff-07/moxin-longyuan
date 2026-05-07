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

pub fn cmd_shell(start_dir: &Path) -> Result<()> {
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

    if io::stdin().is_terminal() {
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
        if let Err(e) = shell.dispatch(trimmed) {
            eprintln!("error: {}", e);
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
        if let Err(e) = shell.dispatch(&trimmed) {
            writeln!(out, "error: {}", e)?;
        }
    }
    Ok(())
}

pub struct Shell {
    root: PathBuf,
    project: Project,
    running: Option<RunningSim>,
}

impl Shell {
    fn dispatch(&mut self, line: &str) -> Result<()> {
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
            "help" | "?" => {
                print_help();
                Ok(())
            }
            _ => bail!("unknown command: {} (try `help`)", head),
        }
    }

    fn cmd_board(&self, rest: &[&str]) -> Result<()> {
        match rest.first().copied() {
            Some("info") | None => {
                println!("{}", board_info());
                Ok(())
            }
            Some(other) => bail!("unknown board subcommand: {}", other),
        }
    }

    /// 添加元件: add <type> [args...] --id <id>
    /// 例: add led red --id led1
    /// 例: add button --id btn1
    fn cmd_add(&mut self, args: &[&str]) -> Result<()> {
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
        println!("✓ added {}", display);
        Ok(())
    }

    /// 连线: wire <from> -> <to>
    fn cmd_wire(&mut self, rest: &str) -> Result<()> {
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
        println!(
            "✓ wired {} -> {}",
            from_canon, to_canon
        );
        Ok(())
    }

    fn cmd_show(&self, args: &[&str]) -> Result<()> {
        match args.first().copied() {
            Some("project") => {
                println!("{}", render_project(&self.project));
                Ok(())
            }
            None => {
                if let Some(sim) = self.running.as_ref() {
                    let st = sim.state.lock().unwrap();
                    println!("{}", render_runtime_frame(&self.project, &st));
                } else {
                    println!(
                        "{}\n(simulator not running — try `run`)",
                        render_project(&self.project)
                    );
                }
                Ok(())
            }
            Some(other) => bail!("unknown show subcommand: {}", other),
        }
    }

    fn cmd_edit(&self) -> Result<()> {
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
        Ok(())
    }

    fn cmd_build(&self) -> Result<()> {
        cmd_build(&self.root)?;
        Ok(())
    }

    fn cmd_run_sim(&mut self) -> Result<()> {
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
        Ok(())
    }

    fn cmd_stop(&mut self) -> Result<()> {
        if let Some(sim) = self.running.take() {
            sim.stop();
            println!("✓ simulator stopped");
        } else {
            println!("(no simulator running)");
        }
        Ok(())
    }

    /// `sleep <ms>` — 仅用于自动化测试,在剧本里观察 LED 翻转
    fn cmd_sleep(&self, args: &[&str]) -> Result<()> {
        let ms: u64 = args
            .first()
            .copied()
            .unwrap_or("1000")
            .parse()
            .map_err(|_| anyhow!("usage: sleep <milliseconds>"))?;
        std::thread::sleep(std::time::Duration::from_millis(ms));
        Ok(())
    }

    fn save_project(&self) -> Result<()> {
        self.project.save(&self.root.join("moxin.toml"))
    }
}

fn print_help() {
    println!(
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
  exit | quit                      leave shell"
    );
}
