use crate::boards::spec::{ArtifactKind, BoardSpec, PinSpec};
use crate::project::{CodeMeta, Project, ProjectMeta, SCHEMA_VERSION};
use crate::sim::{RunningSim, find_bridge_stm32, spawn_bridge_child, spawn_with_state};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGET_FLAGS: &[&str] = &[
    "-mthumb", "-mcpu=cortex-m4", "-mfloat-abi=soft", "-Os",
    "-ffreestanding", "-nostartfiles", "-nostdlib", "-Wall", "-Wextra",
];

pub static STM32F405_SPEC: BoardSpec = BoardSpec {
    board_id: "stm32",
    display_name: "STM32F405 (netduinoplus2)",
    mcu: "STM32F405RG",
    clock_hz: 16_000_000,
    voltage_mv: 3300,
    artifact_kind: ArtifactKind::Elf,
    pins: &[
        PinSpec { name: "PA0",  aliases: &["pa0"],  is_d13_led: false },
        PinSpec { name: "PA1",  aliases: &["pa1"],  is_d13_led: false },
        PinSpec { name: "PA2",  aliases: &["pa2"],  is_d13_led: false },
        PinSpec { name: "PA3",  aliases: &["pa3"],  is_d13_led: false },
        PinSpec { name: "PA4",  aliases: &["pa4"],  is_d13_led: false },
        PinSpec { name: "PA5",  aliases: &["pa5"],  is_d13_led: false },
        PinSpec { name: "PA6",  aliases: &["pa6"],  is_d13_led: false },
        PinSpec { name: "PA7",  aliases: &["pa7"],  is_d13_led: false },
        PinSpec { name: "PA8",  aliases: &["pa8"],  is_d13_led: false },
        PinSpec { name: "PA9",  aliases: &["pa9"],  is_d13_led: false },
        PinSpec { name: "PA10", aliases: &["pa10"], is_d13_led: false },
        PinSpec { name: "PA11", aliases: &["pa11"], is_d13_led: false },
        PinSpec { name: "PA12", aliases: &["pa12"], is_d13_led: false },
        PinSpec { name: "PA13", aliases: &["pa13"], is_d13_led: true  },
        PinSpec { name: "PA14", aliases: &["pa14"], is_d13_led: false },
        PinSpec { name: "PA15", aliases: &["pa15"], is_d13_led: false },
        PinSpec { name: "PB0",  aliases: &["pb0"],  is_d13_led: false },
        PinSpec { name: "PB1",  aliases: &["pb1"],  is_d13_led: false },
        PinSpec { name: "PB2",  aliases: &["pb2"],  is_d13_led: false },
        PinSpec { name: "PB3",  aliases: &["pb3"],  is_d13_led: false },
        PinSpec { name: "PB4",  aliases: &["pb4"],  is_d13_led: false },
        PinSpec { name: "PB5",  aliases: &["pb5"],  is_d13_led: false },
        PinSpec { name: "PB6",  aliases: &["pb6"],  is_d13_led: false },
        PinSpec { name: "PB7",  aliases: &["pb7"],  is_d13_led: false },
        PinSpec { name: "PB8",  aliases: &["pb8"],  is_d13_led: false },
        PinSpec { name: "PB9",  aliases: &["pb9"],  is_d13_led: false },
        PinSpec { name: "PB10", aliases: &["pb10"], is_d13_led: false },
        PinSpec { name: "PB11", aliases: &["pb11"], is_d13_led: false },
        PinSpec { name: "PB12", aliases: &["pb12"], is_d13_led: false },
        PinSpec { name: "PB13", aliases: &["pb13"], is_d13_led: false },
        PinSpec { name: "PB14", aliases: &["pb14"], is_d13_led: false },
        PinSpec { name: "PB15", aliases: &["pb15"], is_d13_led: false },
        PinSpec { name: "PC0",  aliases: &["pc0"],  is_d13_led: false },
        PinSpec { name: "PC1",  aliases: &["pc1"],  is_d13_led: false },
        PinSpec { name: "PC2",  aliases: &["pc2"],  is_d13_led: false },
        PinSpec { name: "PC3",  aliases: &["pc3"],  is_d13_led: false },
        PinSpec { name: "PC4",  aliases: &["pc4"],  is_d13_led: false },
        PinSpec { name: "PC5",  aliases: &["pc5"],  is_d13_led: false },
        PinSpec { name: "PC6",  aliases: &["pc6"],  is_d13_led: false },
        PinSpec { name: "PC7",  aliases: &["pc7"],  is_d13_led: false },
        PinSpec { name: "PC8",  aliases: &["pc8"],  is_d13_led: false },
        PinSpec { name: "PC9",  aliases: &["pc9"],  is_d13_led: false },
        PinSpec { name: "PC10", aliases: &["pc10"], is_d13_led: false },
        PinSpec { name: "PC11", aliases: &["pc11"], is_d13_led: false },
        PinSpec { name: "PC12", aliases: &["pc12"], is_d13_led: false },
        PinSpec { name: "PC13", aliases: &["pc13"], is_d13_led: false },
        PinSpec { name: "PC14", aliases: &["pc14"], is_d13_led: false },
        PinSpec { name: "PC15", aliases: &["pc15"], is_d13_led: false },
        PinSpec { name: "GND",  aliases: &["gnd"],  is_d13_led: false },
        PinSpec { name: "3V3",  aliases: &["3v3", "vcc"], is_d13_led: false },
    ],
    serial_count: 3,
    gpio_count: 51,
    d13_bridge_port: "GPIO",
    d13_bridge_bit: 13,
};

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
    fn spec(&self) -> &'static super::spec::BoardSpec { &STM32F405_SPEC }
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

    fn spawn_sim(&self, root: &Path, artifact: &Path, json_out: bool) -> Result<RunningSim> {
        let bridge = find_bridge_stm32()?;
        if !bridge.exists() {
            bail!("stm32 bridge not found at {} — set $MOXIN_BRIDGE_STM32 or `make` in bridge/stm32/", bridge.display());
        }
        if !artifact.exists() {
            bail!("elf not found: {} — run `build` first", artifact.display());
        }
        let child = spawn_bridge_child(&bridge, &[artifact], root)?;
        spawn_with_state(child, self.voltage_mv(), self.spec().make_is_d13(), json_out)
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

const STARTUP_S: &str = include_str!("../../examples/stm32-blink/support/startup.s");
const LINKER_LD: &str = include_str!("../../examples/stm32-blink/support/linker.ld");

fn find_support_dir() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("MOXIN_STM32_SUPPORT") {
        return Ok(PathBuf::from(p));
    }
    let dir = std::env::temp_dir().join("moxin-stm32-support");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("startup.s"), STARTUP_S)?;
    std::fs::write(dir.join("linker.ld"), LINKER_LD)?;
    Ok(dir)
}
