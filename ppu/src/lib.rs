pub mod dma;
pub mod modes;
pub mod oam;
pub mod regs;
pub mod vram;

use dma::DmaEngine;
use modes::Mode;
use oam::Oam;
use regs::Registers;
use sm83::{
    core::Cycles,
    interrupts::{Interrupt, Interrupts},
};
use tock_registers::interfaces::{ReadWriteable, Readable};
use vram::Vram;

use regs::STAT;
use vram::TILE_WIDTH;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Color {
    White = 0,
    LightGrey = 1,
    DarkGrey = 2,
    Black = 3,
}

/// An index into the palette, which resolves to a specific color
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PaletteIndex {
    Id0 = 0,
    Id1 = 1,
    Id2 = 2,
    Id3 = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Palette(u8);

impl Palette {
    pub fn color(&self, idx: PaletteIndex) -> Color {
        let idx = idx as u8 as usize;
        let color = 3 & (self.0 >> (2 * idx));
        match color {
            0 => Color::White,
            1 => Color::LightGrey,
            2 => Color::DarkGrey,
            3 => Color::Black,
            _ => unreachable!(),
        }
    }
}

impl From<u8> for Palette {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<Palette> for u8 {
    fn from(value: Palette) -> Self {
        value.0
    }
}

pub const DISPLAY_WIDTH: usize = 160;
pub const DISPLAY_HEIGHT: usize = 144;

#[derive(PartialEq, Eq, Debug)]
pub enum PpuResult {
    InProgress(Mode),
    FrameComplete,
}

pub type Frame = [[Color; DISPLAY_WIDTH]; DISPLAY_HEIGHT];

/// The Picture Processing Unit
pub struct Ppu {
    vram: Vram,
    regs: Registers,
    oam: Oam,

    mode: Mode,
    cycles: Cycles,
    stat_irq: bool,

    /// Origin of coordinates is top-left pixel.
    framebuffer: Frame,
}

const CYCLES_PER_FRAME: usize = 70224;

const OAM_SCAN_LEN: usize = 80;
const DRAWING_PIXELS_LEN: usize = 172;
const HBLANK_LEN: usize = 204;
const LINE_LENGTH: usize = OAM_SCAN_LEN + DRAWING_PIXELS_LEN + HBLANK_LEN;

static_assertions::const_assert_eq!(CYCLES_PER_FRAME, LINE_LENGTH * 154);

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
            stat_irq: false,
            framebuffer: [[Color::White; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Runs the PPU for the given number of cycles and then returns the PPU state
    pub fn run(&mut self, cycles: Cycles, dma_engine: &mut DmaEngine) -> (Interrupts, PpuResult) {
        self.cycles = (self.cycles + cycles).wrap(CYCLES_PER_FRAME);

        let line = current_line(self.cycles);
        let new_mode = mode_for_current_cycle_count(self.cycles);

        if self.regs.dma_config.triggered {
            dma_engine.trigger(self.regs.dma_config.address);
            self.regs.dma_config.triggered = false;
        }
        let (interrupts, result) = self.step(new_mode, line);
        (interrupts | self.update_lcd_irq(), result)
    }

    pub fn frame(&self) -> &Frame {
        &self.framebuffer
    }

    fn update_lcd_irq(&mut self) -> Interrupts {
        let lyc_eq_ly = self.regs.status.read(STAT::LYC_INT_SELECT) != 0
            && self.regs.status.read(STAT::LYC_EQ_LY) != 0;
        let mode0_irq =
            self.regs.status.read(STAT::MODE_0_INT_SELECT) != 0 && self.mode == Mode::Hblank;
        let mode1_irq =
            self.regs.status.read(STAT::MODE_1_INT_SELECT) != 0 && self.mode == Mode::Vblank;
        let mode2_irq =
            self.regs.status.read(STAT::MODE_2_INT_SELECT) != 0 && self.mode == Mode::OamScan;
        let status = lyc_eq_ly || mode0_irq || mode1_irq || mode2_irq;

        if !self.stat_irq && status {
            self.stat_irq = true;
            return Interrupt::Lcd.into();
        }

        if self.stat_irq && !status {
            self.stat_irq = false;
        }

        Interrupts::new()
    }

    fn step(&mut self, new_mode: Mode, line: usize) -> (Interrupts, PpuResult) {
        const NO_IRQ: Interrupts = Interrupts::new();
        if self.mode == new_mode {
            self.update_registers(line);
            return (NO_IRQ, PpuResult::InProgress(self.mode));
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
                return (Interrupt::Vblank.into(), PpuResult::FrameComplete);
            }
        }

        self.update_registers(line);
        (NO_IRQ, PpuResult::InProgress(self.mode))
    }

    fn draw_line(&mut self, line_idx: usize) {
        if self.regs.lcdc.read(regs::LCDC::ENABLE) == 0 {
            return;
        }

        let bg_y_offset = self.regs.scy as usize;
        let bg_x_offset = self.regs.scx as usize;
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

        let mut x_inner_offset = bg_x_offset % TILE_WIDTH;
        let x_tile_offset = bg_x_offset / TILE_WIDTH;

        let mut disp_x_offset = 0;

        'outer: for tile_index in self
            .vram
            .get_tile_map(bg_tile_map)
            .line(bg_y_offset + line_idx)
            .iter()
            .cycle()
            .skip(x_tile_offset)
        {
            let tile = self.vram.get_tile(*tile_index, bg_tile_data_area);
            let tile_line_idx = (bg_y_offset + line_idx) % vram::TILE_HEIGHT;
            let tile_line = tile.get_line(tile_line_idx);
            for pixel in tile_line.iter().skip(x_inner_offset) {
                let bit_color = palette.color(pixel);
                self.framebuffer[line_idx][disp_x_offset] = bit_color;
                disp_x_offset += 1;
                if disp_x_offset >= DISPLAY_WIDTH {
                    break 'outer;
                }
            }
            x_inner_offset = 0;
        }
    }

    fn update_registers(&mut self, line: usize) {
        let line = line as u8;
        self.regs.ly = line;
        self.regs.status.modify(
            regs::STAT::PPU_MODE.val(self.mode as u8)
                + regs::STAT::LYC_EQ_LY.val((line == self.regs.lyc) as u8),
        );
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
