# stm32-blink

STM32F405 (netduinoplus2) blink demo: PA13 每秒翻转一次，同时通过 USART2 打印
`PIN13=0/1` 与 loop counter。

## 接线

```
PA13 ──── LED  (green)  anode
GND  ──── LED          cathode
USART2 TX (PA2) → moxin Serial Monitor
```

`moxin.toml` 的 wire 用 STM32 原生引脚名 `board.PA13`，不要写 `D13`（那是
Arduino Uno 风格，STM32 板上不存在）。

## 运行

```bash
cd examples/stm32-blink
moxin build           # arm-none-eabi-gcc 编译 main.c → blink.elf
moxin shell           # 进 shell
moxin> run            # 启动 qemu-system-arm + bridge，进入 TUI
moxin> stop           # 停止仿真
```

预期：

1. TUI Serial Monitor 显示 `STM32F405 blink starting...` banner
2. 每秒一行 `PIN13=1` / `PIN13=0` 交替
3. 每 4 次循环输出一次 `loop counter=<N>`
4. AI Inspector 显示 `GPIO13: HIGH/LOW` 跟随翻转

## 依赖

需要 `qemu-system-arm` + `arm-none-eabi-gcc`。装好后 `moxin doctor` 应该全绿。
