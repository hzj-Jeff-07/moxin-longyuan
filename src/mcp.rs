//! MCP(Model Context Protocol)server —— 让 AI Agent 直接把 MoXin 当 tool 调用。
//!
//! v3 M1:JSON-RPC 2.0 over stdio(每行一条消息),手写解析不引 SDK
//! (守 CLAUDE.md 依赖锁;协议本身就是 JSON-RPC,serde_json 够用)。
//! 权威设计见 `docs/design/v3-mcp-rfc.md`。
//!
//! M1 只做只读 tools(board_info / list_components / describe_project / read_state),
//! 全部可纯单测:`handle_request(Value) -> Option<Value>`。有状态 tools 留 M2。

use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "moxin";

/// `moxin mcp`:stdin 逐行读 JSON-RPC,stdout 逐行写响应,日志走 stderr。
pub fn cmd_mcp() -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    eprintln!("moxin mcp server ready (stdio, protocol {})", PROTOCOL_VERSION);
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => {
                // 解析失败:无 id 可对应,回一个 parse error(id=null)
                write_line(&mut out, &error_response(Value::Null, -32700, "Parse error"))?;
                continue;
            }
        };
        if let Some(resp) = handle_request(&req) {
            write_line(&mut out, &resp)?;
        }
        // notification(无 id)→ handle_request 返回 None,不回响应
    }
    Ok(())
}

fn write_line<W: Write>(out: &mut W, v: &Value) -> Result<()> {
    writeln!(out, "{}", serde_json::to_string(v)?)?;
    out.flush()?;
    Ok(())
}

/// 处理一条 JSON-RPC 消息。返回 `None` = 是 notification(无需响应)。
/// 纯函数(除了读文件的 tool),便于单测。
pub fn handle_request(req: &Value) -> Option<Value> {
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    // 无 id = notification(如 "initialized"):`?` 直接返回 None,不回响应
    let id = req.get("id").cloned()?;

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_specs() })),
        "tools/call" => call_tool(req.get("params")),
        other => {
            return Some(error_response(
                id,
                -32601,
                &format!("Method not found: {}", other),
            ));
        }
    };

    Some(match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err(msg) => error_response(id, -32603, &msg),
    })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// 四个只读 tool 的 MCP 规格(name / description / inputSchema)。
fn tool_specs() -> Value {
    json!([
        {
            "name": "board_info",
            "description": "Get the spec of a MoXin board (mcu, clock, pins, GPIO count).",
            "inputSchema": {
                "type": "object",
                "properties": { "board": { "type": "string", "description": "arduino-uno | arduino-nano | stm32 | gd32vf103" } },
                "required": ["board"]
            }
        },
        {
            "name": "list_components",
            "description": "List all built-in component kinds and their aliases.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "describe_project",
            "description": "Parse a moxin.toml project and return its board, components and wires.",
            "inputSchema": {
                "type": "object",
                "properties": { "path": { "type": "string", "description": "project dir (default: current dir)" } }
            }
        },
        {
            "name": "read_state",
            "description": "Read the latest .moxin-state.json snapshot (all peripheral state) for a project.",
            "inputSchema": {
                "type": "object",
                "properties": { "path": { "type": "string", "description": "project dir (default: current dir)" } }
            }
        }
    ])
}

/// tools/call 分派。成功 → MCP content 结果;工具内部错误 → isError 文本(不是协议错误)。
fn call_tool(params: Option<&Value>) -> std::result::Result<Value, String> {
    let params = params.ok_or_else(|| "missing params".to_string())?;
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| "missing tool name".to_string())?;
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

    let outcome = match name {
        "board_info" => tool_board_info(&args),
        "list_components" => Ok(tool_list_components()),
        "describe_project" => tool_describe_project(&args),
        "read_state" => tool_read_state(&args),
        other => return Err(format!("unknown tool: {}", other)),
    };

    // 工具执行错误 → 作为 isError 结果返回(MCP 惯例:工具级错误不走协议 error)
    Ok(match outcome {
        Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
        Err(e) => json!({ "content": [{ "type": "text", "text": format!("error: {}", e) }], "isError": true }),
    })
}

fn tool_board_info(args: &Value) -> std::result::Result<String, String> {
    let board = args
        .get("board")
        .and_then(|b| b.as_str())
        .ok_or_else(|| "board is required".to_string())?;
    let b = crate::boards::board_from_str(board).map_err(|e| e.to_string())?;
    let spec = b.spec();
    let pins: Vec<&str> = spec.pins.iter().map(|p| p.name).collect();
    let v = json!({
        "board_id": spec.board_id,
        "display_name": spec.display_name,
        "mcu": spec.mcu,
        "clock_hz": spec.clock_hz,
        "voltage_mv": spec.voltage_mv,
        "gpio_count": spec.gpio_count,
        "serial_count": spec.serial_count,
        "pwm_pins": spec.pwm_pins,
        "adc_channels": spec.adc_channels.iter().map(|(a, _)| a).collect::<Vec<_>>(),
        "pins": pins,
        "summary": spec.board_info_string(),
    });
    Ok(serde_json::to_string_pretty(&v).unwrap_or_default())
}

fn tool_list_components() -> String {
    let reg = crate::components::registry();
    let list: Vec<Value> = reg
        .all()
        .iter()
        .map(|d| json!({ "kind": d.kind(), "aliases": d.aliases() }))
        .collect();
    serde_json::to_string_pretty(&json!({ "count": list.len(), "components": list }))
        .unwrap_or_default()
}

/// 项目目录(参数 path 或 cwd)→ 找到 moxin.toml 所在根。
fn project_root(args: &Value) -> std::result::Result<std::path::PathBuf, String> {
    let start = match args.get("path").and_then(|p| p.as_str()) {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir().map_err(|e| e.to_string())?,
    };
    crate::project::Project::find_project_root(&start).map_err(|e| e.to_string())
}

fn tool_describe_project(args: &Value) -> std::result::Result<String, String> {
    let root = project_root(args)?;
    let project =
        crate::project::Project::load(&root.join("moxin.toml")).map_err(|e| e.to_string())?;
    let comps: Vec<Value> = project
        .components
        .iter()
        .map(|c| json!({ "id": c.id, "type": c.kind }))
        .collect();
    let wires: Vec<Value> = project
        .wires
        .iter()
        .map(|w| json!({ "from": w.from, "to": w.to }))
        .collect();
    let v = json!({
        "name": project.project.name,
        "board": project.project.board,
        "components": comps,
        "wires": wires,
    });
    Ok(serde_json::to_string_pretty(&v).unwrap_or_default())
}

fn tool_read_state(args: &Value) -> std::result::Result<String, String> {
    let root = project_root(args)?;
    let state_path = root.join("build").join(".moxin-state.json");
    if !state_path.exists() {
        return Err(format!(
            "no state snapshot at {} — run `moxin run --output json` first",
            state_path.display()
        ));
    }
    std::fs::read_to_string(&state_path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(method: &str, params: Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params })
    }

    #[test]
    fn initialize_handshake() {
        let resp = handle_request(&req("initialize", json!({}))).unwrap();
        assert_eq!(resp["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(resp["result"]["serverInfo"]["name"], "moxin");
        assert!(resp["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn notification_gets_no_response() {
        // 无 id = notification → None
        let n = json!({ "jsonrpc": "2.0", "method": "initialized" });
        assert!(handle_request(&n).is_none());
    }

    #[test]
    fn tools_list_has_four_readonly_tools() {
        let resp = handle_request(&req("tools/list", json!({}))).unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 4);
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"board_info"));
        assert!(names.contains(&"read_state"));
    }

    #[test]
    fn unknown_method_is_method_not_found() {
        let resp = handle_request(&req("frobnicate", json!({}))).unwrap();
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn call_board_info_returns_mcu() {
        let resp = handle_request(&req(
            "tools/call",
            json!({ "name": "board_info", "arguments": { "board": "arduino-uno" } }),
        ))
        .unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("ATmega328P"), "got: {text}");
        assert!(resp["result"].get("isError").is_none());
    }

    #[test]
    fn call_board_info_unknown_board_is_tool_error() {
        let resp = handle_request(&req(
            "tools/call",
            json!({ "name": "board_info", "arguments": { "board": "esp32" } }),
        ))
        .unwrap();
        // 工具级错误:isError=true,但仍是成功的 JSON-RPC 响应(有 result)
        assert_eq!(resp["result"]["isError"], true);
        assert!(resp.get("error").is_none());
    }

    #[test]
    fn call_list_components_counts_all_kinds() {
        let resp = handle_request(&req(
            "tools/call",
            json!({ "name": "list_components", "arguments": {} }),
        ))
        .unwrap();
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["count"], 17); // 与 Registry::builtin 一致
    }

    #[test]
    fn call_unknown_tool_errors() {
        let resp = handle_request(&req(
            "tools/call",
            json!({ "name": "nonesuch", "arguments": {} }),
        ))
        .unwrap();
        // 未知 tool 走协议 error(-32603),因为 call_tool 返回 Err
        assert_eq!(resp["error"]["code"], -32603);
    }

    #[test]
    fn ping_returns_empty() {
        let resp = handle_request(&req("ping", json!({}))).unwrap();
        assert!(resp["result"].is_object());
        assert!(resp.get("error").is_none());
    }
}
