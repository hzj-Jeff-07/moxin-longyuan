mod board;
mod boards;
mod cmd_doctor;
mod cmd_install;
mod cmd_new;
mod cmd_status;
mod inspector;
mod project;
mod render;
mod shell;
mod sim;
mod tui;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};

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
    Run {
        /// 输出模式：tui = 默认交互行为；json = 把 bridge 事件流以 JSON Lines 透传到 stdout
        #[arg(long, value_enum, default_value_t = OutputMode::Tui)]
        output: OutputMode,
    },
    /// 检查外部依赖是否安装
    Doctor,
    /// 将 moxin 安装到 PATH
    Install,
    /// 查询引脚状态
    Status {
        #[arg(long)]
        pin: String,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputMode {
    Tui,
    Json,
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
        Cmd::Doctor => cmd_doctor::cmd_doctor(),
        Cmd::Install => cmd_install::cmd_install(),
        Cmd::Status { pin } => cmd_status::cmd_status(&pin),
        Cmd::Run { output } => {
            let cwd = std::env::current_dir()?;
            let root = project::Project::find_project_root(&cwd)?;
            let project = project::Project::load(&root.join("moxin.toml"))?;
            let board = boards::board_from_str(&project.project.board)?;
            let ext = board.artifact_ext();
            let artifact = root.join("build").join(format!("{}.{}", project.project.name, ext));
            if !artifact.exists() {
                bail!("artifact not found at {} — run `build` first", artifact.display());
            }
            let json_out = output == OutputMode::Json;
            let mut sim = board.spawn_sim(&root, &artifact, json_out)?;
            let state_file = root.join("build").join(".moxin-state.json");
            if json_out {
                // stdout 严格只走 JSON Lines（在 reader 线程里透传），状态提示写 stderr。
                eprintln!("✓ simulator started ({}) — streaming JSON to stdout, Ctrl-C to stop", project.project.board);
                loop {
                    if let Ok(s) = sim.state.lock() {
                        s.write_state_file(&state_file);
                        if s.bridge_exited { break; }
                    }
                    if !sim.is_alive() { break; }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            } else {
                println!("✓ simulator started ({})", project.project.board);
                println!("(press ENTER to stop)");
                let mut buf = String::new();
                let _ = std::io::stdin().read_line(&mut buf);
            }
            sim.stop();
            Ok(())
        }
    }
}
