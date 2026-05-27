# multi-led-chase

Arduino Uno demo:D2-D7 上的 6 颗 LED 走马灯,每 200ms 移动一次,循环往返。

**验证目标**:Phase 2-mini Step 1(bridge 全 PORTD GPIO hook)+ Step 3
(render 接全 GPIO)— 6 颗非 D13 LED 应**真**跟随 firmware 翻转,
而不再静态 OFF。

## 接线

```
D2 ─── LED1 (red)    anode
D3 ─── LED2 (yellow) anode
D4 ─── LED3 (green)  anode
D5 ─── LED4 (blue)   anode
D6 ─── LED5 (white)  anode
D7 ─── LED6 (red)    anode
GND ── LEDx cathode (共地)
```

## 运行

```bash
cd examples/multi-led-chase
moxin build
moxin shell
moxin> run            # 启动 simavr,进 TUI
# Inspector 里看 6 行 LED,光点每 200ms 走一格(2→7→2 循环)
moxin> stop
```

## 预期行为

TUI Inspector 一帧示例(光点在 D4 时):

```
 D2   ━━━━━━━━━━━━━━━━━━━━━━ ○ led1 [RED OFF]
 D3   ━━━━━━━━━━━━━━━━━━━━━━ ○ led2 [YEL OFF]
 D4   ━━━━━━━━━━━━━━━━━━━━━━ ● led3 [GRE ON ]
 D5   ━━━━━━━━━━━━━━━━━━━━━━ ○ led4 [BLU OFF]
 D6   ━━━━━━━━━━━━━━━━━━━━━━ ○ led5 [WHI OFF]
 D7   ━━━━━━━━━━━━━━━━━━━━━━ ○ led6 [RED OFF]
```

## AI 接口验证

```bash
moxin run --output json | grep '"pin"' | head -30
# 应看到 port=D bit=2,3,4,5,6,7 全部出现
moxin status --pin D4   # 实时:HIGH 或 LOW
```

## 依赖

需要 `simavr` + `arduino-cli`。`moxin doctor` 缺什么就装什么。
