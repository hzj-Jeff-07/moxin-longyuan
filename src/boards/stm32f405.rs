use crate::project::{CodeMeta, Project, ProjectMeta, SCHEMA_VERSION};
use crate::sim::{RunningSim, find_bridge_stm32, spawn_bridge_child, spawn_with_state};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET_FLAGS: &[&str] = &[
    "-mthumb", "-mcpu=cortex-m4", "-mfloat-abi=soft", "-Os",
    "-ffreestanding", "-nostartfiles", "-nostdlib", "-Wall", "-Wextra",
];

pub const BLINK_C_TEMPLATE: &str = r#"// 由 moxin new --board=stm32 自动生成
// STM32F405 / netduinoplus2 blink:PA13 每 1000 ms 翻转,通过 USART2 打印
// "PIN13=<v>" 给 bridge 解析,以及 banner / counter 等给 Serial Monitor。
//
// 不依赖任何 SDK,纯寄存器访问。bridge 这一侧用正则把
// "PIN<n>=<v>" 行拆出,其余原样作为 serial line。

#include <stdint.h>

#define RCC_BASE   0x40023800UL
#define RCC_AHB1ENR  (*(volatile uint32_t *)(RCC_BASE + 0x30))
#define RCC_APB1ENR  (*(volatile uint32_t *)(RCC_BASE + 0x40))

#define GPIOA_BASE 0x40020000UL
#define GPIOA_MODER  (*(volatile uint32_t *)(GPIOA_BASE + 0x00))
#define GPIOA_AFRL   (*(volatile uint32_t *)(GPIOA_BASE + 0x20))
#define GPIOA_ODR    (*(volatile uint32_t *)(GPIOA_BASE + 0x14))

#define USART2_BASE 0x40004400UL
#define USART2_SR  (*(volatile uint32_t *)(USART2_BASE + 0x00))
#define USART2_DR  (*(volatile uint32_t *)(USART2_BASE + 0x04))
#define USART2_BRR (*(volatile uint32_t *)(USART2_BASE + 0x08))
#define USART2_CR1 (*(volatile uint32_t *)(USART2_BASE + 0x0C))

#define USART_SR_TXE (1 << 7)

static void uart_putc(char c) {
    while (!(USART2_SR & USART_SR_TXE)) {}
    USART2_DR = (uint32_t)c;
}

static void uart_puts(const char *s) {
    while (*s) {
        if (*s == '\n') uart_putc('\r');
        uart_putc(*s++);
    }
}

static void uart_putu(unsigned v) {
    char buf[12];
    int i = 0;
    if (v == 0) { uart_putc('0'); return; }
    while (v) { buf[i++] = '0' + (v % 10); v /= 10; }
    while (i--) uart_putc(buf[i]);
}

static void delay_busy(volatile uint32_t n) {
    while (n--) { __asm__ volatile("nop"); }
}

int main(void) {
    RCC_AHB1ENR |= (1u << 0);
    RCC_APB1ENR |= (1u << 17);
    GPIOA_MODER &= ~(3u << (13 * 2));
    GPIOA_MODER |=  (1u << (13 * 2));
    GPIOA_MODER &= ~(3u << (2 * 2));
    GPIOA_MODER |=  (2u << (2 * 2));
    GPIOA_MODER &= ~(3u << (3 * 2));
    GPIOA_MODER |=  (2u << (3 * 2));
    GPIOA_AFRL  &= ~(0xFu << (2 * 4));
    GPIOA_AFRL  |=  (7u   << (2 * 4));
    GPIOA_AFRL  &= ~(0xFu << (3 * 4));
    GPIOA_AFRL  |=  (7u   << (3 * 4));
    USART2_BRR = 0x1A1;
    USART2_CR1 = (1u << 13) | (1u << 3);
    uart_puts("STM32F405 blink starting...\n");
    int level = 0;
    unsigned counter = 0;
    for (;;) {
        level = !level;
        if (level) GPIOA_ODR |=  (1u << 13);
        else       GPIOA_ODR &= ~(1u << 13);
        uart_puts("PIN13=");
        uart_putc(level ? '1' : '0');
        uart_putc('\n');
        if ((counter & 3u) == 0u) {
            uart_puts("loop counter=");
            uart_putu(counter);
            uart_putc('\n');
        }
        counter++;
        delay_busy(800000);
    }
}
"#;

pub struct Stm32f405;

impl super::BoardImpl for Stm32f405 {
    fn board_name(&self) -> &'static str { "stm32" }
    fn voltage_mv(&self) -> u32 { 3300 }
    fn artifact_ext(&self) -> &'static str { "elf" }
    fn scaffold_project(&self, name: &str) -> Project {
        Project {
            project: ProjectMeta { name: name.to_string(), board: "stm32".to_string(), version: SCHEMA_VERSION.to_string() },
            components: vec![],
            wires: vec![],
            code: Some(CodeMeta { src: "src/main.c".to_string(), flags: vec![] }),
        }
    }
    fn source_template(&self) -> &'static str { BLINK_C_TEMPLATE }

    fn build(&self, root: &Path) -> Result<(PathBuf, String)> {
        let project = Project::load(&root.join("moxin.toml"))?;
        let src_rel = project.code.as_ref().map(|c| c.src.clone())
            .unwrap_or_else(|| "src/main.c".to_string());
        let src_abs = root.join(&src_rel);
        if !src_abs.exists() {
            bail!("source file not found: {}", src_abs.display());
        }

        ensure_arm_gcc()?;

        let support = find_support_dir()?;
        let startup = support.join("startup.s");
        let linker = support.join("linker.ld");
        if !startup.exists() || !linker.exists() {
            bail!("stm32 support files missing under {} — expected startup.s + linker.ld", support.display());
        }

        let build_dir = root.join("build");
        std::fs::create_dir_all(&build_dir).context("mkdir build")?;
        let target_name = format!("{}.elf", project.project.name);
        let target_elf = build_dir.join(&target_name);

        let mut cmd = Command::new("arm-none-eabi-gcc");
        cmd.args(TARGET_FLAGS)
            .arg(format!("-T{}", linker.display()))
            .arg(&startup).arg(&src_abs)
            .arg("-o").arg(&target_elf);
        let out = cmd.output().context("invoke arm-none-eabi-gcc")?;
        if !out.status.success() {
            bail!("arm-none-eabi-gcc compile failed:\n{}", String::from_utf8_lossy(&out.stderr).trim_end());
        }

        let size = std::fs::metadata(&target_elf).map(|m| m.len()).unwrap_or(0);
        let mut msg = String::new();
        for line in String::from_utf8_lossy(&out.stderr).lines() {
            let t = line.trim_end();
            if !t.is_empty() { msg.push_str(t); msg.push('\n'); }
        }
        msg.push_str(&format!("✓ arm-none-eabi-gcc compile OK → build/{} ({} bytes ELF)", target_name, size));
        Ok((target_elf, msg))
    }

    fn spawn_sim(&self, root: &Path, artifact: &Path) -> Result<RunningSim> {
        let bridge = find_bridge_stm32()?;
        if !bridge.exists() {
            bail!("stm32 bridge not found at {} — set $MOXIN_BRIDGE_STM32 or `make` in bridge/stm32/", bridge.display());
        }
        if !artifact.exists() {
            bail!("elf not found: {} — run `build` first", artifact.display());
        }
        let child = spawn_bridge_child(&bridge, &[artifact], root)?;
        spawn_with_state(child, self.voltage_mv(), Box::new(|port, bit| port == "GPIO" && bit == 13))
    }
}

fn ensure_arm_gcc() -> Result<()> {
    let out = Command::new("arm-none-eabi-gcc").arg("--version").output()
        .map_err(|e| anyhow::anyhow!(
            "arm-none-eabi-gcc not found in PATH: {} — try `brew install --cask gcc-arm-embedded`", e
        ))?;
    if !out.status.success() { bail!("arm-none-eabi-gcc --version exited non-zero"); }
    Ok(())
}

fn find_support_dir() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("MOXIN_STM32_SUPPORT") {
        return Ok(PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("examples").join("stm32-blink").join("support");
            if candidate.exists() { return Ok(candidate); }
        }
    }
    bail!("stm32 support files not found — set $MOXIN_STM32_SUPPORT env var or place support/ next to the moxin binary")
}
