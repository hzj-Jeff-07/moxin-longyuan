# MoXin CLI v3.4.0 — AI Inspector 多方言(OpenAI 兼容端点)

> 发布日期:2026-07-13
> 权威设计:`docs/design/v3.2-ai-inspector-rfc.md`

---

## 一句话总结

**v3.4.0 让 AI Inspector 不再绑死 Anthropic。** `MOXIN_LLM_DIALECT=openai` 一键切到
OpenAI 兼容 Chat Completions 端点——OpenAI / OpenRouter / Azure OpenAI / 本地
llama.cpp server 等都能用。仍是 shell-out curl,**零新依赖**。

---

## 亮点

### `MOXIN_LLM_DIALECT` 一键换供应商

```bash
# Anthropic(默认,不设即可)
export MOXIN_LLM_API_KEY=sk-ant-...
moxin explain

# OpenAI 兼容(OpenAI / OpenRouter / 本地 server …)
export MOXIN_LLM_DIALECT=openai
export MOXIN_LLM_API_KEY=sk-...
# 换 OpenRouter / 本地:再设 MOXIN_LLM_URL + MOXIN_LLM_MODEL
moxin explain
```

`Dialect` enum 驱动一切差异:

| | Anthropic(默认) | OpenAI |
|---|---|---|
| 默认 URL | `api.anthropic.com/v1/messages` | `api.openai.com/v1/chat/completions` |
| 默认模型 | `claude-haiku-4-5` | `gpt-4o-mini` |
| 鉴权头 | `x-api-key` + `anthropic-version` | `Authorization: Bearer` |
| 响应解析 | `content[].text` | `choices[0].message.content` |

请求体两家结构一致(`{model, max_tokens, messages:[{role, content}]}`),不必分。
`MOXIN_LLM_URL` / `MOXIN_LLM_MODEL` 仍可覆盖端点/模型。

### 清理:去掉半吊子的 `MOXIN_LLM_KEY_HEADER`

v3.2/v3.3 里的 `MOXIN_LLM_KEY_HEADER` 只能换鉴权头名、换不了响应解析,对 OpenAI 其实
跑不通。v3.4 用语义清晰的 `MOXIN_LLM_DIALECT` 取代它,一个开关切齐鉴权 + 解析 + 默认值。
(v3.2/v3.3 尚未 tag 发布,无破坏面。)

## 安全 / 默认关闭(不变)

- 密钥经 `curl -K` 配置文件传(**不进 argv**、0600、用后即删),OpenAI 走 `Bearer` 同样如此
- 未设 `MOXIN_LLM_API_KEY` → 全链路不触发,行为零变化
- LLM 只读建议,不驱动仿真

## 质量线

- `cargo test` 232 通过(v3.3.0:228),新增方言解析/鉴权头/端到端单测
- clippy 0 警告;CI explain 假 curl 关卡**兼验 Anthropic + OpenAI 两种响应形态**

## 后续

`explain` 与 TUI `Ctrl+E` 两条路径共用同一套多方言逻辑。更细的 provider 适配
(如 Gemini 的独特 body)视需求再加。
