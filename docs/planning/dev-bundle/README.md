# MoXin 开发部 10 天弹药包

> 这是开发部 Phase 1（10 天版）完整的文档 + ticket 包。
> 把这一包合并进 repo，剩下 10 天就是按 ticket 单干。

## 包里有什么

```
dev-bundle/
├── README.md                          ← 你正在读
├── docs/
│   ├── AI-CONTEXT.md                  ← 项目地图，AI 写代码前必读
│   └── CONVENTIONS.md                 ← 编码约定，AI 写代码遵守
└── tickets/
    ├── INDEX.md                       ← 38 个 ticket 索引 + 用法
    ├── D1-2.md ~ D5-4.md              ← 18 个完整 prompt（地基 + 元件）
    └── D6-1.md ~ D10-5.md             ← 20 个简版 prompt（AI 接口 + 收尾）
```

加上**之前已经给过你**的：

- `moxin-schema-bundle-v1.0.zip`（元件 schema 给建模部用，D1-1 合并）
- `sprint-plan-10day.md`（10 天总方案，已合并到 repo docs/）
- `ticket-D1-1-prompt.md`（D1 第一个 ticket prompt）

加上前两个，**全套 Phase 1 弹药就齐了**。

## 怎么用

### 1. 合并这一包进 repo（5 分钟）

```bash
cd /path/to/moxin-longyuan
unzip dev-bundle.zip -d /tmp/

# AI 上下文文档（AI 每次写代码前会读）
cp /tmp/dev-bundle/docs/AI-CONTEXT.md docs/
cp /tmp/dev-bundle/docs/CONVENTIONS.md docs/

# Ticket 文件（每天看的）
cp -r /tmp/dev-bundle/tickets ./

git add docs/AI-CONTEXT.md docs/CONVENTIONS.md tickets/
git commit -m "docs: AI 上下文 + 编码约定 + 38 个 ticket prompt"
git push
```

### 2. 每天工作流程

**早上**：
```bash
# 1. 拉最新代码
git pull

# 2. 打开当天 ticket 索引
cat tickets/INDEX.md
# 看今天有哪些 ticket（D1 有 5 个，D7 有 4 个 ...）

# 3. 打开第一个 ticket
cat tickets/D1-2.md
```

**给 AI 干活**：

```
（在 Cursor / Claude / 任何 AI 工具）
请先读 docs/AI-CONTEXT.md 和 docs/CONVENTIONS.md，再读下面这份 ticket。
完成后给我 diff + 验收命令的输出。如果遇到歧义，停下问我，不要自由发挥。

[粘贴 ticket 全文]
```

**验收**：
- AI 给的 diff 是否只动了 ticket 允许的文件？
- `cargo test` 全绿？
- `cargo clippy --all-targets` 无新增警告？

**合并**：
```bash
# 按 CONVENTIONS.md §十一 的格式
git add <files>
git commit -m "<scope>(D<n>-<m>): <主题>"
git push
```

**晚上**：
- 录 gif / 写 5 行日记 / 发群里
- 在 INDEX.md 把对应 ticket 的 ☐ 改成 ✓

### 3. 如果你的 AI 工具是 Cursor 或 Claude Code

复制 `docs/AI-CONTEXT.md` 里关键段到 `.cursorrules` 或 `CLAUDE.md`，让工具自动读。
**注意**：MoXin 项目自己的 `.cursorrules` 不要跟 D8-3 给用户的模板搞混了：
- `.cursorrules`（MoXin 仓库根）= AI 写 moxin 自身代码时用
- `docs/ai-templates/cursorrules.template` = 给用 moxin 的最终用户用

## 完整 Phase 1 文件清单（所有产出）

```
moxin-longyuan/
├── docs/
│   ├── AI-CONTEXT.md                  ← 本包
│   ├── CONVENTIONS.md                 ← 本包
│   ├── component-schema.md            ← schema bundle (之前)
│   ├── sprint-plan.md                 ← 10 天方案 (之前)
│   ├── ai-integration.md              ← D8-2 产出
│   ├── ai-templates/
│   │   ├── cursorrules.template       ← D8-3 产出
│   │   └── CLAUDE.md.template         ← D8-3 产出
│   ├── design/
│   │   ├── runtime-query.md           ← D6-1 产出
│   │   ├── assert-dsl.md              ← D7-1 产出
│   │   └── bridge-protocol.md         ← 已有
│   ├── demo/
│   │   ├── buggy-blink/               ← D9-1 产出
│   │   ├── session-log.md             ← D9-2 产出
│   │   ├── video-zh.mp4               ← D9-3 产出
│   │   ├── video-en.mp4               ← D9-3/D9-4 产出
│   │   └── subtitles-zh.srt           ← D9-4 产出
│   └── finance-assets/                ← D10-3 产出
├── components/                         ← schema bundle (之前)
├── pin-anchors-template/               ← schema bundle (之前)
├── pin-anchors/                        ← 建模部填
├── tickets/                            ← 本包
├── src/
│   ├── ... (D1-D8 改动)
│   ├── assert.rs                      ← D7-2 新增
│   ├── cmd_status.rs                  ← D6-3 新增
│   └── cmd_assert.rs                  ← D7-3 新增
├── bridge/
│   └── moxin-simavr-bridge.c          ← D3-1 / D4-1 改
├── examples/
│   ├── multi-led/                     ← D3-2
│   ├── buzzer-tone/                   ← D3-4
│   ├── counter-7seg/                  ← D4-3
│   ├── pot-led-brightness/            ← D5-3
│   ├── button-led/                    ← D5-4
│   └── led-control/                   ← 已有
└── tests/
    └── assert_e2e.rs                  ← D7-4 新增
```

## 给自己的最后提醒

**你不需要把所有 ticket 完美执行**。10 天高强度，60% 顺利做完已经是 phenomenal。

**ticket 文件不是法律**。AI 跑出来发现某个步骤可以更简单 / 验收命令需要调整，
直接改 ticket，commit 一句 `tickets: D5-1 调整为...`，然后继续。
计划是为了对齐预期，不是为了被遵守。

**最重要的是 D9-3 视频**。如果 10 天里只能保一件事，保它——投资人路演没视频，
所有代码都白做。

**遇到不会的不要硬刚 AI**。如果某个 ticket AI 反复跑不对，直接问我，
我看实际报错给具体解决方案。

## 最后

10 天，从现在开始。

D11 早上你应该能：
- ✓ 任何人 cargo install 你的 repo 跑 blink demo
- ✓ Cursor / Claude Code 用户能照 docs 接入
- ✓ 一段 demo 视频在网上能搜到
- ✓ 财经部已经在用你的截图做 PPT

剩下的是 Phase 2 的事，那是 D11 之后再说。

Go.
