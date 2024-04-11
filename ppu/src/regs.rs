//! Implements the memory mapped interface to VRAM, etc

use super::Palette;
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::{register_bitfields, registers::InMemoryRegister};

register_bitfields! [
    u8,

    /// LCDC is the main LCD Control register. Its bits toggle what elements are displayed on
    /// the screen, and how.
    pub LCDC [
        /// This bit controls whether the LCD is on and the PPU is active. Setting it to 0 turns both
        /// off, which grants immediate and full access to VRAM, OAM, etc.
        ENABLE OFFSET(7) NUMBITS(1) [
            Off = 0,
            On = 1,
        ],
        /// This bit controls which background map the Window uses for rendering. When it’s clear (0),
        /// the $9800 tilemap is used, otherwise it’s the $9C00 one.
        WINDOW_TILE_MAP OFFSET(6) NUMBITS(1) [
            LowMap = 0, // Refers to map at 0x9800
            HighMap = 1, // Refers to map at 0x9C00
        ],
        /// This bit controls whether the window shall be displayed or not. This bit is overridden on
        /// DMG by bit 0 if that bit is clear.
        /// Changing the value of this register mid-frame triggers a more complex behaviour.
        WINDOW_ENABLE OFFSET(5) NUMBITS(1) [
            Disabled = 0,
            Enabled = 1,
        ],
        /// This bit controls which addressing mode the BG and Window use to pick tiles.
        /// Objects (sprites) aren’t affected by this, and will always use the $8000 addressing mode.
        BG_AND_WINDOW_TILE_DATA OFFSET(4) NUMBITS(1) [
            Blocks0And1 = 1,
            Blocks2And1 = 0,
        ],
        /// This bit works similarly to LCDC bit 6: if the bit is clear (0), the BG uses tilemap $9800,
        /// otherwise tilemap $9C00.
        BG_TILE_MAP OFFSET(3) NUMBITS(1) [
            LowMap = 0, // Refers to map at 0x9800
            HighMap = 1, // Refers to map at 0x9C00
        ],
        /// This bit controls the size of all objects (1 tile or 2 stacked vertically).
        /// Be cautious when changing object size mid-frame. Changing from 8×8 to 8×16 pixels
        /// mid-frame within 8 scanlines of the bottom of an object causes the object’s second tile
        /// to be visible for the rest of those 8 lines. If the size is changed during mode 2 or 3,
        /// remnants of objects in range could “leak” into the other tile and cause artifacts.
        OBJ_SIZE OFFSET(2) NUMBITS(1) [
            Tile8x8 = 0,
            Tile8x16 = 1,
        ],
        /// This bit toggles whether objects are displayed or not.
        /// This can be toggled mid-frame, for example to avoid objects being displayed on top of a
        /// status bar or text box.
        OBJ_ENABLE OFFSET(1) NUMBITS(1) [
            Disabled = 0,
            Enabled = 1,
        ],
        /// When Bit 0 is cleared, both background and window become blank (white), and the Window
        /// Display Bit is ignored in that case. Only objects may still be displayed (if enabled in Bit 1).
        BG_AND_WINDOW_ENABLE OFFSET(0) NUMBITS(1) [
            Disabled = 0,
            Enabled = 1,
        ]
    ],


    /// LCD Status
    pub STAT [
        /// Selects the value of the LYC register that triggers the LCD interrupt
        LYC_INT_SELECT OFFSET(6) NUMBITS(1) [
            Deselect = 0,
            Select = 1,
        ],
        /// If set, selects the Mode 2 condition for the LCD interrupt.
        MODE_2_INT_SELECT OFFSET(5) NUMBITS(1) [
            Deselect = 0,
            Select = 1,
        ],
        /// If set, selects the Mode 1 condition for the LCD interrupt.
        MODE_1_INT_SELECT OFFSET(4) NUMBITS(1) [
            Deselect = 0,
            Select = 1,
        ],
        /// If set, selects the Mode 0 condition for the LCD interrupt.
        MODE_0_INT_SELECT OFFSET(3) NUMBITS(1) [
            Deselect = 0,
            Select = 1,
        ],
        /// Set when LY contains the same value as LYC. It is constantly updated.
        LYC_EQ_LY OFFSET(2) NUMBITS(1) [],
        /// The current status of the PPU
        PPU_MODE OFFSET(0) NUMBITS(2) [
            Mode0 = 0,
            Mode1 = 1,
            Mode2 = 2,
            Mode3 = 3,
        ],
    ],
];

pub struct Registers {
    // LCDC => 0xFF40
    pub(crate) lcdc: InMemoryRegister<u8, LCDC::Register>,
    // LCD Status => 0xFF41
    pub(crate) status: InMemoryRegister<u8, STAT::Register>,
    // Background viewport Y coordinate => 0xFF42
    pub(crate) scy: u8,
    // Background viewport X coordinate => 0xFF43
    pub(crate) scx: u8,
    // LCD Y coordinate => 0xFF44
    pub(crate) ly: u8,
    // LCD Y compare coordinate => 0xFF45
    pub(crate) lyc: u8,
    // BG palette => 0xFF47
    pub(crate) bg_palette: Palette,
    // Obj palette 0 => 0xFF48
    pub(crate) obj_palette0: Palette,
    // Obj palette 1 => 0xFF49
    pub(crate) obj_palette1: Palette,
    // Window Y coordinate => 0xFF4A
    pub(crate) wy: u8,
    // Window X coordinate => 0xFF4B
    pub(crate) wx: u8,
}

impl Registers {
    pub const fn new() -> Self {
        Self {
            lcdc: InMemoryRegister::new(0),
            status: InMemoryRegister::new(0),
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bg_palette: Palette(0),
            obj_palette0: Palette(0),
            obj_palette1: Palette(0),
            wy: 0,
            wx: 0,
        }
    }

    fn set_status_reg(&mut self, new_val: u8) {
        const RO_BITS: u8 = 0x7;
        self.status
            .set((self.status.get() & RO_BITS) | (new_val & !RO_BITS));
    }
}

impl sm83::memory::Memory for Registers {
    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        match address {
            0xFF40 => self.lcdc.get(),
            0xFF41 => self.status.get(),
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF47 => self.bg_palette.into(),
            0xFF48 => self.obj_palette0.into(),
            0xFF49 => self.obj_palette1.into(),
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            _ => {
                panic!("Unmapped address in PPU registers: {address}");
            }
        }
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        match address {
            0xFF40 => self.lcdc.set(value),
            0xFF41 => self.set_status_reg(value),
            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,
            0xFF44 => {}
            0xFF45 => self.lyc = value,
            0xFF47 => self.bg_palette = value.into(),
            0xFF48 => self.obj_palette0 = value.into(),
            0xFF49 => self.obj_palette1 = value.into(),
            0xFF4A => self.wy = value,
            0xFF4B => self.wx = value,
            _ => {
                panic!("Unmapped address in PPU registers: {address}");
            }
        };
    }
}
