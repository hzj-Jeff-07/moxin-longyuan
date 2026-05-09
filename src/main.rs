mod board;
mod boards;
mod cmd_new;
mod inspector;
mod project;
mod render;
mod shell;
mod sim;
mod tui;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "moxin", version, about = "MoXin CLI demo")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 新建项目
    New {
        name: String,
        #[arg(long, default_value = "uno")]
        board: String,
    },
    /// 进入交互式 shell
    Shell {
        #[arg(long = "no-tui")]
        no_tui: bool,
    },
    /// 编译项目
    Build,
    /// 启动模拟器
    Run,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::New { name, board } => cmd_new::cmd_new(&name, &board),
        Cmd::Shell { no_tui } => {
            let cwd = std::env::current_dir()?;
            shell::cmd_shell(&cwd, no_tui)
        }
        Cmd::Build => {
            let cwd = std::env::current_dir()?;
            let root = project::Project::find_project_root(&cwd)?;
            let project = project::Project::load(&root.join("moxin.toml"))?;
            let board = boards::board_from_str(&project.project.board)?;
            let (_artifact, msg) = board.build(&root)?;
            if !msg.is_empty() { println!("{}", msg); }
            Ok(())
        }
        Cmd::Run => {
            let cwd = std::env::current_dir()?;
            let root = project::Project::find_project_root(&cwd)?;
            let project = project::Project::load(&root.join("moxin.toml"))?;
            let board = boards::board_from_str(&project.project.board)?;
            let ext = if project.project.board == "stm32" { "elf" } else { "hex" };
            let artifact = root.join("build").join(format!("{}.{}", project.project.name, ext));
            if !artifact.exists() {
                bail!("artifact not found at {} — run `build` first", artifact.display());
            }
            let sim = board.spawn_sim(&root, &artifact)?;
            println!("✓ simulator started ({})", project.project.board);
            println!("(press ENTER to stop)");
            let mut buf = String::new();
            let _ = std::io::stdin().read_line(&mut buf);
            sim.stop();
            Ok(())
        }
    }
}
