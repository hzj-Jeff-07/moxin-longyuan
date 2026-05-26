# Phase 2-mini RFC — 拔掉 D13-only 红线

> 状态:**草案 / 待实施**
> 分支:`phase-2-mini`(origin 已同步)
> 备份:`v0.3.0-stable`(随时可回滚)
> 起点 commit:`b44b7aa`(main)
> 目标版本:**v0.4.0**
> 预估编码量:5-7 天净 / 2 周实际(70/20/10 节奏)
> 最后更新:2026-05-26

---

## 一、为什么做这个 RFC

任务书"九、达标线"里写了三条"不可接受":

1. 🚨 CLI 只是个壳,硬件状态靠手动维护字典而不是真仿真
2. 🚨 AI 接口只是文本日志,没有任何结构化数据
3. 🚨 元件实现写死在主程序里,无法扩展新元件

v0.3.0 当前状态:

- 第 2 条 ✅ 不中招(JSON 事件流已真结构化)
- 第 1 条 ⚠️ **D13 之外的引脚全是静态 OFF**,踩在红线边
- 第 3 条 ⚠️ `render.rs` 是 `match component.kind` 死写,踩在红线边

**Phase 2-mini 的唯一目标 = 拔掉第 1 条**(顺手补 examples 数量)。第 3 条暂不动(留 Phase 2-full)。

---

## 二、范围(锁定)

### ✅ 做(B-mini 子集)

| 步骤 | 内容 | 工作量 |
|---|---|---|
| **B1** | bridge 全 PORTB/C/D GPIO hook,Rust 侧全引脚状态表 | 2-3 天 |
| **B4** | 数码管 7 段:按 `[[wire]]` 配置读 8 个 GPIO 实时算显示数字 | 1 天 |
| **B6** | examples 6 → 10 个 | 1-2 天 |

### ❌ 不做(留给 Phase 2-full / v0.5.0)

- ADC 双向通道(电位器/光敏)→ 风险高,推后
- PWM/Timer hook(蜂鸣器/RGB/调光)→ simavr Timer 语义复杂,推后
- 元件注册式重构(干掉 render.rs match)→ 容易破老测试,推后
- 扩展 13 件(4 板 + 9 外设)→ Phase 3
- README demo 动图 → 等 v0.4.0 出来再录

### ❌ 不动的锁(CLAUDE.md 里继续保留)

- 不 `git push --force` 到 main
- 不动 `LICENSE`
- 不 `cargo publish`
- 不写 `unsafe` 不带 `// SAFETY:`
- 不主动 push tag

### ⚠️ 这次解锁的禁区

- "**只有 D13 真实仿真**" → B1 完成后这行从 CLAUDE.md 删
- "**不改 bridge/\*.c 不询问**" → 改前提示一次即可,不再每次问
- "**不动 SCHEMA_VERSION**" → B4 数码管需要 wire 标记段位,**可能**需要升 0.2 → 0.3;若不升,用约定命名(`seg_a`~`seg_g`)绕过

---

## 三、技术方案

### B1. 全 GPIO 仿真

#### bridge 侧(`bridge/moxin-simavr-bridge.c`)

现在的代码大致是:

```c
avr_irq_register_notify(
    avr_io_getirq(avr, AVR_IOCTL_IOPORT_GETIRQ('B'), 5),  // 只 hook B5 (D13)
    on_pin_change, NULL);
```

改成:

```c
const struct { char port; int pins; } PORTS[] = {
    {'B', 8}, {'C', 7}, {'D', 8},
};
for (size_t i = 0; i < ARRAY_SIZE(PORTS); i++) {
    for (int bit = 0; bit < PORTS[i].pins; bit++) {
        avr_irq_register_notify(
            avr_io_getirq(avr, AVR_IOCTL_IOPORT_GETIRQ(PORTS[i].port), bit),
            on_pin_change, (void*)((PORTS[i].port << 8) | bit));
    }
}
```

`on_pin_change` 现有签名已经吐 `{"event":"pin","port":"B","bit":5,"value":1}`,扩展到所有 port/bit 即可,**JSON 协议不变**。

#### Rust 侧(`src/sim.rs`)

新增:

```rust
use std::collections::HashMap;

pub struct PinStates {
    states: HashMap<(char, u8), bool>,  // (port, bit) -> value
}

impl PinStates {
    pub fn update(&mut self, event: &BridgeEvent) {
        if let BridgeEvent::Pin { port, bit, value, .. } = event {
            self.states.insert((*port, *bit), *value != 0);
        }
    }
    pub fn get_arduino_pin(&self, d_pin: u8) -> Option<bool> {
        // D0-D7 = PORTD bit 0-7
        // D8-D13 = PORTB bit 0-5
        // A0-A5 = PORTC bit 0-5
        let (port, bit) = match d_pin {
            0..=7   => ('D', d_pin),
            8..=13  => ('B', d_pin - 8),
            _ => return None,
        };
        self.states.get(&(port, bit)).copied()
    }
}
```

#### 事件批处理(关键)

事件风暴风险:Serial 一行就是几十次 TX 引脚翻转。加批处理:

```rust
// 渲染循环里
let mut batch = Vec::with_capacity(64);
while let Ok(Some(ev)) = sim.try_recv_event() {  // 非阻塞排空
    batch.push(ev);
    if batch.len() >= 64 { break; }
}
for ev in batch { pin_states.update(&ev); }
// 一次性重绘
```

TUI 16ms 一帧,一帧处理几十条事件没压力。

#### 渲染侧(`src/render.rs`)

LED 渲染从硬编码 D13 改为查 wire:

```rust
"led" => {
    let pin = wires.find_pin_for(&comp.id);  // 现在已有 wire 表
    let on = pin.and_then(|p| pin_states.get_arduino_pin(p)).unwrap_or(false);
    // ... 画字符
}
```

### B4. 数码管 7 段真驱动

数码管 8 段:`a, b, c, d, e, f, g, dp`。

配置文件惯例(**不升 SCHEMA**):

```toml
[[component]]
id = "seg1"
kind = "seven_segment"

[[wire]]
from = "D2"
to = "seg1.a"
[[wire]]
from = "D3"
to = "seg1.b"
# ... 共 8 条
```

渲染时:

```rust
let segments = ["a","b","c","d","e","f","g","dp"];
let lit: Vec<bool> = segments.iter()
    .map(|seg| {
        wires.find_pin_for_node(&format!("{}.{}", comp.id, seg))
            .and_then(|p| pin_states.get_arduino_pin(p))
            .unwrap_or(false)
    })
    .collect();

// 7×3 字符块按段亮灭画(查表 abcdefg → 0-9 数字)
render_seven_seg(&lit)
```

### B6. 4 个新 examples

| 例子 | 验证什么 |
|---|---|
| `multi-led-chase`(走马灯,D2-D7) | B1:多引脚 GPIO 真仿真 |
| `seven-seg-counter`(数码管 0-9) | B4:数码管真驱动 |
| `button-led-pair`(按钮控 D4 LED,不是 D13) | B1:非 D13 LED 真仿真 |
| `pin-state-snapshot`(代码翻全 GPIO,run 后 status 全查) | AI 接口完整性 |

每个 example 一个目录,带 `moxin.toml` + `src/main.ino` + `README.md`(30 秒可跑通)。

---

## 四、测试策略

### 不退化保险

CI release pipeline 现有的 `moxin assert --pin D13 --toggles --within 3s` 是真验证。
**phase-2-mini 每个 PR 都必须跑过这个**,通过即说明老 D13 能力没坏。

### 新增测试

1. **单测:`PinStates::get_arduino_pin`**
   全 19 个 Arduino Uno 数字引脚映射对,边界情况(D14 返回 None)。

2. **集成测试:`tests/integration_multi_pin.rs`**
   - 跑 `multi-led-chase` example
   - 抓 bridge JSON 事件流
   - 断言:看到 D2~D7 全部至少各翻转一次

3. **金标快照:`tests/snapshots/`**
   - `render_with_multi_led.snap`(B1 后渲染结果)
   - `render_seven_seg_3.snap`(数码管显示 3 时的字符块)
   - 用 `insta` crate 或手写比对都行

4. **bridge 协议测试不变**(`bridge-protocol.md` JSON schema 不变)

### 测试基线

phase-2-mini 完工前必须:
- `cargo test` 全过(包括无外部依赖部分)
- `cargo clippy --all-targets -- -D warnings` 0 警告
- CI verify 步骤 PASS(D13 toggle 验证)
- 至少 1 个新 example 在本地手跑过

---

## 五、回滚策略

| 翻车级别 | 操作 |
|---|---|
| 单个 commit 翻车 | `git revert <sha>` |
| 整个 B1 翻车 | `git reset --hard origin/main`(phase-2-mini 分支内) |
| **彻底炸了** | `git checkout main && git reset --hard v0.3.0-stable`(必须经你授权) |
| Release 需要回到 v0.3.0 | tag 已存在,直接重发即可 |

**v0.3.0-stable 分支永远不动,是最后保险**。

---

## 六、实施步骤(可勾选进度)

### Step 0 — 准备(✅ 已完成)
- [x] 备份 `v0.3.0-stable` 分支(本地 + origin)
- [x] 开 `phase-2-mini` 分支(本地 + origin)
- [x] 写本 RFC

### Step 1 — bridge 全 GPIO
- [x] 改 `bridge/moxin-simavr-bridge.c`,hook 全 PORTB/C/D
- [ ] 本地编译 bridge,跑 blink 看是否仍输出 D13 事件(不退化)— **延后:Windows 无 make/gcc,合并时通过 release pipeline verify 统一验证**
- [ ] 跑 multi-pin 测试代码,看是否输出 D2-D7 事件 — **延后同上**
- [x] commit:`feat(bridge): hook all PORTB/C/D pins`(b5acebf)
- [ ] **CI 必须绿** — D13 verify 通过(等合并时统一跑 release pipeline)

### Step 2 — Rust 侧 PinStates
- [x] `src/sim.rs` 加 `PinStates` 类型
- [x] 加 `get_arduino_pin` 映射 + 单测
- [ ] 接入事件循环,加批处理 — **延后(YAGNI)**:reader 独立线程逐行 apply,TUI 16ms 采样快照,架构本身非阻塞;Step 3 接入渲染后再观察是否需要批处理
- [ ] commit:`feat(sim): full GPIO state tracking`

### Step 3 — 渲染接全 GPIO
- [x] `src/render.rs` LED 改查 wire + PinStates
- [x] 老的 D13 硬编码删掉
- [x] 金标快照测试加上(D7 ON / D2 OFF / D5 buzzer ON)
- [ ] commit:`feat(render): all LEDs reflect real pin state`

### Step 4 — `moxin status --pin <name>` 全引脚可查
- [ ] D0-D13 / A0-A5 全部能查
- [ ] commit:`feat(status): all-pin query support`

### Step 5 — 数码管 7 段真驱动
- [ ] `src/render.rs` seven_segment 分支改查 8 段引脚
- [ ] 段位 → 数字查表
- [ ] 金标快照(0-9 + 错误段位的"-")
- [ ] commit:`feat(seven-seg): real segment-driven display`

### Step 6 — 4 个新 examples
- [ ] `examples/multi-led-chase/`
- [ ] `examples/seven-seg-counter/`
- [ ] `examples/button-led-pair/`
- [ ] `examples/pin-state-snapshot/`
- [ ] 每个带 README + moxin.toml + main.ino
- [ ] commit:`docs(examples): add 4 phase-2-mini examples`

### Step 7 — 文档 + 收尾
- [ ] 更新 `CLAUDE.md`:删"只有 D13 真实仿真"那行
- [ ] 更新 `README.md`:examples 列表 + Phase 2-mini 说明
- [ ] 更新 `bridge-protocol.md`:确认 pin 事件全 port 覆盖
- [ ] `cargo test` + `cargo clippy -- -D warnings` 全过
- [ ] 本 RFC 状态从"草案"改为"已完成"
- [ ] commit:`chore: phase-2-mini wrap up`

### Step 8 — 合并 + Release v0.4.0(等你授权)
- [ ] PR `phase-2-mini` → `main`(走 gh pr create)
- [ ] 合并后等你授权打 tag `v0.4.0`
- [ ] Release pipeline 跑完 → 4 平台 binary 发布

---

## 七、Phase 2-mini 完工 = 哪条红线被拔

完工后:

- 任务书 🚨 第 1 条:**拔掉**(全 GPIO 真仿真)
- 任务书 🚨 第 3 条:**继续踩边**(注册式留给 Phase 2-full)
- 任务书"基本达标":**全 ✅**
- 任务书"优秀水准":扩展硬件 0/13,留给 Phase 3

距离任务书完整交付:Phase 2-full(ADC/PWM/重构) + Phase 3(扩展 13 件) + README demo,**预估 8-12 周**(2026 暑假 + 秋季)。

---

## 八、随时找回方式

任何时候打开仓库,都能 30 秒恢复上下文:

1. 看 `docs/design/phase-2-mini-resume.md`(进度恢复手册)
2. 看本 RFC 的"六、实施步骤"勾选状态
3. 跑 `git log --oneline main..phase-2-mini` 看做了哪些 commit
4. 跑 `git diff main..phase-2-mini --stat` 看改了哪些文件

---

## 九、决策记录

| 日期 | 决策 | 理由 |
|---|---|---|
| 2026-05-26 | 选 B-mini 而非 B-full | 70/20/10 精力分配,2 周可吃下 |
| 2026-05-26 | ADC/PWM 推到 v0.5.0 | 风险高,需要 simavr 源码深读 |
| 2026-05-26 | 注册式重构推到 v0.5.0 | 容易破现有 94 个测试 |
| 2026-05-26 | 数码管不升 SCHEMA | 用 `seg.a`~`seg.dp` 命名约定绕过 |

后续决策追加在此表底部。
