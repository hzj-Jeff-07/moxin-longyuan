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

#[cfg(target_os = "macos")]
fn hint_arm_gcc() -> &'static str { "brew install --cask gcc-arm-embedded" }
#[cfg(target_os = "linux")]
fn hint_arm_gcc() -> &'static str { "apt install gcc-arm-none-eabi" }
#[cfg(target_os = "windows")]
fn hint_arm_gcc() -> &'static str { "scoop install gcc-arm-none-eabi  # or download from https://developer.arm.com/downloads/-/gnu-rm" }

#[cfg(target_os = "macos")]
fn hint_qemu() -> &'static str { "brew install qemu" }
#[cfg(target_os = "linux")]
fn hint_qemu() -> &'static str { "apt install qemu-system-arm" }
#[cfg(target_os = "windows")]
fn hint_qemu() -> &'static str { "scoop install qemu  # or download from https://www.qemu.org/download/#windows" }

#[cfg(target_os = "macos")]
fn hint_simavr() -> &'static str { "brew install simavr" }
#[cfg(target_os = "linux")]
fn hint_simavr() -> &'static str { "apt install simavr" }
#[cfg(target_os = "windows")]
fn hint_simavr() -> &'static str { "no native Windows package — use WSL (apt install simavr) or build via MSYS2: https://github.com/buserror/simavr" }

/// AI Inspector 的 LLM 依赖是**可选**的:未启用时不影响 doctor 结果,只提示。
/// 用 ○ 表示"可选、未配置",区别于必需依赖缺失的 ✗。密钥只报"是否设置",绝不打印值。
fn print_llm_section() {
    println!("\nAI Inspector (optional — LLM via curl):");
    if crate::llm::curl_available() {
        println!("  \x1b[32m✓\x1b[0m  {:<22} available", "curl");
    } else {
        println!("  \x1b[36m○\x1b[0m  {:<22} not found (needed only for `moxin explain`)", "curl");
    }
    let cfg = crate::llm::LlmConfig::from_process_env();
    if cfg.is_enabled() {
        println!("  \x1b[32m✓\x1b[0m  {:<22} set (model: {})", "MOXIN_LLM_API_KEY", cfg.model);
    } else {
        println!("  \x1b[36m○\x1b[0m  {:<22} unset — LLM features disabled", "MOXIN_LLM_API_KEY");
    }
}

pub fn cmd_doctor() -> anyhow::Result<()> {
    let mut all_ok = true;

    let checks = vec![
        check_tool("arm-none-eabi-gcc", &["--version"], hint_arm_gcc()),
        check_tool("qemu-system-arm", &["--version"], hint_qemu()),
        check_tool("arduino-cli", &["version"],
            "https://arduino.github.io/arduino-cli/latest/installation/"),
        check_tool("simavr", &["--help"], hint_simavr()),
    ];

    let stm32_bridge = {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        let bridge_name = if cfg!(windows) { "bridge-stm32.exe" } else { "bridge-stm32" };
        let cache = home.join(".moxin").join(bridge_name);
        check_file("bridge-stm32", &cache, "run: moxin build  (auto-compiled on first use)")
    };

    let avr_bridge = {
        let bridge_name = if cfg!(windows) { "moxin-simavr-bridge.exe" } else { "moxin-simavr-bridge" };
        let path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(bridge_name)))
            .unwrap_or_else(|| std::path::PathBuf::from(bridge_name));
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

    // AI Inspector (LLM,可选) —— 默认关闭,缺失不算 doctor 失败,只做提示。
    print_llm_section();

    if all_ok {
        println!("\nAll checks passed.");
        Ok(())
    } else {
        anyhow::bail!("\nSome dependencies are missing. Install them and re-run `moxin doctor`.")
    }
}
