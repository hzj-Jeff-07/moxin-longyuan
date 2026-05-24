# W2 · 财经部素材包（B 窗口领）

## 任务

给财经部出一份"投资人路演 PPT 素材包"，包括：

1. **截图集**：6-10 张 TUI 运行时截图，覆盖 LED 闪烁、按钮按下、电位器调节、数码管计数、buggy demo 报错、AI 修好后的对比。
2. **架构图**：1 张项目架构图（仿真层 / 桥接层 / 解释器层 / CLI/TUI），SVG 或 Mermaid 源码。
3. **三句话定位**：给 BP 用，电梯 pitch 一句话、产品定位一段话、技术差异一段话。
4. **数据表**：feature 覆盖率、example 数量、commit 数、测试数、支持的板子数。

## 允许动的文件

- 新增 `docs/finance-assets/`
  - `screenshots/*.png`
  - `architecture.svg` 或 `architecture.mmd`
  - `pitch.md`
  - `metrics.md`
- 不动其它

## 验收

```powershell
# 6-10 张截图
(Get-ChildItem docs/finance-assets/screenshots -Filter *.png).Count -ge 6
# 架构图存在
Test-Path docs/finance-assets/architecture.*
# pitch.md 含三段
Get-Content docs/finance-assets/pitch.md | Select-String "^##" | Measure-Object
```

截图必须：分辨率 ≥ 1280x720、TUI 字符清晰、含时间戳或日志输出证明真实运行（不是 mock）。

## 约束

- 截图必须是真实运行截图，不允许 PS 后期合成
- pitch.md 不允许吹牛，未实现的 feature 标注 "Phase 2"
- 不动 source code

## commit message

`docs(W2): 财经部投资人路演素材包`
