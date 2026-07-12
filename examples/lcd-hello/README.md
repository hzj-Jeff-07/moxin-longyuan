# lcd-hello

Arduino Uno demo: LCD1602 经 PCF8574 I2C 背包显示两行文字（Phase 3 批次 B / v0.7.0）。

## Wiring

```
A4  ──── LCD sda    ← I2C(TWI)
A5  ──── LCD scl
5V/GND ── LCD 供电(背包地址 0x27)
```

## 30 秒跑通

```bash
cd examples/lcd-hello
moxin build
moxin shell        # 进 TUI 后输入 run
```

## Expected Behavior

1. 仿真启动时 moxin 自动下发 `lcd 27` 启用 bridge 的 I2C 从机
2. 固件用裸 `Wire.h` 手写 4-bit 初始化 + 写字（不依赖第三方库）
3. TUI 里 `lcd1` 显示蓝底两行：`Hello MoXin!` / `LCD1602 via I2C`
4. 固件统计每次 `endTransmission` 的返回值——从机全部 ACK 才打 `lcd ok`，
   任何 NACK 打 `lcd err`

## 断言验证（CI / AI 用）

```bash
moxin assert --serial-contains "lcd ok" --within 5s
```

## 原理

bridge 挂 simavr 的 TWI IRQ 实现 PCF8574 从机（应答模式参考 simavr 自带的
i2c_eeprom 测试件）：START 匹配地址 0x27 → ACK；每个写入字节按 P2=EN 的
下降沿锁存 P4-7 的 nibble，两两拼成 HD44780 命令/数据；字符写入 30ms 合并
后以 `lcd` 事件发回 moxin 渲染。协议见 `docs/design/bridge-protocol.md`。
