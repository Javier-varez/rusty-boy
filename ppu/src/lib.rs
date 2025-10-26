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
    line: usize,

    stat_irq: bool,
    current_window_line: usize,

    // Vector of indexes into OAM entries
    selected_oam_entries: heapless::Vec<usize, MAX_SELECTED_OBJECTS>,

    /// Origin of coordinates is top-left pixel.
    framebuffer: Box<Frame>,
}

const OAM_SCAN_LEN: usize = 80;
const DRAWING_PIXELS_LEN: usize = 172;
const HBLANK_LEN: usize = 204;
const LINE_LENGTH: usize = OAM_SCAN_LEN + DRAWING_PIXELS_LEN + HBLANK_LEN;
const NUM_LINES: usize = 154;
const MAX_SELECTED_OBJECTS: usize = 10;
const OBJ_OFFSET_Y: usize = 16;
const OBJ_OFFSET_X: usize = 8;

static_assertions::const_assert_eq!(70224, LINE_LENGTH * NUM_LINES);

#[cfg_attr(feature = "profile", inline(never))]
fn mode_for_current_cycle_count(line_cycles: Cycles, line: usize) -> Mode {
    // This is a simplified implementation that considers a fixed time mode 3.
    let line_cycles: usize = line_cycles.into();
    const VBLANK_START_LINE: usize = 144;

    if line >= VBLANK_START_LINE {
        Mode::Vblank
    } else if line_cycles < OAM_SCAN_LEN {
        Mode::OamScan
    } else if line_cycles < (OAM_SCAN_LEN + DRAWING_PIXELS_LEN) {
        Mode::DrawingPixels
    } else {
        Mode::Hblank
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Ppu::new()
    }
}

impl Ppu {
    /// Constructs a PPU instance
    pub fn new() -> Self {
        type UninitFrame = [[core::mem::MaybeUninit<Color>; DISPLAY_WIDTH]; DISPLAY_HEIGHT];
        let mut framebuffer: Box<UninitFrame> =
            Box::new([[core::mem::MaybeUninit::uninit(); DISPLAY_WIDTH]; DISPLAY_HEIGHT]);

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
            line: 0,

            stat_irq: false,
            current_window_line: 0,
            selected_oam_entries: heapless::Vec::new(),
            framebuffer: unsafe {
                core::mem::transmute::<Box<UninitFrame>, Box<Frame>>(framebuffer)
            },
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn update_line_and_cycles(&mut self, cycles: Cycles) {
        self.cycles = self.cycles + cycles;
        if self.cycles >= Cycles::new(LINE_LENGTH) {
            self.cycles = self.cycles - Cycles::new(LINE_LENGTH);
            self.line += 1;
            if self.line >= NUM_LINES {
                self.line = 0;
            }
        }
    }

    /// Runs the PPU for the given number of cycles and then returns the PPU state
    #[cfg_attr(feature = "profile", inline(never))]
    pub fn step(
        &mut self,
        cycles: Cycles,
        dma_engine: &mut DmaEngine,
        render: bool,
    ) -> (Interrupts, PpuResult) {
        self.update_line_and_cycles(cycles);

        let new_mode = mode_for_current_cycle_count(self.cycles, self.line);

        if self.regs.dma_config.triggered {
            dma_engine.trigger(self.regs.dma_config.address);
            self.regs.dma_config.triggered = false;
        }
        let (interrupts, result) = self.step_inner(new_mode, render);
        (interrupts | self.update_lcd_irq(), result)
    }

    pub fn frame(&self) -> &Frame {
        &self.framebuffer
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn update_lcd_irq(&mut self) -> Interrupts {
        let lyc_eq_ly = self.regs.status.read(STAT::LYC_INT_SELECT) != 0
            && self.regs.status.read(STAT::LYC_EQ_LY) != 0;
        let mode_irq = match self.mode {
            Mode::Hblank => self.regs.status.read(STAT::MODE_0_INT_SELECT) != 0,
            Mode::Vblank => self.regs.status.read(STAT::MODE_1_INT_SELECT) != 0,
            Mode::OamScan => self.regs.status.read(STAT::MODE_2_INT_SELECT) != 0,
            Mode::DrawingPixels => false,
        };
        let status = lyc_eq_ly || mode_irq;

        if status == self.stat_irq {
            return Interrupts::new();
        }

        if status {
            self.stat_irq = true;
            Interrupt::Lcd.into()
        } else {
            self.stat_irq = false;
            Interrupts::new()
        }
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn oam_scan(&mut self) {
        self.selected_oam_entries = heapless::Vec::new();

        let obj_height = (self.regs.lcdc.read(regs::LCDC::OBJ_SIZE) + 1) * 8;

        let cur_obj_line = (self.line + OBJ_OFFSET_Y) as u8;
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

    #[cfg_attr(feature = "profile", inline(never))]
    fn step_inner(&mut self, new_mode: Mode, render: bool) -> (Interrupts, PpuResult) {
        const NO_IRQ: Interrupts = Interrupts::new();
        if self.mode == new_mode {
            self.update_registers();
            return (NO_IRQ, PpuResult::InProgress(self.mode));
        }

        self.mode = new_mode;

        match self.mode {
            Mode::OamScan if render => {
                self.oam_scan();
            }
            Mode::DrawingPixels if render => {
                self.draw_line();
            }
            Mode::Vblank => {
                self.current_window_line = 0;
                self.update_registers();
                return (Interrupt::Vblank.into(), PpuResult::FrameComplete);
            }
            _ => {}
        }

        self.update_registers();
        (NO_IRQ, PpuResult::InProgress(self.mode))
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn draw_line_background(&self, line: &mut [PaletteIndex; DISPLAY_WIDTH]) -> Palette {
        let bg_win_enable = self.regs.lcdc.read(regs::LCDC::BG_AND_WINDOW_ENABLE) != 0;
        if !bg_win_enable {
            // BG is disabled
            line.iter_mut().for_each(|p| *p = PaletteIndex::Id0);
            return Palette(0);
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

        let x_inner_offset = bg_x_offset % TILE_WIDTH;
        let x_tile_offset = bg_x_offset / TILE_WIDTH;

        let tile_line_idx = (bg_y_offset + self.line) % vram::TILE_HEIGHT;

        let background_pixels = self
            .vram
            .get_bg_tile_map(bg_tile_map)
            .line(bg_y_offset + self.line)
            .iter()
            .cycle()
            .skip(x_tile_offset)
            .flat_map(|tile_index| {
                self.vram
                    .get_tile(*tile_index, bg_tile_data_area)
                    .get_line(tile_line_idx)
                    .iter()
            })
            .skip(x_inner_offset)
            .take(DISPLAY_WIDTH);

        line.iter_mut()
            .zip(background_pixels)
            .for_each(|(dest, palette_index)| {
                *dest = palette_index;
            });

        self.regs.bg_palette
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn draw_line_window(&mut self, line: &mut [PaletteIndex; DISPLAY_WIDTH]) {
        let bg_win_enable = self.regs.lcdc.read(regs::LCDC::BG_AND_WINDOW_ENABLE) != 0;
        if !bg_win_enable {
            return;
        }

        let wy = self.regs.wy as usize;
        if self.line < wy {
            // Nothing to draw, return
            return;
        }

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

        const WX_OFFSET: usize = 8;
        let wx = self.regs.wx as usize;
        let disp_x_offset = wx.saturating_sub(WX_OFFSET);
        if disp_x_offset >= DISPLAY_WIDTH {
            return;
        }

        let current_window_line = self.current_window_line;
        self.current_window_line += 1;

        let disp_x_initial_skip = WX_OFFSET.saturating_sub(wx);
        let tile_line_idx = current_window_line % vram::TILE_HEIGHT;
        let window_pixels = self
            .vram
            .get_win_tile_map(win_tile_map)
            .line(current_window_line)
            .iter()
            .flat_map(|tile_idx| {
                self.vram
                    .get_tile(*tile_idx, win_tile_data_area)
                    .get_line(tile_line_idx)
                    .iter()
            })
            .skip(disp_x_initial_skip);

        line.iter_mut()
            .skip(disp_x_offset)
            .zip(window_pixels)
            .for_each(|(dest, palette_index)| *dest = palette_index);
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn draw_line_objects(
        &self,
        bg_line: &[PaletteIndex; DISPLAY_WIDTH],
        line: &mut [Option<(usize, Color)>; DISPLAY_WIDTH],
    ) {
        let double_size_tiles: regs::LCDC::OBJ_SIZE::Value =
            self.regs.lcdc.read_as_enum(regs::LCDC::OBJ_SIZE).unwrap();
        let obj_height = (double_size_tiles as usize + 1) * 8;

        let objects = &self.oam.objects();
        let mut oam_entries: heapless::Vec<usize, MAX_SELECTED_OBJECTS> =
            self.selected_oam_entries.clone();
        // OAM object priorities are based on the X coordinate.
        oam_entries.sort_by_key(|i| objects[*i].x);

        for (obj_prio, object) in oam_entries
            .iter()
            .map(|i| &self.oam.objects()[*i])
            .enumerate()
        {
            let y_flip = object.attrs.read(oam::OBJ_ATTRS::Y_FLIP) != 0;
            let x_flip = object.attrs.read(oam::OBJ_ATTRS::X_FLIP) != 0;

            let tile_line = OBJ_OFFSET_Y + self.line - object.y as usize;
            debug_assert!(tile_line < obj_height);
            let tile_line = if y_flip {
                obj_height - 1 - tile_line
            } else {
                tile_line
            };

            // In 8x16 mode, the tile index is always aligned to 2, enforced by the hardware.
            let tile_idx = if double_size_tiles == regs::LCDC::OBJ_SIZE::Value::Tile8x16 {
                object.tile_idx.discard_bit_zero()
            } else {
                object.tile_idx
            };

            let (tile_idx, tile_line) = if tile_line >= TILE_HEIGHT {
                (tile_idx.next(), tile_line - TILE_HEIGHT)
            } else {
                (tile_idx, tile_line)
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
                if !(OBJ_OFFSET_X..OBJ_OFFSET_X + DISPLAY_WIDTH).contains(&x) {
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

    #[cfg_attr(feature = "profile", inline(never))]
    fn draw_line(&mut self) {
        if self.regs.lcdc.read(regs::LCDC::ENABLE) == 0 {
            return;
        }

        let mut line: [PaletteIndex; DISPLAY_WIDTH] = [PaletteIndex::Id0; DISPLAY_WIDTH];

        let bg_palette = self.draw_line_background(&mut line);

        if self.regs.lcdc.read(regs::LCDC::WINDOW_ENABLE) == 1 {
            self.draw_line_window(&mut line);
        }

        // Either a selected (color, and x coordinate) or nothing
        let mut line_objs: [Option<(usize, Color)>; DISPLAY_WIDTH] = [None; DISPLAY_WIDTH];

        if self.regs.lcdc.read(regs::LCDC::OBJ_ENABLE) == 1 {
            self.draw_line_objects(&line, &mut line_objs);
        }

        for ((pixel, bg_palette_idx), object_color) in self.framebuffer[self.line]
            .iter_mut()
            .zip(line.iter())
            .zip(line_objs.iter())
        {
            if let Some((_, object_color)) = object_color {
                *pixel = *object_color;
            } else {
                *pixel = bg_palette.color(*bg_palette_idx);
            }
        }
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn update_registers(&mut self) {
        let line = self.line as u8;
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
