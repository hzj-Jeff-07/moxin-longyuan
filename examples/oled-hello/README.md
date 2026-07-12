# oled-hello

Arduino Uno demo: SSD1306 128×64 OLED 经 I2C 填充图案（Phase 3 批次 B / v0.7.0，任务书最后一件）。

## Wiring

```
A4  ──── OLED sda    ← I2C(TWI)
A5  ──── OLED scl
5V/GND ── OLED 供电(地址 0x3C)
```

## 30 秒跑通

```bash
cd examples/oled-hello
moxin build
moxin shell        # 进 TUI 后输入 run
```

## Expected Behavior

1. 仿真启动时 moxin 自动下发 `oled 3C` 启用 bridge 的 SSD1306 从机
2. 固件用裸 `Wire.h` 手写 SSD1306 初始化 + 水平寻址写满 1024 字节竖条纹（0xAA）
3. TUI 里 `oled1` 显示亮像素统计（约 4096px）和帧缓冲盲文预览
4. 固件统计每次 `endTransmission` 的 ACK——全部 ACK 才打 `oled ok`

## 断言验证（CI / AI 用）

```bash
moxin assert --serial-contains "oled ok" --within 5s
```

## 原理

bridge 复用 LCD1602 验证过的 TWI 从机骨架，按 START 地址分派到 SSD1306 模型：
控制字节 bit6=D/C# 选命令/数据流，命令流跟踪水平寻址窗口（0x21 列 / 0x22 页），
数据流按 (page,col) 写入 128×64 帧缓冲，col 自增越界翻页。帧缓冲降采样成
盲文点阵（2×4 像素/格）以 `oled` 事件发回渲染。协议见
`docs/design/bridge-protocol.md`。
