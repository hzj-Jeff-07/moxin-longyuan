/*
 * moxin-simavr-bridge.c — moxin demo 的 simavr 子进程桥接
 *
 * 启动:  bridge <hexfile> [mcu] [freq_hz]
 * 标准输出: 每行一条 JSON (JSON Lines 格式,协议见 docs/design/bridge-protocol.md)
 *   {"event":"hello","protocol":"1","capabilities":["adc","serial"]}
 *   {"event":"ready","mcu":"atmega328p","freq":16000000}
 *   {"event":"pin","t_us":NUM,"port":"B","bit":5,"value":0|1}
 *   {"event":"serial","t_us":NUM,"line":"<escaped>"}
 *   {"event":"adc","t_us":NUM,"channel":N,"value":0..1023}
 *   {"event":"exit","state":N}
 * 标准输入: 行命令通道(非阻塞轮询,主循环内处理,避免多线程碰 simavr)
 *   adc <ch> <value>     ch 0..7,value 0..1023 → 注入 ADC IRQ
 *   未识别的行忽略(串口 RX 注入未实现,与旧版行为一致)
 *
 * 设计:本进程链接 libsimavr.a (GPL-3.0+),与 moxin Rust 主进程完全分离,
 * 仅通过 stdio 通信,规避 GPL 传染。
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <time.h>

#include "sim_avr.h"
#include "sim_hex.h"
#include "avr_ioport.h"
#include "avr_adc.h"
#include "avr_uart.h"

static avr_t *g_avr = NULL;

static uint64_t sim_time_us(void) {
    return (uint64_t)((double)g_avr->cycle * 1e6 / (double)g_avr->frequency);
}

/* JSON 字符串 escape,与 bridge-stm32.c 同款:\, ", \r, \n, \t 转义,
 * 其它控制字符丢弃。 */
static size_t json_escape(const char *src, char *dst, size_t dst_cap) {
    size_t out = 0;
    for (const char *p = src; *p && out + 8 < dst_cap; ++p) {
        unsigned char c = (unsigned char)*p;
        switch (c) {
            case '"':  dst[out++]='\\'; dst[out++]='"';  break;
            case '\\': dst[out++]='\\'; dst[out++]='\\'; break;
            case '\n': dst[out++]='\\'; dst[out++]='n';  break;
            case '\r': dst[out++]='\\'; dst[out++]='r';  break;
            case '\t': dst[out++]='\\'; dst[out++]='t';  break;
            default:
                if (c < 0x20) { break; }
                dst[out++] = (char)c;
        }
    }
    dst[out] = '\0';
    return out;
}

static void pin_change_cb(struct avr_irq_t *irq, uint32_t value, void *param) {
    char port = (char)(intptr_t)param;
    uint8_t bit = (uint8_t)irq->irq;
    uint64_t t_us = (uint64_t)((double)g_avr->cycle * 1e6 / (double)g_avr->frequency);
    printf("{\"event\":\"pin\",\"t_us\":%llu,\"port\":\"%c\",\"bit\":%u,\"value\":%u}\n",
           (unsigned long long)t_us, port, bit, value);
    fflush(stdout);
}

static void log_event(const char *json) {
    fputs(json, stdout);
    fputc('\n', stdout);
    fflush(stdout);
}

/* ---- UART0 → serial 事件 ----
 * simavr 默认把 UART TX 直接 dump 到 stdout,会污染 JSON Lines;
 * 这里改为挂 OUTPUT IRQ,按行缓冲后以 serial 事件发出(对齐 stm32 bridge)。 */
static char g_serial_buf[512];
static size_t g_serial_len = 0;

static void flush_serial_line(void) {
    if (g_serial_len == 0) return;
    g_serial_buf[g_serial_len] = '\0';
    char esc[1024];
    json_escape(g_serial_buf, esc, sizeof(esc));
    printf("{\"event\":\"serial\",\"t_us\":%llu,\"line\":\"%s\"}\n",
           (unsigned long long)sim_time_us(), esc);
    fflush(stdout);
    g_serial_len = 0;
}

static void uart_out_cb(struct avr_irq_t *irq, uint32_t value, void *param) {
    (void)irq; (void)param;
    char c = (char)(value & 0xFF);
    if (c == '\n') {
        flush_serial_line();
    } else if (c != '\r' && g_serial_len + 1 < sizeof(g_serial_buf)) {
        g_serial_buf[g_serial_len++] = c;
    } else if (g_serial_len + 1 >= sizeof(g_serial_buf)) {
        flush_serial_line();           /* 行超长:先发出去再继续攒 */
        g_serial_buf[g_serial_len++] = c;
    }
}

/* ---- stdin 命令通道 ----
 * 非阻塞轮询,在主 avr_run 循环的间隙处理;不开线程,
 * 因为 simavr 不是线程安全的,跨线程 avr_raise_irq 会与 avr_run 竞态
 * (对 RFC pthread 草图的偏差,见 RFC 决策记录)。 */
static char g_cmd_buf[256];
static size_t g_cmd_len = 0;

static void process_cmd_line(const char *line) {
    int ch, value;
    if (sscanf(line, "adc %d %d", &ch, &value) == 2) {
        if (ch < 0 || ch > 7) return;
        if (value < 0) value = 0;
        if (value > 1023) value = 1023;
        /* simavr 的 ADC IRQ 以 mV 为单位;10-bit 原始值按 AVCC=5000mV 换算 */
        uint32_t mv = (uint32_t)(((uint64_t)value * 5000ULL) / 1023ULL);
        avr_raise_irq(
            avr_io_getirq(g_avr, AVR_IOCTL_ADC_GETIRQ, ADC_IRQ_ADC0 + ch), mv);
        printf("{\"event\":\"adc\",\"t_us\":%llu,\"channel\":%d,\"value\":%d}\n",
               (unsigned long long)sim_time_us(), ch, value);
        fflush(stdout);
    }
    /* 未识别的行:忽略(TUI 可能把串口字符写进 stdin,保持旧版"空读"行为) */
}

static void poll_stdin_commands(void) {
    char tmp[128];
    ssize_t n;
    while ((n = read(STDIN_FILENO, tmp, sizeof(tmp))) > 0) {
        for (ssize_t i = 0; i < n; i++) {
            char c = tmp[i];
            if (c == '\n') {
                g_cmd_buf[g_cmd_len] = '\0';
                process_cmd_line(g_cmd_buf);
                g_cmd_len = 0;
            } else if (g_cmd_len + 1 < sizeof(g_cmd_buf)) {
                g_cmd_buf[g_cmd_len++] = c;
            } else {
                g_cmd_len = 0;         /* 行超长:整行丢弃 */
            }
        }
    }
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s <hexfile> [mcu] [freq]\n", argv[0]);
        return 2;
    }
    const char *hexpath = argv[1];
    const char *mcu = argc >= 3 ? argv[2] : "atmega328p";
    uint32_t freq = argc >= 4 ? (uint32_t)strtoul(argv[3], NULL, 10) : 16000000U;

    avr_t *avr = avr_make_mcu_by_name(mcu);
    if (!avr) {
        fprintf(stderr, "unknown mcu: %s\n", mcu);
        return 1;
    }
    avr_init(avr);
    avr->frequency = freq;
    /* ADC 参考电压(mV):不设的话 simavr 的 ADC 换算结果恒为 0 */
    avr->vcc = 5000;
    avr->avcc = 5000;
    avr->aref = 5000;
    g_avr = avr;

    /* 通过 simavr 的公开 API 把 .hex 文件加载进 flash */
    uint32_t fwsize = 0, fwstart = 0;
    uint8_t *buf = read_ihex_file(hexpath, &fwsize, &fwstart);
    if (!buf || fwsize == 0) {
        fprintf(stderr, "failed to read hex: %s\n", hexpath);
        return 1;
    }
    if (fwstart + fwsize > avr->flashend + 1) {
        fprintf(stderr, "hex too large for flash\n");
        free(buf);
        return 1;
    }
    memcpy(avr->flash + fwstart, buf, fwsize);
    avr->codeend = fwstart + fwsize - 1;
    free(buf);

    /* 挂钩 Arduino Uno 全部数字 + 模拟引脚的 GPIO IRQ
     *   PORTB bit 0-5  → D8-D13
     *   PORTC bit 0-5  → A0-A5
     *   PORTD bit 0-7  → D0-D7
     * 故意不暴露:
     *   PORTB 6/7 = 晶振脚 XTAL1/XTAL2
     *   PORTC 6   = RESET
     *   PORTC 7   = ATmega328P 上不存在
     */
    static const struct { char port; uint8_t pins; } GPIO_PORTS[] = {
        {'B', 6},  /* B0..B5 = D8..D13 */
        {'C', 6},  /* C0..C5 = A0..A5 */
        {'D', 8},  /* D0..D7 = D0..D7  */
    };

    for (size_t i = 0; i < sizeof(GPIO_PORTS) / sizeof(GPIO_PORTS[0]); i++) {
        char port = GPIO_PORTS[i].port;
        for (uint8_t bit = 0; bit < GPIO_PORTS[i].pins; bit++) {
            avr_irq_t *irq = avr_io_getirq(avr, AVR_IOCTL_IOPORT_GETIRQ(port), bit);
            if (!irq) {
                fprintf(stderr, "failed to get IRQ for PORT%c bit %u\n", port, bit);
                return 1;
            }
            avr_irq_register_notify(irq, pin_change_cb, (void *)(intptr_t)port);
        }
    }

    /* UART0 输出 → serial 事件;并关掉 simavr 默认的 stdout dump,
     * 否则原始串口文本会混进 JSON Lines 流里被 Rust 侧丢弃 */
    {
        avr_irq_t *uart_out =
            avr_io_getirq(avr, AVR_IOCTL_UART_GETIRQ('0'), UART_IRQ_OUTPUT);
        if (uart_out) {
            avr_irq_register_notify(uart_out, uart_out_cb, NULL);
        }
        uint32_t flags = 0;
        if (avr_ioctl(avr, AVR_IOCTL_UART_GET_FLAGS('0'), &flags) == 0) {
            flags &= ~AVR_UART_FLAG_STDIO;
            avr_ioctl(avr, AVR_IOCTL_UART_SET_FLAGS('0'), &flags);
        }
    }

    /* stdin 设成非阻塞,主循环轮询命令 */
    {
        int fl = fcntl(STDIN_FILENO, F_GETFL, 0);
        if (fl >= 0) fcntl(STDIN_FILENO, F_SETFL, fl | O_NONBLOCK);
    }

    log_event("{\"event\":\"hello\",\"protocol\":\"1\",\"capabilities\":[\"adc\",\"serial\"]}");

    {
        char buf[128];
        snprintf(buf, sizeof(buf),
                 "{\"event\":\"ready\",\"mcu\":\"%s\",\"freq\":%u}",
                 mcu, freq);
        log_event(buf);
    }

    int state = cpu_Running;
    /* 把 simavr 节流到墙钟时间,LED 才会按人眼可见的速率闪烁 */
    struct timespec t0;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    uint64_t t0_us = (uint64_t)t0.tv_sec * 1000000ULL +
                     (uint64_t)t0.tv_nsec / 1000ULL;
    int debug = getenv("MOXIN_BRIDGE_DEBUG") != NULL;
    uint64_t chunk_n = 0;
    while (state != cpu_Done && state != cpu_Crashed) {
        for (int i = 0; i < 2000 && state == cpu_Running; i++) {
            state = avr_run(avr);
        }
        poll_stdin_commands();
        uint64_t sim_us = (uint64_t)((double)avr->cycle * 1e6 / (double)avr->frequency);
        struct timespec now;
        clock_gettime(CLOCK_MONOTONIC, &now);
        uint64_t now_us = (uint64_t)now.tv_sec * 1000000ULL +
                          (uint64_t)now.tv_nsec / 1000ULL;
        uint64_t real_us = now_us - t0_us;
        if (debug && (chunk_n % 1000 == 0)) {
            fprintf(stderr, "[bridge] chunk=%llu cycle=%llu sim_us=%llu real_us=%llu state=%d\n",
                    (unsigned long long)chunk_n,
                    (unsigned long long)avr->cycle,
                    (unsigned long long)sim_us,
                    (unsigned long long)real_us,
                    state);
        }
        chunk_n++;
        if (sim_us > real_us + 200) {
            usleep((useconds_t)(sim_us - real_us));
        }
    }

    flush_serial_line();   /* 固件最后没换行的串口输出也别丢 */
    {
        char buf[64];
        snprintf(buf, sizeof(buf), "{\"event\":\"exit\",\"state\":%d}", state);
        log_event(buf);
    }
    return 0;
}
