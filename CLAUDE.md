# CLAUDE.md — MoXin CLI

让 AI 工具直接读懂 MCU 仿真状态的命令行模拟器。仿真后端 = 外部 `simavr` / `qemu-system-arm`,本仓库是 Rust 编排层。**不写 CPU 内核。**

---

## Phase 1 范围(锁定)

✅ 做:
- Arduino Uno (simavr) + STM32F405 (qemu netduinoplus2) — 已可跑
- 元件:LED / Button(已实现);新元件先与 `bridge/` 对齐再加
- 新增 `moxin run --output json`,透传 bridge 事件流到 stdout
- 新增 `moxin status --pin <name>`,快照查询
- 新增 examples:`button-counter`、`serial-echo`(达到 ≥4 个)
- Windows / Linux 跑通(当前 brew-only)

❌ 不做(明确禁区):
- 不写 AVR / ARM / RISC-V CPU 内核
- 不加 ESP32 / RP2040 / Arduino Nano(没现成 bridge)
- 不加 I2C / SPI / OLED / LCD1602 / DHT11(需 bridge + 建模配合)
- 不做 GUI / Tauri / Web 前端
- 不实现 MCP server(留 v3)
- 不改 `bridge/*.c`,除非用户明确同意
- 不改 `SCHEMA_VERSION`(当前 `"0.2"`)
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
  sim.rs            bridge 子进程 + JSON 事件流
  tui.rs render.rs  ratatui 渲染
  inspector.rs      AI Inspector 面板
  boards/
    mod.rs          BoardImpl trait + board_from_str
    spec.rs         BoardSpec / PinSpec
    arduino_uno.rs  simavr bridge 调用
    stm32f405.rs    qemu bridge 调用
    gd32vf103.rs    占位:build/spawn_sim 必须 bail "not yet implemented"
bridge/             C 源码,Claude 不主动改
examples/           ≤10 个,新增需含 README + moxin.toml
docs/design/        设计文档(bridge-protocol 与 cli-vision 是权威)
```

新增板子:`boards/<name>.rs` + `mod.rs` 注册 + spec 单测,不要往 `mod.rs` 堆代码。

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
board = "arduino-uno"   # arduino-uno | stm32 | gd32vf103
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
fn install_hint() -> &'static str { "scoop install simavr  # 或下载 release" }
#[cfg(target_os = "macos")]
fn install_hint() -> &'static str { "brew install simavr" }
#[cfg(target_os = "linux")]
fn install_hint() -> &'static str { "apt install simavr" }
```

bridge 二进制查找:先 `$MOXIN_BRIDGE` env,再 exe 同目录 `moxin-simavr-bridge[.exe]`。

---

## 测试约定

- 单测与代码同文件 `mod tests`(已是惯例)
- 涉及外部进程的测试加探针,缺依赖直接 `return`:

```rust
#[test]
fn run_blink_e2e() {
    if which::which("simavr").is_err() { return; }
    // 真跑 examples/blink,不要 mock BridgeEvent
}
```

- `cargo test` 在无 simavr/qemu 的环境必须全过

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
- 不假设 simavr/qemu 已装,先 `which::which()` 探针
- 不动 `SCHEMA_VERSION` 不写迁移提示

---

## DOD(Phase 1 完工标准)

- [ ] `cargo test` 全过(无外部依赖部分)
- [ ] `cargo clippy -- -D warnings` 无 warning
- [ ] `moxin doctor` 在 Windows/macOS/Linux 三平台输出可执行提示
- [ ] `moxin run --output json` 在 stdout 至少输出 ready / pin / serial 三类事件
- [ ] `moxin status --pin D13` 返回 `HIGH` / `LOW` / `UNKNOWN`
- [ ] examples ≥4 个(blink-uno、blink-stm32、button-counter、serial-echo),每个 README 30 秒可跑通
- [ ] 三平台 `cargo build --release` 出 binary

---

## 当前已知坑(改之前先看)

- `gd32vf103.rs::build / spawn_sim` 留 `bail "not yet implemented"` — 不要随意补全,RISC-V bridge 未写
- `sim.rs::bridge_log_path` / `arduino_uno.rs::dirs_home` 用 `HOME` env,Windows 上为空 → 需加 `USERPROFILE` fallback
- `find_bridge_avr` 同目录查找未加 `.exe` 后缀,Windows 找不到
- `cmd_install` 当前 macOS-only,其它平台占位

每月复审本文件,删过时条目。
