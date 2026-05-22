# A3 · 6 模拟引脚 ADC 仿真

## 任务

仿真 Arduino UNO 的 6 个模拟引脚 A0-A5 的 ADC 输入。每个引脚：

- 接受 0-5V 的"虚拟电压"输入（来自电位器、光敏电阻等 analog_out 元件）
- 通过 simavr 的 ADC IRQ 上报给 MCU
- RunState 里记录每个引脚当前电压值 `voltage_mv: u16`（0-5000 毫伏）

A5 是最后一个,UNO 板硬件就 6 路 ADC 不能多。

## 允许动的文件

- `bridge/moxin-simavr-bridge.c`(ADC IRQ 注入)
- `src/sim/board.rs`(板子定义加 analog_pins)
- `src/sim/runstate.rs`(RunState 加 analog_pins: [u16; 6])
- `tests/adc_e2e.rs`

## 验收

```powershell
cargo test adc
cargo clippy --all-targets
# 用 analogRead(A0) 例子,注入 2500mV,读到值约 512(10-bit ADC: 2500/5000*1024)
```

测试要点：
- 注入 0mV / 2500mV / 5000mV,MCU 读到 0 / ~512 / ~1023
- 6 路独立,A0 改变不影响 A1
- 采样频率匹配 simavr 默认 ADC 时钟

## 约束

- 只支持 10-bit ADC (UNO 默认)。12-bit (Due/Mega) Phase 2
- 不支持 AVcc/AREF 切换,固定 5V 满量程
- 不动数字引脚部分

## commit message

`feat(A3): 6 模拟引脚 ADC 仿真`
