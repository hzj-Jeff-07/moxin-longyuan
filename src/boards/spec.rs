use crate::board::PinRef;

#[derive(Debug)]
#[allow(dead_code)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind { Hex, Elf }

#[derive(Debug)]
#[allow(dead_code)]
pub struct PinSpec {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub is_d13_led: bool,
}

#[allow(dead_code)]
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
}
