# A1 · 14 数字引脚全仿真

## 任务

当前桥接层只仿了 D13 / PA13 一根引脚。扩展到 Arduino UNO 全部 14 路数字引脚（D0-D13）。每根引脚都能：

- 接收 MCU 的数字 HIGH/LOW 输出，触发 BridgeEvent 上报
- 被声明在 moxin.toml 的 wire 段，自动建立路由
- 在 TUI 实时显示电平状态

事件去重：同一引脚短时间内（< 100us）多次同电平变化只上报一次。

## 允许动的文件

- `bridge/moxin-simavr-bridge.c`（IRQ 监听扩展）
- `src/sim/board.rs` 或类似板子定义处（引脚枚举扩展）
- `src/sim/runstate.rs`（RunState 增加 digital_pins: [PinLevel; 14]）
- `tests/digital_pins_e2e.rs`（新增 e2e 测试）

## 验收

```powershell
cargo test digital_pins
cargo clippy --all-targets
# 跑一个 D0-D13 全亮的 example
moxin run examples/multi-led
# TUI 显示 14 行引脚状态,全 HIGH
```

测试要点：
- 14 路引脚独立切换互不串扰
- 同一引脚 100us 内重复事件被去重
- TUI 显示与实际 GPIO 状态一致

## 约束

- 只支持 UNO 板。STM32 引脚以后做（Phase 2）
- 不破坏现有 D13 单引脚 example
- 引脚命名严格按 `board.D0` ~ `board.D13`,大小写不敏感

## commit message

`feat(A1): 14 数字引脚全仿真`
