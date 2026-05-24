# A7 · RunState 快照写入 `~/.cache/moxin/`

## 任务

每 100ms 把当前 RunState 序列化为 JSON,写入 `~/.cache/moxin/runstate-<pid>.json`。

进程退出时清理该文件。

这是 B 窗口 `moxin status` 命令的数据源,A 窗口必须先把写入侧做好。

## 允许动的文件

- `src/sim/runstate.rs`(增加 `snapshot_to_disk()` 方法)
- `src/sim/loop.rs` 或主循环处(每 100ms tick 调用)
- `src/main.rs`(进程退出 hook 清理快照文件)
- `Cargo.toml`(可能要加 `dirs` crate 找 cache 目录)
- `tests/runstate_snapshot.rs`

## 验收

```powershell
cargo test runstate_snapshot
cargo clippy --all-targets
# 跑一个 example,另开终端看 cache 目录有 runstate JSON
moxin run examples/led-control &
ls $env:LOCALAPPDATA\moxin   # Windows 上 dirs 给的 cache_dir
# 杀进程,文件被清理
```

测试要点：
- 文件路径含 PID,多实例运行不冲突
- JSON schema 稳定 (字段加新的不删旧的)
- 进程正常退出 / Ctrl-C 都能清理
- panic 退出时残留文件 OK,B6 status 命令会跳过过期文件

## 约束

- Windows 走 `dirs::cache_dir()` (通常 `%LOCALAPPDATA%`),不要硬编码 `~/.cache`
- 写入失败不要 panic,只 warn (cache 目录可能只读)
- JSON 字段名用 snake_case

## commit message

`feat(A7): RunState 每 100ms 快照到 cache 目录`
