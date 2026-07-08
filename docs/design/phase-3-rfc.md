# Phase 3 RFC — 外设批量扩展(v0.6.0 / v0.7.0)

> 状态:**草案 / 待用户批准启动**
> 前置:v0.5.0(Phase 2-full)已合并 main,tag 待打
> 起点 commit:`5031d14`(main,PR #3 merge)
> 目标版本:**v0.6.0(模拟量外设包)→ v0.7.0(总线外设包)**
> 预估工作量:v0.6.0 约 2-3 周;v0.7.0 约 3-4 周(2026 暑假)
> 最后更新:2026-07-07

---

## 一、为什么做这个 RFC

任务书"优秀水准"剩最后一块:**扩展 13 件(4 板 + 9 外设),当前 0/13**。

Phase 2-full 已经把三条基础设施铺好:

1. **ComponentDef 注册式** — 加外设 = `src/components/<name>.rs` + 注册一行,不碰主路径
2. **ADC 注入通道**(bridge protocol "1" stdin 命令) — 光敏等模拟输入元件的数据来源
3. **PWM 追踪**(Rust 侧边沿推导) — 舵机 / RGB / 电机的信号解读能力

13 件里的大多数现在是"接线"工作而不是"修路"工作。但 13 件难度差异极大,
一锅炖必翻车 —— 本 RFC 的核心主张是**按依赖的基础设施分两批**。

## 二、13 件盘点与分批(核心决策)

### 批次 A:v0.6.0"模拟量外设包" — 只用现有通道,bridge 零改动或微改

| # | 件 | 依赖 | 难度 | 说明 |
|---|---|---|---|---|
| 1 | 光敏电阻 LDR | ADC 注入(现成) | ★ | potentiometer 的孪生:环境光注入 → analogRead |
| 2 | RGB LED | PWM 追踪(现成) | ★★ | 三个 anode 端子接三个 pwm 引脚,渲染混色色块 |
| 3 | 舵机 SG90 | PWM 追踪(现成) | ★★ | 50Hz 脉宽 0.5-2.5ms → 0-180°;`PwmSample` 的 duty×period 即脉宽 |
| 4 | 直流电机(+L298N) | PWM 追踪(现成) | ★★ | duty → 转速百分比 + 方向脚电平 → 正反转显示 |
| 5 | HC-SR04 超声波 | bridge 微改(echo 回脉冲) | ★★★ | stdin `dist <cm>` 注入距离;bridge 监听 trigger 上升沿,按距离换算 delay 后拉 echo |
| 6 | Arduino Nano 板 | 无(同 ATmega328P/simavr) | ★ | 复用现有 bridge,仅 BoardSpec 引脚丝印不同 |

### 批次 B:v0.7.0"总线外设包" — 需要 bridge 挂 simavr TWI/单总线 hook

| # | 件 | 依赖 | 难度 | 说明 |
|---|---|---|---|---|
| 7 | DHT11 | 单总线状态机(bridge 新写) | ★★★★ | stdin `env <id> temp/hum <v>` 注入;bridge 按 DHT 时序回 40bit |
| 8 | LCD1602 (I2C) | simavr TWI hook + PCF8574 模型 | ★★★★ | TUI 渲染 16×2 字符区 |
| 9 | OLED SSD1306 | simavr TWI hook + 帧缓冲 | ★★★★★ | 128×64 → TUI 盲文点阵(⣿)降采样渲染 |
| 10 | STM32F103C8T6 板 | QEMU stm32vldiscovery 机型验证 | ★★★ | qemu 支持 F1 系机型,bridge-stm32 参数化 |
| 11 | 红外收发 | 38kHz 载波建模 | ★★★★ | NEC 协议注入/解码 |

### ❌ 仍不做(禁区维持)

- **ESP32 / RP2040(Pico)板** — 主线 QEMU/simavr 没有现成可用机型,CLAUDE.md 禁区不动
- MCP server(留 v3)/ GUI / CPU 内核
- 被动元件 runtime(电阻分压)、面包板连通性引擎 — 独立 feature,不塞进外设批次

### 里程碑对照任务书

- v0.6.0 交付后:13 件完成 6 件(4 外设包含蜂鸣器扩已在 v0.5.0 覆盖的不重计)
- v0.7.0 交付后:13 件完成 11 件;ESP32/Pico 以"无上游模拟器支持"为由书面豁免

## 三、技术方案(批次 A 细则;批次 B 启动前再补细则)

### 1. 光敏电阻 `photoresistor`

- `src/components/photoresistor.rs`:构造参数 `lux_max`(可选);渲染同电位器
  (进度条 + %),读 `adc_values`,无值回退 `☀ ?`
- TUI:Tab 聚焦候选从 `kind == "potentiometer"` 扩成"实现了 `knob_channel()`
  的元件" — `ComponentDef` 加可选方法 `fn knob(&self) -> bool { false }`
- example:`ldr-nightlight`(光低于阈值点亮 D13)

### 2. RGB LED `rgb_led`

- 三端子 `r/g/b`,wire 到三个 pwm 引脚;渲染:各通道 duty → RGB 混色,
  ratatui `Color::Rgb(duty_r, duty_g, duty_b)` 色块 + 三通道百分比
- 接非 pwm 引脚的通道按数字电平 0/255 处理
- example:`rgb-rainbow`(HSV 轮转)

### 3. 舵机 `servo`

- 复用 `PwmSample`:`pulse_us = duty as u64 * period_us / 255`,
  500-2500us 线性映射 0-180°,渲染角度指针(`↺ 90°`)
- 注意:Arduino Servo 库用 Timer1,D9/D10;50Hz 信号频率低,
  `PWM_DISPLAY_MIN_FREQ_HZ = 20` 的门槛已兼容(50 > 20),LED 侧不受影响
- example:`servo-sweep`

### 4. 直流电机 `dc_motor`

- 参数:`ena`(PWM 调速端子)+ `in1/in2`(方向,数字电平)
- 渲染:`⚙ ▶ 75%` / `⚙ ◀ 30%` / `⚙ ■ 0%`
- example 与 servo 合并或单开(examples 上限见第六节)

### 5. HC-SR04 `ultrasonic`(批次 A 唯一 bridge 改动)

- stdin 命令扩:`dist <cm>`(0-400,存 bridge 全局)
- bridge 挂 trigger 引脚 IRQ:收到 ≥10us 高脉冲 → 按
  `echo_us = cm * 58` 用 simavr cycle timer 调度 echo 引脚拉高/拉低
- 引脚从 moxin.toml wire 推导不可行(bridge 不读 project),
  约定俗成:trigger/echo 引脚由 stdin 命令声明 `sr04 <trig_pin> <echo_pin>`
  (bridge 启动后由 Rust 侧根据 wires 自动下发)
- capabilities 加 `"sr04"`;协议文档三处同步规则照旧
- example:`ultrasonic-radar`(串口打印距离,TUI 滑块调 dist)

### 6. Arduino Nano 板

- `boards/arduino_nano.rs`:复制 UNO spec 改 `board_id/display_name`,
  引脚表加 A6/A7(Nano 独有,ADC-only,无数字功能 → `PinSpec` 需一个
  `analog_only` 标记或单列),bridge/mcu/freq 全同 UNO
- `arduino-cli` FQBN 参数化:`arduino:avr:uno` → `arduino:avr:nano`
  (`BoardImpl::build` 里已按板分派,加常量即可)

### Component schema 决策(延续 2-full 的方案 A 评估)

RGB LED 三端子、电机 ena/in1/in2 都走 **wire 端子名**(`rgb1.r`),
不需要新 Component 字段;servo/ldr 无新字段。批次 A **继续 fat-struct,
不升 SCHEMA_VERSION**。批次 B 的 DHT11/LCD 需要注入型"环境量",届时评估
`params: BTreeMap` + SCHEMA 0.3(迁移提示照 CLAUDE.md 规矩写)。

## 四、测试与质量线

- 每个新元件:构造/渲染(有信号、无信号、接错引脚)≥3 单测
- Nano:spec 单测(引脚表、A6/A7、FQBN)
- HC-SR04:bridge 桩头语法校验 + CI verify 加 `assert --serial-contains`
  距离 example 关卡(模式照 v0.5.0 的 serial 关卡)
- 目标:v0.6.0 收尾 `cargo test` ≥180 / clippy 0 警告 / CI verify 全绿

## 五、回滚策略

- 每个外设独立 commit,单件翻车单件 revert
- bridge 改动(仅 HC-SR04)独立 commit,revert 后其余 5 件不受影响
- v0.5.0 tag 是整体回退锚点

## 六、需要用户拍板的问题(启动前必须回答)

1. **分批方案认可?** 批次 A(v0.6.0)= LDR/RGB/舵机/电机/HC-SR04/Nano,
   批次 B(v0.7.0)= DHT11/LCD/OLED/F103/红外 —— 或者你想调整优先级?
2. **examples 上限**:当前 12/12 已满。批次 A 至少 +4 个 example。
   提议:上限调到 **18**,同时把 `assert-*` 两个验证用例移入 `tests/fixtures/`
   不占额度 —— 认可哪种?
3. **CLAUDE.md 禁区更新**:Arduino Nano 从"不加"名单移除(同 simavr,
   零 bridge 成本);ESP32/Pico 维持禁区 —— 确认?
4. **HC-SR04 的 bridge 改动**(stdin 命令 +2、trigger IRQ hook)按本 RFC
   预授权,还是动手前再确认一次?

## 七、实施步骤(批次 A,可勾选)

- [x] Step 0:交互提问通道故障,按用户连续"继续/按照你的要求来"的委托采用推荐项开工
      (1 分批=RFC 方案;2 examples 上限→18;3 Nano 移出禁区;4 bridge **保守项:动手前再确认**)。
      用户可随时推翻,推翻即 revert 对应 commit
- [ ] Step 1:photoresistor + ldr-nightlight example
- [ ] Step 2:rgb_led + rgb-rainbow example
- [ ] Step 3:servo + servo-sweep example
- [ ] Step 4:dc_motor(example 视上限决定)
- [ ] Step 5:HC-SR04(bridge sr04/dist 命令 + echo 调度)+ example + CI 关卡
- [ ] Step 6:Arduino Nano 板 + spec 单测
- [ ] Step 7:文档收尾(README / CLAUDE.md / bridge-protocol.md)+ v0.6.0 release

## 八、决策记录

| 日期 | 决策 | 理由 |
|---|---|---|
| 2026-07-07 | 13 件按"模拟量/总线"分两批两个版本 | 难度断层:批次 A 用现成 ADC/PWM 通道,批次 B 要新写 TWI/单总线状态机;分批可独立交付、独立回滚 |
| 2026-07-07 | ESP32 / Pico 维持禁区 | 主线 QEMU 无 ESP32(xtensa fork 不可靠),RP2040 机型不成熟;换 Nano + F103 凑板数,工程上可达 |
| 2026-07-07 | HC-SR04 引脚由 Rust 侧经 stdin 下发而非 bridge 读配置 | bridge 保持"不读 moxin.toml"的边界;复用 protocol 1 命令通道 |

后续决策追加在此表底部。
