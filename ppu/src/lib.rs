pub mod modes;
pub mod oam;
pub mod regs;
pub mod vram;

use modes::Mode;
use oam::Oam;
use regs::Registers;
use sm83::core::Cycles;
use vram::Vram;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Color {
    White = 0,
    LightGrey = 1,
    DarkGrey = 2,
    Black = 3,
}

const DISPLAY_WIDTH: usize = 160;
const DISPLAY_HEIGHT: usize = 144;

pub enum PpuResult {
    InProgress(Mode),
}

/// The Picture Processing Unit
pub struct Ppu {
    vram: Vram,
    regs: Registers,
    oam: Oam,

    mode: Mode,
    cycles: Cycles,

    /// Origin of coordinates is top-left pixel.
    framebuffer: [[Color; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
}

const CYCLES_PER_FRAME: usize = 70224;

impl Ppu {
    /// Constructs a PPU instance
    pub const fn new() -> Self {
        Self {
            vram: Vram::new(),
            regs: Registers::new(),
            oam: Oam::new(),

            mode: Mode::OamScan,
            cycles: Cycles::new(0),
            framebuffer: [[Color::White; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
        }
    }

    /// Runs the PPU for the given number of cycles and then returns the PPU state
    pub fn run(&mut self, cycles: Cycles) -> PpuResult {
        self.cycles = self.cycles + cycles;

        PpuResult::InProgress(self.mode)
    }
}

impl sm83::memory::Memory for Ppu {
    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        match address {
            0x8000..=0x9FFF => self.vram.read(address),
            0xFF40..=0xFF4B => self.regs.read(address),
            _ => {
                panic!("Unmapped address in PPU: {address}")
            }
        }
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        match address {
            0x8000..=0x9FFF => self.vram.write(address, value),
            0xFF40..=0xFF4B => self.regs.write(address, value),
            _ => {
                panic!("Unmapped address in PPU: {address}")
            }
        }
    }
}
