# MoXin CLI v3.2.0 — AI Inspector 接真 LLM(`moxin explain`)

> 发布日期:2026-07-13
> 主线代号:**AI Inspector · LLM**
> 权威设计:`docs/design/v3.2-ai-inspector-rfc.md`

---

## 一句话总结

**v3.2.0 让 MoXin 第一次真正"用 AI 读硬件"。** `moxin explain` 把全外设状态快照喂给
外部 LLM,打印模型对"固件此刻在干什么、有没有异常"的分析。MoXin **不内置模型**——
shell-out 到 `curl` 调你自己配置的端点,**不引任何 HTTP crate**,守死依赖锁。

---

## 亮点

### 1. `moxin explain` —— 一次性 AI 状态解读

```bash
export MOXIN_LLM_API_KEY=sk-...
moxin run --output json &     # 落一份状态快照
moxin explain                 # LLM 分析当前 MCU 状态
```

读 `build/.moxin-state.json`(全外设:GPIO/PWM/ADC/DHT/超声波/IR/LCD/OLED/串口尾)
→ 拼 prompt → curl 调 LLM → 打印分析。

### 2. shell-out curl,零新依赖

和调 simavr/qemu/arduino-cli 一个哲学:MoXin 只编排,外部运行时干活。请求体拼装 +
响应解析全用现有 `serde_json`,**不引 reqwest/ureq/任何 LLM SDK**。`curl` 成为又一个
按需外部依赖,`moxin doctor` 一并自检。

### 3. 默认关闭 + 密钥安全

- 未设 `MOXIN_LLM_API_KEY` → `explain` 只给启用指引、不发请求,全项目行为零变化
- 密钥经 `curl -K` 配置文件传递:**不进 argv**(`ps` 看不到)、文件 0600、用后即删
- 密钥不落盘长存、不入库、不进日志;prompt 里只有硬件状态,无源码外泄
- `moxin doctor` 只报密钥"是否设置",绝不打印值

### 配置(全走环境变量)

| 变量 | 默认 |
|---|---|
| `MOXIN_LLM_API_KEY` | (无 → 功能关闭) |
| `MOXIN_LLM_URL` | `https://api.anthropic.com/v1/messages` |
| `MOXIN_LLM_MODEL` | `claude-haiku-4-5` |
| `MOXIN_LLM_KEY_HEADER` | `x-api-key`(切 OpenAI 兼容端点改 `Authorization`) |

## 质量线

- `cargo test` 224 通过(v3.1.0:215),新增 9 条 llm 单测(config/prompt/body/parse/escape)
- clippy 0 警告
- CI verify 新增 **AI Inspector explain 假 curl shell-out 关卡**(不打真 API、不进真密钥,
  验证"snapshot → prompt → curl → parse → 打印"闭环 + 未设 key 默认关闭)

## 已知限制 / 后续

- **M2 留后续**:TUI 面板里的实时 LLM 解读(边跑边问、后台线程非阻塞、缓存降级)是下一版
- 请求体目前是 Anthropic Messages 形态;OpenAI 兼容端点靠 `MOXIN_LLM_KEY_HEADER` 切
  鉴权头已能打通多数网关,完整 body 适配留后续
- LLM 为**只读建议**,不驱动仿真(驱动仿真是 v3.0 MCP server 的职责)
