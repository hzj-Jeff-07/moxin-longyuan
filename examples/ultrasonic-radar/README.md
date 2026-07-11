# ultrasonic-radar

Arduino Uno demo: HC-SR04 超声波测距（Phase 3 批次 A / v0.6.0）。

## Wiring

```
D7  ──── SR04 trig
D8  ──── SR04 echo
5V  ──── SR04 vcc
GND ──── SR04 gnd
```

## 30 秒跑通

```bash
cd examples/ultrasonic-radar
moxin build
moxin shell --no-tui
```

```
moxin> run
moxin> sleep 1500
moxin> dist 200        # 把"障碍物"挪到 2 米外
✓ dist = 200cm
```

## Expected Behavior

1. 仿真启动时 moxin 自动把 trig=D7 / echo=D8 下发给 bridge（stdin `sr04 D 7 B 0`）
2. 固件打 10us 触发脉冲，bridge 约 200us 后拉高 echo，持续 58us × 距离
3. 未注入时用默认 50cm：串口打印 `cm=50` 上下（±1 舍入）
4. `dist 200` 之后串口读数变为 `cm=200` 上下；TUI 里 `us1` 显示 `[SR04 200cm]`

## 断言验证（CI / AI 用）

```bash
moxin assert --serial-contains "cm=" --within 3s
```

## 原理

bridge 在 trigger 引脚的下降沿（脉宽 ≥2us）用 simavr cycle timer 调度 echo
脉冲，脉宽 = 58us/cm——与真实模块的时序公式一致，固件的 `pulseIn` 代码
不需要任何改动。协议见 `docs/design/bridge-protocol.md`。
