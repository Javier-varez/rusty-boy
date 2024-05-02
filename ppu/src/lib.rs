#![no_std]

extern crate alloc;

use alloc::boxed::Box;

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
use vram::{TILE_HEIGHT, TILE_WIDTH};

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

    // Vector of indexes into OAM entries
    selected_oam_entries: heapless::Vec<usize, MAX_SELECTED_OBJECTS>,

    /// Origin of coordinates is top-left pixel.
    framebuffer: Box<Frame>,
}

const CYCLES_PER_FRAME: usize = 70224;

const OAM_SCAN_LEN: usize = 80;
const DRAWING_PIXELS_LEN: usize = 172;
const HBLANK_LEN: usize = 204;
const LINE_LENGTH: usize = OAM_SCAN_LEN + DRAWING_PIXELS_LEN + HBLANK_LEN;
const MAX_SELECTED_OBJECTS: usize = 10;
const OBJ_OFFSET_Y: usize = 16;
const OBJ_OFFSET_X: usize = 8;

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
    pub fn new() -> Self {
        let mut framebuffer: [[core::mem::MaybeUninit<Color>; DISPLAY_WIDTH]; DISPLAY_HEIGHT] =
            [[core::mem::MaybeUninit::uninit(); DISPLAY_WIDTH]; DISPLAY_HEIGHT];

        for row in framebuffer.iter_mut() {
            for elem in row {
                elem.write(Color::Black);
            }
        }

        Self {
            vram: Vram::new(),
            regs: Registers::new(),
            oam: Oam::new(),

            mode: Mode::OamScan,
            cycles: Cycles::new(0),
            stat_irq: false,
            selected_oam_entries: heapless::Vec::new(),
            framebuffer: Box::new(unsafe { core::mem::transmute::<_, Frame>(framebuffer) }),
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Runs the PPU for the given number of cycles and then returns the PPU state
    pub fn step(
        &mut self,
        cycles: Cycles,
        dma_engine: &mut DmaEngine,
        render: bool,
    ) -> (Interrupts, PpuResult) {
        self.cycles = (self.cycles + cycles).wrap(CYCLES_PER_FRAME);

        let line = current_line(self.cycles);
        let new_mode = mode_for_current_cycle_count(self.cycles);

        if self.regs.dma_config.triggered {
            dma_engine.trigger(self.regs.dma_config.address);
            self.regs.dma_config.triggered = false;
        }
        let (interrupts, result) = self.step_inner(new_mode, line, render);
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

    fn oam_scan(&mut self, line: usize) {
        self.selected_oam_entries = heapless::Vec::new();

        let obj_height = (self.regs.lcdc.read(regs::LCDC::OBJ_SIZE) + 1) * 8;

        let cur_obj_line = (line + OBJ_OFFSET_Y) as u8;
        let is_object_relevant = |(_, object): &(usize, &oam::Object)| -> bool {
            cur_obj_line >= object.y && cur_obj_line < object.y + obj_height
        };

        // Walk all entries from 0 to NUM_OBJS
        self.selected_oam_entries = self
            .oam
            .iter()
            .enumerate()
            .filter(is_object_relevant)
            .map(|(i, _)| i)
            .take(MAX_SELECTED_OBJECTS)
            .collect();
    }

    fn step_inner(&mut self, new_mode: Mode, line: usize, render: bool) -> (Interrupts, PpuResult) {
        const NO_IRQ: Interrupts = Interrupts::new();
        if self.mode == new_mode {
            self.update_registers(line);
            return (NO_IRQ, PpuResult::InProgress(self.mode));
        }

        self.mode = new_mode;

        match self.mode {
            Mode::OamScan if render => {
                self.oam_scan(line);
            }
            Mode::DrawingPixels if render => {
                self.draw_line(line);
            }
            Mode::Vblank => {
                self.update_registers(line);
                return (Interrupt::Vblank.into(), PpuResult::FrameComplete);
            }
            _ => {}
        }

        self.update_registers(line);
        (NO_IRQ, PpuResult::InProgress(self.mode))
    }

    fn draw_line_background(&mut self, line_idx: usize, line: &mut [PaletteIndex; DISPLAY_WIDTH]) {
        let bg_win_enable = self.regs.lcdc.read(regs::LCDC::BG_AND_WINDOW_ENABLE) != 0;
        if !bg_win_enable {
            // BG is disabled
            for c in &mut self.framebuffer[line_idx] {
                *c = Color::White;
            }
            return;
        }

        let bg_y_offset = self.regs.scy as usize;
        let bg_x_offset = self.regs.scx as usize;

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
            .get_bg_tile_map(bg_tile_map)
            .line(bg_y_offset + line_idx)
            .iter()
            .cycle()
            .skip(x_tile_offset)
        {
            let tile = self.vram.get_tile(*tile_index, bg_tile_data_area);
            let tile_line_idx = (bg_y_offset + line_idx) % vram::TILE_HEIGHT;
            let tile_line = tile.get_line(tile_line_idx);
            for palette_index in tile_line.iter().skip(x_inner_offset) {
                line[disp_x_offset] = palette_index;
                disp_x_offset += 1;
                if disp_x_offset >= DISPLAY_WIDTH {
                    break 'outer;
                }
            }
            x_inner_offset = 0;
        }
    }

    fn draw_line_window(&mut self, line_idx: usize, line: &mut [PaletteIndex; DISPLAY_WIDTH]) {
        let bg_win_enable = self.regs.lcdc.read(regs::LCDC::BG_AND_WINDOW_ENABLE) != 0;
        if !bg_win_enable {
            return;
        }

        let wy = self.regs.wy as usize;
        if line_idx < wy {
            // Nothing to draw, return
            return;
        }
        let win_line = line_idx - wy;

        let win_tile_map: crate::regs::LCDC::WINDOW_TILE_MAP::Value = self
            .regs
            .lcdc
            .read_as_enum(crate::regs::LCDC::WINDOW_TILE_MAP)
            .expect("Invalid LCDC bit 6");

        let win_tile_data_area: crate::regs::LCDC::BG_AND_WINDOW_TILE_DATA::Value = self
            .regs
            .lcdc
            .read_as_enum(crate::regs::LCDC::BG_AND_WINDOW_TILE_DATA)
            .expect("Invalid LCDC bit 4");

        const WX_OFFSET: usize = 7;
        let wx = self.regs.wx as usize;
        let mut disp_x_offset = if wx >= WX_OFFSET { wx - WX_OFFSET } else { 0 };
        let mut disp_x_initial_skip = if wx >= WX_OFFSET { 0 } else { WX_OFFSET - wx };
        if disp_x_offset >= DISPLAY_WIDTH {
            return;
        }

        'outer: for tile_index in self
            .vram
            .get_win_tile_map(win_tile_map)
            .line(win_line)
            .iter()
        {
            let tile = self.vram.get_tile(*tile_index, win_tile_data_area);
            let tile_line_idx = win_line % vram::TILE_HEIGHT;
            let tile_line = tile.get_line(tile_line_idx);
            for palette_index in tile_line.iter().skip(disp_x_initial_skip) {
                line[disp_x_offset] = palette_index;
                disp_x_offset += 1;
                if disp_x_offset >= DISPLAY_WIDTH {
                    break 'outer;
                }
            }
            disp_x_initial_skip = 0;
        }
    }

    fn draw_line_objects(
        &mut self,
        line_idx: usize,
        bg_line: &[PaletteIndex; DISPLAY_WIDTH],
        line: &mut [Option<(usize, Color)>; DISPLAY_WIDTH],
    ) {
        for (obj_prio, object) in self
            .selected_oam_entries
            .iter()
            .map(|i| &self.oam.objects()[*i])
            .enumerate()
        {
            let y_flip = object.attrs.read(oam::OBJ_ATTRS::Y_FLIP) != 0;
            let x_flip = object.attrs.read(oam::OBJ_ATTRS::X_FLIP) != 0;

            let tile_line = OBJ_OFFSET_Y + line_idx - object.y as usize;
            let (tile_idx, tile_line) = if tile_line >= TILE_HEIGHT {
                (object.tile_idx.next(), tile_line - TILE_HEIGHT)
            } else {
                (object.tile_idx, tile_line)
            };

            assert!(tile_line < TILE_HEIGHT);
            let tile_line = if y_flip {
                TILE_HEIGHT - 1 - tile_line
            } else {
                tile_line
            };

            let palette = match object
                .attrs
                .read_as_enum(oam::OBJ_ATTRS::PALETTE_SELECTOR)
                .unwrap()
            {
                oam::OBJ_ATTRS::PALETTE_SELECTOR::Value::Palette0 => self.regs.obj_palette0,
                oam::OBJ_ATTRS::PALETTE_SELECTOR::Value::Palette1 => self.regs.obj_palette1,
            };

            let tile = self.vram.get_tile(
                tile_idx,
                regs::LCDC::BG_AND_WINDOW_TILE_DATA::Value::Blocks0And1,
            );

            let bg_prio = object.attrs.read(oam::OBJ_ATTRS::PRIO) != 0;
            let tile_line = tile.get_line(tile_line);

            for (i, pixel) in tile_line.iter().enumerate() {
                if pixel == PaletteIndex::Id0 {
                    // Transparent
                    continue;
                }

                let x = if x_flip { OBJ_OFFSET_X - 1 - i } else { i } + object.x as usize;
                if x < OBJ_OFFSET_X || x >= DISPLAY_WIDTH + OBJ_OFFSET_X {
                    continue;
                }
                let x = x - OBJ_OFFSET_X;

                if bg_line[x] != PaletteIndex::Id0 && bg_prio {
                    continue;
                }

                let color = palette.color(pixel);
                line[x] = Some(match line[x] {
                    None => (obj_prio, color),
                    Some((prio, _)) if prio > obj_prio => (obj_prio, color),
                    Some(v) => v,
                });
            }
        }
    }

    fn draw_line(&mut self, line_idx: usize) {
        if self.regs.lcdc.read(regs::LCDC::ENABLE) == 0 {
            return;
        }

        let mut line: [PaletteIndex; DISPLAY_WIDTH] = [PaletteIndex::Id0; DISPLAY_WIDTH];

        self.draw_line_background(line_idx, &mut line);

        if self.regs.lcdc.read(regs::LCDC::WINDOW_ENABLE) == 1 {
            self.draw_line_window(line_idx, &mut line);
        }

        // either a selected (color, and x coordinate) or nothing
        let mut line_objs: [Option<(usize, Color)>; DISPLAY_WIDTH] = [None; DISPLAY_WIDTH];

        if self.regs.lcdc.read(regs::LCDC::OBJ_ENABLE) == 1 {
            self.draw_line_objects(line_idx, &line, &mut line_objs);
        }

        for ((pixel, bg_palette_idx), object_color) in self.framebuffer[line_idx]
            .iter_mut()
            .zip(line.iter())
            .zip(line_objs.iter())
        {
            let palette = self.regs.bg_palette;
            if let Some((_, object_color)) = object_color {
                *pixel = *object_color;
            } else {
                *pixel = palette.color(*bg_palette_idx);
            }
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

    pub fn read(&self, address: sm83::memory::Address) -> u8 {
        match address {
            0x8000..=0x9FFF => self.vram.read(address),
            0xFE00..=0xFE9F => self.oam.read(address),
            0xFF40..=0xFF4B => self.regs.read(address),
            _ => {
                panic!("Unmapped address in PPU: {address}")
            }
        }
    }

    pub fn write(&mut self, address: sm83::memory::Address, value: u8) {
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
