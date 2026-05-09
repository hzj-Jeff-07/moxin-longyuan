mod board;
mod cmd_build;
mod cmd_build_stm32;
mod cmd_new;
mod cmd_run;
mod cmd_run_stm32;
mod inspector;
mod project;
mod render;
mod shell;
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
        /// 板号: uno (Arduino Uno) 或 stm32 (STM32F405 / netduinoplus2)
        #[arg(long, default_value = "uno")]
        board: String,
    },
    /// 进入交互式 shell
    Shell {
        /// 强制退回旧 rustyline 提示符,不进 TUI
        #[arg(long = "no-tui")]
        no_tui: bool,
    },
    /// 编译项目(板分发:uno→arduino-cli,stm32→arm-none-eabi-gcc)
    Build,
    /// 启动模拟器,跑到回车键按下退出
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
            let msg = match project.project.board.as_str() {
                "arduino-uno" => {
                    let (_hex, msg) = cmd_build::cmd_build(&root)?;
                    msg
                }
                "stm32" => {
                    let (_elf, msg) = cmd_build_stm32::cmd_build_stm32(&root)?;
                    msg
                }
                other => bail!("unsupported board `{}` in moxin.toml", other),
            };
            if !msg.is_empty() {
                println!("{}", msg);
            }
            Ok(())
        }
        Cmd::Run => {
            let cwd = std::env::current_dir()?;
            let root = project::Project::find_project_root(&cwd)?;
            let project = project::Project::load(&root.join("moxin.toml"))?;
            let sim = match project.project.board.as_str() {
                "arduino-uno" => {
                    let hex = root
                        .join("build")
                        .join(format!("{}.hex", project.project.name));
                    cmd_run::cmd_run(&root, &hex)?
                }
                "stm32" => {
                    let elf = root
                        .join("build")
                        .join(format!("{}.elf", project.project.name));
                    cmd_run_stm32::cmd_run_stm32(&root, &elf)?
                }
                other => bail!("unsupported board `{}` in moxin.toml", other),
            };
            println!("✓ simulator started ({})", project.project.board);
            println!("(press ENTER to stop)");
            let mut buf = String::new();
            let _ = std::io::stdin().read_line(&mut buf);
            sim.stop();
            Ok(())
        }
    }
}
