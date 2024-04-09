pub mod modes;
pub mod oam;
pub mod regs;
pub mod vram;

use modes::Mode;
use oam::Oam;
use regs::Registers;
use sm83::core::Cycles;
use tock_registers::interfaces::Readable;
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

pub enum PpuResult<'a> {
    InProgress(Mode),
    FrameComplete(&'a [[Color; DISPLAY_WIDTH]; DISPLAY_HEIGHT]),
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

const OAM_SCAN_LEN: usize = 80;
const DRAWING_PIXELS_LEN: usize = 172;
const HBLANK_LEN: usize = 204;
const LINE_LENGTH: usize = OAM_SCAN_LEN + DRAWING_PIXELS_LEN + HBLANK_LEN;

fn current_line(cycles: Cycles) -> usize {
    let cycles: usize = cycles.into();
    cycles / LINE_LENGTH
}

fn mode_for_current_cycle_count(cycles: Cycles) -> Mode {
    // This is a simplified implementation that considers a fixed time mode 3.
    let cycles: usize = cycles.into();
    const VBLANK_START: usize = LINE_LENGTH * 144;
    let line_offset = cycles % LINE_LENGTH;

    if cycles >= VBLANK_START {
        Mode::Vblank
    } else if line_offset < OAM_SCAN_LEN {
        Mode::OamScan
    } else if line_offset < (OAM_SCAN_LEN + DRAWING_PIXELS_LEN) {
        Mode::DrawingPixels
    } else {
        Mode::Hblank
    }
}

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
        self.cycles = (self.cycles + cycles).wrap(CYCLES_PER_FRAME);

        let line = current_line(self.cycles);
        let new_mode = mode_for_current_cycle_count(self.cycles);

        self.step(new_mode, line)
    }

    fn step(&mut self, new_mode: Mode, line: usize) -> PpuResult {
        if self.mode == new_mode {
            self.update_registers(line);
            return PpuResult::InProgress(self.mode);
        }

        self.mode = new_mode;

        match self.mode {
            Mode::OamScan => {
                // TODO: Do OAM scan
            }
            Mode::Hblank => {
                // Nothing to do
            }
            Mode::DrawingPixels => {
                // Fill in current line in framebuffer
                self.draw_line(line);
            }
            Mode::Vblank => {
                // Frame is complete
                self.update_registers(line);

                return PpuResult::FrameComplete(&self.framebuffer);
            }
        }

        self.update_registers(line);

        PpuResult::InProgress(self.mode)
    }

    fn draw_line(&mut self, line: usize) {
        let bg_y_offset = self.regs.scy;
        let bg_x_offset = self.regs.scx;
        let palette = self.regs.bg_palette;

        let bg_tile_map: crate::regs::LCDC::BG_TILE_MAP::Value = self
            .regs
            .lcdc
            .read_as_enum(crate::regs::LCDC::BG_TILE_MAP)
            .expect("Invalid LCDC bit 3");

        let bg_tile_data_area: crate::regs::LCDC::BG_AND_WINDOW_TILE_DATA::Value = self
            .regs
            .lcdc
            .read_as_enum(crate::regs::LCDC::BG_AND_WINDOW_TILE_DATA)
            .expect("Invalid LCDC bit 4");

        unimplemented!()
    }

    fn update_registers(&mut self, line: usize) {
        unimplemented!()
    }
}

impl sm83::memory::Memory for Ppu {
    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        match address {
            0x8000..=0x9FFF => self.vram.read(address),
            0xFE00..=0xFE9F => self.oam.read(address),
            0xFF40..=0xFF4B => self.regs.read(address),
            _ => {
                panic!("Unmapped address in PPU: {address}")
            }
        }
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        match address {
            0x8000..=0x9FFF => self.vram.write(address, value),
            0xFE00..=0xFE9F => self.oam.write(address, value),
            0xFF40..=0xFF4B => self.regs.write(address, value),
            _ => {
                panic!("Unmapped address in PPU: {address}")
            }
        }
    }
}
