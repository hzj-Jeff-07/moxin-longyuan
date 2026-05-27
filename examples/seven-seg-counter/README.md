# seven-seg-counter

Arduino Uno + 共阴 1 位 7 段数码管,每秒计数 0→9→0 循环。

**验证目标**:Phase 2-mini Step 5 数码管真驱动 — `moxin show` TUI 里
`[?] 7SEG seg1` 字符应跟 firmware 跳数字,而不是硬编码 `[8]`。

## 接线

```
D2 ─── seg1.seg_a (a 段)
D3 ─── seg1.seg_b (b 段)
D4 ─── seg1.seg_c (c 段)
D5 ─── seg1.seg_d (d 段)
D6 ─── seg1.seg_e (e 段)
D7 ─── seg1.seg_f (f 段)
D8 ─── seg1.seg_g (g 段)
D9 ─── seg1.seg_dp (小数点)
GND ── seg1.com   (共阴)
```

## 运行

```bash
cd examples/seven-seg-counter
moxin build           # arduino-cli 编译
moxin shell
moxin> run            # 启动 simavr,进 TUI
# 看 7SEG 块每秒跳一个数字:0 → 1 → 2 → ... → 9 → 0
moxin> stop
```

## 预期行为

TUI Inspector 里:

```
 D2   ━━━━━━━━━━━━━━━━━━━━━━ [3] 7SEG seg1
```

数字每秒变化(0/1/2/.../9 循环)。串口同步打印 `DIGIT=<n>`。

## AI 接口验证

```bash
moxin run --output json | grep '"pin"' | head -20
# 应看到 port=D bit=2..7 + port=B bit=0(D8) 的事件
moxin status --pin D2     # HIGH/LOW 实时反映
moxin status --pin D8     # 同上
```

## 依赖

需要 `simavr` + `arduino-cli`。`moxin doctor` 缺什么就装什么。
