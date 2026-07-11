# servo-sweep

Arduino Uno demo: SG90 舵机 0-180° 来回扫（Phase 3 批次 A / v0.6.0）。

## Wiring

```
D9  ──── Servo signal    ← 50Hz PWM,脉宽 500-2500us
5V  ──── Servo vcc
GND ──── Servo gnd
```

## 30 秒跑通

```bash
cd examples/servo-sweep
moxin build
moxin shell        # 进 TUI 后输入 run
```

## Expected Behavior

1. 固件手写 50Hz 软 PWM（不依赖 Servo 库），角度每 100ms 走 15°
2. TUI 里 `sv1` 显示实时角度 `[SERVO 90°]`，指针字符随角度转（← ↖ ↑ ↗ →）
3. 串口每步打印 `angle=<deg>`，与显示对得上
4. 490Hz 的 analogWrite 波形不会被误判成舵机信号（频率窗口 40-100Hz）

## 断言验证（CI / AI 用）

```bash
moxin assert --serial-contains "angle=" --within 2s
```
