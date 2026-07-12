# v3 RFC — MCP Server(让 AI Agent 直接调用 MoXin)

> 状态:**进行中(2026-07-12 启动,用户授权开 v3)**
> 前置:v0.7.0(Phase 3 完工)已合并 main
> 目标版本:**v3.0.0**(大版本;MCP 是 v3 头牌,见 CLAUDE.md"留 v3")
> 起点 commit:`c7371a1`(main)
> 最后更新:2026-07-12

---

## 一、为什么做

MoXin 到 v0.7.0 已经把"AI 读硬件状态"做扎实了:JSON Lines 事件流、
`.moxin-state.json` 全外设快照、`assert` 退出码。但这些是**被动接口**——
AI Agent 要自己 spawn 进程、解析 stdout、读文件。

MCP(Model Context Protocol)是 AI Agent 调用外部工具的标准协议。给 MoXin
装一个 MCP server,Claude Desktop / Cursor / 任何 MCP 客户端就能**直接调用**
MoXin 的能力作为 tool:编译、跑仿真、读状态、注入激励、断言 —— 无需了解
命令行细节。这是 MoXin 从"AI 能读的 CLI"进化成"AI 能用的工具"。

---

## 二、范围与决策(锁定)

### ✅ 做

| 增量 | 内容 |
|---|---|
| **M1** | MCP stdio JSON-RPC 2.0 核心 + `moxin mcp` 子命令 + 只读 tools(board_info / list_components / describe_project / read_state) |
| **M2** | 有状态 tools:build / run / stop / inject(adc/dist/env/ir/serial)/ assert |
| **M3** | resources:把 `.moxin-state.json` 暴露成 MCP resource;文档 + example client 配置 |

### ❌ 不做(维持禁区 / 留后续)

- **不引入新 crate**:MCP 就是 JSON-RPC 2.0,用现有 `serde_json` 手写解析,
  不引 `rmcp` / `jsonrpc` 等 SDK(守 CLAUDE.md 依赖锁)
- **不做 SSE / HTTP transport**:只做 stdio(MCP 本地工具的标准 transport,
  与 MoXin 的 stdio 事件流一脉相承)
- **不做 LLM Inspector**:AI Inspector 接真模型仍是独立 v3 项,与 MCP 解耦
- **不改 bridge / SCHEMA_VERSION / LICENSE**

### 关键决策

| 决策 | 理由 |
|---|---|
| 手写 JSON-RPC over stdio,不引 SDK | CLAUDE.md 锁依赖;MCP 协议本身简单(JSON-RPC 2.0),serde_json 够用;避免 SDK 版本/传染风险 |
| stdio transport,消息按行分隔(每行一条 JSON-RPC) | 与 MoXin 现有 JSON Lines 一致;MCP stdio transport 官方即换行分隔 |
| 先只读 tools(M1)再有状态(M2) | 只读 tools 无需 simavr,可纯单测(喂 JSON-RPC 请求断言响应);先把协议骨架验证可靠 |
| tool 命名 `snake_case`(board_info / read_state) | MCP 生态惯例;与 CLI 子命令区分 |

---

## 三、协议实现(M1)

### transport

stdio,每行一条 JSON-RPC 2.0 消息(request / response / notification)。
`moxin mcp` 从 stdin 逐行读,处理后往 stdout 写一行响应;日志走 stderr。

### 必须实现的方法

| 方法 | 说明 |
|---|---|
| `initialize` | 握手:返回 serverInfo(name=moxin, version)+ capabilities(tools) |
| `initialized`(notification)| 客户端就绪通知,无需响应 |
| `tools/list` | 列出所有 tool 的 name / description / inputSchema |
| `tools/call` | 调用一个 tool,返回 content(text) |
| `ping` | 返回空对象(保活) |

未知方法 → JSON-RPC error `-32601 Method not found`。
解析失败 → `-32700 Parse error`。

### M1 只读 tools

| tool | 入参 | 返回 |
|---|---|---|
| `board_info` | `{board: string}` | 板规格(mcu/主频/引脚/GPIO 数) |
| `list_components` | `{}` | 17 种元件的 kind + 别名(注册表派生) |
| `describe_project` | `{path?: string}` | 解析 moxin.toml → 元件/接线/板 |
| `read_state` | `{path?: string}` | 读 `.moxin-state.json` 全外设快照 |

### 结构(src/mcp.rs)

```rust
pub fn cmd_mcp() -> Result<()> {
    // stdin 逐行 → dispatch → stdout 逐行
}
fn dispatch(req: JsonRpcRequest) -> JsonRpcResponse { ... }
fn tool_board_info(args) -> Result<String> { ... }
// 纯函数,便于单测:dispatch(请求 Value) -> 响应 Value
```

---

## 四、测试策略

- M1 全部可纯单测:构造 JSON-RPC 请求 `Value`,断言响应 `Value`(initialize
  握手、tools/list 含 4 个 tool、tools/call board_info 返回含 "ATmega328P"、
  未知方法返回 -32601、坏 JSON 返回 -32700)
- M2 的 build/run/assert tools 与现有 CLI 复用同一后端,CI verify 关卡沿用
- 目标:M1 收尾 `cargo test` ≥210 / clippy 0 警告

---

## 五、里程碑

- [x] M1:协议核心 + 4 只读 tools + `moxin mcp` + 单测(2026-07-12,cargo test 208,含真机 stdio 冒烟 + CI 关卡)
- [ ] M2:有状态 tools(build/run/stop/inject/assert)+ CI
- [ ] M3:resources + example MCP client 配置 + README + tag v3.0.0

---

## 六、决策记录

| 日期 | 决策 | 理由 |
|---|---|---|
| 2026-07-12 | 启动 v3,MCP server 从禁区解锁 | 用户明确授权做下一个大版本;MCP 是 CLAUDE.md 标注的 v3 头牌 |
| 2026-07-12 | 手写 JSON-RPC,不引 MCP SDK | 守依赖锁;协议简单 serde_json 够用 |
| 2026-07-12 | M1 先只读,可纯单测 | 先验证协议骨架可靠,再接有状态/需 simavr 的 tools |

后续决策追加在此表底部。
