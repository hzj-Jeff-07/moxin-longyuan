//! AI Inspector 面板的数据派生层。
//!
//! v2a 阶段:**不接 LLM / MCP**,只把 RunState / Project 里能直接派生的状态
//! 渲染成 5 行结构化输出。trait + StubInspector 是给 v3-sprint 接外部模型
//! 留的占位 hook。
//!
//! 设计依据:`docs/design/cli-vision.md` §3
//!   "AI Inspector 走外接模型:LLM API / MCP server 出来的结果,MoXin 不自训、
//!    不内置模型,只负责提供结构化状态 + 渲染外部模型回答。"

use crate::cmd_run::{LedLevel, RunState};
use crate::project::Project;
use ratatui::style::Color;

/// 单条 inspector 输出。v2a 简单结构,v3 接外部模型时可能扩 Severity / source 字段。
#[derive(Debug, Clone)]
pub struct InspectorLine {
    /// 行首符号:✓ 表示已采集到的状态,空格表示无数据
    pub icon: char,
    pub label: String,
    pub value: String,
    /// value 的渲染色;label 默认色
    pub color: Color,
}

/// Status 行(渲染为单独一段)
#[derive(Debug, Clone)]
pub struct InspectorStatus {
    pub label: String,        // "OK" / "ERROR"
    pub color: Color,
    pub note: String,         // "No issues detected." / 错误原因
}

pub trait Inspector {
    fn inspect(&self, project: &Project, state: &RunState) -> (Vec<InspectorLine>, InspectorStatus);
}

/// v2a 占位实现。**只渲染派生数据**,不调任何 LLM。
///
/// TODO(v3-sprint): replace with MCP/LLM-driven analysis.
/// per project vision (`docs/design/cli-vision.md` §3): external model only,
/// MoXin doesn't train, doesn't bundle weights, only provides structured
/// state + renders external model answers.
pub struct StubInspector;

impl Inspector for StubInspector {
    fn inspect(&self, project: &Project, state: &RunState) -> (Vec<InspectorLine>, InspectorStatus) {
        let mut out = Vec::with_capacity(5);

        // 1. Voltage(板子常量,从 RunState.voltage_mv 直接派生)
        let v_int = state.voltage_mv / 1000;
        let v_frac = (state.voltage_mv % 1000) / 10;
        out.push(InspectorLine {
            icon: '✓',
            label: "Voltage".to_string(),
            value: format!("{}.{:02}V", v_int, v_frac),
            color: Color::Reset,
        });

        // 2. GPIO13(d13 字段实时派生)
        let (g_text, g_color) = match state.d13 {
            LedLevel::On => ("HIGH".to_string(), Color::Rgb(40, 220, 80)),
            LedLevel::Off => ("LOW".to_string(), Color::DarkGray),
        };
        out.push(InspectorLine {
            icon: '✓',
            label: "GPIO13".to_string(),
            value: g_text,
            color: g_color,
        });

        // 3. Button(当前 demo 没接按钮,看 project.components 有没有 button 类型)
        let has_button = project.components.iter().any(|c| c.kind == "button");
        let btn_value = if has_button {
            "UP".to_string()
        } else {
            "—".to_string()
        };
        out.push(InspectorLine {
            icon: if has_button { '✓' } else { ' ' },
            label: "Button".to_string(),
            value: btn_value,
            color: Color::Reset,
        });

        // 4. Loop Time(两次连续 pin event 的 t_us 差)
        let loop_value = match state.loop_time_us() {
            Some(us) => {
                let ms = us as f64 / 1000.0;
                format!("{:.0}ms", ms)
            }
            None => "—".to_string(),
        };
        out.push(InspectorLine {
            icon: if state.loop_time_us().is_some() { '✓' } else { ' ' },
            label: "Loop Time".to_string(),
            value: loop_value,
            color: Color::Reset,
        });

        // Status 段
        let status = if let Some(reason) = state.bridge_exit_reason.as_ref() {
            InspectorStatus {
                label: "ERROR".to_string(),
                color: Color::Rgb(255, 80, 80),
                note: format!("bridge exited: {}", reason),
            }
        } else if state.ready {
            InspectorStatus {
                label: "OK".to_string(),
                color: Color::Rgb(40, 220, 80),
                note: "No issues detected.".to_string(),
            }
        } else {
            InspectorStatus {
                label: "...".to_string(),
                color: Color::DarkGray,
                note: "waiting for bridge ready".to_string(),
            }
        };

        (out, status)
    }
}
