# A4 · 7 段数码管元件 schema + 仿真

## 任务

加入共阴极 7 段数码管元件 (7-segment display, single digit)。8 路输入 (a/b/c/d/e/f/g/dp) + 1 路公共阴极。

仿真层：根据 8 路引脚电平,实时计算当前显示的字符,RunState 里更新 `display_char: String`(可能是 "0"-"9"、"A"-"F"、" "、"-" 或 "??" 未识别)。

## 允许动的文件

- `components/seven_segment.toml`(S1 没建则本 ticket 建)
- `pin-anchors-template/seven_segment.json`
- `src/sim/components/seven_segment.rs`(新文件,8 路电平 → 字符查表)
- `src/sim/runstate.rs`(注册实例状态)
- `tests/seven_segment_e2e.rs`

## 验收

```powershell
cargo test seven_segment
cargo clippy --all-targets
# 拼出 "8" (全段亮): display_char == "8"
# 拼出 "1" (只 b/c 亮): display_char == "1"
```

测试要点：
- 0-9 全部能识别
- A-F (十六进制) 能识别
- 不存在的段组合显示 "?"
- dp 段单独跟踪,字符里以 "1." 形式表示

## 约束

- 只支持单位数码管。4 位串联型 Phase 2
- 只支持共阴极 (cathode common)。共阳极元件需另起 `seven_segment_anode`
- pin name 严格 `a`/`b`/`c`/`d`/`e`/`f`/`g`/`dp`/`cathode`,electrical 全部 `digital_in` 除 cathode 是 `gnd`

## commit message

`feat(A4): 7 段数码管元件 schema 与仿真`
