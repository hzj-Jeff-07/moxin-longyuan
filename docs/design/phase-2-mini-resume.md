# Phase 2-mini 进度恢复手册

> 任何时候打开仓库,先读这一页 → 30 秒进入状态。

---

## 我现在在做什么

**Phase 2-mini** = 把"D13-only 真仿真"扩展为"全 Arduino Uno 数字引脚 + 数码管真驱动"。

**目标版本**:v0.4.0
**工作分支**:`phase-2-mini`
**备份分支**:`v0.3.0-stable`(炸了能回这里)
**详细方案**:`docs/design/phase-2-mini-rfc.md`

---

## 30 秒恢复指令

```bash
git fetch origin
git checkout phase-2-mini
git pull
git log --oneline main..HEAD          # 看本分支做了几步
cat docs/design/phase-2-mini-rfc.md    # 看勾选进度("六、实施步骤")
cargo test && cargo clippy --all-targets -- -D warnings
```

进度看 RFC 第六节的勾选框。下一步 = 第一个未勾选的。

---

## 当前进度快照

> ⚠️ 每次推进一个 Step,**手动更新这一节** + RFC 勾选框。

- **当前位置**:Step 1 编码完成,Step 2(Rust PinStates)未开始
- **最后一个 commit**(本分支):`b5acebf feat(bridge): hook all PORTB/C/D GPIO pins`
- **最后一次 `cargo test`**:94 passed / 0 failed(2026-05-26 phase-2-mini 分支)
- **最后一次 `cargo clippy --all-targets -- -D warnings`**:0 警告
- **CI 状态**:phase-2-mini 已 push,合并时再触发 release pipeline 完整验证
- **未解决问题**:Windows 本地无法编译 bridge(无 make/gcc),所有 bridge 真编译验证由 Linux CI 兜底

---

## 找回所有关键文件

| 文件 | 作用 |
|---|---|
| `docs/design/phase-2-mini-rfc.md` | 完整方案 + 7 步实施清单 + 勾选进度 |
| `docs/design/phase-2-mini-resume.md` | 本文件,快速恢复 |
| `docs/design/bridge-protocol.md` | bridge JSON 协议(改 bridge 前先看) |
| `docs/design/cli-vision.md` | CLI 长期愿景 |
| `CLAUDE.md` | 全局禁区,改之前必看 |
| `bridge/moxin-simavr-bridge.c` | B1 改这里 |
| `bridge/Makefile` | 已支持 Linux + macOS |
| `src/sim.rs` | B1 加 `PinStates` |
| `src/render.rs` | B1/B4 改 LED + 数码管渲染 |
| `examples/` | B6 加 4 个新例子 |

---

## 翻车回滚清单(从轻到重)

```bash
# 1. 单 commit 翻车
git revert <sha>

# 2. 整步翻车,回到分支起点(b44b7aa = main)
git reset --hard origin/main

# 3. 整个 phase-2-mini 不要了
git checkout main
git branch -D phase-2-mini
git push origin --delete phase-2-mini

# 4. 终极保险(需用户授权)
git checkout main
git reset --hard v0.3.0-stable
# 然后 force push 需要授权
```

---

## 跨会话上下文(给下一次 Claude 看)

如果你是新的 Claude session,要继续 phase-2-mini:

1. **先读 `~/.claude/CLAUDE.md`** — 用户身份、纪律、禁区
2. **读 `CLAUDE.md`(项目根)** — 项目锁定范围
3. **读本文件** — 知道现在做到哪
4. **读 `docs/design/phase-2-mini-rfc.md`** — 知道整个方案
5. **跑 `git status && git log --oneline main..HEAD`** — 看实际进度
6. **跑 `cargo test`** — 确认基线没坏
7. **再动手**

**核心硬规矩(永远不破)**:
- 不主动 push tag,所有 tag 操作必须用户逐次授权
- 不动 `LICENSE`、`SCHEMA_VERSION`(除非 RFC 里同意升)
- 不 `git push --force` 到 main
- 改 `bridge/*.c` 前提示用户(本 RFC 内已默认同意,推进 Step 1 时再确认一次)

---

## 进度日志(按时间倒序追加)

> 每次有实质推进,在这里加一行。

- **2026-05-26** — Step 1 编码完成:bridge 全 PORTB/C/D hook(commit b5acebf)。cargo test 94 过、clippy 0 警告。bridge 编译验证延后到合并时由 release pipeline 兜底。
- **2026-05-26** — RFC + 备份分支 + 工作分支建好,Step 0 完成,Step 1 待启动。
