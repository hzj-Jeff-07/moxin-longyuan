# MoXin CLI v3.3.0 — AI Inspector 实时面板(TUI `Ctrl+E`)

> 发布日期:2026-07-13
> 主线代号:**AI Inspector · M2(实时面板)**
> 权威设计:`docs/design/v3.2-ai-inspector-rfc.md`

---

## 一句话总结

**v3.3.0 让 AI Inspector 在 TUI 里活起来。** 四面板界面跑仿真时按 `Ctrl+E`,
AI Inspector 面板直接调 LLM 解读当前 MCU 状态——后台线程跑,**不卡界面**。
这补上 v3.2 RFC 的 M2,AI Inspector 三里程碑(M1 基础 + M3 `explain` + M2 实时面板)全部完成。

---

## 亮点

### TUI 里 `Ctrl+E` 实时问 LLM

```
moxin shell → run   # 进四面板 TUI
# 按 Ctrl+E:AI Inspector 面板显示 analyzing… → LLM 对当前状态的分析
```

- **非阻塞**:`Ctrl+E` 触发后台 worker 线程跑 curl,渲染循环照常 33ms 刷新,界面不冻。
  结果经 `Arc<Mutex<LlmPanel>>` 回传,下一帧自动显示。
- **状态机**:`Disabled`(未设 key)/ `Idle`(Ctrl+E to ask)/ `Pending`(analyzing…)/
  `Ready`(分析结果,Ctrl+E 可刷新)/ `Error`(curl 缺失 / 网络 / API 报错,Ctrl+E 重试)。
- **降级**:LLM 出错不影响面板上半部的派生状态(电压/GPIO/Loop/Sensors),照常显示。
- 复用 v3.2 的 `src/llm.rs`(`build_prompt`/`build_request_body`/`parse_answer`/`call_llm`),
  快照走 `RunState::to_json`,与 `moxin explain` 同一套 shell-out curl,**不引任何新依赖**。

### 安全 / 默认关闭(同 v3.2)

- 未设 `MOXIN_LLM_API_KEY` → 面板只显示一行 "LLM: off",`Ctrl+E` 给提示、不发请求
- 密钥经 `curl -K` 配置文件传(不进 argv、0600、用后即删),prompt 只含硬件状态
- LLM 只读建议,不驱动仿真

## 质量线

- `cargo test` 228 通过(v3.2.0:224),新增 4 条 TUI 面板渲染单测
- clippy 0 警告;CI 十一道真机/协议关卡(九外设 + MCP e2e + explain 假 curl)不变

## 后续

AI Inspector 至此完整。请求体仍是 Anthropic Messages 形态;OpenAI 兼容端点靠
`MOXIN_LLM_KEY_HEADER` 切鉴权头,完整 body 适配视需求再加。
