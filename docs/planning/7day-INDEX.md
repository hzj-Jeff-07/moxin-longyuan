# 7day-tickets · 29 ticket 索引 + 进度追踪

每个文件 = 一个独立 AI prompt。打开 → 粘开场白 + 全文给 AI → 跑验收 → commit。

**用法**：
1. 先看[7day-final-plan.md](./7day-final-plan.md)了解三窗口分工和时间表。
2. D1 三个 S 系列必须串行做完才能开 worktree。
3. D2-D5 三窗口并行，每个窗口按本表领自己的 ticket。
4. D6 C 独占视频。
5. D7 三窗口分头领 W 系列。

---

## D1 · 串行段（一个窗口跑完，A/B/C worktree 还没建）

- [ ] **S1** · schema bundle 合并 + CI 校验脚本 → [S1.md](./7day-tickets/S1.md)
- [ ] **S2** · 修 BridgeEvent::Button `_t_us` bug + README cargo install → [S2.md](./7day-tickets/S2.md)
- [ ] **S3** · STM32 wire PA13 统一 + examples 文件结构对齐 → [S3.md](./7day-tickets/S3.md)

S1-S3 全绿合并 main 后，建三个 worktree：

```powershell
git worktree add ../moxin-A feat/sim-core
git worktree add ../moxin-B feat/cli-assert
git worktree add ../moxin-C feat/examples-docs
```

---

## D2-D5 · A 窗口 · 仿真核心（7 ticket）

- [ ] **A1** · 14 数字引脚全仿真 → [A-A1.md](./7day-tickets/A-A1.md)
- [ ] **A2** · 蜂鸣器元件 schema + 仿真 → [A-A2.md](./7day-tickets/A-A2.md)
- [ ] **A3** · 6 模拟引脚 ADC 仿真 → [A-A3.md](./7day-tickets/A-A3.md)
- [ ] **A4** · 7 段数码管元件 schema + 仿真 → [A-A4.md](./7day-tickets/A-A4.md)
- [ ] **A5** · 电位器元件 + TUI 快捷键 → [A-A5.md](./7day-tickets/A-A5.md)
- [ ] **A6** · 被动元件 schema 层 → [A-A6.md](./7day-tickets/A-A6.md)
- [ ] **A7** · RunState 快照写入 `~/.cache/moxin/` → [A-A7.md](./7day-tickets/A-A7.md)

---

## D2-D5 · B 窗口 · CLI/TUI/Assert（7 ticket）

- [ ] **B1** · TUI 模式切换 Ctrl-S / Esc → [B-B1.md](./7day-tickets/B-B1.md)
- [ ] **B2** · TUI 三连修(unicode-width + RunningSim::stop + 死代码) → [B-B2.md](./7day-tickets/B-B2.md)
- [ ] **B3** · assert DSL grammar 设计 → [B-B3.md](./7day-tickets/B-B3.md)
- [ ] **B4** · assert DSL parser + matcher → [B-B4.md](./7day-tickets/B-B4.md)
- [ ] **B5** · `moxin assert` CLI + e2e 测试 → [B-B5.md](./7day-tickets/B-B5.md)
- [ ] **B6** · RFC runtime query IPC + `moxin status` → [B-B6.md](./7day-tickets/B-B6.md)
- [ ] **B7** · `moxin run --output json` 模式 → [B-B7.md](./7day-tickets/B-B7.md)

---

## D2-D5 · C 窗口 · 用户面（前 7 ticket，C8/C9 留给 D6）

- [ ] **C1** · multi-led example + RunState 扩展 → [C-C1.md](./7day-tickets/C-C1.md)
- [ ] **C2** · buzzer-tone example → [C-C2.md](./7day-tickets/C-C2.md)
- [ ] **C3** · counter-7seg example → [C-C3.md](./7day-tickets/C-C3.md)
- [ ] **C4** · pot-led-brightness example → [C-C4.md](./7day-tickets/C-C4.md)
- [ ] **C5** · button-led example → [C-C5.md](./7day-tickets/C-C5.md)
- [ ] **C6** · AI 接入指南文档 → [C-C6.md](./7day-tickets/C-C6.md)
- [ ] **C7** · `.cursorrules` + `CLAUDE.md` 模板 → [C-C7.md](./7day-tickets/C-C7.md)

---

## D6 · C 窗口独占（A/B 暂停 push）

- [ ] **C8** · buggy firmware demo 准备 + AI session 演练记录 → [C-C8.md](./7day-tickets/C-C8.md)
- [ ] **C9** · 录视频 3-5 分钟 + 中英双字幕 → [C-C9.md](./7day-tickets/C-C9.md)

**这是整个 7 天最重要的一票。其它都可以打折，这个不行。**

---

## D7 · 三窗口分头收尾

- [ ] **W1** · README 首屏改造 + GitHub Release v0.5.0（A 窗口领） → [W-W1.md](./7day-tickets/W-W1.md)
- [ ] **W2** · 财经部素材包（B 窗口领） → [W-W2.md](./7day-tickets/W-W2.md)
- [ ] **W3** · Phase 2 backlog 整理（C 窗口领） → [W-W3.md](./7day-tickets/W-W3.md)

---

## 进度统计

```
S 段（D1）        [ ] 0/3
A 窗口（D2-D5）   [ ] 0/7
B 窗口（D2-D5）   [ ] 0/7
C 窗口（D2-D5）   [ ] 0/7
C 视频（D6）      [ ] 0/2
W 收尾（D7）      [ ] 0/3
                  ──────
总计              [ ] 0/29
```

每做完一个，把对应行的 `[ ]` 改成 `[x]`，commit message `tickets: 完成 <id>`。

---

## 如果 AI 跑偏

| 跑偏类型 | 处理 |
|---|---|
| 改了 ticket 没明说的文件 | 拒收，重做 |
| 加了禁止的依赖 | 拒收，重做 |
| 测试不绿就交付 | 拒收，重做 |
| 答非所问 | 重新读 ticket，把验收标准粘一遍强调 |
| 顺手"优化"无关代码 | 让 AI 撤销 diff 中无关部分 |
| 同一个 bug 反复跑不对超过 90 分钟 | **跳过，晚上回头**，不要陪它死磕 |

## 如果某天提前做完

可以提前开下一天的 ticket。**但不要跨过当天的合并动作**：commit、push、rebase、跑测试这套必须做完才能开下一天。

## 如果某天做不完

按优先级砍：example > 测试覆盖 > assert DSL 高级特性 > AI 模板 > README 首屏 > **视频不能砍**。
