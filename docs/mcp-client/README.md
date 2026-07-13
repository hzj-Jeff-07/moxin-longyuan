# mcp-client

把 MoXin 接入支持 MCP 的 AI 客户端（Claude Desktop / Cursor 等），让 AI 直接把
MoXin 当工具调用，而不是解析命令行输出。

## 配置

`claude_desktop_config.json` 是一份最小配置片段：把 `command` 换成你机器上
`moxin` 二进制的绝对路径，合并进客户端的 MCP 配置即可。

```json
{
  "mcpServers": {
    "moxin": { "command": "/absolute/path/to/moxin", "args": ["mcp"] }
  }
}
```

## 可用 tools

| tool | 作用 |
|------|------|
| `board_info` | 查板子规格（mcu / 主频 / 引脚 / GPIO 数） |
| `list_components` | 列全部 17 种内置元件及别名 |
| `describe_project` | 解析 moxin.toml → 板 / 元件 / 接线 |
| `read_state` | 读 `.moxin-state.json` 全外设快照 |
| `build` | 编译固件 |
| `run` / `stop` | 启动 / 停止仿真（会话内保持） |
| `sim_state` | 运行中仿真的实时全外设快照 |
| `inject` | 注入激励：`kind` = adc / dist / env / ir / serial |
| `assert` | 断言条件，返回 PASS/FAIL/TIMEOUT：`pin`+`eq` / `pin`+`toggles` / `serial_contains` |

以及 resource `moxin://state`（可寻址的状态快照）。

## AI 的典型闭环

```
describe_project → build → run → inject{adc,channel:0,value:800}
  → sim_state（确认 adc[0]=800、串口输出、LED 状态）
  → assert{serial_contains:"A0="}（拿 PASS/FAIL 判定）→ stop
```

`moxin mcp` 走 stdio JSON-RPC 2.0（手写、不引 SDK）。协议细节见
`docs/design/v3-mcp-rfc.md` 和 `docs/design/bridge-protocol.md`。
