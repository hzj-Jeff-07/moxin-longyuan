# MoXin 开发部 · 7 天 AI 执行版方案

> 从 10 天弹药包压缩到 7 天 AI 多窗口执行版。
> 38 ticket → 29 ticket，按代码模块拆 3 个并行窗口。
> 维护者：龙渊 · 更新日期：2026-05-22

---

## 一、为什么是 3 窗口而不是 1 文件夹 1 窗口

**结论：按代码模块切，不按文件夹切。**

文件夹切法的问题：`bridge/moxin-simavr-bridge.c` 和 `src/lib.rs` 在 D3/D4/D6/D8 都会被反复改。如果两个窗口都对着同一个仿真桥写 C，merge 时手工解冲突的代价远大于并行收益。

按"模块责任"切，每个窗口的"主战场文件"几乎不重叠，merge 摩擦最小。

| 窗口 | 模块责任 | Ticket 集 | 主战场文件 |
|---|---|---|---|
| **A · 仿真核心** | bridge C 代码 + simavr 接入 + 元件 schema | A1-A7 | `bridge/*.c`、`src/sim/*.rs`、`components/*.toml` |
| **B · CLI/TUI/Assert** | 命令行 + TUI + assert DSL + 解释器 | B1-B7 | `src/tui.rs`、`src/assert.rs`(新)、`src/cmd_*.rs` |
| **C · 用户面** | examples + docs + 视频 + AI 模板 | C1-C9 | `examples/*`、`README.md`、`docs/ai-templates/*` |

**串行段**：D1（schema bundle 合并 + 5 个低优 bug）必须先于并行段做完，因为它阻塞建模部和后续所有 ticket 的 git 基线。

**收尾段**：D7 由三个窗口分头领走，互不冲突。

---

## 二、7 天时间表

| 日 | 模式 | A 窗口 | B 窗口 | C 窗口 |
|---|---|---|---|---|
| **D1** | 串行（一个窗口跑完，其它待命） | S1 → S2 → S3（schema 合并 + bug 修复 + STM32 wire 对齐） |||
| **D2** | 三并行 | A1 数字引脚 14 路 | B1 TUI 模式切换 | C1 multi-led example |
| **D3** | 三并行 | A2 蜂鸣器仿真 + A3 模拟引脚 ADC | B2 TUI 三连修 + B3 assert grammar | C2 buzzer-tone + C3 counter-7seg |
| **D4** | 三并行 | A4 7段数码管 + A5 电位器 | B4 assert parser + matcher | C4 pot-led + C5 button-led |
| **D5** | 三并行 | A6 被动元件 schema + A7 RunState 快照 | B5 assert CLI + e2e + B6 RFC IPC + status | C6 AI 接入指南 + C7 cursorrules 模板 |
| **D6** | **C 独占（A/B 暂停 commit）** | 待命，可本地试错不 push | 待命，可本地试错不 push | C8 buggy demo + AI session 演练 → C9 录视频 + 双字幕 |
| **D7** | 三并行收尾 | W1 README 首屏 + GitHub Release | W2 财经部素材包 | W3 Phase 2 backlog 整理 |

**Day 6 视频 solo 规则**：录屏期间任何 push 到 main 都会让视频里 `git log` 截图过期。A/B 当天可继续本地开发，但禁止 push。

---

## 三、给 AI 的固定开场白

每开一个新窗口、每贴一个 ticket 前，前面固定这一段：

```
你正在 MoXin (Rust + simavr) 项目里干活。先读这三份文件建立上下文：
1. docs/AI-CONTEXT.md
2. docs/CONVENTIONS.md
3. docs/component-schema.md（如果 ticket 涉及元件）

规则：
- 只动 ticket 授权的文件，不顺手优化无关代码
- 写完跑 `cargo test` 和 `cargo clippy --all-targets`,全绿再交付
- 给我 diff + 验收命令输出
- 遇到歧义停下问我,不要自由发挥
- commit message 格式:<scope>(<id>): <主题>

下面是 ticket:
[粘贴 ticket 全文]
```

---

## 四、git worktree 三窗口起手

```powershell
# 在 moxin-longyuan 仓库根目录
git worktree add ../moxin-A feat/sim-core
git worktree add ../moxin-B feat/cli-assert
git worktree add ../moxin-C feat/examples-docs

# 三个窗口分别 cd 进去
# A 窗口: cd ../moxin-A
# B 窗口: cd ../moxin-B
# C 窗口: cd ../moxin-C
```

每天早上每个窗口先：

```powershell
git fetch origin
git rebase origin/main         # 强制每日 rebase,避免分支偏离
cargo test                     # 起步基线
```

**铁律**：任何窗口超过 24 小时未 commit/push，必须先 stash + rebase + 重跑测试，再继续。

---

## 五、合并兼容性矩阵

| 文件 | 冲突风险 | 处理 |
|---|---|---|
| `src/assert.rs`（新建） | **零** | B 窗口独占，新文件不可能冲突 |
| `examples/*/`（新目录） | **零** | C 窗口独占，新目录不可能冲突 |
| `pin-anchors/*.json`（新文件） | **零** | A 窗口独占 |
| `components/*.toml`（新文件） | **零** | A 窗口独占（D1 之后） |
| `Cargo.toml` 依赖段 | 中 | 三窗口都可能加依赖。merge 时按字母顺序合并，再 `cargo update` |
| `src/lib.rs` 的 `pub mod` 行 | 中 | rebase 时按字母顺序保留所有 mod 声明 |
| `bridge/moxin-simavr-bridge.c` | **高** | A 窗口独占。B/C 不准动 |
| 根 `README.md` | **高** | D7 由 A 窗口领走（W1）。其它窗口和阶段不准动 |
| `docs/AI-CONTEXT.md`、`docs/CONVENTIONS.md` | **锁定** | 7 天内不允许任何窗口编辑。如需调整，三窗口暂停后串行改 |

---

## 六、五条纪律

1. **D1 串行不可越级**：S1-S3 没合到 main 之前，A/B/C 三个 worktree 不许动。
2. **每日强制 rebase**：早上第一件事是 `git fetch && git rebase origin/main`，不 rebase 不开工。
3. **高冲突文件先喊话**：要动 `bridge/*.c` 或根 `README.md` 之前，群里 / 笔记里说一声"我要锁 X 文件 30 分钟"。
4. **D6 视频 solo**：录屏开始前 A/B 各自本地保存进度但停止 push。视频出片后再恢复。
5. **AI 跑偏直接拒收**：改了未授权文件、跳过测试、加了禁止依赖、答非所问 —— 一律打回重做，不要心软。

---

## 七、保命优先级

如果 7 天里只能保一件事：**C9 录视频 + 双字幕**。投资人路演没视频，所有代码都白做。

如果只能保两件事：再加 **S1 schema bundle 合并**，因为它阻塞建模部，建模部停一周比开发部停一周代价大。

如果某天某窗口卡死：跳过当前 ticket 进入下一个，晚上回头复盘是不是 ticket 描述有歧义。**不要陪 AI 死磕同一个 bug 超过 90 分钟**。

---

## 八、文件清单

```
方案优化/
├── 7day-final-plan.md            ← 你正在读
├── 7day-INDEX.md                 ← 29 ticket 进度追踪
└── 7day-tickets/
    ├── S1.md ~ S3.md             ← D1 串行段
    ├── A-A1.md ~ A-A7.md         ← 仿真核心窗口
    ├── B-B1.md ~ B-B7.md         ← CLI/TUI/Assert 窗口
    ├── C-C1.md ~ C-C9.md         ← 用户面窗口
    └── W-W1.md ~ W-W3.md         ← D7 收尾段
```

Go.
