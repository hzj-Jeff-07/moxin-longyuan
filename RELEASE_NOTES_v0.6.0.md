# MoXin CLI v0.6.0 — "外设批量进场"

> 发布日期:2026-07-07
> 主线代号:**Phase 3 批次 A**
> 权威计划:`docs/design/phase-3-rfc.md`

---

## 一句话总结

**v0.6.0 = 元件 8 → 13、板子 3 → 4。** 注册式抽象层兑现了它的承诺:五件新外设 + Arduino Nano,全部零主路径改动接入。

---

## 亮点(Highlights)

### 1. 五件新外设

| 元件 | 玩法 |
|---|---|
| **photoresistor**(光敏) | 注入环境光,TUI 显示 ☾/☁/☀ + 百分比;Tab 聚焦 ←/→ 调光 |
| **rgb_led** | r/g/b 三端子接 PWM 引脚,TUI 色块实时混色(`#FF8000`) |
| **servo**(SG90) | 50Hz 脉宽 → 0-180° 角度,指针字符随角度转 |
| **dc_motor**(L298N) | ena duty = 转速 %,in1/in2 = 正反转(▶ / ◀ / ■) |
| **ultrasonic**(HC-SR04) | bridge 按 58us/cm 生成 echo 回波,`pulseIn` 固件零改动 |

### 2. HC-SR04 真回波仿真(bridge 唯一改动)

```
moxin> run          # moxin 自动下发 trig/echo 引脚映射
moxin> dist 200     # 障碍物挪到 2 米
✓ dist = 200cm      # 固件 pulseIn 读数随之变为 cm=200
```

trigger 脉冲 → simavr cycle timer 调度 echo 脉宽(58us × 距离),与真实模块时序公式一致。CI verify 新增 ultrasonic 断言关卡。

### 3. Arduino Nano

同 ATmega328P 复用现有 simavr bridge,`moxin new myproj --board nano` 即用;A6/A7 两个 ADC-only 引脚已建模(通道 6/7)。

## 新 examples(共 16 个)

`ldr-nightlight` / `rgb-rainbow` / `servo-sweep` / `ultrasonic-radar`

## 质量线

- `cargo test` 173 通过(v0.5.0:146)
- clippy 0 警告;`check_schema` 13 件元件
- CI verify 三关卡:blink 翻转 + 串口事件 + 超声波回波

## 已知限制

- ADC / PWM / 超声波仅 AVR 板(Uno/Nano);STM32 留批次 B
- 距离/光强来自注入,不是电路级仿真
- 批次 B(v0.7.0):DHT11 / LCD1602 / OLED / 红外 / STM32F103
- ESP32 / Pico:主线 QEMU/simavr 无机型,书面豁免
