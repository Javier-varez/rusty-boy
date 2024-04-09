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
            framebuffer: [[Color::White; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
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

    fn draw_line(&mut self, line_idx: usize) {
        let bg_y_offset = self.regs.scy as usize;
        // TODO: use bg_x_offset, currently it is completely ignored
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

        let mut x_offset = 0;
        'outer: for tile_index in self
            .vram
            .get_tile_map(bg_tile_map)
            .line(bg_y_offset + line_idx)
        {
            let tile = self.vram.get_tile(*tile_index, bg_tile_data_area);
            let tile_line_idx = (bg_y_offset + line_idx) % vram::TILE_HEIGHT;
            let tile_line = tile.get_line(tile_line_idx);
            for pixel in tile_line.iter() {
                let bit_color = palette.color(pixel);
                self.framebuffer[line_idx][x_offset] = bit_color;
                x_offset += 1;
                if x_offset >= DISPLAY_WIDTH {
                    break 'outer;
                }
            }
        }
    }

    fn update_registers(&mut self, line: usize) {
        // TODO: Implement the rest of register updates
        self.regs.ly = line as u8;
        // unimplemented!()
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