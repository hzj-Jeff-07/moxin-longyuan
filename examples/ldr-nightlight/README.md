# ldr-nightlight

Arduino Uno demo: 光敏小夜灯——环境光低于阈值自动点亮 LED（Phase 3 批次 A / v0.6.0）。

## Wiring

```
A0  ──── LDR (光敏电阻) out    ← ADC 通道 0
GND ──── LDR gnd
D13 ──── LED (yellow) anode
```

## 30 秒跑通

```bash
cd examples/ldr-nightlight
moxin build
moxin shell        # 进 TUI 后输入 run
```

## 调环境光

- **TUI**：`Tab` 聚焦 `ldr1`，`←` 调暗 / `→` 调亮，`Home` 全黑 / `End` 全亮
- **REPL**（`moxin shell --no-tui`）：`adc A0 100`（暗）/ `adc A0 800`（亮）

## Expected Behavior

1. 注入值 < 300 → 固件判定"天黑"，D13 LED 点亮，串口打印 `light=100 (dark, LED on)`
2. 注入值 ≥ 300 → LED 熄灭，打印 `(bright, LED off)`
3. TUI 里 `ldr1` 随光强显示 ☾ / ☁ / ☀ 图标和百分比

## 断言验证（CI / AI 用）

```bash
moxin assert --serial-contains "light=" --within 2s
```
