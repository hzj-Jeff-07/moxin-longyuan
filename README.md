[![License: BUSL-1.1](https://img.shields.io/badge/license-BUSL--1.1-orange)](LICENSE)

# 模芯 MoXin

在终端里仿真运行嵌入式固件，不需要实体开发板。

## 当前状态

v0.4.0,两块板子可以跑通:

- **Arduino Uno**(通过 simavr):全 PORTB/C/D GPIO、UART、按钮输入、数码管 8 段真驱动
- **STM32F405**(通过 QEMU netduinoplus2):GPIO、UART

四面板 TUI 界面：

```
┌[Board 接线图]──────────────────┐┌[AI Inspector]──────────────┐
│ D13 ━━━━━━━━━━━━━━━━━━━━━━ ● led1 [GRE ON ] ││ ✓ Voltage:   3.30V         │
│  L (built-in)  ●            ││ ✓ GPIO13:    HIGH           │
│                             ││ ✓ Loop Time: 2ms            │
│                             ││ Status: OK                  │
└─────────────────────────────┘│                             │
┌[Serial Monitor]─────────────┐│                             │
│> loop counter=1024          ││                             │
│> loop counter=1028          ││                             │
└─────────────────────────────┘└─────────────────────────────┘
moxin >
```

## 安装

先决条件：Rust 1.75+。

```bash
cargo install --git https://github.com/hzj-Jeff-07/moxin-longyuan
```

安装完拿 `moxin doctor` 自检外部依赖：

```bash
moxin doctor
```

### 外部依赖（按需）

| 板子 | 依赖 | macOS | Linux (Debian/Ubuntu) | Windows |
|------|------|-------|------------------------|---------|
| Arduino Uno | simavr | `brew install simavr` | `apt install simavr` | `scoop install simavr`（或自行编译） |
| Arduino Uno | arduino-cli | `brew install arduino-cli` | 见 [arduino-cli 官方安装](https://arduino.github.io/arduino-cli/latest/installation/) | 同左 |
| STM32F405 | qemu-system-arm | `brew install qemu` | `apt install qemu-system-arm` | `scoop install qemu` |
| STM32F405 | arm-none-eabi-gcc | `brew install --cask gcc-arm-embedded` | `apt install gcc-arm-none-eabi` | `scoop install gcc-arm-none-eabi` |

只跑哪块板就装哪块板的依赖。`moxin doctor` 输出会告诉你缺什么。

## 快速开始

```bash
# 跑 STM32 blink demo
cd examples/stm32-blink
moxin build    # 编译固件（support 文件已内嵌，无需额外配置）
moxin shell    # 进入交互 shell
```

进入 shell 后：

```
moxin> run      # 启动仿真，进入 TUI
moxin> stop     # 停止仿真，回到 shell
moxin> help     # 查看所有命令
```

## Shell 命令

| 命令 | 说明 |
|------|------|
| `run` | 启动仿真，进入四面板 TUI |
| `stop` | 停止仿真，保持在 shell |
| `build` | 编译当前项目固件 |
| `add led <颜色> --id <id>` | 添加 LED 组件 |
| `add button --id <id>` | 添加按钮组件 |
| `wire <引脚> -> <组件.端子>` | 连线 |
| `show` | 查看当前接线状态 |
| `board info` | 查看板子规格 |

示例：
```
moxin> add led red --id led1
moxin> wire pin13 -> led1.a
moxin> build
moxin> run
```

TUI 运行中按 `Esc` 退出。

## JSON 输出（给 AI 工具消费）

`moxin run --output json` 不开 TUI，把 bridge 的事件流以 JSON Lines 直接透传到
stdout，每行一个事件，可被 `jq -c` 消费；状态提示走 stderr，不污染 stdout。

```bash
moxin run --output json | jq -c
# {"event":"ready","mcu":"atmega328p","freq":16000000}
# {"event":"pin","t_us":12345,"port":"B","bit":5,"value":1}
# {"event":"serial","t_us":12346,"line":"hello"}
```

Ctrl-C 停止；bridge 自行退出时也会自动结束。

## 项目结构

```
examples/
  stm32-blink/        STM32F405 blink(推荐入门)
  led-control/        Arduino Uno,双 LED + 按钮 + serial 控制
  button-counter/     Arduino Uno,按 'b' 计数 + D13 LED 翻转
  serial-echo/        Arduino Uno,串口回显 + D13 RX 指示灯
  multi-led-chase/    Arduino Uno,6 颗 LED D2-D7 ping-pong 走马灯
  seven-seg-counter/  Arduino Uno,数码管 0-9 滚动(8 段真驱动)
  button-led-pair/    Arduino Uno,Serial 'b' 翻 D4 LED(故意非 D13)
  pin-state-snapshot/ Arduino Uno,D2-D12 棋盘快照,供 status 全引脚查询
  assert-blink-toggles/ moxin assert --pin --toggles 验证用
  assert-serial-hello/  moxin assert --serial 验证用
bridge/
  stm32/           STM32 bridge 源码(已内嵌进 moxin binary)
  moxin-simavr-bridge.c   AVR bridge 源码
src/               Rust 主程序
```

## 已知限制

- Arduino Uno 需要额外安装 simavr
- AI Inspector 当前为纯状态展示,外接 LLM 接口预留在 v3
- ADC / PWM / I2C / SPI 暂不仿真(留 v0.5.0 / Phase 2-full)

## License

本项目采用 Business Source License 1.1 (BUSL-1.1) 协议。

非商业用途（个人学习、研究、非营利使用）免费。

商业授权请联系：pyroviafire@gmail.com（备用：19136311901）

根据 BUSL 协议，本项目将于 2030-05-10 自动转为 Apache License 2.0。
