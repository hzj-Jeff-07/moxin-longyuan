# MoXin CLI v3.1.0 — MCP `assert` tool

> 发布日期:2026-07-13
> 权威设计:`docs/design/v3-mcp-rfc.md`

---

## 一句话总结

**v3.1.0 = 给 MCP server 补上第 10 个 tool `assert`。** AI Agent 现在能直接对运行中的
仿真下断言,拿到 `PASS` / `FAIL` / `TIMEOUT` 判定,不必再自己轮询 `sim_state` 并推理。
补齐了 MCP 与 CLI 的断言能力对等(v3.0.0 RFC 里 "assert-via-MCP" 曾被暂缓)。

---

## 亮点

### `assert` tool(第 10 个)

三种模式,与 `moxin assert` CLI 一一对应:

| 参数 | 模式 | 返回 |
|---|---|---|
| `pin` + `eq`(HIGH/LOW)+ `after` | 稳定后读一次电平做相等判定 | PASS / FAIL / TIMEOUT |
| `pin` + `toggles` + `within` | 窗口内观察到任一次电平翻转即过 | PASS / TIMEOUT |
| `serial_contains` + `within` | 窗口内某行串口输出含子串即过 | PASS / TIMEOUT |

对 session 里**已在运行**的仿真求值(先 `run`),阻塞至多 `within`/`after`。

### 零重复:判定逻辑 CLI/MCP 共用

从 `cmd_assert` 抽出 `pub(crate) fn evaluate(sim, spec, mode)`,CLI(自 spawn sim)与
MCP(复用 session 的 sim)走同一套判定,`AssertMode::resolve` 也复用——引脚可观测性、
时长解析、模式互斥校验全部一致,不再各写一份。

## 质量线

- `cargo test` 214 通过(v3.0.0:213)
- clippy 0 警告
- CI MCP e2e 关卡追加一次 `assert{serial_contains:"A0="}`,真机验证 assert tool 闭环

## 已知限制 / 后续

- `assert` 对**当前累积状态**求值:`serial_contains` 会匹配到 `run` 以来的历史串口输出
  (AI 驱动场景符合预期);需"从此刻起"的语义可先 `stop`/`run` 重置
- 引脚可观测性同 CLI:Arduino 任意 D0-D13/A0-A5;STM32/gd32 仅板载 D13
