# C6 · AI 接入指南文档

## 任务

写 `docs/ai-integration.md`,告诉 Claude Code / Cursor / 其它 AI 工具的用户：怎么把 moxin 接入他们的 AI 工作流。

内容大纲：
1. **moxin 给 AI 提供什么** (run --output json、assert DSL、status 命令)
2. **AI 接入三种姿势**
   - 模式 A：让 AI 写 .assert 文件,用户跑 `moxin assert` 自动验证
   - 模式 B：AI 调用 `moxin run --output json` 直接看运行结果
   - 模式 C：AI 通过 `moxin status` 看实时状态
3. **示例：用 Claude Code 调试 blink demo** (含真实对话片段,从 C8 演练里抠出来)
4. **常见坑** (路径处理、Windows 路径分隔符、JSON 流缓冲)

## 允许动的文件

- 新增 `docs/ai-integration.md`
- 不动 src/

## 验收

```powershell
Test-Path docs/ai-integration.md
(Get-Content docs/ai-integration.md | Select-String "^## ").Count -ge 4
# 含至少一段真实 Claude/Cursor 对话示例
Get-Content docs/ai-integration.md | Select-String "claude|cursor" -CaseSensitive:$false
```

## 约束

- 不要写营销话术,写实用文档
- 示例对话要真实 (等 C8 演练完成后,把记录抠进来)
- 不超过 500 行

## commit message

`docs(C6): AI 接入指南`
