//! AI Inspector 的 LLM 接入层 —— shell-out 到 `curl`,**不引 HTTP crate**。
//!
//! 权威设计:`docs/design/v3.2-ai-inspector-rfc.md`。
//! 原则(cli-vision §3):MoXin 不内置模型,只把结构化硬件状态喂给用户配置的外部
//! LLM,渲染回答。密钥只从 env 读,**不进 argv**(经 `curl -K` 配置文件传递,
//! 文件 0600 且用后即删)、不落盘长存、不入库、不进日志。
//!
//! M1 只提供纯函数 + shell-out + 探针,默认关闭(未设 `MOXIN_LLM_API_KEY` 时
//! 全链路不触发)。`moxin explain` (M3) 是第一个用户可见入口;TUI 实时面板留 M2。

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::project::Project;

const MAX_TOKENS: u32 = 512;
const CURL_TIMEOUT_SECS: u32 = 20;

/// LLM 端点方言。决定鉴权头、默认 URL/模型、以及响应解析路径。
/// 请求体两家结构一致(`{model, max_tokens, messages:[{role, content}]}`),不必分。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// Anthropic Messages API(`x-api-key` + `anthropic-version`,回答在 `content[].text`)
    Anthropic,
    /// OpenAI 兼容 Chat Completions(`Authorization: Bearer`,回答在 `choices[0].message.content`)。
    /// 覆盖 OpenAI / OpenRouter / Azure / 本地 llama.cpp server 等。
    OpenAi,
}

impl Dialect {
    fn parse(s: &str) -> Dialect {
        match s.trim().to_lowercase().as_str() {
            "openai" | "openai-compatible" | "chatgpt" | "gpt" | "openrouter" => Dialect::OpenAi,
            _ => Dialect::Anthropic,
        }
    }

    fn default_url(self) -> &'static str {
        match self {
            Dialect::Anthropic => "https://api.anthropic.com/v1/messages",
            Dialect::OpenAi => "https://api.openai.com/v1/chat/completions",
        }
    }

    fn default_model(self) -> &'static str {
        match self {
            Dialect::Anthropic => "claude-haiku-4-5",
            Dialect::OpenAi => "gpt-4o-mini",
        }
    }
}

/// LLM 配置,全部来自环境变量(密钥不进 moxin.toml,避免误提交)。
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub dialect: Dialect,
}

impl LlmConfig {
    /// 从 env getter 构造。测试注入用(不用 `env::set_var`,守测试约定:并行下与
    /// `getenv` 竞态)。空字符串视作未设。URL/模型未指定时按方言取默认。
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Self {
        let non_empty = |k: &str| get(k).filter(|v| !v.trim().is_empty());
        let dialect = non_empty("MOXIN_LLM_DIALECT")
            .map(|d| Dialect::parse(&d))
            .unwrap_or(Dialect::Anthropic);
        LlmConfig {
            url: non_empty("MOXIN_LLM_URL").unwrap_or_else(|| dialect.default_url().to_string()),
            model: non_empty("MOXIN_LLM_MODEL")
                .unwrap_or_else(|| dialect.default_model().to_string()),
            api_key: non_empty("MOXIN_LLM_API_KEY"),
            dialect,
        }
    }

    pub fn from_process_env() -> Self {
        Self::from_env(|k| std::env::var(k).ok())
    }

    /// 未设密钥 → LLM 功能默认关闭(行为与不接 LLM 时完全一致)。
    pub fn is_enabled(&self) -> bool {
        self.api_key.is_some()
    }
}

/// 把项目 + 状态快照拼成给 LLM 的 prompt。**只含硬件仿真状态,无密钥、无源码**。
///
/// `snapshot` 是 `RunState::to_json` 的产物(实时)或 `.moxin-state.json`(落盘),
/// 两条路径共用,所以这里收 `Value` 而非 `&RunState`。
pub fn build_prompt(project: &Project, snapshot: &Value) -> String {
    let snapshot_str = serde_json::to_string_pretty(snapshot).unwrap_or_default();
    let comps: Vec<String> = project
        .components
        .iter()
        .map(|c| format!("{} ({})", c.id, c.kind))
        .collect();
    let comps = if comps.is_empty() {
        "none".to_string()
    } else {
        comps.join(", ")
    };
    format!(
        "You are an embedded-systems assistant reading a simulated MCU's live state.\n\
         Board: {board}. Components: {comps}.\n\
         Current simulator state (JSON):\n{snapshot}\n\n\
         In 3-4 short lines, explain what the firmware appears to be doing right now and \
         flag any anomaly (a stuck pin, no serial output, an unexpected sensor value). \
         Be concise and concrete. Do not repeat the raw JSON back.",
        board = project.project.board,
        comps = comps,
        snapshot = snapshot_str,
    )
}

/// 构造请求体。Anthropic Messages 与 OpenAI Chat Completions 的 body 结构一致
/// (`{model, max_tokens, messages:[{role, content}]}`),故不分方言。
pub fn build_request_body(model: &str, prompt: &str) -> Value {
    json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "messages": [{ "role": "user", "content": prompt }],
    })
}

/// 按方言从响应抽出文本。错误响应(`{"error":{"message":...}}`,两家通用)→ `Err`。
/// - Anthropic:`content[].text` 拼接
/// - OpenAI:`choices[0].message.content`
pub fn parse_answer(dialect: Dialect, resp: &str) -> Result<String> {
    let v: Value = serde_json::from_str(resp).context("LLM 响应不是合法 JSON")?;
    if let Some(err) = v.get("error") {
        let msg = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        bail!("LLM API error: {}", msg);
    }
    let text = match dialect {
        Dialect::Anthropic => {
            let content = v
                .get("content")
                .and_then(|c| c.as_array())
                .ok_or_else(|| anyhow!("Anthropic 响应缺少 content 数组"))?;
            content
                .iter()
                .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        }
        Dialect::OpenAi => v
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| anyhow!("OpenAI 响应缺少 choices[0].message.content"))?
            .to_string(),
    };
    if text.trim().is_empty() {
        bail!("LLM 响应里没有文本内容");
    }
    Ok(text.trim().to_string())
}

/// 探针:`curl` 是否可用(同 simavr/qemu,缺了给提示不崩)。
pub fn curl_available() -> bool {
    Command::new("curl")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// shell-out 到 `curl` 调 LLM。密钥经 `-K` 配置文件传(不进 argv),请求体走 stdin。
pub fn call_llm(cfg: &LlmConfig, body: &Value) -> Result<String> {
    let api_key = cfg.api_key.as_deref().ok_or_else(|| {
        anyhow!("MOXIN_LLM_API_KEY 未设置 — AI Inspector 的 LLM 功能默认关闭")
    })?;
    if !curl_available() {
        bail!("curl 未找到 — LLM 调用需要 curl(安装 curl 后重试)");
    }
    let body_str = serde_json::to_string(body)?;

    // headers(含密钥)写进 -K 配置文件,不落 argv;文件 0600、用后即删。
    let cfg_path = write_temp_config(&render_curl_config(cfg, api_key))?;
    let result = run_curl(&cfg_path, &body_str);
    let _ = std::fs::remove_file(&cfg_path); // 无论成败都删,忽略删除错误
    let output = result?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let code = output.status.code().unwrap_or(-1);
        bail!("curl 退出码 {}: {}", code, err.trim());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_answer(cfg.dialect, &stdout)
}

/// 转义 curl 配置文件里的双引号字符串值(`\` 与 `"`)。
fn curl_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn render_curl_config(cfg: &LlmConfig, api_key: &str) -> String {
    let mut lines = vec![
        format!("url = \"{}\"", curl_escape(&cfg.url)),
        "request = \"POST\"".to_string(),
        "header = \"content-type: application/json\"".to_string(),
    ];
    // 鉴权头按方言:Anthropic 用 x-api-key + anthropic-version;OpenAI 用 Bearer
    match cfg.dialect {
        Dialect::Anthropic => {
            lines.push("header = \"anthropic-version: 2023-06-01\"".to_string());
            lines.push(format!("header = \"x-api-key: {}\"", curl_escape(api_key)));
        }
        Dialect::OpenAi => {
            lines.push(format!(
                "header = \"authorization: Bearer {}\"",
                curl_escape(api_key)
            ));
        }
    }
    lines.push(format!("max-time = {}", CURL_TIMEOUT_SECS));
    lines.push("silent".to_string());
    lines.push("show-error".to_string());
    lines.push("data-binary = \"@-\"".to_string());
    lines.push(String::new());
    lines.join("\n")
}

/// 写 curl 配置到临时文件。unix 上以 0600 原子创建(`create_new` + mode),
/// 密钥不经历"默认 umask 权限"的窗口。
#[cfg(unix)]
fn write_temp_config(content: &str) -> Result<PathBuf> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let path = temp_config_path();
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .with_context(|| format!("创建 curl 配置文件 {}", path.display()))?;
    f.write_all(content.as_bytes())
        .with_context(|| format!("写 curl 配置 {}", path.display()))?;
    Ok(path)
}

#[cfg(not(unix))]
fn write_temp_config(content: &str) -> Result<PathBuf> {
    let path = temp_config_path();
    std::fs::write(&path, content)
        .with_context(|| format!("写 curl 配置 {}", path.display()))?;
    Ok(path)
}

fn temp_config_path() -> PathBuf {
    let unique = format!(
        "moxin-llm-{}-{}.curl",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    std::env::temp_dir().join(unique)
}

fn run_curl(cfg_path: &Path, body: &str) -> Result<std::process::Output> {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new("curl")
        .arg("-K")
        .arg(cfg_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("启动 curl 失败")?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(body.as_bytes())
            .context("写请求体到 curl stdin")?;
    }
    child.wait_with_output().context("等待 curl 结束")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Project, ProjectMeta};

    fn empty_project(board: &str) -> Project {
        Project {
            project: ProjectMeta {
                name: "t".into(),
                board: board.into(),
                version: "0.2".into(),
            },
            components: vec![],
            wires: vec![],
            code: None,
        }
    }

    #[test]
    fn config_defaults_anthropic_when_unset() {
        let cfg = LlmConfig::from_env(|_| None);
        assert_eq!(cfg.dialect, Dialect::Anthropic);
        assert_eq!(cfg.url, "https://api.anthropic.com/v1/messages");
        assert_eq!(cfg.model, "claude-haiku-4-5");
        assert!(cfg.api_key.is_none());
        assert!(!cfg.is_enabled());
    }

    #[test]
    fn config_openai_dialect_flips_defaults() {
        let cfg = LlmConfig::from_env(|k| match k {
            "MOXIN_LLM_DIALECT" => Some("openai".into()),
            "MOXIN_LLM_API_KEY" => Some("sk-o".into()),
            _ => None,
        });
        assert_eq!(cfg.dialect, Dialect::OpenAi);
        assert_eq!(cfg.url, "https://api.openai.com/v1/chat/completions");
        assert_eq!(cfg.model, "gpt-4o-mini");
        assert!(cfg.is_enabled());
    }

    #[test]
    fn config_reads_overrides_and_ignores_blank() {
        let cfg = LlmConfig::from_env(|k| match k {
            "MOXIN_LLM_URL" => Some("https://x/y".into()),
            "MOXIN_LLM_MODEL" => Some("claude-opus-4-8".into()),
            "MOXIN_LLM_API_KEY" => Some("sk-secret".into()),
            "MOXIN_LLM_DIALECT" => Some("   ".into()), // 空白 → 回退默认(anthropic)
            _ => None,
        });
        assert_eq!(cfg.url, "https://x/y");
        assert_eq!(cfg.model, "claude-opus-4-8");
        assert_eq!(cfg.api_key.as_deref(), Some("sk-secret"));
        assert_eq!(cfg.dialect, Dialect::Anthropic); // 空白被忽略
        assert!(cfg.is_enabled());
    }

    #[test]
    fn prompt_includes_board_and_snapshot_fields() {
        let p = empty_project("arduino-uno");
        let snap = json!({"ready": true, "pin_states": {"B:5": 1}});
        let prompt = build_prompt(&p, &snap);
        assert!(prompt.contains("arduino-uno"), "prompt: {prompt}");
        assert!(prompt.contains("B:5"), "prompt should embed snapshot");
        assert!(prompt.contains("Components: none"));
    }

    #[test]
    fn request_body_is_messages_shape() {
        let body = build_request_body("claude-haiku-4-5", "hi");
        assert_eq!(body["model"], "claude-haiku-4-5");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "hi");
        assert!(body["max_tokens"].is_number());
    }

    #[test]
    fn parse_answer_anthropic_extracts_text_blocks() {
        let resp = r#"{"content":[{"type":"text","text":"D13 is toggling"},{"type":"text","text":" at 1Hz."}]}"#;
        assert_eq!(
            parse_answer(Dialect::Anthropic, resp).unwrap(),
            "D13 is toggling at 1Hz."
        );
    }

    #[test]
    fn parse_answer_openai_extracts_message_content() {
        let resp = r#"{"choices":[{"message":{"role":"assistant","content":"D13 blinks; all nominal."}}]}"#;
        assert_eq!(
            parse_answer(Dialect::OpenAi, resp).unwrap(),
            "D13 blinks; all nominal."
        );
    }

    #[test]
    fn parse_answer_surfaces_api_error_both_dialects() {
        let resp = r#"{"error":{"type":"authentication_error","message":"invalid key"}}"#;
        for d in [Dialect::Anthropic, Dialect::OpenAi] {
            let e = parse_answer(d, resp).unwrap_err().to_string();
            assert!(e.contains("invalid key"), "got: {e}");
        }
    }

    #[test]
    fn parse_answer_wrong_dialect_shape_errors() {
        // OpenAI 响应喂给 Anthropic 解析(反之亦然)应报错,不 panic
        let openai = r#"{"choices":[{"message":{"content":"hi"}}]}"#;
        assert!(parse_answer(Dialect::Anthropic, openai).is_err());
        let anthropic = r#"{"content":[{"type":"text","text":"hi"}]}"#;
        assert!(parse_answer(Dialect::OpenAi, anthropic).is_err());
    }

    #[test]
    fn parse_answer_rejects_garbage_and_empty() {
        assert!(parse_answer(Dialect::Anthropic, "not json").is_err());
        assert!(parse_answer(Dialect::Anthropic, r#"{"content":[]}"#).is_err());
        assert!(parse_answer(Dialect::Anthropic, r#"{"content":[{"type":"text","text":"  "}]}"#).is_err());
    }

    #[test]
    fn curl_config_anthropic_headers_and_escape() {
        let cfg = LlmConfig::from_env(|k| match k {
            "MOXIN_LLM_API_KEY" => Some("sk-\"quoted\"".into()),
            _ => None,
        });
        let rendered = render_curl_config(&cfg, cfg.api_key.as_deref().unwrap());
        assert!(rendered.contains("data-binary = \"@-\""));
        assert!(rendered.contains("anthropic-version"));
        assert!(rendered.contains("x-api-key: sk-\\\"quoted\\\""), "escaped: {rendered}");
        assert!(rendered.contains("api.anthropic.com"));
    }

    #[test]
    fn curl_config_openai_uses_bearer_no_anthropic_header() {
        let cfg = LlmConfig::from_env(|k| match k {
            "MOXIN_LLM_DIALECT" => Some("openai".into()),
            "MOXIN_LLM_API_KEY" => Some("sk-o".into()),
            _ => None,
        });
        let rendered = render_curl_config(&cfg, cfg.api_key.as_deref().unwrap());
        assert!(rendered.contains("authorization: Bearer sk-o"), "got: {rendered}");
        assert!(!rendered.contains("anthropic-version"), "openai 不应带 anthropic 头");
        assert!(!rendered.contains("x-api-key"));
    }

    #[test]
    fn call_llm_without_key_errors_before_curl() {
        let cfg = LlmConfig::from_env(|_| None);
        let e = call_llm(&cfg, &json!({})).unwrap_err().to_string();
        assert!(e.contains("MOXIN_LLM_API_KEY"), "got: {e}");
    }
}
