//! `moxin explain` —— 一次性 AI Inspector:把最近一次 `run --output json` 落盘的
//! 全外设状态快照喂给外部 LLM,打印模型对"固件此刻在干什么 / 有无异常"的分析。
//!
//! 这是 AI Inspector 接真 LLM 的第一个用户入口(v3.2 M3);TUI 实时面板留 M2。
//! 默认关闭:未设 `MOXIN_LLM_API_KEY` 时直接给出启用指引,不发任何请求。

use anyhow::{bail, Context, Result};
use std::process::ExitCode;

use crate::llm::{self, LlmConfig};
use crate::project::Project;

pub fn cmd_explain() -> Result<ExitCode> {
    let cwd = std::env::current_dir()?;
    let root = Project::find_project_root(&cwd)?;
    let project = Project::load(&root.join("moxin.toml"))?;

    let cfg = LlmConfig::from_process_env();
    if !cfg.is_enabled() {
        // 默认关闭:给指引而不是报错退栈(exit 0,便于脚本探测)
        println!("AI Inspector (LLM) 未启用。");
        println!("设置 MOXIN_LLM_API_KEY 后重试,例如:");
        println!("  export MOXIN_LLM_API_KEY=sk-...   # 你的 Anthropic / 兼容端点密钥");
        println!("  moxin explain");
        println!("配置项与安全说明见 docs/design/v3.2-ai-inspector-rfc.md。");
        return Ok(ExitCode::SUCCESS);
    }

    let state_path = root.join("build").join(".moxin-state.json");
    let snapshot_str = std::fs::read_to_string(&state_path).with_context(|| {
        format!(
            "读不到状态快照 {} — 先跑 `moxin run --output json` 生成",
            state_path.display()
        )
    })?;
    let snapshot: serde_json::Value =
        serde_json::from_str(&snapshot_str).context("状态快照不是合法 JSON")?;

    let prompt = llm::build_prompt(&project, &snapshot);
    let body = llm::build_request_body(&cfg.model, &prompt);
    let answer = llm::call_llm(&cfg, &body)?;

    if answer.is_empty() {
        bail!("LLM 返回空分析");
    }
    println!("{}", answer);
    Ok(ExitCode::SUCCESS)
}
