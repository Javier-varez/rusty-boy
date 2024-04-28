#![no_std]

use sm83::interrupts::Interrupt;
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::{register_bitfields, registers::InMemoryRegister};

use sm83::{core::Cycles, interrupts::Interrupts};

pub struct Timer {
    div: u16,
    tima: u8,
    tma: u8,
    tac: InMemoryRegister<u8, TAC::Register>,
    request_div_reset: bool,
}

register_bitfields! [
    u8,

    /// Timer Control
    pub TAC [
        /// Controls whether TIMA is incremented. Note that DIV is always counting, regardless of this bit.
        ENABLE OFFSET(2) NUMBITS(1) [
            Off = 0,
            On = 1,
        ],

        /// Controls whether TIMA is incremented. Note that DIV is always counting, regardless of this bit.
        CLK_SELECT OFFSET(0) NUMBITS(2) [
            MCycles256 = 0,
            MCycles4 = 1,
            MCycles16 = 2,
            MCycles64 = 3,
        ]
    ]
];

// Each second contains 4194304 cycles. The div counter increments at a rate of 16384 Hz.
// Therefore, the number of cycles per div tick is 256.
const NUM_CYCLES_PER_DIV_TICK: usize = 4194304 / 16384;
static_assertions::const_assert_eq!(NUM_CYCLES_PER_DIV_TICK, 256);

// The low byte of the div member is reserved for the invisible part of the div register
const HIDDEN_BITS: u32 = NUM_CYCLES_PER_DIV_TICK.trailing_zeros();
static_assertions::const_assert_eq!(HIDDEN_BITS, 8);

impl Timer {
    pub const fn new() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: InMemoryRegister::new(0),
            request_div_reset: false,
        }
    }

    fn clk_select_bit(&self) -> u16 {
        match self.tac.read_as_enum(TAC::CLK_SELECT).unwrap() {
            // 4 m-cycles are 16 clk-cycles. 16 clk-cycles are represented in 4 bits
            TAC::CLK_SELECT::Value::MCycles4 => 4,
            // 16 m-cycles are 64 clk-cycles. 64 clk-cycles are represented in 6 bits
            TAC::CLK_SELECT::Value::MCycles16 => 6,
            // 64 m-cycles are 256 clk-cycles. 256 clk-cycles are represented in 8 bits
            TAC::CLK_SELECT::Value::MCycles64 => 8,
            // 256 m-cycles are 1024 clk-cycles. 1024 clk-cycles are represented in 10 bits
            TAC::CLK_SELECT::Value::MCycles256 => 10,
        }
    }

    pub fn step(&mut self, cycles: Cycles) -> Interrupts {
        // TODO: do not run timer if cpu is disabled

        let cycles: usize = cycles.into();
        debug_assert!(cycles <= <u16 as Into<usize>>::into(u16::max_value()));

        let prev_div = self.div;
        let cur_div = if self.request_div_reset {
            0
        } else {
            prev_div.wrapping_add(cycles as u16)
        };
        self.div = cur_div;

        if self
            .tac
            .read_as_enum(TAC::ENABLE)
            .is_some_and(|v: TAC::ENABLE::Value| v == TAC::ENABLE::Value::Off)
        {
            // Timer is disabled
            return Interrupts::new();
        }

        let bit_mask = !((1 << self.clk_select_bit()) - 1);
        if ((prev_div ^ cur_div) & bit_mask) == 0 {
            // No increment of TIMA required
            return Interrupts::new();
        }

        self.tima = self.tima.wrapping_add(1);
        if self.tima == 0x0 {
            Interrupt::Timer.into()
        } else {
            Interrupts::new()
        }
    }
}

impl sm83::memory::Memory for Timer {
    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        match address {
            0xFF04 => (self.div >> HIDDEN_BITS) as u8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac.get(),
            _ => unreachable!("Unexpected timer read from {:#x}", address),
        }
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        match address {
            0xFF04 => self.request_div_reset = true,
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => self.tac.set(value),
            _ => unreachable!(
                "Unexpected timer write to {:#x}, value {:#x}",
                address, value
            ),
        };
    }
}
