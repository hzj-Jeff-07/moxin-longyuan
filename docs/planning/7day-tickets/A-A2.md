# A2 · 蜂鸣器元件 schema + 仿真

## 任务

加入有源蜂鸣器 (active buzzer) 元件，支持 PWM 输入控制频率。

仿真层：监听 PWM 信号的 duty cycle 和频率，在 RunState 里更新 `frequency_hz` 和 `playing: bool` 状态字段。

## 允许动的文件

- `components/buzzer.toml`（如果 S1 没建,本 ticket 建）
- `pin-anchors-template/buzzer.json`（同上）
- `bridge/moxin-simavr-bridge.c`（PWM IRQ 监听）
- `src/sim/components/buzzer.rs`（新文件,元件状态机）
- `src/sim/runstate.rs`(注册 buzzer 实例状态)
- `tests/buzzer_e2e.rs`

## 验收

```powershell
cargo test buzzer
cargo clippy --all-targets
# 用 tone(440) 跑蜂鸣器,RunState 里 frequency_hz=440
```

测试要点：
- PWM 频率 100Hz / 440Hz / 1kHz 三档,RunState frequency_hz 字段误差 ±5%
- duty=0 时 playing=false
- 元件 schema 校验通过 (scripts/check_schema.py)

## 约束

- 只支持有源蜂鸣器(频率由信号决定)。无源蜂鸣器(频率由方波驱动)Phase 2
- 不引入音频输出依赖,只在 RunState 里更新数值
- pin name 严格用 `signal` / `gnd`,electrical 字段 `pwm_in` / `gnd`

## commit message

`feat(A2): 蜂鸣器元件 schema 与仿真`
