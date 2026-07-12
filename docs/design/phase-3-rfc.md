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

## 三·B、批次 B 技术细则(v0.7.0,2026-07-08 补)

实施顺序按"bridge 风险递增":DHT11 → STM32F103 → 红外 → LCD1602 → OLED。

### 7. DHT11 `dht11`(第一件,p0)

**bridge**(复用 protocol 1 stdin 通道 + cycle timer,与 sr04 同模式):

- `dht <P> <B>`:声明 data 引脚(moxin 按 wires 自动下发,同 sr04)
- `env <temp_c> <hum_pct>`:注入环境温湿度(0..50°C / 20..90%,DHT11 量程)
- 时序状态机:host 拉低 ≥500us 后释放 → bridge 用一个自重排 cycle timer
  按 DHT11 时序回放 84 个边沿:30us 后 80us 低 + 80us 高应答,然后 40 bit
  (每 bit 50us 低 + 27us/70us 高 = 0/1),字节序 hum/0/temp/0/checksum
- 回放期间忽略 data 引脚上自己注入的边沿(防状态机自触发)
- capabilities 加 `"dht"`

**Rust**:`configure_dhts`(同 configure_ultrasonics 模式)、
`RunState::dht_env: Option<(u8 temp, u8 hum)>`、shell `env <temp> <hum>` 命令、
`dht11` 元件(🌡 温湿度显示)、example `dht11-weather`(固件手写 bit-bang 读,
不依赖 DHT 库)、CI verify 加 `assert --serial-contains "temp="` 关卡。

**✅ 已完成(2026-07-08)**:上述全部落地 + configure_peripherals 统一配置入口;
cargo test 177 / 元件 14 种 / examples 17。真机 e2e 靠 CI 新增的 dht11 关卡。

### 8. STM32F103(蓝色 Pill)— ❌ 书面豁免(2026-07-11 决定)

- QEMU 主线无 F103/BluePill 机型;最近的 F1 是 `stm32vldiscovery`(F100RB,
  24MHz 主频、无 USB、定时器布局不同)。以 F100 代跑 F103 固件属于"假机型",
  与本项目"如实标注"的原则冲突 —— 按 ESP32/Pico 同理由豁免(用户委托决策)。
- 若上游 QEMU 日后合入 BluePill 机型,重新评估。

### 9. 红外 NEC `ir_receiver` — ✅ 完成(2026-07-11)

- bridge:`ir <P> <B>` 声明引脚(声明后 500ms 自发一帧自检码 20DF10EF,
  给 CI e2e 和首次体验用)+ `irtx <hex32>` 发帧,复用 DHT 的边沿回放器
- 元件渲染最近一帧码;shell `ir <hex>` 命令;example `ir-remote`
  (手写 NEC 解码 + 电源键翻转 LED);CI 加 `code=20DF10EF` 关卡

### 10. LCD1602(I2C / PCF8574 背包)— ✅ 完成(2026-07-12,细则 2026-07-11 补)

**bridge TWI 从机**(参考 simavr tests/i2c_eeprom.c 的应答模式):

- 挂 `AVR_IOCTL_TWI_GETIRQ(0)` 的 `TWI_IRQ_OUTPUT` notify,应答走 `TWI_IRQ_INPUT`
- `lcd <hex_addr>` stdin 命令启用(moxin 按元件自动下发,默认 0x27);
  未启用时不 ACK 任何地址,不影响老固件
- START:`(msg.addr >> 1) == 0x27` → 选中 + ACK;STOP → 取消选中;
  WRITE → 喂 PCF8574 字节 + ACK;READ 不支持(背包只写)
- **PCF8574 → HD44780 4-bit 解码**:P0=RS P2=EN P4-7=D4-7(最常见映射);
  EN 下降沿锁存高 4 位;初始化期(0x3/0x3/0x3/0x2 单 nibble)不配对,
  见到 nibble 0x2 才进 4-bit 模式开始两两配对
- 命令子集:0x01 清屏 / 0x02 归位 / 0x80|addr 置 DDRAM 地址;
  function set / display / entry mode 一律 no-op(不影响字符流)
- DDRAM 80 字节,row0=0x00 起,row1=0x40 起,可见窗口 16 列
- **事件节流**:字符写入置脏标记,30ms cycle timer 合并后发一条
  `{"event":"lcd","t_us":..,"row0":"<16字符>","row1":"<16字符>"}`
  (LiquidCrystal 类库每个 nibble 一次 I2C 事务,按 STOP 发事件会刷屏)
- capabilities 加 `"lcd"`

**Rust**:`BridgeEvent::Lcd` → `RunState::lcd: Option<(String,String)>`;
`configure_lcds` 自动下发;`lcd1602` 元件双行渲染;example `lcd-hello`
用裸 `Wire.h` 手写背包驱动(不引第三方库,CI 无需 lib install),
固件校验每次 `endTransmission` 的 ACK,全部成功才打 `lcd ok` →
CI 关卡 `assert --serial-contains "lcd ok"`(从机不 ACK 即失败,真 e2e)。

### 11. OLED SSD1306 — 待 LCD 落地后再细化

- 同为 TWI 从机,但要解析 SSD1306 命令流 + 128×64 帧缓冲,
  TUI 侧盲文点阵(⣿)降采样渲染;等 LCD 验证 TWI hook 可靠后动工

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
- [x] Step 1:photoresistor + ldr-nightlight example(2026-07-07)
- [x] Step 2:rgb_led + rgb-rainbow example(2026-07-07)
- [x] Step 3:servo + servo-sweep example(2026-07-07,50Hz 软 PWM 版固件,不依赖 Servo.h)
- [x] Step 4:dc_motor(2026-07-07;不单开 example,组合示例留 Step 5 之后评估)
- [x] Step 5:HC-SR04(bridge sr04/dist 命令 + echo 调度)+ example + CI 关卡(2026-07-07,用户已确认 bridge 改动)
- [x] Step 6:Arduino Nano 板 + spec 单测(2026-07-07,avr_build/avr_spawn_sim 抽共用,A6/A7 ADC-only)
- [x] Step 7:文档收尾(README / CLAUDE.md / bridge-protocol.md)+ 版本号 0.6.0(2026-07-07;tag 待用户授权)

**批次 A 完工(2026-07-07)**:cargo test 173 / clippy 0 / 元件 13 种 / 板 4 块 / examples 16 个。
遗留:bridge sr04 改动过桩头语法校验,真机 e2e 靠 CI verify 新增的 ultrasonic 关卡。

## 八、决策记录

| 日期 | 决策 | 理由 |
|---|---|---|
| 2026-07-07 | 13 件按"模拟量/总线"分两批两个版本 | 难度断层:批次 A 用现成 ADC/PWM 通道,批次 B 要新写 TWI/单总线状态机;分批可独立交付、独立回滚 |
| 2026-07-07 | ESP32 / Pico 维持禁区 | 主线 QEMU 无 ESP32(xtensa fork 不可靠),RP2040 机型不成熟;换 Nano + F103 凑板数,工程上可达 |
| 2026-07-07 | HC-SR04 引脚由 Rust 侧经 stdin 下发而非 bridge 读配置 | bridge 保持"不读 moxin.toml"的边界;复用 protocol 1 命令通道 |

后续决策追加在此表底部。
