# MoXin AI 上下文文档

> 任何 AI 在动 moxin-longyuan 代码前，先读完本文件。
> 本文件是项目地图，不是教程。读完应该能定位到 5 分钟内开工。

---

## 一、项目一句话

MoXin 是个 **CLI 嵌入式仿真器**：用户在 `.moxin.toml` 里声明用什么板、接什么元件、怎么连线，然后 `moxin run` 编译 firmware（Arduino sketch / C 代码）、起仿真后端（simavr 或 QEMU）、ratatui 实时显示 LED 亮灭 / 串口输出。**核心卖点是 AI 友好**：所有运行时事件以 JSON Lines 输出，AI 工具能直接消费。

## 二、技术栈

| 层 | 用什么 |
|---|---|
| 主语言 | Rust 1.75+，edition 2021 |
| CLI parsing | `clap = 4`（derive macros） |
| 错误处理 | `anyhow`（lib 内部用 Result，对外保留具体错误说明） |
| 序列化 | `serde` + `serde_json` + `toml = 0.8` |
| TUI | `ratatui = 0.30` + `rustyline = 14` |
| Arduino 后端 | `simavr` C 库 + 自写 `bridge/moxin-simavr-bridge.c`（通过 stdout 发 JSON） |
| STM32 后端 | `qemu-system-arm` + 自写 `bridge/stm32/bridge-stm32.c`（解析 firmware semihosting 输出） |
| 测试 | `cargo test`，每个文件底部 `#[cfg(test)] mod tests` |
| Dev deps | `tempfile`（集成测试用临时目录） |

**重要**：bridge 是 C 写的，独立编译出来的二进制，moxin 主程序通过 `Command::new` spawn 它，stdin/stdout 通信，**不是** linked library。

## 三、目录结构

```
moxin-longyuan/
├── Cargo.toml
├── README.md
├── docs/
│   ├── component-schema.md       # 元件契约（已合并，D1-1）
│   ├── sprint-plan.md            # 10 天方案（D1-1 同步合并）
│   ├── AI-CONTEXT.md             # 本文件
│   ├── CONVENTIONS.md            # 编码约定
│   └── design/
│       ├── bridge-protocol.md    # bridge JSON 协议规范
│       └── cli-vision.md         # CLI 愿景
├── src/
│   ├── main.rs           # 入口 + clap 子命令路由（76 行）
│   ├── board.rs          # PinRef 类型 + 解析（221 行）
│   ├── project.rs        # moxin.toml 反序列化（272 行）
│   ├── sim.rs            # RunningSim + BridgeEvent + reader_loop（297 行）
│   ├── shell.rs          # 交互 shell：cmd_add / wire / run / status / ...（366 行）
│   ├── render.rs         # 静态项目渲染（395 行）
│   ├── tui.rs            # ratatui 四面板（480 行）
│   ├── inspector.rs      # TUI 右下角状态面板（128 行）
│   ├── cmd_new.rs        # `moxin new` 子命令（25 行）
│   └── boards/
│       ├── mod.rs        # BoardImpl trait + board_from_str（205 行）
│       ├── spec.rs       # BoardSpec / PinSpec 结构（90 行）
│       ├── arduino_uno.rs    # simavr 后端（199 行）
│       ├── stm32f405.rs      # QEMU 后端（255 行）
│       └── gd32vf103.rs      # 占位，build/spawn_sim 未实现（91 行）
├── bridge/
│   ├── moxin-simavr-bridge.c   # AVR 仿真桥（用户 make）
│   └── stm32/bridge-stm32.c    # STM32 桥（moxin 自动 cc 编译到 ~/.cache/moxin/bridge-stm32）
├── examples/
│   ├── led-control/      # Arduino blink（.ino）
│   └── stm32-blink/      # STM32 blink（.c + Makefile）
├── components/           # 元件 schema（D1-1 合并）
├── pin-anchors-template/ # 给建模部的待填表（D1-1 合并）
├── pin-anchors/          # 建模部填好后放这（D1-1 创建空目录）
└── scripts/
    └── build-stm32.sh    # 手动 build 脚本，cmd_new 也会调
```

**绝对的代码总量**：~3300 行 Rust + ~220 行 C。10 天工期会增加约 800-1200 行 Rust。

## 四、核心抽象（每个 AI 必懂）

### 4.1 Project（在 src/project.rs）

`Project` 反序列化自 `moxin.toml`，是一切的入口数据：

```rust
pub struct Project {
    pub project: ProjectMeta,   // { name, board, version }
    pub components: Vec<Component>,
    pub wires: Vec<Wire>,
}

pub struct Component {
    pub id: String,             // 用户起的名字，如 "led1"
    pub r#type: String,         // 元件类型，如 "led" / "buzzer"
    pub color: Option<String>,  // 元件特定参数都是 Option<...>，缺省即用默认值
    pub ohms: Option<u32>,
    // ... 这里有点冗余，每加一种元件就要加字段
}

pub struct Wire {
    pub from: String,           // "board.D13" 或 "led1.anode"
    pub to: String,
}
```

**已知技术债**：`Component` 用 Option 字段堆参数不优雅。Phase 2 改 `parameters: HashMap<String, toml::Value>`，10 天版不动它，加新元件按现有套路。

### 4.2 BoardImpl trait（在 src/boards/mod.rs）

每个开发板（Arduino Uno / STM32 / GD32）实现这个 trait：

```rust
pub trait BoardImpl {
    fn spec(&self) -> &'static BoardSpec;
    fn scaffold_project(&self, name: &str) -> Project;  // 给 cmd_new 生成默认 toml
    fn source_template(&self) -> &'static str;          // 默认源码（blink）
    fn build(&self, root: &Path) -> Result<(PathBuf, String)>;       // 编译 firmware
    fn spawn_sim(&self, root: &Path, artifact: &Path) -> Result<RunningSim>; // 启 bridge
}
```

**重要**：要加新板，新建 `src/boards/<name>.rs`，写一个空结构体 impl 它，再在 mod.rs 的 `board_from_str` 加一个 match arm。

### 4.3 BridgeEvent（在 src/sim.rs）

bridge 进程通过 stdout 发 JSON Lines，每行一条事件，Rust 端反序列化为：

```rust
#[serde(tag = "event")]
enum BridgeEvent {
    Ready { mcu: String, freq: u32 },
    Pin { t_us: u64, port: String, bit: u8, value: u8 },
    Serial { t_us: u64, line: String },
    Exit { state: i32 },
    Button { _t_us: u64, pressed: bool },  // ⚠️ 这里有 bug，D1-2 修
}
```

`reader_loop` 每读一行 JSON 就调 `apply_event` 更新 `RunState`（在 Arc<Mutex<...>> 里）。TUI 60 FPS 渲染同一份 `RunState`。

**重要约束**：bridge 协议是**双向**——`apply_event` 是 firmware → moxin；要给 firmware 注入输入（如电位器调值、按钮按下），是 moxin → bridge stdin，bridge 解析命令然后调 simavr / QEMU API 注入。Phase 1 协议入向命令很少（只有 button），D5 加入电位器调值时**要扩展协议**。

### 4.4 RunningSim（在 src/sim.rs）

正在跑的仿真实例：

```rust
pub struct RunningSim {
    pub child: Option<Child>,           // bridge 子进程
    pub stdin: Option<ChildStdin>,
    pub state: Arc<Mutex<RunState>>,
    pub reader: Option<JoinHandle<()>>, // stdout reader 线程
    pub stderr_reader: Option<JoinHandle<()>>,
    // ⚠️ stop() 当前 detach 一个线程做善后，D2-3 修
}
```

### 4.5 PinRef（在 src/board.rs）

引脚引用的抽象语法树：

```rust
pub enum PinRef {
    BoardName(String),   // board.D13 / board.PA13 / board.GND / board.5V
    BoardDigital(u8),    // pin13（Arduino 风格的短写）
    Component { id: String, pin: String },  // led1.anode / led1.a
}
```

`PinRef::parse` 输入字符串解析成 enum。各 board 的 `spec.pin_ref_valid` 校验某个引脚名在该板上是否合法。

## 五、build / run 时实际发生了什么

举 Arduino Uno 例子（STM32 类似但用 QEMU）：

```
$ moxin run    （在某 .moxin.toml 项目里）
  │
  ├─ ArduinoUno::build(root) → 调 `arduino-cli compile` → 产出 .hex
  │     · 临时编 firmware 二进制
  │
  └─ ArduinoUno::spawn_sim(root, hex)
        │
        ├─ find_bridge_avr() → 找 moxin-simavr-bridge 路径
        │     · 默认在 moxin binary 同目录或 $MOXIN_BRIDGE
        │
        ├─ Command::new(bridge).arg(hex).spawn() → bridge 子进程起来
        │     · bridge 加载 simavr，运行 firmware
        │     · firmware 跑 digitalWrite(13, HIGH) 时，simavr 触发 pin_change_cb
        │     · bridge 回调里 printf 一行 JSON：{"event":"pin","port":"B","bit":5,"value":1,"t_us":12345}
        │
        ├─ 起 reader_loop 线程：循环读 bridge stdout → 反序列化 BridgeEvent → 更新 RunState
        │
        └─ 返回 RunningSim
              · TUI 拿到 state Arc<Mutex>，60 FPS 渲染
              · 用户按 's' 停止 → 给 bridge 发 SIGTERM
```

## 六、当前测试约定

- **单元测试**：在每个 .rs 文件底部 `#[cfg(test)] mod tests { ... }`
- **集成测试**：目前**没有**，10 天版不强制加，但 D7 写 assert 命令时**必须**加几个
- **跑测试**：`cargo test`（全部）/ `cargo test --lib sim` （只跑 sim 模块）
- **lint**：`cargo clippy --all-targets`（每个 ticket 完成前必跑）

测试命名：`<行为>_<场景>`，例如：
- `pin_ref_parse_board_digital`
- `wire_validates_components_exist`
- `apply_event_button_updates_state`

## 七、错误处理约定

- **lib 函数**：返回 `anyhow::Result<T>`，用 `bail!("xxx")` 或 `.context("...")?` 串错误
- **CLI 入口**（main.rs 路由）：返回 `Result<()>`，错误打印到 stderr 并 `std::process::exit(1)`
- **不要**用 `unwrap()` / `expect()` 在 lib 代码里（除非有充分注释说明 invariant）
- **测试代码可以**用 `unwrap()`

错误消息约定：
- 小写开头："simavr bridge not found — set $MOXIN_BRIDGE env var"
- 加修复提示："unsupported board `xxx` — supported: arduino-uno, stm32, gd32vf103"

## 八、Bridge 协议（双向）

### 8.1 Bridge → moxin（出向，JSON Lines）

```
{"event":"ready","mcu":"atmega328p","freq":16000000}
{"event":"pin","t_us":12345,"port":"B","bit":5,"value":1}
{"event":"serial","t_us":12346,"line":"hello"}
{"event":"button","t_us":12347,"pressed":true}
{"event":"exit","state":0}
```

详见 `docs/design/bridge-protocol.md`。

### 8.2 moxin → bridge（入向，单行命令）

当前只有：
- `button down\n` / `button up\n`（仅 simavr bridge 解析）

D5 加电位器调值时加：
- `analog A0 512\n`

未来扩展协议**都要先升 schema version 字段**（目前没有，D6 加）。

## 九、AI 必须遵守的硬约束

| 不要做的 | 原因 |
|---|---|
| ❌ 改 Cargo.toml 依赖版本 | 锁定的版本经过测试 |
| ❌ 新增 dependency（除非 ticket 明说） | crate 选型是产品决策 |
| ❌ 重命名 public 函数 / 改 trait 签名 | 散布在多个文件，连锁修改 |
| ❌ 把 anyhow::Result 改成自定义 Error 枚举 | Phase 2 才考虑 |
| ❌ "顺手优化"无关代码 | 增大 PR review 成本 |
| ❌ 删 _project_marker / Deprecated 函数 | 除非 ticket 明说 |
| ❌ 改 `[profile.release]` | LTO / opt-level 是 trade-off 已优化 |
| ❌ `cargo update` | 锁文件锁定有意为之 |
| ❌ 加 async / tokio | 项目坚持同步设计 |
| ❌ 改 unsafe 代码 | 没有 unsafe，不要引入 |

## 十、AI 协作流程

1. **读 ticket 文件**：`tickets/D<n>-<m>.md`
2. **读 AI-CONTEXT（本文件）** 和 **CONVENTIONS.md**
3. **读 ticket 引用的源文件**
4. **写代码**
5. **跑** `cargo build && cargo test && cargo clippy --all-targets`，三个全绿
6. **输出 diff** + 验收命令结果

如果有歧义，**停下问用户**，不要自由发挥。

## 十一、运行项目最快路径

```bash
# 编译 + 安装到 PATH
cargo install --path .

# 编 simavr bridge（一次性）
cd bridge && make && cp moxin-simavr-bridge ~/.cargo/bin/
# 或：export MOXIN_BRIDGE=$(pwd)/moxin-simavr-bridge

# 新建项目跑 blink
cd /tmp && moxin new demo --board=uno && cd demo
moxin run
# 进入 TUI，按 r run，按 s stop，按 q quit
```

## 十二、当前已知 bug（10 天内会修）

| ID | bug | 修的 ticket |
|---|---|---|
| BUG-1 | `BridgeEvent::Button._t_us` 字段名拼错，反序列化失败 | D1-2 |
| BUG-2 | TUI sim 运行时第一个字符同时写 serial 和 buffer | D2-1 |
| BUG-3 | 输入条光标 CJK / emoji 错位 | D2-2 |
| BUG-4 | `RunningSim::stop` 泄漏 detach 线程 | D2-3 |
| BUG-5 | STM32 wire D13 别名误导 | D1-4 |
| BUG-6 | examples/stm32-blink 结构与 cmd_new 不一致 | D1-5 |

---

读完这份，你应该能：
- 知道改某种功能要动哪个文件
- 知道项目用 anyhow 不用 thiserror，用 ratatui 不用 termion
- 知道不要做什么（第九节）
- 知道每个 ticket 完成的硬指标（第十节最后一句）

有问题先翻这份，没答案再问用户。
