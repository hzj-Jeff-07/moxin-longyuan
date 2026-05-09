// examples/stm32-blink/main.c · 给 v2a 的 stm32-blink demo 用
// 跟 cmd_new.rs 里 BLINK_C_TEMPLATE 同源 —— 这一份是 example 的 canonical 拷贝,
// `moxin new --board=stm32` 生成的项目模板就是从这一份直接 inline 进 cmd_new 的。
//
// 行为:USART2 banner + PA13 1Hz toggle + "PIN13=<v>" + counter 行
//
// bridge 这一侧:正则把 "PIN<n>=<v>" 行拆出,其余原样作为 serial line。

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
