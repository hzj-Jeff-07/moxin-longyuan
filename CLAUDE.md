# CLAUDE.md — MoXin CLI

让 AI 工具直接读懂 MCU 仿真状态的命令行模拟器。仿真后端 = 外部 `simavr` / `qemu-system-arm`,本仓库是 Rust 编排层。**不写 CPU 内核。**

---

## 阶段状态(2026-07 更新)

- **Phase 1**:✅ 完成(双板跑通 / `run --output json` / `status --pin` / doctor 三平台 / examples ≥4)
- **Phase 2-mini(v0.4.0)**:✅ 完成(全 PORTB/C/D GPIO 追踪、七段真段驱动、`moxin assert`)
- **Phase 2-full(目标 v0.5.0)**:🚧 进行中,权威计划见 `docs/design/phase-2-full-rfc.md`
  - Step 1 ComponentDef 注册式重构 ✅(已合并 main)
  - Step 3 PWM 追踪 ✅(纯 Rust 侧 PwmTracker,含 pwm-fade example,2026-07-07)
  - Step 2 ADC 真仿真 ✅(bridge stdin 命令通道 + IRQ 注入,经用户确认,2026-07-07;顺带修复 AVR serial 事件缺失)
  - Step 4 examples ✅(adc-potentiometer + pwm-fade)/ Step 5 文档收尾 ✅(README + 版本号 0.5.0,2026-07-07)
  - PR #3 已合并 main(2026-07-07);CI verify 真机关卡全绿(bridge 编译 + blink/serial e2e)
  - 剩:tag `v0.5.0` + push tag(本环境推不了 tag,**需用户本地执行**)
- **Phase 3 批次 A(v0.6.0)**:✅ 代码完成(2026-07-07),权威计划见 `docs/design/phase-3-rfc.md`
  - 5 外设(photoresistor / rgb_led / servo / dc_motor / ultrasonic)+ Arduino Nano 全部落地
  - HC-SR04 bridge 改动经用户确认;真机 e2e 靠 CI verify 的 ultrasonic 新关卡
  - 剩:tag `v0.6.0`(需用户授权;v0.5.0 tag 也还没打)
- **Phase 3 批次 B(v0.7.0)**:🚧 进行中(RFC 三·B 节细则已补,2026-07-08)
  - B7 DHT11 ✅(bridge 边沿回放器 + dht/env 命令;CI dht11 关卡)
  - 剩:红外(复用边沿回放器)/ LCD1602 / OLED(需 TWI)/ STM32F103(机型待用户拍板,QEMU 无真 F103)
- `docs/planning/`(7day 计划)已与实际历史脱节,仅作参考,进度以 git log + RFC 勾选为准

✅ 当前范围:
- Arduino Uno / Arduino Nano (simavr,同 bridge) + STM32F405 (qemu netduinoplus2)
- 元件 14 种(led / rgb_led / button / resistor / buzzer / potentiometer / photoresistor / servo / dc_motor / ultrasonic / dht11 / seven_segment / breadboard / dupont),经 `src/components/` 注册式接入
- 新增元件 = `src/components/<name>.rs` 实现 `ComponentDef` + `Registry::builtin()` 注册一行,**不改 render/shell/inspector 主路径**

❌ 不做(明确禁区):
- 不写 AVR / ARM / RISC-V CPU 内核
- 不加 ESP32 / RP2040(主线 QEMU/simavr 无现成机型;Nano 已于 Phase 3 解禁,同 ATmega328P 复用 simavr bridge)
- 不加 I2C / SPI / OLED / LCD1602 / DHT11(留 Phase 3 / v0.6.0)
- 不做 GUI / Tauri / Web 前端
- 不实现 MCP server(留 v3)
- 不改 `bridge/*.c`,除非用户明确同意(phase-2-full RFC 已默认同意 ADC/stdin 通道改动,动手前仍再确认一次)
- 不改 `SCHEMA_VERSION`(当前 `"0.2"`;Phase 3 撑不住再升 0.3,见 RFC 决策记录)
- 不改 `LICENSE`(BUSL-1.1)

---

## 技术栈(已固定)

```toml
edition = "2021"
clap = "4"           # derive
ratatui = "0.30"
serde = "1"          # derive
serde_json = "1"
toml = "0.8"
anyhow = "1"         # 不要换成 color-eyre / thiserror
rustyline = "14"
tempfile = "3"       # dev-dep
```

外部运行时依赖:`simavr` / `qemu-system-arm` / `arduino-cli` / `arm-none-eabi-gcc`。

---

## 常用命令

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- doctor               # 检查外部依赖
cargo run -- new myproj           # 默认 uno;或 --board stm32
cargo run -- shell                # 进 TUI
cargo run -- shell --no-tui       # REPL 模式
```

---

## 项目布局(锁定)

```
src/
  main.rs           clap 入口,只做命令分发
  shell.rs          REPL + dispatch
  project.rs        moxin.toml(SCHEMA_VERSION=0.2)
  sim.rs            bridge 子进程 + JSON 事件流 + RunState
  tui.rs render.rs  ratatui 渲染(渲染分支走 components registry,不再 match kind)
  inspector.rs      AI Inspector 面板
  board.rs          板层公共逻辑
  cmd_*.rs          doctor / install / new / status / assert 子命令
  components/
    mod.rs          ComponentDef trait + Registry::builtin()
    <name>.rs       每个元件一个文件(led / button / resistor / ...)
  boards/
    mod.rs          BoardImpl trait + board_from_str
    spec.rs         BoardSpec / PinSpec
    arduino_uno.rs  simavr bridge 调用
    stm32f405.rs    qemu bridge 调用
    gd32vf103.rs    占位:build/spawn_sim 必须 bail "not yet implemented"
bridge/             C 源码,Claude 不主动改
components/         元件 schema TOML(与 src/components/ 对齐)
examples/           ≤18 个(Phase 3 起上调,当前 17),新增需含 README + moxin.toml
docs/design/        设计文档(bridge-protocol、cli-vision、phase-2-full-rfc 是权威)
```

新增板子:`boards/<name>.rs` + `mod.rs` 注册 + spec 单测,不要往 `mod.rs` 堆代码。
新增元件:`src/components/<name>.rs` 实现 `ComponentDef` + `Registry::builtin()` 注册 + 单测,不要回退到 match 硬编码。

---

## Bridge 协议(对齐 `sim.rs::BridgeEvent`)

bridge 子进程从 stdout 每行输出一条 JSON:

```json
{"event":"ready","mcu":"atmega328p","freq":16000000}
{"event":"pin","t_us":1234,"port":"B","bit":5,"value":1}
{"event":"serial","t_us":1235,"line":"hello"}
{"event":"button","t_us":1240,"pressed":true}
{"event":"exit","state":0}
```

加新事件类型 = 同步改 `BridgeEvent` enum + bridge C 源码 + `docs/design/bridge-protocol.md`,三处一起改否则丢事件。

---

## moxin.toml schema 0.2(不要破坏)

```toml
[project]
name = "blink"
board = "arduino-uno"   # arduino-uno | arduino-nano | stm32 | gd32vf103
version = "0.2"

[code]
src = "src/main.ino"

[[component]]
id = "led1"
type = "led"
color = "red"

[[wire]]
from = "D13"
to = "led1.anode"
```

破坏性改 schema = 必须升 `SCHEMA_VERSION` + 在 `Project::load` 写迁移提示(参考现有错误信息)。

---

## 错误处理约定

```rust
use anyhow::{Context, Result, bail};

pub fn load(path: &Path) -> Result<Self> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    if !ok { bail!("invalid schema: {}", reason); }
    Ok(parsed)
}
```

不混 `?` + `.unwrap()`;库代码不 `panic!`;CLI 顶层异常由 `main()` 的 `Result<()>` 自动打印。

---

## 跨平台规则

- Windows 没有 `$HOME`,必须 `USERPROFILE` fallback;不要直接读 `HOME` env
- 路径只用 `Path::join`,不硬编码 `/`
- 不直接调 `brew`;`cmd_install` / `cmd_doctor` 写 `#[cfg]` 分支:

```rust
#[cfg(target_os = "windows")]
fn install_hint() -> &'static str { "no native Windows package — use WSL (apt install simavr) or build via MSYS2" }
#[cfg(target_os = "macos")]
fn install_hint() -> &'static str { "brew install simavr" }
#[cfg(target_os = "linux")]
fn install_hint() -> &'static str { "apt install simavr" }
```

注意:scoop 主 bucket 没有 simavr 包,Windows 提示不要写 `scoop install simavr`(2026-07 已修正,勿回退)。

bridge 二进制查找:先 `$MOXIN_BRIDGE` env,再 exe 同目录 `moxin-simavr-bridge[.exe]`。

---

## 测试约定

- 单测与代码同文件 `mod tests`(已是惯例)
- 涉及外部进程的测试加探针,缺依赖直接 `return`(`which` crate 不在依赖列表里,用 `Command` 探针):

```rust
#[test]
fn run_blink_e2e() {
    if std::process::Command::new("simavr").arg("--help").output().is_err() { return; }
    // 真跑 examples/blink,不要 mock BridgeEvent
}
```

- `cargo test` 在无 simavr/qemu 的环境必须全过
- 测试里不要 `std::env::set_var` / `remove_var`(并行测试线程下与 `getenv` 竞态);
  需要注入 env 的函数拆 `*_impl(env_override: Option<String>)`,参考 `sim.rs::find_bridge_avr_impl`

---

## Claude 禁止动作

- 不 `git push` / `git push -f` — 远程由用户控制
- 不 `cargo publish`
- 不 `cargo install <tool>` 全局工具(询问用户执行)
- 不 `brew install` / `apt install` / `scoop install` — 写指引,不执行
- 不动 `bridge/*.c` 不询问
- 不改 `LICENSE` 或 `Cargo.toml::license-file`
- 不写 `unsafe` 块不带 `// SAFETY:` 注释
- 不 `cargo update` 跨大版本(0.30→0.31 OK,0.30→1.0 先问)
- 不假设 simavr/qemu 已装,先 `Command` 探针(见测试约定)
- 不动 `SCHEMA_VERSION` 不写迁移提示

---

## DOD

### Phase 1(✅ 全部达成,2026-05 验收)

- [x] `cargo test` 全过(无外部依赖部分)
- [x] `cargo clippy -- -D warnings` 无 warning
- [x] `moxin doctor` 在 Windows/macOS/Linux 三平台输出可执行提示
- [x] `moxin run --output json` 在 stdout 至少输出 ready / pin / serial 三类事件
- [x] `moxin status --pin D13` 返回 `HIGH` / `LOW` / `UNKNOWN`
- [x] examples ≥4 个,每个 README 30 秒可跑通
- [x] 三平台 `cargo build --release` 出 binary(release workflow 4 平台产物 + Linux simavr 真机断言关卡)

### Phase 2-full / v0.5.0(进行中,细则见 RFC 六节)

- [x] ADC 真仿真:bridge stdin 命令通道 + simavr IRQ 注入 + `BridgeEvent::Hello/Adc`(2026-07-07)
- [x] PWM 追踪:`PwmTracker` 边沿推导 duty/freq,buzzer/led 渲染升级(2026-07-07)
- [x] examples + 2:`adc-potentiometer` ✅、`pwm-fade` ✅
- [x] `cargo test` ≥130(当前 146)/ clippy 0 警告 / bridge-protocol.md 同步(protocol "1" + hello/adc/serial)
- [x] Step 5 收尾:README 更新 + 版本号 0.4.0 → 0.5.0(2026-07-07);tag `v0.5.0` 待用户授权
- [ ] ⚠️ bridge 改动只过了桩头文件语法校验,真机 simavr 编译 + e2e 待 CI verify 关卡 / 本地有 simavr 的机器验证

---

## 当前已知坑(改之前先看)

- `gd32vf103.rs::build / spawn_sim` 留 `bail "not yet implemented"` — 不要随意补全,RISC-V bridge 未写
- ADC 注入仅 Uno(bridge protocol "1");STM32 `adc_channels` 为空,注入会被 `set_adc` 拒绝
- ADC 值来自注入(`adc` 命令 / TUI 旋钮),不是电路仿真;没注入过时 potentiometer 回退静态 `max_ohms` 显示
- PWM 是 Rust 侧边沿推导(非 bridge 真 hook):duty 到 0/255 时无边沿,采样过期回退 ON/OFF,属预期;STM32 `pwm_pins` 为空,PWM 显示仅 Uno 生效
- 老 bridge 二进制(protocol 前)不发 hello/serial/adc:`set_adc` 会报错提示重编 bridge;Uno serial 需要新 bridge 才有
- Windows 上 simavr 无现成包(WSL / MSYS2 自编译),`moxin doctor` 提示已如实说明;考虑在 release 附预编译 bridge
- `.moxin-state.json` 只在 `run --output json` 模式落盘;TUI/REPL 模式下 `moxin status` 读到的是上一次 json run 的快照

每月复审本文件,删过时条目。(上次复审:2026-07,删除了 4 条已修复的跨平台坑)
