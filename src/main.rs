mod board;
mod cmd_build;
mod cmd_new;
mod cmd_run;
mod project;
mod render;
mod shell;

use anyhow::Result;
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
    New { name: String },
    /// 进入交互式 shell
    Shell,
    /// 编译项目(arduino-cli)
    Build,
    /// 启动模拟器,跑到回车键按下退出
    Run,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::New { name } => cmd_new::cmd_new(&name),
        Cmd::Shell => {
            let cwd = std::env::current_dir()?;
            shell::cmd_shell(&cwd)
        }
        Cmd::Build => {
            let cwd = std::env::current_dir()?;
            let root = project::Project::find_project_root(&cwd)?;
            let _hex = cmd_build::cmd_build(&root)?;
            Ok(())
        }
        Cmd::Run => {
            let cwd = std::env::current_dir()?;
            let root = project::Project::find_project_root(&cwd)?;
            let project = project::Project::load(&root.join("moxin.toml"))?;
            let hex = root
                .join("build")
                .join(format!("{}.hex", project.project.name));
            let sim = cmd_run::cmd_run(&root, &hex)?;
            println!("(press ENTER to stop)");
            let mut buf = String::new();
            let _ = std::io::stdin().read_line(&mut buf);
            sim.stop();
            Ok(())
        }
    }
}
