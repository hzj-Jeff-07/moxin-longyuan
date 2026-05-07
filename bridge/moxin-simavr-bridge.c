/*
 * moxin-simavr-bridge.c — moxin demo 的 simavr 子进程桥接
 *
 * 启动:  bridge <hexfile> [mcu] [freq_hz]
 * 标准输出: 每行一条 JSON (JSON Lines 格式)
 *   {"event":"ready","mcu":"atmega328p","freq":16000000}
 *   {"event":"pin","t_us":NUM,"port":"B","bit":5,"value":0|1}
 *   {"event":"exit","state":N}
 *
 * 设计:本进程链接 libsimavr.a (GPL-3.0+),与 moxin Rust 主进程完全分离,
 * 仅通过 stdio 通信,规避 GPL 传染。
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>
#include <time.h>

#include "sim_avr.h"
#include "sim_hex.h"
#include "avr_ioport.h"

static avr_t *g_avr = NULL;

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

    /* 挂钩 PORTB 第 5 位 (即 Arduino D13 / 板载 LED 引脚) */
    avr_irq_t *irq_b5 = avr_io_getirq(avr, AVR_IOCTL_IOPORT_GETIRQ('B'), 5);
    if (!irq_b5) {
        fprintf(stderr, "failed to get IRQ for PORTB bit 5\n");
        return 1;
    }
    avr_irq_register_notify(irq_b5, pin_change_cb, (void *)(intptr_t)'B');

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

    {
        char buf[64];
        snprintf(buf, sizeof(buf), "{\"event\":\"exit\",\"state\":%d}", state);
        log_event(buf);
    }
    return 0;
}
