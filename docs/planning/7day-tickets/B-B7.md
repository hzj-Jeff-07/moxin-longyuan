# B7 · `moxin run --output json` 模式

## 任务

`moxin run` 加 `--output json` 标志。开启后：

- 不启动 TUI
- 把每个 BridgeEvent 用 JSON Lines 格式 (一行一个 JSON) 输出到 stdout
- 元件状态变化也输出 (限流：同一元件同一字段 100ms 内只输出一次)
- Ctrl-C 退出时输出一个 `{"event": "shutdown"}` 行

这个模式给 AI 工具 (Claude Code / Cursor) 调用用,方便它们 parse moxin 行为做调试。

## 允许动的文件

- `src/cmd_run.rs`(加 --output 标志)
- 新增 `src/output/json_lines.rs`
- `src/main.rs`(标志解析)
- 新增 `tests/run_json_output.rs`

## 验收

```powershell
cargo test run_json
cargo clippy --all-targets
# 跑 example 看 JSON Lines 输出
moxin run examples/led-control --output json | Select-Object -First 10
# 每行都是合法 JSON,含 timestamp_us / event_type / payload 字段
```

测试要点：
- 输出每行 `serde_json::from_str` 能解析
- 同 LED 在 100ms 内多次切换被限流为一行
- exit on Ctrl-C 输出 shutdown 事件

## 约束

- 不支持 --output yaml / xml / 别的格式 (只 json + 默认 TUI)
- stderr 仍然保留人类可读 log,JSON 只走 stdout
- 不动现有 TUI 代码,只加分支

## commit message

`feat(B7): moxin run --output json 模式`
