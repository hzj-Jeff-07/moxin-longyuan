# tickets · 10 天 ticket 索引

每个文件是一个独立 AI prompt，按"任务 + 验收 + 约束 + 现有代码"四段式写成。
打开任一文件 → 整段贴给 Cursor / Claude / 我 → AI 即可干活。

**用法**：每天早上打开当天的 ticket 文件，按顺序一个一个交给 AI 做。
做完一个 commit 一个，再开下一个。

## 进度追踪

D1 ☐ D1-1 schema bundle 合并（已有独立 prompt：`ticket-D1-1-prompt.md`）
D1 ☐ D1-2 修 BridgeEvent::Button _t_us bug
D1 ☐ D1-3 README cargo install
D1 ☐ D1-4 STM32 wire PA13 统一
D1 ☐ D1-5 examples/stm32-blink 文件结构对齐

D2 ☐ D2-1 TUI 模式切换 Ctrl-S / Esc
D2 ☐ D2-2 unicode-width 光标修复
D2 ☐ D2-3 RunningSim::stop 同步 join
D2 ☐ D2-4 删 _project_marker 死代码

D3 ☐ D3-1 14 数字引脚全仿真
D3 ☐ D3-2 multi-led example + RunState 扩展
D3 ☐ D3-3 蜂鸣器元件
D3 ☐ D3-4 buzzer-tone example

D4 ☐ D4-1 6 模拟引脚 ADC 仿真
D4 ☐ D4-2 7 段数码管元件
D4 ☐ D4-3 counter-7seg example

D5 ☐ D5-1 电位器元件 + TUI 快捷键
D5 ☐ D5-2 被动元件 schema 层
D5 ☐ D5-3 pot-led-brightness example
D5 ☐ D5-4 button-led example

D6 ☐ D6-1 RFC runtime query IPC 方案
D6 ☐ D6-2 RunState 快照写入 ~/.cache/moxin/
D6 ☐ D6-3 `moxin status` 命令

D7 ☐ D7-1 assert DSL grammar 设计
D7 ☐ D7-2 assert DSL parser + matcher
D7 ☐ D7-3 `moxin assert` CLI 入口
D7 ☐ D7-4 集成测试 assert e2e

D8 ☐ D8-1 `moxin run --output json` 模式
D8 ☐ D8-2 AI 接入指南文档
D8 ☐ D8-3 .cursorrules + CLAUDE.md 模板

D9 ☐ D9-1 buggy firmware demo 准备
D9 ☐ D9-2 AI 调试 session 演练 + 记录
D9 ☐ D9-3 录视频 + 剪辑（3-5 分钟）
D9 ☐ D9-4 中英双字幕

D10 ☐ D10-1 README 首屏改造
D10 ☐ D10-2 GitHub Release v0.5.0
D10 ☐ D10-3 财经部素材包
D10 ☐ D10-4 Phase 2 backlog 24 个 issue
D10 ☐ D10-5 Buffer / 庆祝

---

## ticket 详尽程度说明

- **D1-D5（18 个）**：完整 prompt，含代码片段 / 验收命令 / commit message。
  AI 拿到即可干活，不需要你额外补 context。

- **D6-D10（20 个）**：简版 prompt，含任务 / 验收 / 约束 / commit message。
  足够 AI 起步；如需更详细，你前一晚加 10-15 分钟把代码片段补进来。

## 如果 AI 跑偏怎么办

| 跑偏类型 | 处理 |
|---|---|
| 改了 ticket 没明说的文件 | 拒收，重做 |
| 加了禁止的依赖 | 拒收，重做 |
| 测试不绿就交付 | 拒收，重做 |
| 答非所问 | 重新读 ticket，把验收标准粘贴一遍强调 |
| 顺手"优化"无关代码 | 让 AI 撤销 diff 中无关的部分 |

## 给 AI 的标准开场白

每个 ticket 给 AI 时，前面加一句固定开场白：

```
请先读 docs/AI-CONTEXT.md 和 docs/CONVENTIONS.md，再读下面这份 ticket。
完成后给我 diff + 验收命令的输出。如果遇到歧义，停下问我，不要自由发挥。

<贴 ticket 全文>
```

这样 AI 每次都先建立项目上下文，不会因为对话过长而漂移。

## 如果某天提前做完

可以提前开下一天的 ticket。**但不要跨过当天的"晚结"**：commit & push、跑测试、
发群里 gif 这套动作必须做完才能开下一天。否则容易"做了但没人知道"。

## 如果某天做不完

按 sprint-plan.md §五"风险与对策"里的应对策略，优先砍 example 和测试覆盖，
保留核心 feature。
