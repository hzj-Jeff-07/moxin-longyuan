# B2 · TUI 三连修

## 任务

三个小修一起做：

1. **unicode-width 光标修复**：当前光标位置在含中文/emoji 的状态行计算错位。引入 `unicode-width` crate,改用字符宽度而非字节长度计算列位置。
2. **RunningSim::stop 同步 join**：当前 stop 是 fire-and-forget,主线程可能比仿真线程先退出导致快照截断。改成同步 join。
3. **删 `_project_marker` 死代码**：grep 一下,这个字段已经没有任何读处。连同相关初始化一起删掉。

## 允许动的文件

- `src/tui.rs`(光标修复)
- `src/sim/running.rs` 或类似(RunningSim::stop)
- `Cargo.toml`(加 unicode-width)
- 任何 `_project_marker` 出现处(全删)
- `tests/tui_unicode.rs`(光标测试)

## 验收

```powershell
cargo test
cargo clippy --all-targets
# grep 确认死代码已删
Select-String -Path src -Pattern "_project_marker" -Recurse
# 应该 0 个结果
# TUI 显示中文元件名,光标位置正确
moxin run examples/with-chinese-name   # 临时构造测试
```

## 约束

- 这三个修不要拆成三个 commit,合并到一起做完一起提
- 删 `_project_marker` 前确认全 repo 0 引用,有引用先问
- unicode-width 是熟成 crate,不要自己造轮子

## commit message

`fix(B2): TUI unicode 光标 + Sim 同步 join + 删死代码`
