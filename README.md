[![License: BUSL-1.1](https://img.shields.io/badge/license-BUSL--1.1-orange)](LICENSE)

# 模芯 MoXin

在终端里仿真运行嵌入式固件，不需要实体开发板。

## 当前状态

v0.1.0-demo，两块板子可以跑通：

- **Arduino Uno**（通过 simavr）：GPIO、UART、按钮输入
- **STM32F405**（通过 QEMU netduinoplus2）：GPIO、UART

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

## 项目结构

```
examples/
  stm32-blink/     STM32F405 blink（推荐入门）
  led-control/     Arduino Uno，双 LED + 按钮 + serial 控制
bridge/
  stm32/           STM32 bridge 源码（已内嵌进 moxin binary）
  moxin-simavr-bridge.c   AVR bridge 源码
src/               Rust 主程序
```

## 已知限制

- 目前只有 D13 引脚有真实仿真状态，其他引脚的 LED 显示静态 OFF
- Arduino Uno 需要额外安装 simavr
- AI Inspector 当前为纯状态展示，外接 LLM 接口预留在 v3

## License

本项目采用 Business Source License 1.1 (BUSL-1.1) 协议。

非商业用途（个人学习、研究、非营利使用）免费。

商业授权请联系：pyroviafire@gmail.com（备用：19136311901）

根据 BUSL 协议，本项目将于 2030-05-10 自动转为 Apache License 2.0。
