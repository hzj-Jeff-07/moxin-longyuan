# MoXin CLI v0.7.0 — "外设收官"

> 发布日期:2026-07-12
> 主线代号:**Phase 3 批次 B(总线外设包)**
> 权威计划:`docs/design/phase-3-rfc.md`

---

## 一句话总结

**v0.7.0 = 任务书 13 件外设 100% 处置。** 完成 10 件、豁免 3 件(无上游模拟器),元件总数 17 种。项目第一次实现真正的 I2C 从机模型(LCD1602 + OLED)和单总线/红外时序回放。

---

## 亮点(Highlights)

### 1. 单总线 / 红外(边沿回放器)

| 元件 | 玩法 |
|---|---|
| **DHT11** | `env 31 75` 注入温湿度,bridge 按 DHT11 时序回 40bit |
| **红外 NEC** | `ir 20DF10EF` 发码,声明引脚后 500ms 自发一帧自检 |

一个通用边沿回放器(预排时间表 + 自重排 cycle timer)同时驱动 DHT11 应答和 NEC 帧。

### 2. I2C 从机(simavr TWI hook)

| 元件 | 玩法 |
|---|---|
| **LCD1602** | PCF8574 背包 @0x27,HD44780 4-bit 解码,TUI 蓝底双行 |
| **OLED SSD1306** | @0x3C,控制字节分命令/数据流,128×64 帧缓冲,盲文降采样渲染 |

bridge 挂 simavr 的 TWI IRQ,按 START 地址分派到对应从机;LCD/OLED 的固件 example 都用裸 `Wire.h` 手写驱动,统计每次 `endTransmission` 的 ACK —— 从机不应答即 CI 红。

### 3. STM32F103 书面豁免

QEMU 主线无 F103/BluePill 机型,用 F100 代跑属"假机型",与项目如实标注原则冲突 —— 按 ESP32/Pico 同理由豁免。

## 新 examples(共 20 个)

`dht11-weather` / `ir-remote` / `lcd-hello` / `oled-hello`

## 质量线

- `cargo test` 189 通过(v0.6.0:173)
- clippy 0 警告;`check_schema` 17 件元件
- CI verify **七道真机外设关卡**:blink / serial / ultrasonic / dht11 / ir / lcd / oled

## 任务书达标

- 三条 🚨 红线:全 ✅(自 v0.5.0 起)
- 优秀水准 13 件扩展:**完成 10 + 豁免 3(ESP32/Pico/F103),100% 处置**
- 剩:README 演示动图(可选)

## 已知限制

- 全部传感器/显示屏外设仅 AVR 板(Uno/Nano);STM32 只支持 GPIO/UART
- 注入类量来自命令/旋钮,不是电路级仿真
- OLED 帧在板面板按单行摘要显示(亮像素数 + 盲文预览),完整 16 行帧存 RunState
