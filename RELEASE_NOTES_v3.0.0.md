# MoXin CLI v3.0.0 — "AI 直接调用"

> 发布日期:2026-07-12
> 主线代号:**MCP server**
> 权威设计:`docs/design/v3-mcp-rfc.md`

---

## 一句话总结

**v3.0.0 = MoXin 从"AI 能读的 CLI"进化成"AI 能用的工具"。** `moxin mcp` 起一个
MCP server,Claude Desktop / Cursor 等客户端可以直接调用 MoXin 编译、跑仿真、
注入激励、读全外设状态 —— 不再解析 stdout,而是驱动 MoXin。

---

## 亮点

### 1. MCP server(stdio JSON-RPC 2.0,手写不引 SDK)

`moxin mcp` 走标准 MCP stdio transport,按行分隔 JSON-RPC。**不引入任何新 crate**——
协议本身就是 JSON-RPC,手写在 serde_json 上,守住依赖锁。

### 2. 9 个 tool + 1 个 resource

| tool | 作用 |
|---|---|
| board_info / list_components / describe_project | 内省:板 / 元件 / 项目 |
| build / run / stop | 编译 + 会话内管理仿真 |
| sim_state / read_state | 实时 / 落盘的全外设状态快照 |
| inject | 注入 adc / dist / env / ir / serial |

resource `moxin://state`:可寻址的状态快照(运行中读实时,否则读文件)。

### 3. AI 端到端闭环

```
describe_project → build → run → inject{adc,0,800} → sim_state(确认) → stop
```

CI verify 新增 MCP e2e 关卡(`scripts/mcp_smoke.py` 真机驱动 adc-potentiometer 全流程)。

## 配置

```json
{ "mcpServers": { "moxin": { "command": "/path/to/moxin", "args": ["mcp"] } } }
```

见 `docs/mcp-client/`。

## 质量线

- `cargo test` 213 通过(v0.7.0:189)
- clippy 0 警告;CI 十道真机关卡(九外设 + MCP e2e)+ MCP 协议冒烟

## 已知限制 / 后续

- run 用 json_out=false,仿真事件只进 RunState,不污染 MCP 的 stdout JSON-RPC 通道
- assert-via-MCP:现可用 read_state/sim_state + inject 组合替代;独立 assert tool 视需求再加
- HTTP/SSE transport 不做(只 stdio);AI Inspector 接真 LLM 仍是独立后续项
