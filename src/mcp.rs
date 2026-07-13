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

use crate::sim::RunningSim;

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "moxin";

/// MCP server 会话状态:跨 tool 调用持有运行中的仿真(M2 有状态 tools)。
#[derive(Default)]
pub struct Session {
    sim: Option<RunningSim>,
    root: Option<std::path::PathBuf>,
}

impl Session {
    fn stop_sim(&mut self) {
        if let Some(sim) = self.sim.take() {
            sim.stop();
        }
    }
}

/// `moxin mcp`:stdin 逐行读 JSON-RPC,stdout 逐行写响应,日志走 stderr。
pub fn cmd_mcp() -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut session = Session::default();
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
        if let Some(resp) = handle_request(&req, &mut session) {
            write_line(&mut out, &resp)?;
        }
        // notification(无 id)→ handle_request 返回 None,不回响应
    }
    session.stop_sim();
    Ok(())
}

fn write_line<W: Write>(out: &mut W, v: &Value) -> Result<()> {
    writeln!(out, "{}", serde_json::to_string(v)?)?;
    out.flush()?;
    Ok(())
}

/// 处理一条 JSON-RPC 消息。返回 `None` = 是 notification(无需响应)。
/// 只读 tool 不碰 session;有状态 tool(build/run/stop/sim_state/inject)驱动 session。
pub fn handle_request(req: &Value, session: &mut Session) -> Option<Value> {
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    // 无 id = notification(如 "initialized"):`?` 直接返回 None,不回响应
    let id = req.get("id").cloned()?;

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {}, "resources": {} },
            "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_specs() })),
        "tools/call" => call_tool(req.get("params"), session),
        "resources/list" => Ok(json!({ "resources": resource_specs() })),
        "resources/read" => read_resource(req.get("params"), session),
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
        },
        {
            "name": "build",
            "description": "Compile a MoXin project's firmware (arduino-cli / arm-gcc).",
            "inputSchema": {
                "type": "object",
                "properties": { "path": { "type": "string", "description": "project dir (default: current dir)" } }
            }
        },
        {
            "name": "run",
            "description": "Start the simulator for a project (held in the MCP session). Build first.",
            "inputSchema": {
                "type": "object",
                "properties": { "path": { "type": "string", "description": "project dir (default: current dir)" } }
            }
        },
        {
            "name": "stop",
            "description": "Stop the running simulator in this session.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "sim_state",
            "description": "Live state snapshot of the running simulator (all peripherals).",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "inject",
            "description": "Drive a stimulus into the running sim: kind adc|dist|env|ir|serial with its params.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["adc", "dist", "env", "ir", "serial"] },
                    "channel": { "type": "integer", "description": "adc: 0..7" },
                    "value": { "type": "integer", "description": "adc: 0..1023" },
                    "cm": { "type": "integer", "description": "dist: 2..400" },
                    "temp": { "type": "integer", "description": "env: 0..50" },
                    "hum": { "type": "integer", "description": "env: 20..90" },
                    "code": { "type": "string", "description": "ir: 32-bit hex, e.g. 20DF10EF" },
                    "text": { "type": "string", "description": "serial: text to feed firmware Serial.read" }
                },
                "required": ["kind"]
            }
        },
        {
            "name": "assert",
            "description": "Assert a condition on the running sim, returns PASS/FAIL/TIMEOUT. Modes: pin+eq (level check after `after`), pin+toggles (edge within `within`), or serial_contains (line within `within`).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pin": { "type": "string", "description": "pin name, e.g. D13 / A0 (Arduino: any GPIO; STM32/gd32: only D13)" },
                    "eq": { "type": "string", "description": "expected level HIGH|LOW (with pin, no toggles)" },
                    "after": { "type": "string", "description": "settle delay before pin+eq read, e.g. 1s / 250ms (default 1s)" },
                    "toggles": { "type": "boolean", "description": "with pin: pass on any edge within `within`" },
                    "serial_contains": { "type": "string", "description": "pass if a serial line contains this substring within `within`" },
                    "within": { "type": "string", "description": "observation window for toggles/serial, e.g. 3s (default: toggles 3s, serial 2s)" }
                }
            }
        }
    ])
}

/// MCP resources:把仿真状态暴露成可寻址 resource(与 tools 互补,方便订阅式客户端)。
fn resource_specs() -> Value {
    json!([
        {
            "uri": "moxin://state",
            "name": "MoXin simulator state",
            "description": "Live all-peripheral snapshot of the running sim, or the last .moxin-state.json.",
            "mimeType": "application/json"
        }
    ])
}

fn read_resource(params: Option<&Value>, session: &Session) -> std::result::Result<Value, String> {
    let uri = params
        .and_then(|p| p.get("uri"))
        .and_then(|u| u.as_str())
        .ok_or_else(|| "resources/read requires `uri`".to_string())?;
    if uri != "moxin://state" {
        return Err(format!("unknown resource: {}", uri));
    }
    // 优先运行中仿真的实时快照;否则回退到 cwd 项目的 .moxin-state.json
    let text = if let Some(sim) = session.sim.as_ref() {
        let s = sim.state.lock().map_err(|_| "state lock poisoned".to_string())?;
        serde_json::to_string_pretty(&s.to_json()).unwrap_or_default()
    } else {
        let root = std::env::current_dir()
            .ok()
            .and_then(|cwd| crate::project::Project::find_project_root(&cwd).ok())
            .ok_or_else(|| "no running sim and no project in cwd".to_string())?;
        let path = root.join("build").join(".moxin-state.json");
        std::fs::read_to_string(&path)
            .map_err(|_| format!("no state at {} — run first", path.display()))?
    };
    Ok(json!({
        "contents": [{ "uri": uri, "mimeType": "application/json", "text": text }]
    }))
}

/// tools/call 分派。成功 → MCP content 结果;工具内部错误 → isError 文本(不是协议错误)。
fn call_tool(params: Option<&Value>, session: &mut Session) -> std::result::Result<Value, String> {
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
        "build" => tool_build(&args),
        "run" => tool_run(&args, session),
        "stop" => tool_stop(session),
        "sim_state" => tool_sim_state(session),
        "inject" => tool_inject(&args, session),
        "assert" => tool_assert(&args, session),
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

// ---- M2:有状态 tools(build/run/stop/sim_state/inject)----

fn tool_build(args: &Value) -> std::result::Result<String, String> {
    let root = project_root(args)?;
    let project =
        crate::project::Project::load(&root.join("moxin.toml")).map_err(|e| e.to_string())?;
    let board = crate::boards::board_from_str(&project.project.board).map_err(|e| e.to_string())?;
    let (_artifact, msg) = board.build(&root).map_err(|e| e.to_string())?;
    Ok(msg)
}

fn tool_run(args: &Value, session: &mut Session) -> std::result::Result<String, String> {
    if let Some(sim) = session.sim.as_mut() {
        if sim.is_alive() {
            return Err("simulator already running — stop it first".to_string());
        }
    }
    let root = project_root(args)?;
    let project =
        crate::project::Project::load(&root.join("moxin.toml")).map_err(|e| e.to_string())?;
    let board = crate::boards::board_from_str(&project.project.board).map_err(|e| e.to_string())?;
    let ext = board.artifact_ext();
    let artifact = root.join("build").join(format!("{}.{}", project.project.name, ext));
    if !artifact.exists() {
        return Err(format!(
            "artifact not found at {} — call `build` first",
            artifact.display()
        ));
    }
    // json_out=false:事件只进 RunState,绝不能污染 MCP 的 stdout(JSON-RPC 通道)
    let mut sim = board
        .spawn_sim(&root, &artifact, false)
        .map_err(|e| e.to_string())?;
    crate::sim::configure_peripherals(&mut sim, &project, board.spec()).map_err(|e| e.to_string())?;
    session.sim = Some(sim);
    session.root = Some(root);
    Ok(format!("simulator started ({})", project.project.board))
}

fn tool_stop(session: &mut Session) -> std::result::Result<String, String> {
    if session.sim.is_none() {
        return Ok("no simulator running".to_string());
    }
    session.stop_sim();
    Ok("simulator stopped".to_string())
}

fn tool_sim_state(session: &mut Session) -> std::result::Result<String, String> {
    let sim = session
        .sim
        .as_ref()
        .ok_or_else(|| "no simulator running — call `run` first".to_string())?;
    let s = sim.state.lock().map_err(|_| "state lock poisoned".to_string())?;
    Ok(serde_json::to_string_pretty(&s.to_json()).unwrap_or_default())
}

fn tool_inject(args: &Value, session: &mut Session) -> std::result::Result<String, String> {
    let sim = session
        .sim
        .as_mut()
        .ok_or_else(|| "no simulator running — call `run` first".to_string())?;
    let kind = args
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| "inject requires `kind`".to_string())?;
    let int = |k: &str| args.get(k).and_then(|v| v.as_i64());
    match kind {
        "adc" => {
            let ch = int("channel").ok_or("adc requires `channel`")? as u8;
            let v = int("value").ok_or("adc requires `value`")?.clamp(0, 1023) as u16;
            sim.set_adc(ch, v).map_err(|e| e.to_string())?;
            Ok(format!("adc ch{} = {}", ch, v))
        }
        "dist" => {
            let cm = int("cm").ok_or("dist requires `cm`")?.clamp(2, 400) as u16;
            sim.set_distance(cm).map_err(|e| e.to_string())?;
            Ok(format!("dist = {}cm", cm))
        }
        "env" => {
            let t = int("temp").ok_or("env requires `temp`")?.clamp(0, 50) as u8;
            let h = int("hum").ok_or("env requires `hum`")?.clamp(20, 90) as u8;
            sim.set_env(t, h).map_err(|e| e.to_string())?;
            Ok(format!("env = {}°C {}%", t, h))
        }
        "ir" => {
            let code_str = args
                .get("code")
                .and_then(|c| c.as_str())
                .ok_or("ir requires `code` (hex)")?;
            let hex = code_str.trim_start_matches("0x").trim_start_matches("0X");
            let code = u32::from_str_radix(hex, 16)
                .map_err(|_| format!("invalid NEC code: {}", code_str))?;
            sim.send_ir(code).map_err(|e| e.to_string())?;
            Ok(format!("ir {:08X}", code))
        }
        "serial" => {
            let text = args
                .get("text")
                .and_then(|t| t.as_str())
                .ok_or("serial requires `text`")?;
            sim.send_serial(text).map_err(|e| e.to_string())?;
            Ok(format!("sent {} byte(s)", text.len()))
        }
        other => Err(format!("unknown inject kind: {}", other)),
    }
}

/// 对 session 里**已在运行**的仿真求值一条断言,复用 CLI 的判定逻辑,
/// 返回 `PASS` / `FAIL` / `TIMEOUT`。会阻塞至多 `within`/`after`(客户端等待响应)。
fn tool_assert(args: &Value, session: &mut Session) -> std::result::Result<String, String> {
    // 需要板 spec 做引脚可观测性判定;从当次 run 记下的项目根推导(spec 是 'static)。
    let root = session
        .root
        .clone()
        .ok_or_else(|| "no simulator running — call `run` first".to_string())?;
    let project =
        crate::project::Project::load(&root.join("moxin.toml")).map_err(|e| e.to_string())?;
    let spec = crate::boards::board_from_str(&project.project.board)
        .map_err(|e| e.to_string())?
        .spec();

    let sstr = |k: &str| {
        args.get(k)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    let toggles = args.get("toggles").and_then(|v| v.as_bool()).unwrap_or(false);
    let mode = crate::cmd_assert::AssertMode::resolve(
        &sstr("pin"),
        &sstr("eq"),
        &sstr("after"),
        toggles,
        &sstr("within"),
        &sstr("serial_contains"),
    )
    .map_err(|e| e.to_string())?;

    let sim = session
        .sim
        .as_mut()
        .ok_or_else(|| "no simulator running — call `run` first".to_string())?;
    let result = crate::cmd_assert::evaluate(sim, spec, mode).map_err(|e| e.to_string())?;
    Ok(result.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(method: &str, params: Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params })
    }

    /// 便捷:处理一条请求(全新会话),期望有响应。
    fn call(method: &str, params: Value) -> Value {
        handle_request(&req(method, params), &mut Session::default()).unwrap()
    }

    #[test]
    fn initialize_handshake() {
        let resp = call("initialize", json!({}));
        assert_eq!(resp["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(resp["result"]["serverInfo"]["name"], "moxin");
        assert!(resp["result"]["capabilities"]["tools"].is_object());
        assert!(resp["result"]["capabilities"]["resources"].is_object());
    }

    #[test]
    fn resources_list_exposes_state() {
        let resp = call("resources/list", json!({}));
        let res = resp["result"]["resources"].as_array().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0]["uri"], "moxin://state");
    }

    #[test]
    fn resources_read_unknown_uri_errors() {
        let resp = call("resources/read", json!({ "uri": "moxin://nope" }));
        // unknown resource → call_tool 路径外,走 handle_request 的 Err → -32603
        assert_eq!(resp["error"]["code"], -32603);
    }

    #[test]
    fn notification_gets_no_response() {
        // 无 id = notification → None
        let n = json!({ "jsonrpc": "2.0", "method": "initialized" });
        assert!(handle_request(&n, &mut Session::default()).is_none());
    }

    #[test]
    fn tools_list_has_all_ten_tools() {
        let resp = call("tools/list", json!({}));
        let tools = resp["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names.len(), 10);
        for want in [
            "board_info", "read_state", "build", "run", "stop", "sim_state", "inject", "assert",
        ] {
            assert!(names.contains(&want), "missing tool {want}");
        }
    }

    #[test]
    fn unknown_method_is_method_not_found() {
        let resp = call("frobnicate", json!({}));
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn call_board_info_returns_mcu() {
        let resp = call(
            "tools/call",
            json!({ "name": "board_info", "arguments": { "board": "arduino-uno" } }),
        );
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("ATmega328P"), "got: {text}");
        assert!(resp["result"].get("isError").is_none());
    }

    #[test]
    fn call_board_info_unknown_board_is_tool_error() {
        let resp = call(
            "tools/call",
            json!({ "name": "board_info", "arguments": { "board": "esp32" } }),
        );
        // 工具级错误:isError=true,但仍是成功的 JSON-RPC 响应(有 result)
        assert_eq!(resp["result"]["isError"], true);
        assert!(resp.get("error").is_none());
    }

    #[test]
    fn call_list_components_counts_all_kinds() {
        let resp = call(
            "tools/call",
            json!({ "name": "list_components", "arguments": {} }),
        );
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["count"], 17); // 与 Registry::builtin 一致
    }

    #[test]
    fn call_unknown_tool_errors() {
        let resp = call(
            "tools/call",
            json!({ "name": "nonesuch", "arguments": {} }),
        );
        // 未知 tool 走协议 error(-32603),因为 call_tool 返回 Err
        assert_eq!(resp["error"]["code"], -32603);
    }

    #[test]
    fn ping_returns_empty() {
        let resp = call("ping", json!({}));
        assert!(resp["result"].is_object());
        assert!(resp.get("error").is_none());
    }

    // ---- M2:有状态 tool 在无运行仿真时的边界(不需要 simavr)----

    #[test]
    fn sim_state_without_run_is_tool_error() {
        let mut sess = Session::default();
        let resp = handle_request(
            &req("tools/call", json!({ "name": "sim_state", "arguments": {} })),
            &mut sess,
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
        assert!(resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("no simulator running"));
    }

    #[test]
    fn inject_without_run_is_tool_error() {
        let mut sess = Session::default();
        let resp = handle_request(
            &req(
                "tools/call",
                json!({ "name": "inject", "arguments": { "kind": "adc", "channel": 0, "value": 512 } }),
            ),
            &mut sess,
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn assert_without_run_is_tool_error() {
        let mut sess = Session::default();
        let resp = handle_request(
            &req(
                "tools/call",
                json!({ "name": "assert", "arguments": { "pin": "D13", "toggles": true } }),
            ),
            &mut sess,
        )
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
        assert!(resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("no simulator running"));
    }

    #[test]
    fn stop_without_run_is_ok() {
        let mut sess = Session::default();
        let resp = handle_request(
            &req("tools/call", json!({ "name": "stop", "arguments": {} })),
            &mut sess,
        )
        .unwrap();
        assert!(resp["result"].get("isError").is_none());
        assert!(resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("no simulator"));
    }
}
