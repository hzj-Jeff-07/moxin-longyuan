[![License: BUSL-1.1](https://img.shields.io/badge/license-BUSL--1.1-orange)](LICENSE)

# MoXin

MoXin is a hardware simulator CLI for embedded development — run your Arduino/STM32 firmware in a terminal without physical hardware.

## Why not Wokwi?

Wokwi is a web-based visual simulator. MoXin is a local CLI tool designed for:
- **AI-friendly**: structured JSON event stream, inspector panel for LLM integration (v3)
- **Offline**: no browser, no account, runs in your terminal
- **Scriptable**: pipe commands, automate smoke tests, integrate with CI

## Current Status

v2b — two boards working end-to-end:
- **Arduino Uno** (via simavr): GPIO, UART, button input
- **STM32F405** (via QEMU netduinoplus2): GPIO, UART

## Install

```bash
# Prerequisites
brew install arduino-cli
brew install --cask gcc-arm-embedded
brew install qemu

# Build
cargo build --release

# Run the led-control demo
cd examples/led-control
moxin build
moxin shell
```

## Demo

```
Board          Wires          Components
─────          ─────          ──────────
D13 ━━━━━━━━━━━━━━━━━━━━━━ ● led_r [RED ON ]
D12 ━━━━━━━━━━━━━━━━━━━━━━ ○ led_g [GRE OFF]
D2  ━━━━━━━━━━━━━━━━━━━━━━ ● btn1  [BTN UP ]
```

Type `r`, `g`, `s`, `?` in the Serial Monitor to control LEDs.

## License

本项目采用 Business Source License 1.1 (BUSL-1.1) 协议。

非商业用途（个人学习、研究、非营利使用）免费。

商业授权请联系：pyroviafire@gmail.com

根据 BUSL 协议，本项目将于 2030-05-10 自动转为 Apache License 2.0。
