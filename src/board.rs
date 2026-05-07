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
    // 形如 PIN13 / D13 / 13
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(s: &str) -> PinRef {
        PinRef::parse(s).unwrap_or_else(|e| panic!("parse({:?}) failed: {}", s, e))
    }

    #[test]
    fn parse_board_pin_in_all_forms() {
        for s in [
            "pin13", "PIN13", "Pin13", "D13", "d13", "13",
            "board.D13", "board.PIN13", "BOARD.d13",
        ] {
            assert_eq!(
                parse_ok(s),
                PinRef::BoardDigital(13),
                "input {:?} should parse to BoardDigital(13)",
                s
            );
        }
    }

    #[test]
    fn parse_gnd_and_5v() {
        for s in ["gnd", "GND", "Gnd", "board.GND", "board.gnd"] {
            assert_eq!(parse_ok(s), PinRef::BoardGnd, "{:?}", s);
        }
        for s in ["5v", "5V", "vcc", "VCC", "board.5V", "board.VCC"] {
            assert_eq!(parse_ok(s), PinRef::Board5V, "{:?}", s);
        }
    }

    #[test]
    fn parse_component_terminal() {
        assert_eq!(
            parse_ok("led1.a"),
            PinRef::Component {
                id: "led1".to_string(),
                terminal: "a".to_string(),
            }
        );
        // 大小写归一:terminal 应该归一到小写
        match parse_ok("LED1.A") {
            PinRef::Component { id, terminal } => {
                assert_eq!(id, "LED1", "component id 应保留原大小写");
                assert_eq!(terminal, "a", "terminal 应归一为小写");
            }
            other => panic!("expected Component, got {:?}", other),
        }
    }

    #[test]
    fn parse_rejects_d14() {
        assert!(PinRef::parse("D14").is_err(), "D14 越界应拒绝");
        assert!(PinRef::parse("pin14").is_err(), "pin14 越界应拒绝");
        assert!(PinRef::parse("board.D14").is_err(), "board.D14 越界应拒绝");
    }

    #[test]
    fn parse_rejects_a6() {
        assert!(PinRef::parse("A6").is_err(), "A6 越界应拒绝");
        assert!(PinRef::parse("a6").is_err(), "a6 越界应拒绝");
        assert!(PinRef::parse("board.A6").is_err(), "board.A6 越界应拒绝");
    }

    #[test]
    fn render_canonical_board_pins_have_prefix() {
        for p in [
            PinRef::BoardDigital(13),
            PinRef::BoardDigital(0),
            PinRef::BoardAnalog(5),
            PinRef::BoardGnd,
            PinRef::Board5V,
        ] {
            let s = p.render_canonical();
            assert!(
                s.starts_with("board."),
                "{:?} -> {} 应以 'board.' 开头",
                p, s
            );
        }
        // 精确值
        assert_eq!(PinRef::BoardDigital(13).render_canonical(), "board.D13");
        assert_eq!(PinRef::BoardAnalog(0).render_canonical(), "board.A0");
        assert_eq!(PinRef::BoardGnd.render_canonical(), "board.GND");
        assert_eq!(PinRef::Board5V.render_canonical(), "board.5V");
    }

    #[test]
    fn render_canonical_component_no_prefix() {
        let p = PinRef::Component {
            id: "led1".to_string(),
            terminal: "a".to_string(),
        };
        assert_eq!(p.render_canonical(), "led1.a");
        assert!(
            !p.render_canonical().starts_with("board."),
            "元件引用不应带 'board.' 前缀"
        );
    }
}
