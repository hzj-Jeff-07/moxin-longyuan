use crate::board::PinRef;

type IsD13Fn = Box<dyn Fn(&str, u32) -> bool + Send + 'static>;

#[derive(Debug)]
pub struct BoardSpec {
    pub board_id: &'static str,
    pub display_name: &'static str,
    pub mcu: &'static str,
    pub clock_hz: u32,
    pub voltage_mv: u32,
    pub artifact_kind: ArtifactKind,
    pub pins: &'static [PinSpec],
    pub serial_count: u8,
    pub gpio_count: u16,
    /// bridge 协议里 D13 LED 对应的 port 字符串（simavr: "B", stm32: "GPIO"）
    pub d13_bridge_port: &'static str,
    /// bridge 协议里 D13 LED 对应的 bit 编号（simavr: 5, stm32: 13）
    pub d13_bridge_bit: u32,
    /// 支持硬件 PWM(analogWrite)的数字引脚号。LED 只对这些引脚显示占空比,
    /// 避免把慢速 blink 方波误读成调光(Uno: D3/D5/D6/D9/D10/D11)。
    pub pwm_pins: &'static [u8],
    /// (板上 A 引脚号, MCU ADC 通道) 映射。Uno: A0..A5 = ADC0..ADC5。
    /// 空 = 该板不支持 ADC 注入。
    pub adc_channels: &'static [(u8, u8)],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind { Hex, Elf }

#[derive(Debug)]
pub struct PinSpec {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub is_d13_led: bool,
}

impl BoardSpec {
    pub fn artifact_ext(&self) -> &'static str {
        match self.artifact_kind {
            ArtifactKind::Hex => "hex",
            ArtifactKind::Elf => "elf",
        }
    }

    /// Find a pin by name or alias (case-insensitive).
    pub fn find_pin(&self, name: &str) -> Option<&'static PinSpec> {
        let up = name.to_uppercase();
        self.pins.iter().find(|p| {
            p.name.to_uppercase() == up || p.aliases.iter().any(|a| a.to_uppercase() == up)
        })
    }

    /// Validate a board PinRef against this spec. Returns true if the pin is known.
    pub fn pin_ref_valid(&self, pin: &PinRef) -> bool {
        match pin {
            PinRef::BoardDigital(n) => self.find_pin(&format!("D{}", n)).is_some(),
            PinRef::BoardAnalog(n) => self.find_pin(&format!("A{}", n)).is_some(),
            PinRef::BoardGnd => self.find_pin("GND").is_some(),
            PinRef::Board5V => self.find_pin("5V").is_some(),
            PinRef::BoardPort { port, pin } => self.find_pin(&format!("{}{}", port, pin)).is_some(),
            PinRef::Component { .. } => true,
        }
    }

    /// Returns true if the given PinRef is the board's D13 LED pin.
    pub fn is_d13_pin(&self, pin: &PinRef) -> bool {
        match pin {
            PinRef::BoardDigital(n) => self.find_pin(&format!("D{}", n)).map(|p| p.is_d13_led).unwrap_or(false),
            PinRef::BoardAnalog(n) => self.find_pin(&format!("A{}", n)).map(|p| p.is_d13_led).unwrap_or(false),
            PinRef::BoardPort { port, pin } => self.find_pin(&format!("{}{}", port, pin)).map(|p| p.is_d13_led).unwrap_or(false),
            _ => false,
        }
    }

    /// Returns a closure that maps (port, bit) → bool for the D13 LED, derived from spec.
    pub fn make_is_d13(&self) -> IsD13Fn {
        let port = self.d13_bridge_port;
        let bit = self.d13_bridge_bit;
        Box::new(move |p: &str, b: u32| p == port && b == bit)
    }

    /// 板上 A 引脚号 → MCU ADC 通道。
    pub fn adc_channel_for(&self, a_pin: u8) -> Option<u8> {
        self.adc_channels
            .iter()
            .find(|(a, _)| *a == a_pin)
            .map(|(_, ch)| *ch)
    }

    /// Board info string derived from spec fields.
    pub fn board_info_string(&self) -> String {
        format!(
            "{} ({}) · {}MHz · {} pins · {}mV · {} UART · {} GPIO",
            self.display_name,
            self.mcu,
            self.clock_hz / 1_000_000,
            self.pins.len(),
            self.voltage_mv,
            self.serial_count,
            self.gpio_count,
        )
    }
}
