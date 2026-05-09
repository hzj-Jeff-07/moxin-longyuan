use crate::project::Project;
use anyhow::{Context, Result, bail};
use std::path::Path;

pub const BLINK_INO_TEMPLATE: &str = r#"// 由 moxin new 自动生成
// 经典 blink:D13 每 1000 ms 翻转一次
const int LED_PIN = 13;

void setup() {
  pinMode(LED_PIN, OUTPUT);
}

void loop() {
  digitalWrite(LED_PIN, HIGH);
  delay(1000);
  digitalWrite(LED_PIN, LOW);
  delay(1000);
}
"#;

/// STM32F405 (netduinoplus2 via QEMU) blink 模板。
/// PA13 toggle + USART2 printf(给 bridge 解析)。
/// 注意:不依赖 CMSIS / pico-sdk / HAL,纯寄存器访问 + 仓库自带 startup.s。
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
    // 时钟使能:GPIOA + USART2
    RCC_AHB1ENR |= (1u << 0);
    RCC_APB1ENR |= (1u << 17);

    // PA13 输出(MODER bit26..27 = 01)
    GPIOA_MODER &= ~(3u << (13 * 2));
    GPIOA_MODER |=  (1u << (13 * 2));

    // PA2 / PA3 alternate function 7 (USART2)
    GPIOA_MODER &= ~(3u << (2 * 2));
    GPIOA_MODER |=  (2u << (2 * 2));
    GPIOA_MODER &= ~(3u << (3 * 2));
    GPIOA_MODER |=  (2u << (3 * 2));
    GPIOA_AFRL  &= ~(0xFu << (2 * 4));
    GPIOA_AFRL  |=  (7u   << (2 * 4));
    GPIOA_AFRL  &= ~(0xFu << (3 * 4));
    GPIOA_AFRL  |=  (7u   << (3 * 4));

    // USART2 BRR for 16 MHz / 38400 ≈ 0x1A1
    // (QEMU 不严格校验 baud,任何非 0 值都收;给个真实值好看)
    USART2_BRR = 0x1A1;
    USART2_CR1 = (1u << 13) | (1u << 3);  // UE | TE

    uart_puts("STM32F405 blink starting...\n");

    int level = 0;
    unsigned counter = 0;
    for (;;) {
        level = !level;
        if (level) GPIOA_ODR |=  (1u << 13);
        else       GPIOA_ODR &= ~(1u << 13);

        // bridge 抓的关键行
        uart_puts("PIN13=");
        uart_putc(level ? '1' : '0');
        uart_putc('\n');

        // Serial Monitor 喂数据用的非 PIN 行
        if ((counter & 3u) == 0u) {
            uart_puts("loop counter=");
            uart_putu(counter);
            uart_putc('\n');
        }
        counter++;

        // 大约 0.5s @ 16MHz busy loop。两次翻转 = 1Hz blink。
        delay_busy(800000);
    }
}
"#;

pub fn cmd_new(name: &str, board: &str) -> Result<()> {
    let dir = Path::new(name);
    if dir.exists() {
        bail!("./{} already exists", name);
    }
    std::fs::create_dir_all(dir.join("src"))
        .with_context(|| format!("create {}/src", name))?;

    match board {
        "uno" | "arduino-uno" => {
            let project = Project::new_blink_uno(name);
            project.save(&dir.join("moxin.toml"))?;
            std::fs::write(dir.join("src").join("main.ino"), BLINK_INO_TEMPLATE)
                .context("write src/main.ino")?;
        }
        "stm32" | "stm32f405" => {
            let project = Project::new_blink_stm32(name);
            project.save(&dir.join("moxin.toml"))?;
            std::fs::write(dir.join("src").join("main.c"), BLINK_C_TEMPLATE)
                .context("write src/main.c")?;
        }
        other => bail!(
            "unknown board `{}` — supported: uno, stm32",
            other
        ),
    }

    println!("✓ created ./{} (board={})", name, board);
    Ok(())
}
