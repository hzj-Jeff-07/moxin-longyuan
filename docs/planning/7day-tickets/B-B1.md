# B1 · TUI 模式切换 Ctrl-S / Esc

## 任务

TUI 加两种模式：

- **观察模式 (默认)**：仅展示运行时状态,所有按键忽略 (除 Ctrl-C 退出)
- **交互模式**：Ctrl-S 进入,Esc 退出。只在此模式接受按钮按下、电位器调节等用户输入

底部状态栏显示当前模式：`[OBSERVE]` 或 `[INTERACT]`,颜色区分。

## 允许动的文件

- `src/tui.rs`(加 `Mode` 枚举,按键 dispatch 分流)
- `src/tui_state.rs` 或类似(状态机)
- `tests/tui_mode_switch.rs`

## 验收

```powershell
cargo test tui_mode
cargo clippy --all-targets
moxin run examples/led-control
# 启动后底部 [OBSERVE] 灰色,按 Ctrl-S 变 [INTERACT] 黄色,按 Esc 回到 OBSERVE
```

测试要点：
- 启动默认 OBSERVE
- OBSERVE 下按钮快捷键不响应
- INTERACT 下 Ctrl-S 不再触发(避免误触)
- Ctrl-C 任何模式都能退出

## 约束

- 不动元件状态显示逻辑,只动按键处理
- 模式名用 OBSERVE / INTERACT,大写
- 不引入新的 TUI 库依赖

## commit message

`feat(B1): TUI 观察/交互双模式切换`
