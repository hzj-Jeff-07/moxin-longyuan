# pwm-fade

Arduino Uno demo: D9 呼吸灯，验证 MoXin 的 PWM 占空比追踪（v0.5.0 / Phase 2-full Step 3）。

## Wiring

```
D9  ──── LED (green) anode    ← Timer1 硬件 PWM 引脚
GND ──── LED cathode
```

## 30 秒跑通

```bash
cd examples/pwm-fade
moxin build
moxin shell        # 进 TUI 后输入 run
```

## Expected Behavior

1. 固件用 `analogWrite(9, duty)` 让 duty 在 0→255→0 之间来回扫（每 50ms 一步）
2. TUI 接线图里 `led1` 不再显示 `ON/OFF`，而是随呼吸变化的占空比百分比（如 `[GRE 50%]`）
3. Serial Monitor 每 50ms 打一行 `duty=<n>`，与显示的百分比对得上
4. duty 扫到 0 或 255 时没有边沿，PWM 采样过期，显示回退到 `OFF` / `ON` —— 这是预期行为

## 断言验证（CI / AI 用）

```bash
moxin assert --pin D9 --toggles --within 3s   # PWM 波形在翻转
moxin assert --serial-contains "duty=" --within 2s
```

## 原理

MoXin 不改 bridge C 代码：`simavr` 推送的每条 `pin` 边沿事件在 Rust 侧喂给
`PwmTracker`，连续 3 个周期频率偏差 ≤5% 即判定为稳定 PWM，由边沿时间差算出
duty / freq（见 `docs/design/phase-2-full-rfc.md` Step 3）。
