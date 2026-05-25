# MoXin CLI v0.3.0 — "让 AI 直接调试 MCU"

> 发布日期:2026-05-25
> 主线代号:**AI 闭环可用**
> 目标:Arduino Uno (simavr) + STM32F405 (qemu netduinoplus2) 双板,AI Agent 可以"写代码 → 编译 → 仿真 → 断言"全程无人值守。

---

## 一句话总结

**v0.3.0 = MoXin 第一个对 AI Agent 真正可用的版本。**

退出码即结论,JSON Lines 即事件流,状态快照文件即"当前世界"。Claude Code / Cursor 等 Agent 现在可以把 MoXin 当作"硬件世界的 pytest"使用。

---

## 亮点(Highlights)

### 1. `moxin assert` — 退出码即真相

新增断言子命令,专为 CI / AI 自动验证设计。**三种互斥模式**:

```bash
# 引脚电平等值断言
moxin assert --pin D13 --eq HIGH --after 1s

# 引脚翻转断言(闪烁验证)
moxin assert --pin D13 --toggles --within 3s

# 串口子串断言
moxin assert --serial-contains "hello" --within 2s
```

**退出码契约**:`0 = pass / 1 = fail / 2 = timeout`。AI 不需要解析任何文本,直接看 `$?`。

### 2. `moxin run --output json` — 纯净 JSON Lines

stdout 严格只输出 bridge 事件流(每行一条 JSON),状态提示走 stderr。Agent 可以放心 `JSON.parse` 每一行。

事件类型已稳定:`ready / pin / serial / button / exit`。

### 3. `build/.moxin-state.json` — 实时状态快照

`moxin run --output json` 运行期间,持续把"当前各引脚电平 / 串口缓冲"快照写到 `build/.moxin-state.json`。Agent 不必维护事件流状态机,**随时读文件就是当前世界**。

### 4. `moxin status --pin <name>` — 一次性快照查询

```bash
moxin status --pin D13   # → HIGH / LOW / UNKNOWN
```

### 5. Inspector 面板新增 Components 汇总

TUI 模式下,Inspector 面板现在显示所有元件的实时状态汇总(LED 颜色/亮灭、Button 按下/释放等),一屏看清整个电路。

### 6. 元件扩展:7 种新元件 + TUI 渲染

`Component` 结构扩展,shell 解析支持 7 种新元件,TUI 渲染 6 种新元件,plain text 模式提供通用线缆遍历视图。

### 7. AI 接入指南(docs/ai-integration-guide.md)

新增完整文档,讲清楚 AI Agent 与 MoXin 协作的"四步法":`edit → build → run/json → assert`。包含 Claude Code Skill 接入示例。

### 8. 双示例覆盖 assert 用法

- `examples/assert-blink-toggles/` — 验证 D13 在 3 秒内翻转
- `examples/assert-serial-hello/` — 验证串口出现指定子串

---

## 完整命令一览

| 命令 | 状态 | 说明 |
|---|---|---|
| `moxin new <name> [--board uno\|stm32]` | ✅ 稳定 | 新建项目 |
| `moxin build` | ✅ 稳定 | 编译固件 |
| `moxin run [--output tui\|json]` | ✅ 稳定 | 启动仿真 |
| `moxin shell [--no-tui]` | ✅ 稳定 | TUI / REPL |
| `moxin status --pin <name>` | 🆕 v0.3 | 快照查询 |
| `moxin assert ...` | 🆕 v0.3 | 断言(CI/AI) |
| `moxin doctor` | ✅ 稳定 | 三平台依赖检查 |
| `moxin install` | ⚠️ macOS-only | 其他平台返回安装提示 |

---

## 支持的板子

| 板子 | bridge | 状态 |
|---|---|---|
| `arduino-uno` | simavr | ✅ 完整支持(D13 真实仿真) |
| `stm32` (STM32F405) | qemu netduinoplus2 | ✅ 完整支持(PA13 真实仿真) |
| `gd32vf103` (RISC-V) | — | ⛔ 占位,`build`/`spawn_sim` 直接 `bail`,**不在本版承诺范围** |

---

## 跨平台

- ✅ Windows / macOS / Linux 三平台 `cargo build --release` 出 binary
- ✅ Windows 路径修复:`USERPROFILE` fallback、bridge 二进制 `.exe` 后缀查找
- ✅ `moxin doctor` 三平台输出针对性安装提示(brew / apt / scoop)

> ⚠️ 注:`moxin install` 当前仅 macOS 实现自动安装,Windows/Linux 返回安装指引。

---

## 质量基线

- **94 个测试** 全部通过(无外部 simavr/qemu 依赖部分)
- `cargo clippy --all-targets -- -D warnings` **0 警告**
- `cargo build --release` 三平台通过

---

## 破坏性变更(Breaking)

**无。** v0.3.0 完全向后兼容 v0.2.x 的 `moxin.toml`(`SCHEMA_VERSION = "0.2"`)。

---

## 已知坑(Known Issues)

1. **只有 D13 / PA13 是真实仿真**,其他引脚静态返回 OFF。这是 Phase 1 锁定范围,不在本版修复。
2. **GD32VF103 是占位**,任何对它的 `build` / `run` 会直接报 `not yet implemented`。RISC-V bridge 留到 v0.4+。
3. **元件级仿真有限**:LED / Button 完整,I2C / SPI / OLED / LCD / DHT11 等需要 bridge 配合,**不在本版**。
4. **MCP server 未实现**:留到 v3,本版只提供 CLI + JSON Lines 接口。
5. **`moxin install` 仅 macOS 自动化**:其他平台请按 `moxin doctor` 提示手动装依赖。

---

## AI Agent 接入(关键提示)

如果你在 Claude Code / Cursor 中使用 MoXin,请按以下四步固定流程:

```text
1. edit         你/AI 改 src/main.ino
2. moxin build  编译固件
3. moxin run --output json
                启动仿真,事件流走 stdout,状态写 build/.moxin-state.json
4. moxin assert ...
                看退出码:0 pass / 1 fail / 2 timeout,据此决定下一步
```

完整指南:[docs/ai-integration-guide.md](docs/ai-integration-guide.md)

---

## 升级指南(从 v0.2.x)

无需修改 `moxin.toml`。直接替换 binary:

```bash
# 源码安装
cargo install --path . --force

# 或下载本版 release binary(三平台)
# (待 release 上传后填链接)
```

---

## 致谢

- `simavr` 项目:Arduino Uno 仿真后端
- `qemu` 项目:STM32 仿真后端
- BUSL-1.1 许可证,详见 LICENSE

---

## 下一站(v0.4 预告 — 非承诺)

- 全引脚级仿真(目前只有 D13/PA13)
- I2C / SPI 元件(OLED / LCD1602 / DHT11)
- `moxin install` Windows / Linux 自动化
- MCP server(让 AI 直接调用 MoXin 而不走 CLI)

---

## 完整变更日志(v0.1.0 → v0.3.0)

主要提交(按时间倒序):

```
f4cc6a6 docs(examples): 补 assert 演示用例 (toggles + serial-contains)
84764ba feat(phase3): moxin assert + AI 接入指南 (v0.3.0 收尾)
db1fb4a test(P3-3): render_runtime_frame 覆盖 Phase 2 新元件
ea2f10d docs(examples): led-control README 补 Run 与 Dependencies 段
5bdcc2a feat(P3-1): Inspector 新增 Components 汇总行
2c04369 feat(P2-2): TUI 渲染 6 种新元件 + plain text 通用线缆遍历
c0b1ea2 feat(P2-1): Component 结构扩展 + shell 解析 7 种新元件
2d0e4b0 feat(P1):   新增 cmd_status + doctor 跨平台提示 + Windows 兼容
6d16353 main/shell(D8-1): moxin run --output json 模式
be634af feat:        新增 serial-echo example
529034c feat:        新增 button-counter example
6e129ba refactor(S3): STM32 wire 改用 PA13 + 新增 stm32-blink README
95ec964 fix(S2):     Button 事件时间戳 + README 改用 cargo install
1045e9b schema(S1):  合并 schema bundle 与引脚校验脚本
88cd220 feat:        新增 doctor/install 命令、Windows editor fallback
```

完整 git log:`git log --oneline` 或 GitHub Compare 页。

---

> 龙渊归鞘,代码参禅。
> — MoXin 团队,2026-05-25
