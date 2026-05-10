use std::process::Command;

struct Check {
    name: &'static str,
    status: CheckStatus,
}

enum CheckStatus {
    Ok(String),
    Missing(&'static str),
}

fn check_tool(bin: &'static str, args: &[&str], install_hint: &'static str) -> Check {
    let status = Command::new(bin).args(args).output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let out = String::from_utf8_lossy(&o.stdout);
            out.lines().next().map(|l| l.trim().to_string())
        })
        .map(CheckStatus::Ok)
        .unwrap_or(CheckStatus::Missing(install_hint));
    Check { name: bin, status }
}

fn check_file(name: &'static str, path: &std::path::Path, hint: &'static str) -> Check {
    let status = if path.exists() {
        CheckStatus::Ok(path.display().to_string())
    } else {
        CheckStatus::Missing(hint)
    };
    Check { name, status }
}

pub fn cmd_doctor() -> anyhow::Result<()> {
    let mut all_ok = true;

    let checks = vec![
        check_tool("arm-none-eabi-gcc", &["--version"],
            "brew install --cask gcc-arm-embedded  (macOS) / apt install gcc-arm-none-eabi (Linux)"),
        check_tool("qemu-system-arm", &["--version"],
            "brew install qemu  (macOS) / apt install qemu-system-arm (Linux)"),
        check_tool("arduino-cli", &["version"],
            "https://arduino.github.io/arduino-cli/latest/installation/"),
        check_tool("simavr", &["--help"],
            "brew install simavr  (macOS) / apt install simavr (Linux)"),
    ];

    // bridge-stm32
    let stm32_bridge = {
        let cache = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir())
            .join(".moxin")
            .join("bridge-stm32");
        check_file("bridge-stm32", &cache, "run: moxin build  (auto-compiled on first use)")
    };

    // bridge-avr
    let avr_bridge = {
        let path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("moxin-simavr-bridge")))
            .unwrap_or_else(|| std::path::PathBuf::from("moxin-simavr-bridge"));
        check_file("bridge-avr", &path, "run: make -C bridge/")
    };

    let all_checks: Vec<Check> = checks.into_iter().chain([stm32_bridge, avr_bridge]).collect();

    for c in &all_checks {
        match &c.status {
            CheckStatus::Ok(ver) => println!("  \x1b[32m✓\x1b[0m  {:<22} {}", c.name, ver),
            CheckStatus::Missing(hint) => {
                println!("  \x1b[31m✗\x1b[0m  {:<22} not found", c.name);
                println!("       \x1b[33mhint:\x1b[0m {}", hint);
                all_ok = false;
            }
        }
    }

    if all_ok {
        println!("\nAll checks passed.");
        Ok(())
    } else {
        anyhow::bail!("\nSome dependencies are missing. Install them and re-run `moxin doctor`.")
    }
}
