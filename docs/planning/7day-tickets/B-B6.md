# B6 · RFC IPC + `moxin status` 命令

## 任务

两部分合并：

1. **RFC 文档 `docs/design/runtime-query.md`**：写一份 ADR/RFC,说明为什么选 "cache 目录 JSON 快照" 而不是 socket / D-Bus / 命名管道。1-2 页够。
2. **`moxin status` CLI 实现**：读 `~/.cache/moxin/runstate-*.json` (跨平台路径走 dirs crate),列出当前所有运行中的 moxin 实例和它们的 RunState 摘要。

## 允许动的文件

- 新增 `docs/design/runtime-query.md`
- 新增 `src/cmd_status.rs`
- `src/main.rs`(注册子命令)
- 新增 `tests/status_cmd.rs`

## 验收

```powershell
cargo test status
cargo clippy --all-targets
# 一个终端跑 example
moxin run examples/led-control &
# 另一个终端
moxin status
# 输出:
# PID 12345  project=led-control  uptime=12s
#   board=arduino-uno
#   led1.level=on  led1.brightness=255
```

测试要点：
- 没有快照文件时输出 "no running instances",exit 0
- 过期快照 (mtime > 5s) 不显示
- 多实例都能列出

## 约束

- 这一票依赖 A7 (cache 写入)。A7 没合到 main 前不要开
- 不实现 follow / watch 模式 (Phase 2)
- RFC 不超过 200 行

## commit message

`feat(B6): runtime-query RFC + moxin status 命令`
