# adc-potentiometer

Arduino Uno demo: A0 电位器真 ADC 采样，验证 MoXin 的 ADC 注入通道（v0.5.0 / Phase 2-full Step 2）。

## Wiring

```
A0  ──── POT (10kΩ) wiper    ← ADC 通道 0
GND ──── POT 一端
5V  ──── POT 另一端
```

## 30 秒跑通

```bash
cd examples/adc-potentiometer
moxin build
moxin shell        # 进 TUI 后输入 run
```

## 转旋钮

两种方式：

- **TUI**：`Tab` 聚焦 `pot1`，`←` / `→` 每步 ±32（约 3%），`Home` / `End` 直接到 0 / 1023
- **REPL / 脚本**（`moxin shell --no-tui`）：

```
moxin> run
moxin> adc A0 512
✓ adc ch0 = 512
```

## Expected Behavior

1. 固件每 200ms `analogRead(A0)` 并打印 `A0=<raw> (<pct>%)`
2. 注入 `adc A0 512` 后，串口输出变为 `A0=511 (49%)` 上下（simavr 内部 mV 换算有 ±1 舍入）
3. TUI 里 `pot1` 显示进度条和百分比（`[POT ▮▮▮▮▮▯▯▯▯▯ 50% (512) 10kΩ]`），不再是静态阻值
4. 没注入过值时显示静态 `[POT 10kΩ]` —— 旋钮位置未知，属预期

## 断言验证（CI / AI 用）

```bash
moxin assert --serial-contains "A0=" --within 2s
```

## 原理

`moxin` 把 `adc <ch> <value>` 写进 bridge 子进程 stdin；bridge 在 `avr_run`
间隙非阻塞轮询命令，换算成 mV 后经 `avr_raise_irq` 注入 simavr 的 ADC IRQ，
固件下次 `analogRead` 即读到注入值。协议见 `docs/design/bridge-protocol.md`。
