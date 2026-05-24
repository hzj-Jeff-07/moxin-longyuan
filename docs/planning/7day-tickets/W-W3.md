# W3 · Phase 2 backlog 整理（C 窗口领）

## 任务

把 7 天没做完的事 + 已经识别的下一阶段需求，整理成 GitHub issue 形式的 backlog。每个 issue 含：标题、背景、验收标准、预估工作量、依赖关系。

参考来源：
- `docs/component-schema.md` 第十节"未决问题"
- 7 天里 AI 跑偏后留下的 TODO 注释
- 现有 issue tracker 里 status=defer 的票
- D9 demo 视频脚本里"现在还做不到"的部分

预计 20-30 个 issue。

## 允许动的文件

- 新增 `docs/phase2-backlog.md`（汇总文档,含全部 issue 列表）
- 或直接用 `gh issue create` 创建 GitHub issue（推荐,加 milestone "Phase 2"）

## 验收

```powershell
# 方案 A: 本地文档
Test-Path docs/phase2-backlog.md
(Get-Content docs/phase2-backlog.md | Select-String "^## ").Count -ge 20

# 方案 B: GitHub issues
gh issue list --milestone "Phase 2" --limit 100 | Measure-Object -Line
```

每个 issue / 条目必须含：
- 标题（imperative，"Add X" 而不是 "X 应该加"）
- 背景 1-3 句
- 验收标准（checkbox 列表）
- 预估工作量（S/M/L/XL）
- 依赖 / 阻塞关系

## 约束

- 不动 source code
- 不动现有文档，只新增
- 不要把无关需求塞进来（"做 GUI"这种远期愿景不算 Phase 2）

## commit message

`docs(W3): Phase 2 backlog 整理`
