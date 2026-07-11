# rgb-rainbow

Arduino Uno demo: RGB LED 循环混色（Phase 3 批次 A / v0.6.0）。

## Wiring

```
D9  ──── RGB r    ← Timer1 PWM
D10 ──── RGB g    ← Timer1 PWM
D11 ──── RGB b    ← Timer2 PWM
GND ──── RGB cathode
```

## 30 秒跑通

```bash
cd examples/rgb-rainbow
moxin build
moxin shell        # 进 TUI 后输入 run
```

## Expected Behavior

1. 固件让三通道 duty 按相位错开的三角波循环（每 60ms 一步）
2. TUI 里 `rgb1` 的色块 `██` 连续变色，标签显示当前混色 `#RRGGBB`
3. 串口每步打印 `rgb=<r>,<g>,<b>`，与色块对得上
4. duty 扫到 0/255 的通道瞬间无边沿，按数字电平 0/255 处理——混色仍正确

## 断言验证（CI / AI 用）

```bash
moxin assert --serial-contains "rgb=" --within 2s
```
