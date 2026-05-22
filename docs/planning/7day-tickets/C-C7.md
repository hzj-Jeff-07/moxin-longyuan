# C7 · `.cursorrules` + `CLAUDE.md` 模板

## 任务

给用 moxin 的最终用户出两份模板,放在 `docs/ai-templates/`：

1. `cursorrules.template` — 用户复制到自己项目的 `.cursorrules`,让 Cursor 知道项目用了 moxin
2. `CLAUDE.md.template` — 用户复制到自己项目的 `CLAUDE.md`,让 Claude Code 知道项目用了 moxin

模板内容核心：
- moxin 是什么 (一句话)
- 项目结构惯例 (firmware/、moxin.toml)
- 用什么命令验证 (`moxin run`、`moxin assert`)
- AI 改固件前必读 (`moxin status` 看当前状态,改完跑 assert)
- 禁忌：不要修改 moxin.toml 的 wire 段除非用户明说

## 允许动的文件

- 新增 `docs/ai-templates/cursorrules.template`
- 新增 `docs/ai-templates/CLAUDE.md.template`
- 新增 `docs/ai-templates/README.md` (说明这俩怎么用)

## 验收

```powershell
Test-Path docs/ai-templates/cursorrules.template
Test-Path docs/ai-templates/CLAUDE.md.template
Test-Path docs/ai-templates/README.md
# 模板里有占位符 <PROJECT_NAME>,用户自己替换
Select-String -Path docs/ai-templates/*.template -Pattern "<PROJECT_NAME>"
```

## 约束

- **重要**：这俩是给用户的模板,**不是** moxin 仓库自己的 `.cursorrules`。不要 cp 到根。
- 长度控制：每份模板不超过 100 行
- 用占位符 `<PROJECT_NAME>`、`<BOARD>` 等,用户自己填

## commit message

`docs(C7): cursorrules 与 CLAUDE.md 模板`
