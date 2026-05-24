# A5 · 电位器元件 + TUI 快捷键

## 任务

加入 10kΩ 电位器元件,3 引脚 (vcc / wiper / gnd),wiper 输出 0-5V 模拟电压。

支持 TUI 快捷键调节当前选中电位器的位置：`[` 减小、`]` 增大,每次 5%。

## 允许动的文件

- `components/potentiometer.toml`
- `pin-anchors-template/potentiometer.json`
- `src/sim/components/potentiometer.rs`
- `src/tui.rs`(增加电位器选中 + 快捷键处理)
- `src/sim/runstate.rs`(注册实例状态: position_pct: u8, voltage_mv: u16)
- `tests/potentiometer_e2e.rs`

## 验收

```powershell
cargo test potentiometer
cargo clippy --all-targets
# TUI 启动后选中 pot1,按 ] 6 次,position_pct=30,voltage_mv≈1500
moxin run examples/pot-led-brightness
```

测试要点：
- position_pct 0/50/100 对应 voltage_mv 0/2500/5000
- `[` `]` 快捷键每次 5%,不超出 [0,100]
- TUI 显示当前 % 和电压

## 约束

- TUI 快捷键只在电位器被选中时生效,不要影响其它输入
- voltage 计算: `voltage_mv = position_pct * 5000 / 100`,整数运算
- 不要在 TUI 加新的菜单/对话框,只加快捷键

## commit message

`feat(A5): 电位器元件 + TUI [ ] 调节快捷键`
