//! Abstractions for CPU interrupts.

/// The set of memory-mapped interrupt registers
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct InterruptRegs {
    enable_reg: Interrupts,
    flags_reg: Interrupts,
}

impl InterruptRegs {
    /// Constructs the interrupt registers
    pub fn new() -> Self {
        Self {
            enable_reg: Interrupts::new(),
            flags_reg: Interrupts::new(),
        }
    }

    /// Returns the current set of active interrupts (flag is active and it is not masked)
    pub fn active_interrupts(&self) -> Interrupts {
        self.enable_reg & self.flags_reg
    }

    /// Acknowledges the given interrupt
    pub fn acknowledge(&mut self, interrupt: Interrupt) {
        self.flags_reg = self.flags_reg.acknowledge(interrupt);
    }

    /// Triggers the given interrupt
    pub fn trigger(&mut self, interrupts: Interrupts) {
        self.flags_reg = self.flags_reg | interrupts;
    }

    /// Reads the interrupt registers. Interrupt enable register (`0xFFFF`) and Interrupt flags
    /// register (`0xFF0F`)
    pub fn read(&self, address: crate::memory::Address) -> u8 {
        match address {
            0xFFFF => self.enable_reg.into(),
            0xFF0F => self.flags_reg.into(),
            _ => {
                panic!("Read from unknown interrupt register {address:#x}")
            }
        }
    }

    /// Writes the interrupt registers. Interrupt enable register (`0xFFFF`) and Interrupt flags
    /// register (`0xFF0F`)
    pub fn write(&mut self, address: crate::memory::Address, value: u8) {
        let value = Interrupts(value) & ALL_INTERRUPTS;
        match address {
            0xFFFF => self.enable_reg = value,
            0xFF0F => self.flags_reg = value,
            _ => {
                panic!("Write to unknown interrupt register {address:#x}")
            }
        };
    }
}

/// Represents a single interrupt source
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Interrupt {
    /// Interrupt triggered when the LCD enters VBLANK.
    Vblank = 0x01,
    /// LCD interrupt. Configurable to trigger on specific lines or on entry to a specific mode.
    Lcd = 0x02,
    /// Timer interrupt
    Timer = 0x04,
    /// Serial port interrupt
    Serial = 0x08,
    /// Joypad interrupt
    Joypad = 0x10,
}

/// Represents an aggregation of interrupt sources
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Interrupts(u8);

impl From<Interrupts> for u8 {
    fn from(value: Interrupts) -> Self {
        value.0
    }
}

impl From<Interrupt> for Interrupts {
    fn from(value: Interrupt) -> Self {
        Self(value as u8)
    }
}

impl core::ops::BitOr<Self> for Interrupts {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOr<Interrupt> for Interrupts {
    type Output = Self;
    fn bitor(self, rhs: Interrupt) -> Self::Output {
        Self(self.0 | rhs as u8)
    }
}

impl core::ops::BitAnd<Self> for Interrupts {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

const ALL_INTERRUPTS: Interrupts = Interrupts(
    (Interrupt::Vblank as u8)
        | (Interrupt::Lcd as u8)
        | (Interrupt::Timer as u8)
        | (Interrupt::Serial as u8)
        | (Interrupt::Joypad as u8),
);

impl core::ops::Not for Interrupts {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & ALL_INTERRUPTS.0)
    }
}

impl Default for Interrupts {
    fn default() -> Self {
        Self::new()
    }
}

impl Interrupts {
    /// Constructs a set of inactive interrupts
    pub const fn new() -> Self {
        Self(0)
    }

    /// Returns the interrupt with the highest priority out of all active interrupts (if any)
    #[must_use]
    pub fn highest_priority(self) -> Option<Interrupt> {
        let trailing_zeros = self.0.trailing_zeros();
        match trailing_zeros {
            0 => Some(Interrupt::Vblank),
            1 => Some(Interrupt::Lcd),
            2 => Some(Interrupt::Timer),
            3 => Some(Interrupt::Serial),
            4 => Some(Interrupt::Joypad),
            _ => None,
        }
    }

    /// Acknowledges the given interrupt
    #[must_use]
    pub fn acknowledge(self, other: Interrupt) -> Self {
        let other: Interrupts = other.into();
        self & !other
    }

    /// Returns true if any interrupt is active
    #[must_use]
    pub fn has_any(self) -> bool {
        self.0 != 0
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    #[test]
    fn neg_irqs() {
        let interrupts = Interrupts::new() | Interrupt::Lcd | Interrupt::Serial;
        assert_eq!(
            !interrupts,
            Interrupts::new() | Interrupt::Vblank | Interrupt::Timer | Interrupt::Joypad
        );
    }

    #[test]
    fn all_interrupts() {
        assert_eq!(ALL_INTERRUPTS.0, 0x1f);
    }

    #[test]
    fn ack_irq() {
        let interrupts = Interrupts::new() | Interrupt::Lcd | Interrupt::Serial;

        assert_eq!(
            Interrupts::new() | Interrupt::Serial,
            interrupts.acknowledge(Interrupt::Lcd)
        );
    }
}
