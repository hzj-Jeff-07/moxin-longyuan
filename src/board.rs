use anyhow::{Result, bail};

/// 标准化引脚/端子引用。
/// 支持的输入形式:
///   board.D13 / board.PIN13 / board.GND / board.5V
///   pin13 / PIN13 / D13 / d13
///   gnd / GND / 5v / 5V
///   led1.anode / btn1.a (元件引脚)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinRef {
    /// 板子上的数字引脚 D0..D13
    BoardDigital(u8),
    /// 板子模拟引脚 A0..A5 (demo 不用,留接口)
    BoardAnalog(u8),
    BoardGnd,
    Board5V,
    Component { id: String, terminal: String },
}

impl PinRef {
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        // 元件引脚: id.terminal
        if let Some((id, term)) = s.split_once('.') {
            // 区分 board.* 和 元件.*
            if id.eq_ignore_ascii_case("board") {
                return parse_board_terminal(term);
            }
            return Ok(PinRef::Component {
                id: id.to_string(),
                terminal: term.to_lowercase(),
            });
        }
        // 没有 dot 的视作 board 端子
        parse_board_terminal(s)
    }

    pub fn render(&self) -> String {
        match self {
            PinRef::BoardDigital(n) => format!("D{}", n),
            PinRef::BoardAnalog(n) => format!("A{}", n),
            PinRef::BoardGnd => "GND".to_string(),
            PinRef::Board5V => "5V".to_string(),
            PinRef::Component { id, terminal } => format!("{}.{}", id, terminal),
        }
    }

    /// 用于 moxin.toml 持久化:始终带 board. 前缀,以匹配文档第六章
    pub fn render_canonical(&self) -> String {
        match self {
            PinRef::BoardDigital(n) => format!("board.D{}", n),
            PinRef::BoardAnalog(n) => format!("board.A{}", n),
            PinRef::BoardGnd => "board.GND".to_string(),
            PinRef::Board5V => "board.5V".to_string(),
            PinRef::Component { id, terminal } => format!("{}.{}", id, terminal),
        }
    }
}

fn parse_board_terminal(t: &str) -> Result<PinRef> {
    let up = t.to_uppercase();
    if up == "GND" {
        return Ok(PinRef::BoardGnd);
    }
    if up == "5V" || up == "VCC" {
        return Ok(PinRef::Board5V);
    }
    // PIN13 / D13 / 13
    let num_part: String = up
        .trim_start_matches("PIN")
        .trim_start_matches('D')
        .to_string();
    if let Ok(n) = num_part.parse::<u8>() {
        if n <= 13 {
            return Ok(PinRef::BoardDigital(n));
        }
    }
    if let Some(rest) = up.strip_prefix('A') {
        if let Ok(n) = rest.parse::<u8>() {
            if n <= 5 {
                return Ok(PinRef::BoardAnalog(n));
            }
        }
    }
    bail!("unknown board terminal: {}", t);
}

pub fn board_info() -> &'static str {
    "arduino-uno · 16MHz · pins: D0..D13, A0..A5, GND, 5V"
}
