# MoXin CLI v0.5.0 — "模拟量进场"

> 发布日期:2026-07-07
> 主线代号:**Phase 2-full**
> 权威计划:`docs/design/phase-2-full-rfc.md`

---

## 一句话总结

**v0.5.0 = 元件不再硬编码 + ADC/PWM 真通道。** 加新元件从"改三处 match"变成"注册一个 `ComponentDef`";电位器可以转、呼吸灯能看到占空比、蜂鸣器显示音调频率。

---

## 亮点(Highlights)

### 1. ComponentDef 注册式(拔掉最后一条硬编码红线)

`src/components/` 下每个元件一个文件,render / shell / inspector 三处 match 收敛为单一 registry 调度。新增元件 = 实现 trait + 注册一行,不碰主路径。

### 2. ADC 真仿真(bridge protocol "1")

```
moxin> run
moxin> adc A0 512
✓ adc ch0 = 512
```

- bridge 新增 stdin 命令通道,`adc <ch> <value>` 经 simavr ADC IRQ 注入,固件 `analogRead` 读到真值
- TUI:`Tab` 聚焦电位器,`←/→` 转旋钮,`Home/End` 到 0/1023
- 电位器渲染升级:`[POT ▮▮▮▮▮▯▯▯▯▯ 50% (512) 10kΩ]`
- bridge 启动先发 `hello` 宣告协议版本与能力,老 bridge 二进制会被明确报错提示重编

### 3. PWM 追踪(纯 Rust 侧,bridge 不参与)

- `PwmTracker` 从 pin 边沿时间差推导 duty/freq,连续 3 周期偏差 ≤5% 判定稳定
- LED 在 PWM 引脚(Uno D3/5/6/9/10/11)显示占空比:`[GRE 50%]`
- 蜂鸣器任意引脚显示 tone 频率:`♪ 1000Hz`
- 慢速 blink(<20Hz)不会被误判成调光

### 4. 修复:Uno 串口输出终于进事件流了

排查发现 AVR bridge 从未发过 `serial` 事件——`Serial.println` 的输出被 simavr 直接 dump 到 stdout,再被 Rust 侧当非 JSON 丢弃。现在 UART0 挂 IRQ 按行发事件,`serial-echo` / `assert-serial-hello` 在 Uno 上真正可用,CI verify 关卡同步加了串口断言。

## 新 examples(共 12 个)

| 例子 | 验证什么 |
|---|---|
| `adc-potentiometer` | ADC 注入 + TUI 旋钮 + `adc` 命令 |
| `pwm-fade` | D9 呼吸灯,PWM 占空比追踪 |

## 质量线

- `cargo test` 146 通过(v0.4.0:119)
- `cargo clippy --all-targets -- -D warnings` 0 警告
- CI verify 关卡新增 UART 串口断言

## 已知限制

- ADC / PWM 仅 Arduino Uno;STM32 留 Phase 3
- ADC 值来自注入(旋钮/命令),不是电路级仿真
- PWM 为边沿推导:duty 到 0/255 时无边沿,显示回退 ON/OFF(预期行为)
- 串口 RX 注入(向固件发字符)仍未实现
- I2C / SPI / OLED / 13 件外设扩展 → v0.6.0(Phase 3)
