//! AI Inspector 面板的数据派生层。
//!
//! v2a 阶段:**不接 LLM / MCP**,只把 RunState / Project 里能直接派生的状态
//! 渲染成 5 行结构化输出。trait + StubInspector 是给 v3-sprint 接外部模型
//! 留的占位 hook。
//!
//! 设计依据:`docs/design/cli-vision.md` §3
//!   "AI Inspector 走外接模型:LLM API / MCP server 出来的结果,MoXin 不自训、
//!    不内置模型,只负责提供结构化状态 + 渲染外部模型回答。"

use crate::sim::{LedLevel, RunState};
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
        let (btn_value, btn_color) = if has_button {
            if state.button_pressed {
                ("DOWN".to_string(), Color::Rgb(255, 200, 40))
            } else {
                ("UP".to_string(), Color::Reset)
            }
        } else {
            ("—".to_string(), Color::DarkGray)
        };
        out.push(InspectorLine {
            icon: if has_button { '✓' } else { ' ' },
            label: "Button".to_string(),
            value: btn_value,
            color: btn_color,
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

        // 5. Components(项目静态元件汇总,纯派生,Phase 2 新元件在这里可见)
        let comp_summary = summarize_components(&project.components);
        out.push(InspectorLine {
            icon: if project.components.is_empty() { ' ' } else { '✓' },
            label: "Components".to_string(),
            value: comp_summary,
            color: Color::Reset,
        });

        // 6. Sensors(运行时外设状态汇总,有注入/仿真值才显示;Phase 3 外设可见)
        if let Some(sensor_summary) = summarize_sensors(state) {
            out.push(InspectorLine {
                icon: '✓',
                label: "Sensors".to_string(),
                value: sensor_summary,
                color: Color::Rgb(120, 200, 255),
            });
        }

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

/// 把运行时外设状态聚合成一行摘要,如 "A0:512, 31°C/75%, 120cm, IR:20DF10EF"。
/// 只汇总有值的外设(AI 一眼看清当前注入/仿真了什么);全无 → None(不显示这行)。
fn summarize_sensors(state: &RunState) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    // ADC:按通道号排序,A<ch>:<value>
    let mut adc: Vec<(&u8, &u16)> = state.adc_values.iter().collect();
    adc.sort_by_key(|(ch, _)| **ch);
    for (ch, v) in adc {
        parts.push(format!("A{}:{}", ch, v));
    }
    if let Some((t, h)) = state.dht_env {
        parts.push(format!("{}°C/{}%", t, h));
    }
    if let Some(cm) = state.ultrasonic_cm {
        parts.push(format!("{}cm", cm));
    }
    if let Some(code) = state.ir_code {
        parts.push(format!("IR:{:08X}", code));
    }
    if state.lcd.is_some() {
        parts.push("LCD".to_string());
    }
    if state.oled.is_some() {
        parts.push("OLED".to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

/// 把 components 按 kind 聚合,返回如 "led×1, button×1, resistor×2(470Ω,10kΩ)" 的紧凑摘要。
/// 纯派生函数,不依赖运行时状态。每个 kind 的扩展信息由对应 ComponentDef::inspector_extra 提供。
fn summarize_components(components: &[crate::project::Component]) -> String {
    if components.is_empty() {
        return "—".to_string();
    }
    use std::collections::BTreeMap;
    let reg = crate::components::registry();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut extras: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for c in components {
        *counts.entry(c.kind.as_str()).or_insert(0) += 1;
        if let Some(def) = reg.resolve(&c.kind) {
            if let Some(extra) = def.inspector_extra(c) {
                extras.entry(c.kind.as_str()).or_default().push(extra);
            }
        }
    }
    let mut parts: Vec<String> = counts
        .iter()
        .map(|(k, n)| match extras.get(k) {
            Some(list) if !list.is_empty() => format!("{}×{}({})", k, n, list.join(",")),
            _ => format!("{}×{}", k, n),
        })
        .collect();
    parts.sort();
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Component;

    fn comp(id: &str, kind: &str) -> Component {
        Component {
            id: id.to_string(),
            kind: kind.to_string(),
            color: None,
            pos: None,
            ohms: None,
            max_ohms: None,
            wire_color: None,
        }
    }

    #[test]
    fn summarize_empty() {
        assert_eq!(summarize_components(&[]), "—");
    }

    #[test]
    fn sensors_none_when_idle() {
        assert!(summarize_sensors(&RunState::default()).is_none());
    }

    #[test]
    fn sensors_summarize_injected_state() {
        let mut s = RunState {
            dht_env: Some((31, 75)),
            ultrasonic_cm: Some(120),
            ir_code: Some(0x20DF10EF),
            ..Default::default()
        };
        s.adc_values.insert(0, 512);
        let out = summarize_sensors(&s).expect("has sensors");
        assert!(out.contains("A0:512"), "got: {out}");
        assert!(out.contains("31°C/75%"), "got: {out}");
        assert!(out.contains("120cm"), "got: {out}");
        assert!(out.contains("IR:20DF10EF"), "got: {out}");
    }

    #[test]
    fn inspector_shows_sensors_line_when_present() {
        let project = crate::project::Project {
            project: crate::project::ProjectMeta {
                name: "t".into(),
                board: "arduino-uno".into(),
                version: "0.2".into(),
            },
            components: vec![],
            wires: vec![],
            code: None,
        };
        let s = RunState {
            ultrasonic_cm: Some(42),
            ..Default::default()
        };
        let (lines, _) = StubInspector.inspect(&project, &s);
        assert!(lines.iter().any(|l| l.label == "Sensors" && l.value.contains("42cm")));
        // idle 时不出现 Sensors 行
        let (lines2, _) = StubInspector.inspect(&project, &RunState::default());
        assert!(!lines2.iter().any(|l| l.label == "Sensors"));
    }

    #[test]
    fn summarize_mixed() {
        let comps = vec![
            comp("l1", "led"),
            comp("b1", "button"),
            comp("bz1", "buzzer"),
        ];
        let s = summarize_components(&comps);
        assert!(s.contains("led×1"));
        assert!(s.contains("button×1"));
        assert!(s.contains("buzzer×1"));
    }

    #[test]
    fn summarize_resistor_shows_ohms() {
        let mut r1 = comp("r1", "resistor");
        r1.ohms = Some(470);
        let mut r2 = comp("r2", "resistor");
        r2.ohms = Some(10_000);
        let s = summarize_components(&[r1, r2]);
        assert!(s.contains("resistor×2"));
        assert!(s.contains("470Ω"));
        assert!(s.contains("10kΩ"));
    }

    #[test]
    fn summarize_counts_multiple_same_kind() {
        let comps = vec![comp("l1", "led"), comp("l2", "led"), comp("l3", "led")];
        assert_eq!(summarize_components(&comps), "led×3");
    }
}
