/* bridge/stm32/bridge-stm32.c · v2a Pico-pivot 后的 STM32F405 bridge
 *
 * 角色:跟 simavr bridge 平行 sibling。
 * 输入:命令行 argv[1] = .elf 路径
 * 后端:fork/exec qemu-system-arm,机器 = netduinoplus2,串口接 stdio
 * 输出:JSON Lines 到自己 stdout,跟 simavr bridge 协议兼容(并扩展 serial event)
 *   {"event":"ready","mcu":"stm32f405","freq":16000000}
 *   {"event":"pin","t_us":<NUM>,"port":"GPIO","bit":<n>,"value":0|1}
 *   {"event":"serial","t_us":<NUM>,"line":"<escaped>"}
 *   {"event":"exit","state":<n>}
 *
 * 解析逻辑:从 QEMU stdout(= 仿真器 USART2 TX 流)逐行读;
 *   "PIN<n>=<v>" 行解析为 pin event(任何 n 值都接,demo 主要看 PIN13)
 *   其它行原样作为 serial event(给 V2 Serial Monitor 喂数据)
 *
 * QEMU stderr → 直接 inherit 到 bridge stderr(主进程已经把 bridge stderr
 * 接到 ~/.cache/moxin/bridge.log,不污染主屏)。
 */

#include <ctype.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

static pid_t qemu_pid = -1;

static uint64_t now_us(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000ull + (uint64_t)(ts.tv_nsec / 1000);
}

static uint64_t g_t0_us = 0;

static uint64_t mcu_t_us(void) {
    /* QEMU netduinoplus2 -icount 我们没启用,这里把"距 ready 的 host 微秒数"
       当作 t_us 出去。给 RunState.last_event_t_us / loop_time_us 用足够。 */
    uint64_t now = now_us();
    return now > g_t0_us ? (now - g_t0_us) : 0;
}

/* JSON 字符串 escape:把 line 里的 \, ", \r, \n, \t 转义。控制字符简单丢弃。
 * 输出写到 dst,容量 dst_cap。返回写入字节数(不含 null)。 */
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
                if (c < 0x20) { /* 其它控制字符:跳过 */ break; }
                dst[out++] = (char)c;
        }
    }
    dst[out] = '\0';
    return out;
}

/* "PIN<n>=<v>" 解析。成功返回 1 并填 *bit、*value;失败返回 0。 */
static int parse_pin_line(const char *line, int *bit, int *value) {
    if (strncmp(line, "PIN", 3) != 0) return 0;
    const char *p = line + 3;
    if (!isdigit((unsigned char)*p)) return 0;
    int n = 0;
    while (isdigit((unsigned char)*p)) {
        n = n * 10 + (*p - '0');
        ++p;
    }
    if (*p != '=') return 0;
    ++p;
    if (*p != '0' && *p != '1') return 0;
    *bit = n;
    *value = (*p - '0');
    return 1;
}

static void emit_ready(void) {
    printf("{\"event\":\"ready\",\"mcu\":\"stm32f405\",\"freq\":16000000}\n");
    fflush(stdout);
}

static void emit_pin(uint64_t t_us, int bit, int value) {
    printf("{\"event\":\"pin\",\"t_us\":%llu,\"port\":\"GPIO\",\"bit\":%d,\"value\":%d}\n",
           (unsigned long long)t_us, bit, value);
    fflush(stdout);
}

static void emit_serial(uint64_t t_us, const char *line) {
    char buf[1024];
    json_escape(line, buf, sizeof(buf));
    printf("{\"event\":\"serial\",\"t_us\":%llu,\"line\":\"%s\"}\n",
           (unsigned long long)t_us, buf);
    fflush(stdout);
}

static void emit_exit(int state) {
    printf("{\"event\":\"exit\",\"state\":%d}\n", state);
    fflush(stdout);
}

static void on_signal(int sig) {
    if (qemu_pid > 0) kill(qemu_pid, SIGTERM);
    /* 让 main 走自然退出路径(waitpid 会拿到子进程退出码) */
    (void)sig;
}

static int run_qemu(const char *elf_path) {
    int pipefd[2];
    if (pipe(pipefd) != 0) {
        fprintf(stderr, "bridge-stm32: pipe failed: %s\n", strerror(errno));
        return -1;
    }

    pid_t pid = fork();
    if (pid < 0) {
        fprintf(stderr, "bridge-stm32: fork failed: %s\n", strerror(errno));
        return -1;
    }
    if (pid == 0) {
        /* 子进程:把 QEMU stdout 接到 pipe[1],stderr 走 inherit */
        close(pipefd[0]);
        dup2(pipefd[1], 1);  /* qemu stdout → pipe write */
        close(pipefd[1]);
        /* qemu stdin 接 /dev/null,免得它从 inherit 的 stdin 抢字符 */
        int devnull = open("/dev/null", O_RDONLY);
        if (devnull >= 0) { dup2(devnull, 0); close(devnull); }

        execlp("qemu-system-arm",
               "qemu-system-arm",
               "-M", "netduinoplus2",
               "-cpu", "cortex-m4",
               "-kernel", elf_path,
               "-nographic",
               "-monitor", "none",
               "-serial", "stdio",
               (char *)NULL);
        fprintf(stderr, "bridge-stm32: execlp qemu-system-arm failed: %s\n",
                strerror(errno));
        _exit(127);
    }

    /* 父进程:从 pipe 读 UART 文本,逐行解析 */
    qemu_pid = pid;
    close(pipefd[1]);

    signal(SIGINT,  on_signal);
    signal(SIGTERM, on_signal);
    signal(SIGPIPE, SIG_IGN);

    g_t0_us = now_us();
    emit_ready();

    FILE *qemu_out = fdopen(pipefd[0], "r");
    if (!qemu_out) {
        fprintf(stderr, "bridge-stm32: fdopen failed\n");
        kill(pid, SIGTERM);
        waitpid(pid, NULL, 0);
        return -1;
    }

    char line[1024];
    while (fgets(line, sizeof(line), qemu_out)) {
        /* 去掉行尾 \r\n */
        size_t len = strlen(line);
        while (len && (line[len - 1] == '\n' || line[len - 1] == '\r')) {
            line[--len] = '\0';
        }
        if (len == 0) continue;

        int bit, value;
        if (parse_pin_line(line, &bit, &value)) {
            emit_pin(mcu_t_us(), bit, value);
        } else {
            emit_serial(mcu_t_us(), line);
        }
    }
    fclose(qemu_out);

    int status = 0;
    waitpid(pid, &status, 0);
    int exit_state = WIFEXITED(status) ? WEXITSTATUS(status) : -1;
    emit_exit(exit_state);
    return exit_state;
}

int main(int argc, char *argv[]) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s <elf>\n", argv[0]);
        return 2;
    }
    const char *elf_path = argv[1];
    if (access(elf_path, 4) != 0) { /* R_OK = 4 */
        fprintf(stderr, "bridge-stm32: cannot read %s: %s\n", elf_path, strerror(errno));
        return 1;
    }
    return run_qemu(elf_path);
}
