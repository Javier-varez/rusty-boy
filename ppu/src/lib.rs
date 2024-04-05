#![no_std]

mod modes;

use static_assertions::const_assert_eq;

/// The width of the GameBoy display in pixels
pub const DISPLAY_WIDTH: usize = 160;

/// The height of the GameBoy display in pixels
pub const DISPLAY_HEIGHT: usize = 144;

/// The width of a single tile in pixels
pub const TILE_WIDTH: usize = 8;

/// The height of a single tile in pixels
pub const TILE_HEIGHT: usize = 8;

/// Number of bits used to represent one pixel inside a tile
pub const BITS_PER_PIXEL: usize = 2;

/// Represents an object on top of the background and window.
struct Object {}

/// The size of the VRAM in bytes (6 KiB)
const VRAM_SIZE: usize = 0x1800;

pub struct PixelColor(u8);

// A single line of a tile
#[repr(C)]
pub struct TileLine(u16);

const_assert_eq!(
    core::mem::size_of::<TileLine>() * 8,
    TILE_WIDTH * BITS_PER_PIXEL
);

/// 16 bytes, 2 bytes per line. Top line to bottom line.
#[repr(C)]
pub struct Tile([TileLine; TILE_HEIGHT]);

/// The Picture Processing Unit
pub struct PPU {
    // Memory range: 0x8000–0x97FF
    vram: [u8; VRAM_SIZE],
}
