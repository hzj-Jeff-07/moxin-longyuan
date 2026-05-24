# C8 · buggy firmware demo + AI session 演练记录

## 任务

为视频准备一个"含 bug 的 blink demo",再用 AI 调试一遍,全程记录。

1. **造 bug**：新建 `examples/buggy-blink/`,固件代码里植入一个常见 bug (推荐：把 delay(500) 写成 delay(50000),或者把 LED 引脚写错)。
2. **AI 调试演练**：在 Cursor/Claude Code 里,只给 AI 看 firmware 代码 + `moxin run --output json` 输出,让它指出 bug 并改对。
3. **记录 session**：把对话保存为 `docs/demo/session-log.md`,含用户提问、AI 回复、最终 diff。这份记录是 C9 录视频的"剧本"。

## 允许动的文件

- 新增 `examples/buggy-blink/`(同 example 标准结构)
- 新增 `docs/demo/session-log.md`
- 可选：新增 `docs/demo/buggy-blink-fixed.diff`(展示最终修复)

## 验收

```powershell
moxin run examples/buggy-blink
# 看到 LED 一直不闪 / 或者闪得超慢,确认 bug 真的存在

# 看 session log 是真实对话
Test-Path docs/demo/session-log.md
(Get-Content docs/demo/session-log.md | Measure-Object -Line).Lines -ge 50
```

## 约束

- bug 必须是真实可触发、AI 能合理推理出来的 (不要太刁钻,目标是展示流程而不是炫技)
- session log 要真实截屏的对话,不要伪造
- demo 修复后的代码可以保留,但 main 分支的 example 要保留 buggy 版本 (视频要用)

## commit message

`demo(C8): buggy blink + AI 调试 session 记录`
