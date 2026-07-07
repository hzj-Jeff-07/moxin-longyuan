# Phase 2-full RFC — 拔掉元件硬编码红线 + ADC/PWM 真仿真

> 状态:**草案 / 待用户批准启动**
> 分支:`phase-2-full`(待开)
> 备份:`v0.4.0-stable`(待建)
> 起点 commit:`58aeb5d`(main)
> 目标版本:**v0.5.0**
> 预估工作量:2-3 周(2026-05-28 → 06-15 弹性)
> 最后更新:2026-05-27

---

## 一、为什么做这个 RFC

任务书"九、达标线"三条 🚨 中:

1. ~~CLI 只是个壳,硬件状态靠手动维护字典~~ → ✅ Phase 2-mini 已拔
2. ~~AI 接口只是文本日志,没有结构化数据~~ → ✅ Phase 1 已拔
3. 🚨 **元件实现写死在主程序里,无法扩展新元件** → 仍踩

v0.4.0 当前状态:

- `src/render.rs` 有**两处并行**的 `match comp.kind.as_str()` 死写(plain 8 分支 + styled 8 分支),增改任一元件须双改且不能漏。
- `src/project.rs::Component` 已堆 4 个类型专用 Optional 字段(`color` / `ohms` / `max_ohms` / `wire_color`),再加 13 个外设 = 字段爆炸。
- `src/inspector.rs::summarize_components` 又一处 `if c.kind == "resistor"` 硬编码。
- `src/shell.rs::cmd_add` 第三处构造分支 match。

这些是 Phase 3(扩展 13 件)的硬阻塞 —— 必须先把"元件抽象层"立起来,后续每加一个外设都是注册一个 `ComponentDef` 实现而不是改主路径。

**同时**:Phase 2-mini RFC 二节"❌ 不做"里写明 ADC / PWM 是 v0.5.0 范围。13 件外设里至少 5 件依赖这两项基础设施(servo / photoresistor / RGB LED / ultrasonic / DHT11),先打通通道再批量上外设。

---

## 二、范围(锁定)

### ✅ 做(三件大事)

| 步骤 | 内容 | 工作量 |
|---|---|---|
| **Step 1** | 组件注册式重构(`ComponentDef` trait + registry,替换 render/inspector/shell 三处 match) | 1 周 |
| **Step 2** | ADC 真仿真(simavr ADC IRQ inject + 协议 + 旋钮 TUI 交互) | 4-5 天 |
| **Step 3** | PWM 真仿真(simavr Timer 边沿 → duty/freq + 协议) | 4-5 天 |
| **Step 4** | 2 个新 examples(`adc-potentiometer` + `pwm-fade`) | 1-2 天 |
| **Step 5** | 文档收尾 + release v0.5.0 | 半天 |

### ❌ 不做(留给 Phase 3 / v0.6.0)

- 13 件外设扩展(舵机 / RGB LED / DHT11 / 超声波 / 光敏 / 蜂鸣器扩 / 4 个新板 等) → Phase 3 RFC
- I2C / SPI 总线建模 → Phase 3 部分外设依赖,与外设一起做
- LCD1602 / OLED → Phase 3
- README demo 动图 → 等 v0.6.0 出来再录

### ❌ 不动的锁(CLAUDE.md 全保留)

- 不 `git push --force` 到 main
- 不动 `LICENSE`(BUSL-1.1)
- 不 `cargo publish`
- 不写 `unsafe` 不带 `// SAFETY:`
- **不主动 push tag**(v0.5.0 tag 必须逐次授权)
- 不引入新 crate 依赖(`inventory` / `linkme` / `regex` 都不要;用 `HashMap<&'static str, Arc<dyn ComponentDef>>` + `register_builtins()` 函数初始化)
- bridge 大改前先汇报(本 RFC 默认同意,推进时再确认一次)
- `SCHEMA_VERSION` **可能升 0.2 → 0.3**(见 Step 1 决策点),需写迁移路径

### ⚠️ 这次解锁的禁区

- "**不改 bridge/\*.c 不询问**" → Step 2/3 必须改(加 stdin 命令通道 + ADC IRQ + Timer hook),RFC 默认同意
- "**没有协议版本字段**" → Step 2 起加 `{"event":"hello","protocol":"1"}`,老 moxin 跑新 bridge 安全降级
- bridge stdin 当前空读 → Step 2 起读取行命令(`adc <ch> <value>` 等)

---

## 三、技术方案

### Step 1. 组件注册式重构

#### 目标

收敛三处硬编码 match 到单一调度入口,新增元件 = 实现 `ComponentDef` + 在 `register_builtins()` 加一行。

#### `ComponentDef` trait(新文件 `src/components/mod.rs`)

```rust
use crate::project::{Component, Project};
use crate::sim::RunState;
use crate::boards::spec::BoardSpec;
use ratatui::text::Line;
use anyhow::Result;

pub trait ComponentDef: Send + Sync {
    fn kind(&self) -> &'static str;
    fn aliases(&self) -> &'static [&'static str] { &[] }

    /// 从 shell `add` 命令构造 Component(参数解析)
    fn build(&self, id: String, args: &[String]) -> Result<Component>;

    /// 渲染:plain ASCII(用于 --no-tui / 文本快照测试)
    fn render_plain(&self, comp: &Component, project: &Project,
                    state: &RunState, spec: &BoardSpec) -> String;

    /// 渲染:ratatui 富样式(用于 TUI)
    fn render_styled(&self, comp: &Component, project: &Project,
                     state: &RunState, spec: &BoardSpec) -> Line<'static>;

    /// AI Inspector 摘要行(可选,无返回则使用默认 "<id>: <kind>")
    fn summarize(&self, _comp: &Component) -> Option<String> { None }
}

pub struct Registry {
    by_kind: std::collections::HashMap<&'static str, std::sync::Arc<dyn ComponentDef>>,
    by_alias: std::collections::HashMap<&'static str, &'static str>,
}

impl Registry {
    pub fn builtin() -> Self {
        let mut r = Self::default();
        r.register(Arc::new(led::Led));
        r.register(Arc::new(button::Button));
        r.register(Arc::new(resistor::Resistor));
        r.register(Arc::new(buzzer::Buzzer));
        r.register(Arc::new(potentiometer::Potentiometer));
        r.register(Arc::new(seven_segment::SevenSegment));
        r.register(Arc::new(breadboard::Breadboard));
        r.register(Arc::new(dupont::Dupont));
        r
    }
    pub fn resolve(&self, kind_or_alias: &str) -> Option<&dyn ComponentDef> { ... }
}
```

#### 文件拆分

```
src/components/
  mod.rs           trait + Registry + register_builtins
  led.rs           Led 实现(从 render.rs 搬过来)
  button.rs
  resistor.rs
  buzzer.rs
  potentiometer.rs
  seven_segment.rs (含原 segments_to_glyph / seven_seg_segments 等 helper)
  breadboard.rs
  dupont.rs
```

#### `Component` schema 决策

**保留方案 A(向后兼容,不升 SCHEMA)**:`Component` struct 不动,所有现有字段(color / ohms / max_ohms / wire_color)继续存在,新增字段同样走 Optional;`ComponentDef` 实现自行读自己关心的字段。

**升级方案 B(SCHEMA 0.2 → 0.3)**:把字段改成 `params: BTreeMap<String, toml::Value>`,所有 9 个现存 examples 走迁移。

**RFC 决策**:**走 A**。理由:
- 保 9 个现有 examples / 测试快照不破。
- B 的"扩展性"在 13 件外设之前是过度设计,Phase 3 启动时若真撑不住再升 0.3。
- CLAUDE.md "不动 SCHEMA_VERSION 不写迁移提示" 守住。

#### 改动点清单(Step 1 提交前必须三处都迁完)

1. `src/render.rs`:`render_runtime_frame` 和 `wire_row_line` 两个 match → `registry.resolve(comp.kind).render_plain/styled(...)`。原 helper(`format_led` / `resistance_color_rings` / `seven_seg_segments` 等)迁到对应模块,作为关联函数或私有辅助。
2. `src/inspector.rs::summarize_components`:resistor 特判改为 `registry.resolve(c.kind).summarize(c)`。
3. `src/shell.rs::cmd_add` 的 match → `registry.resolve(kind).build(id, &positional)?`。

#### 不退化保险(Step 1 必跑)

- 现有 119 个测试 0 破。注意 `render.rs:670-913` 的 6 个文本快照测试断言具体子串(`"BUZZ ON"` / `"[3] 7SEG s1"` / `"red ON #"`),迁移后必须输出完全一致。
- 新增测试:`Registry::resolve("led")` / `resolve("btn") == resolve("button")`(alias)/ `resolve("unknown") == None`。
- `cargo clippy --all-targets -- -D warnings` 0 警告。

### Step 2. ADC 真仿真

#### bridge 侧(`bridge/moxin-simavr-bridge.c`)

加两件事:

1. **stdin 命令循环(独立线程)**:
   ```c
   void *stdin_cmd_loop(void *_) {
       char line[256];
       while (fgets(line, sizeof line, stdin)) {
           int ch, value;
           if (sscanf(line, "adc %d %d", &ch, &value) == 2) {
               // simavr ADC IRQ inject
               avr_raise_irq(
                   avr_io_getirq(g_avr, AVR_IOCTL_ADC_GETIRQ, ADC_IRQ_ADC0 + ch),
                   value); // 0..1023(10-bit)
           }
       }
       return NULL;
   }
   pthread_create(&tid, NULL, stdin_cmd_loop, NULL);
   ```
2. **协议 hello 事件(版本字段)**:
   ```c
   printf("{\"event\":\"hello\",\"protocol\":\"1\",\"capabilities\":[\"adc\",\"pwm\"]}\n");
   fflush(stdout);
   ```
   出 `ready` 之前先出 `hello`。Rust 侧老 moxin 不识别就 silently ignore,新 moxin 按 capabilities 决定能不能用 ADC。

#### Rust 侧

```rust
// src/sim.rs
pub enum BridgeEvent {
    // ... 现有 5 种
    #[serde(rename = "hello")]
    Hello { protocol: String, capabilities: Vec<String> },
    #[serde(rename = "adc")]
    Adc { t_us: u64, channel: u8, value: u16 },
}

pub struct RunState {
    // ... 现有字段
    pub adc_values: HashMap<u8, u16>,        // 板 channel -> 0..1023
    pub bridge_capabilities: Vec<String>,    // 来自 hello
}

impl RunningSim {
    pub fn set_adc(&self, channel: u8, value: u16) -> Result<()> {
        let line = format!("adc {} {}\n", channel, value.min(1023));
        self.stdin.lock().write_all(line.as_bytes())?;
        Ok(())
    }
}
```

`apply_event` 加 `Adc` 分支写 `adc_values`,加 `Hello` 分支写 `bridge_capabilities`。

#### `BoardSpec` 扩展

```rust
pub struct BoardSpec {
    // ... 现有字段
    pub adc_channels: &'static [(u8 /* arduino A pin, e.g. 0 = A0 */, u8 /* mcu ch */)],
}
```

Arduino UNO:`&[(0,0), (1,1), (2,2), (3,3), (4,4), (5,5)]`(A0..A5 = ADC0..ADC5)。STM32 暂不实现(Phase 3 再补)。

#### `potentiometer` 渲染升级

`src/components/potentiometer.rs` 的 `render_styled` 从板 wire 找出 A 引脚 → 查 `state.adc_values[ch]` → 算百分比/弧度 → 画进度条。无值时回退到现有的 `max_ohms` 静态显示。

#### TUI 旋钮交互

`src/tui.rs` 加 component focus(Tab 切换聚焦),聚焦电位器时:
- `←` → `set_adc(ch, value - 32)`(每步约 3%)
- `→` → `set_adc(ch, value + 32)`
- `Home/End` → 0 / 1023

非阻塞,与现有 char→stdin 路径互不冲突(只有聚焦电位器时拦截方向键)。

#### 测试

- 单测:`RunState::apply_event` 处理 Adc 事件后 `adc_values[ch] == value`。
- 单测:`set_adc(0, 2000)` 截断到 1023。
- 单测:`Registry::resolve("potentiometer").render_plain` 在 `adc_values` 有值时输出含 "%" 字符。
- 集成:`tests/integration_adc.rs`(简化版,无 simavr 时 return)。

### Step 3. PWM 真仿真

#### bridge 侧

simavr 的 GPIO IRQ 已经会在每次 OCR 翻转时触发 `pin_change_cb`。**最简方案**:在 Rust 侧基于 `pin` 事件的边沿时间差算 duty / freq,bridge 不动。

加 helper(`src/sim.rs`):
```rust
struct PwmTracker {
    last_high: Option<u64>,
    last_low: Option<u64>,
    last_period_start: Option<u64>,
}
impl PwmTracker {
    fn observe(&mut self, value: u8, t_us: u64) -> Option<PwmSample> {
        // 检测稳定的方波:连续 N 个周期 freq 偏差 <5% 即视为 PWM
        ...
    }
}
pub struct RunState {
    // ...
    pub pwm: HashMap<String /* "B:1" */, PwmSample>,
}
pub struct PwmSample { pub duty: u8, pub freq_hz: u32, pub stable: bool }
```

**优势**:bridge 不改 C,纯 Rust 侧推导,降低风险。
**劣势**:PWM 数据不实时(需要至少一个完整周期才能算出),也无法区分"GPIO 翻转"和"真 PWM"。但对 buzzer / LED 调光场景够用。

如果上述方案在测试中暴露问题,Plan B 是 bridge 真 hook timer compare ioctl(`AVR_IOCTL_TIMER_GETIRQ`),延后 Step 3.5 实施。

#### `BoardSpec` 扩展

```rust
pub struct BoardSpec {
    // ...
    pub pwm_pins: &'static [u8],   // Arduino UNO: &[3,5,6,9,10,11]
}
```

#### `buzzer` / `led` 渲染升级

- buzzer 显示 "♪ 1000Hz" / "♪ MUTE";
- led 在 `pwm.stable && duty > 0` 时显示 "● 50%"(占空比),无 PWM 时回到现 `ON/OFF`。

#### 测试

- 单测:`PwmTracker::observe` 喂入 1kHz 50% 序列(1000 t_us 一个周期,500 us high / 500 us low)→ 输出 `duty=128, freq_hz≈1000, stable=true`。
- 单测:翻转 1 次后 `stable=false`(数据不足)。
- 单测:不规则间隔 → 不识别为 PWM。
- 渲染单测覆盖 buzzer/led 在有/无 PwmSample 两种状态。

### Step 4. 2 个新 examples

| 例子 | 验证什么 |
|---|---|
| `adc-potentiometer`(读 A0,Serial 打印,旋钮调) | Step 2:ADC + 注册式 + TUI 交互 |
| `pwm-fade`(D9 LED 0→255→0 呼吸) | Step 3:PWM duty 跟踪 + LED 调光显示 |

每个一个目录:`moxin.toml` + `src/main.ino` + `README.md`(30 秒可跑通)。

### Step 5. 文档收尾

- `CLAUDE.md`:删"`render.rs` 是 `match component.kind` 死写"相关表述(若有);确认"不引入新 crate"仍生效。
- `README.md`:版本 0.4.0 → 0.5.0 + examples 列表 +2 + Phase 2-full 已完成 + 已知限制更新(13 件外设留 v0.6.0)。
- `docs/design/bridge-protocol.md`:加 `hello` / `adc` 事件 + stdin 命令格式(`adc <ch> <value>`)+ 协议版本 = "1"。
- 本 RFC 状态从"草案"改为"已完成"。

---

## 四、测试策略

### 不退化保险

- Phase 2-mini 的 119 个测试 0 破,数字只能涨。
- CI release pipeline 现有 D13 toggle assert 必须仍 pass(注册式重构后老路径不能断)。

### 新增测试估算

| Step | 新增测试数 |
|---|---|
| Step 1 注册式 | +6(Registry resolve / alias / 8 个 builtin 注册) |
| Step 2 ADC | +5(apply_event Adc / set_adc 截断 / 渲染百分比 / hello 解析 / capabilities 缺失降级) |
| Step 3 PWM | +6(PwmTracker stable / 不稳 / 渲染 buzzer 频率 / led 调光 / pwm_pins 校验) |

**目标**:v0.5.0 收尾时 `cargo test` ≥ 130 通过 / 0 clippy warnings / `cargo build --release` ok。

### 测试基线(每个 commit 前必跑)

- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- 至少 1 个新 example 在本地手跑(无 simavr 环境用 mock 路径,Linux/macOS 跑过实测一遍)

---

## 五、回滚策略

| 翻车级别 | 操作 |
|---|---|
| 单个 commit 翻车 | `git revert <sha>` |
| 整个 Step 翻车 | `git reset --hard origin/main`(phase-2-full 分支内,未 push 之前) |
| 注册式重构破老快照 | revert Step 1 commit,改回老 match 路径,但保留 components/ 目录文件作为后续参考 |
| **彻底炸了** | `git checkout main && git reset --hard v0.4.0-stable`(必须经用户授权) |
| Release 需要回到 v0.4.0 | `v0.4.0` tag 已存在,直接重发 |

**v0.3.0-stable + v0.4.0-stable + v0.4.0 tag 是最后保险**。

---

## 六、实施步骤(可勾选进度)

### Step 0 — 准备
- [ ] 备份 `v0.4.0-stable` 分支(本地 + origin,**push 需授权**)
- [ ] 开 `phase-2-full` 分支(本地;origin push 需授权)
- [x] 写本 RFC

### Step 1 — 组件注册式重构
- [ ] 新建 `src/components/` 目录 + `mod.rs` + 8 个元件文件
- [ ] 实现 `ComponentDef` trait + `Registry`
- [ ] 把 `render.rs` 两个 match 收敛(plain + styled 都走 registry)
- [ ] `inspector.rs::summarize_components` 改用 registry
- [ ] `shell.rs::cmd_add` 改用 registry
- [ ] 全部 119 测试 0 破 + 注册式新增 6 测试
- [ ] commit:`refactor(components): introduce ComponentDef registry`
- [ ] 单 commit 范围控制(尽量),便于翻车 revert

### Step 2 — ADC 真仿真(✅ 2026-07-07)
- [x] bridge:stdin 命令通道(非阻塞轮询,非 pthread,见决策记录)+ simavr ADC IRQ inject + hello 事件
- [x] bridge 附带:UART0 → serial 事件(修复 Uno 串口输出从未进过事件流的老 bug)
- [x] `BridgeEvent::Hello / Adc` + `RunState::adc_values / bridge_capabilities` + `apply_event` 分支
- [x] `BoardSpec::adc_channels` + Arduino UNO 配置(A0..A5 = ADC0..ADC5)
- [x] `RunningSim::set_adc(channel, value)`(bridge 无 adc 能力时明确报错)
- [x] `potentiometer` 渲染改查 `adc_values`(进度条 + % + 原始值,无值回退静态阻值)
- [x] TUI 加 Tab 聚焦电位器 + ←/→/Home/End 调旋钮;另加 shell REPL `adc <A0..A5|ch> <value>`
- [x] 11 个新单测 + 1 个 example(`adc-potentiometer`)— cargo test 146 过
- [x] commit:`feat(adc): real ADC injection via simavr IRQ`

### Step 3 — PWM 真仿真(✅ 2026-07-07,先于 Step 2 完成 — 纯 Rust 侧,不动 bridge)
- [x] `PwmTracker` 实现 + 单测(波形识别)
- [x] `RunState::pwm` + 事件循环接入(含样本过期判定 `get_pwm`,3 周期无边沿即过期)
- [x] `BoardSpec::pwm_pins` + Arduino UNO 配置(&[3,5,6,9,10,11])
- [x] `buzzer` / `led` 渲染升级(buzzer 显 "♪ 1000Hz",led 显占空比 %;<20Hz 慢速 blink 不误判)
- [x] 6+ 个新单测 + 1 个 example(`pwm-fade`)— cargo test 135 过
- [x] commit:`feat(pwm): edge-based PWM duty/freq tracking`

### Step 4 — examples(✅ 2026-07-07)
- [x] `examples/adc-potentiometer/`
- [x] `examples/pwm-fade/`
- [x] 各带 README + moxin.toml + main.ino
- [x] (examples 随 Step 2/3 各自的 feat commit 提交,未单开 docs commit)

### Step 5 — 文档 + 收尾
- [ ] CLAUDE.md / README.md / bridge-protocol.md 全更
- [ ] RFC 状态草案 → 已完成
- [ ] `cargo test` + `cargo clippy -- -D warnings` 全过(目标 ≥130 / 0)
- [ ] commit:`chore: phase-2-full wrap up`

### Step 6 — 合并 + Release v0.5.0
- [ ] PR `phase-2-full` → `main`(**用户授权**)
- [ ] Cargo.toml + Cargo.lock 0.4.0 → 0.5.0
- [ ] tag `v0.5.0`(**用户授权**)+ push tag(**用户授权**)
- [ ] Release pipeline 全绿(4 平台二进制)

---

## 七、Phase 2-full 完工 = 哪条红线被拔

完工后:

- 任务书 🚨 第 3 条:**拔掉**(注册式 = 加新元件不改主路径)
- 任务书三条 🚨:**全 ✅**
- 任务书"基本达标":**全 ✅**(已自 v0.4.0 起)
- 任务书"优秀水准":扩展硬件 0/13(留 v0.6.0)+ ADC/PWM ✅

距离任务书完整交付:
- v0.6.0 / Phase 3:13 件外设(预估 3-4 周,2026 暑假)
- README demo 录屏 / 动图(v0.6.0 出来再录)

---

## 八、随时找回方式

任何时候打开仓库,30 秒恢复上下文:

1. 看 `docs/design/phase-2-full-resume.md`(Step 0 一并写入)
2. 看本 RFC "六、实施步骤"勾选状态
3. 跑 `git log --oneline main..phase-2-full` 看做了哪些 commit
4. 跑 `git diff main..phase-2-full --stat` 看改了哪些文件

---

## 九、决策记录

| 日期 | 决策 | 理由 |
|---|---|---|
| 2026-05-27 | 把 v0.5.0(Phase 2-full)与 v0.6.0(Phase 3)拆两版本 | 注册式是 Phase 3 的硬阻塞;一锅炖 PR 巨大且翻车成本高;分版本可独立 revert |
| 2026-05-27 | 不升 `SCHEMA_VERSION`(保 0.2) | 走 Component fat-struct + Optional 字段;现有 9 个 examples / 6 个文本快照测试不破;Phase 3 真撑不住再升 |
| 2026-05-27 | 不引入 `inventory` / `linkme` 等注册宏 | CLAUDE.md 已锁依赖列表;`HashMap<&'static str, Arc<dyn ComponentDef>>` + `register_builtins()` 函数已够用 |
| 2026-05-27 | PWM 在 Rust 侧基于边沿时间推导(优先) | bridge C 改动最小,降低风险;不行再切 Plan B 真 hook simavr Timer |
| 2026-05-27 | bridge 协议加 `hello` + version `"1"` | 避免老 moxin 跑新 bridge 时静默丢事件;为 Phase 3 留升级口 |
| 2026-05-27 | ADC 走 stdin 命令(`adc <ch> <value>`) | 复用现有 stdin 管道(simavr 之前空读,改成读行);不引入额外 IPC 机制 |
| 2026-07-07 | Step 3 先于 Step 2 落地 | PWM 方案纯 Rust 侧、零 bridge 改动,可立即做;ADC 需改 bridge/*.c,按 CLAUDE.md 约定等用户再确认一次 |
| 2026-07-07 | PWM 采样加 `t_us` + `get_pwm` 过期判定(3 周期无边沿即过期) | 呼吸灯扫到 0/255 时波形停止,旧样本不能一直挂着;渲染回退 ON/OFF |
| 2026-07-07 | LED 调光显示限 `pwm_pins` + ≥20Hz | 防止 D13 慢速 blink(1Hz 方波也"稳定")被误显示成占空比;buzzer 不限引脚(tone() 任意脚) |
| 2026-07-07 | bridge stdin 用主循环非阻塞轮询,不用 RFC 草图的 pthread | simavr 不是线程安全的,跨线程 `avr_raise_irq` 与 `avr_run` 竞态;轮询在 2000 条指令的 chunk 间隙做,延迟可忽略 |
| 2026-07-07 | hello capabilities = ["adc","serial"] 而非草图的 ["adc","pwm"] | capabilities 描述 bridge 自身能力;PWM 是 Rust 侧推导,bridge 并不提供 |
| 2026-07-07 | 顺手修 AVR 串口:UART0 IRQ → serial 事件 + 关 simavr stdout dump | 排查发现 Uno 的 Serial.println 从未进过事件流(raw 文本被 Rust 侧当非 JSON 丢弃),serial-echo / assert-serial-hello 两个例子在 Uno 上一直是坏的;正好在同一文件同一授权范围内 |
| 2026-07-07 | `set_adc` 对无 adc 能力的 bridge 直接报错 | 命令写给老 bridge 只会被静默忽略,用户看不出为什么没反应;报错并提示重编 bridge |
| 2026-07-07 | 加 shell REPL `adc` 命令(RFC 原文只有 TUI 旋钮) | AI Agent / CI 走 REPL 或 JSON 模式,不开 TUI;没有命令入口 ADC 通道对主要用户(AI)不可达 |

后续决策追加在此表底部。
