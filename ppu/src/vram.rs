//! Implements the memory mapped interface to VRAM

use super::PaletteIndex;

/// The width of the tile
pub const TILE_WIDTH: usize = 8;
/// The number of lines in each tile (height)
pub const TILE_HEIGHT: usize = 8;

/// The number of bits used to index the tile palette.
pub const COLOR_BITS: usize = 2;

/// The number of bytes that make up a single tile line.
pub const TILE_LINE_SIZE: usize = TILE_WIDTH * COLOR_BITS / 8;

/// The number of tiles in a block
pub const TILES_PER_BLOCK: usize = 128;

/// The number of tile blocks in VRAM
pub const NUM_TILE_BLOCKS: usize = 3;

/// The number of tile maps in VRAM
pub const NUM_TILE_MAPS: usize = 2;

/// Represents a single line of a tile. Each byte in the u16 indicates
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub(crate) struct TileLine(pub(crate) [u8; TILE_LINE_SIZE]);

impl TileLine {
    const fn new() -> Self {
        Self([0; TILE_LINE_SIZE])
    }

    pub fn iter(&self) -> TileLineIter {
        TileLineIter {
            tile_line: *self,
            idx: 8,
        }
    }

    pub fn pixel(&self, index: usize) -> PaletteIndex {
        let bit = 1 << index;
        let lsb = (self.0[0] & bit) != 0;
        let msb = (self.0[1] & bit) != 0;

        match (msb, lsb) {
            (false, false) => PaletteIndex::Id0,
            (false, true) => PaletteIndex::Id1,
            (true, false) => PaletteIndex::Id2,
            (true, true) => PaletteIndex::Id3,
        }
    }
}

// A tile line must occupy 2 bytes
static_assertions::assert_eq_size!([u8; 2], TileLine);

impl sm83::memory::Memory for TileLine {
    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        self.0[address as usize] = value;
    }

    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        self.0[address as usize]
    }
}

pub struct TileLineIter {
    tile_line: TileLine,
    idx: usize,
}

impl Iterator for TileLineIter {
    type Item = PaletteIndex;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx > 0 {
            self.idx -= 1;
            let result = self.tile_line.pixel(self.idx);
            Some(result)
        } else {
            None
        }
    }
}

#[repr(C)]
#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) struct Tile(pub(crate) [TileLine; TILE_HEIGHT]);

// A tile must occupy 16 bytes
static_assertions::assert_eq_size!([u8; 16], Tile);

impl Tile {
    const fn new() -> Self {
        Self([TileLine::new(); TILE_HEIGHT])
    }

    fn tile_address_to_tile_line_address(
        address: sm83::memory::Address,
    ) -> (usize, sm83::memory::Address) {
        let line_idx = address as usize / core::mem::size_of::<TileLine>();
        let line_address = address % core::mem::size_of::<TileLine>() as u16;
        (line_idx, line_address)
    }

    pub fn get_line(&self, line: usize) -> TileLine {
        self.0[line]
    }
}

impl sm83::memory::Memory for Tile {
    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        let (line_idx, line_address) = Self::tile_address_to_tile_line_address(address);
        self.0[line_idx].write(line_address, value)
    }

    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        let (line_idx, line_address) = Self::tile_address_to_tile_line_address(address);
        self.0[line_idx].read(line_address)
    }
}

#[repr(C)]
#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) struct TileBlock(pub(crate) [Tile; TILES_PER_BLOCK]);

// A tile block must occupy 2048 bytes
static_assertions::assert_eq_size!([u8; 2048], TileBlock);

impl TileBlock {
    const fn new() -> Self {
        const TILE: Tile = Tile::new();
        Self([TILE; TILES_PER_BLOCK])
    }

    fn block_address_to_tile_address(
        address: sm83::memory::Address,
    ) -> (usize, sm83::memory::Address) {
        let tile_idx = address as usize / core::mem::size_of::<Tile>();
        let tile_address = address % core::mem::size_of::<Tile>() as u16;
        (tile_idx, tile_address)
    }

    pub(crate) fn get_tile(&self, index: usize) -> &Tile {
        &self.0[index]
    }
}

impl sm83::memory::Memory for TileBlock {
    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        let (tile_idx, tile_address) = Self::block_address_to_tile_address(address);
        self.0[tile_idx].write(tile_address, value)
    }

    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        let (tile_idx, tile_address) = Self::block_address_to_tile_address(address);
        self.0[tile_idx].read(tile_address)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TileIndex(u8);

impl TileIndex {
    pub const fn new(index: u8) -> Self {
        Self(index)
    }
}

impl From<u8> for TileIndex {
    fn from(val: u8) -> Self {
        Self(val)
    }
}

impl From<TileIndex> for u8 {
    fn from(val: TileIndex) -> Self {
        val.0
    }
}

pub const TILE_MAP_WIDTH: usize = 32;
pub const TILE_MAP_HEIGHT: usize = 32;

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TileMap(pub(crate) [[TileIndex; TILE_MAP_WIDTH]; TILE_MAP_HEIGHT]);

impl TileMap {
    const fn new() -> Self {
        Self([[TileIndex(0); TILE_MAP_WIDTH]; TILE_MAP_HEIGHT])
    }

    pub fn line(&self, line: usize) -> &[TileIndex; TILE_MAP_WIDTH] {
        let line = (line / TILE_HEIGHT) & (TILE_MAP_HEIGHT - 1);
        &self.0[line]
    }
}

impl sm83::memory::Memory for TileMap {
    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        let line = address as usize / TILE_MAP_WIDTH;
        let offset = address as usize % TILE_MAP_WIDTH;
        self.0[line][offset] = TileIndex(value);
    }

    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        let line = address as usize / TILE_MAP_WIDTH;
        let offset = address as usize % TILE_MAP_WIDTH;
        self.0[line][offset].0
    }
}

#[repr(C)]
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Vram {
    pub(crate) tile_blocks: [TileBlock; NUM_TILE_BLOCKS],
    pub(crate) tile_maps: [TileMap; NUM_TILE_MAPS],
}

// The VRAM occupies 0x2000 bytes.
static_assertions::assert_eq_size!([u8; 0x2000], Vram);

impl Vram {
    const VRAM_BASE: u16 = 0x8000;
    const TILE_MAP_BASE: u16 = 0x9800;
    pub const fn new() -> Self {
        const TILE_BLOCK: TileBlock = TileBlock::new();
        const TILE_MAP: TileMap = TileMap::new();
        Self {
            tile_blocks: [TILE_BLOCK; NUM_TILE_BLOCKS],
            tile_maps: [TILE_MAP; NUM_TILE_MAPS],
        }
    }

    const fn vram_address_to_block_address(
        address: sm83::memory::Address,
    ) -> (usize, sm83::memory::Address) {
        let address = address - Self::VRAM_BASE;
        let blk_idx = address as usize / core::mem::size_of::<TileBlock>();
        let blk_address = address % core::mem::size_of::<TileBlock>() as u16;
        debug_assert!(blk_idx < 3);
        (blk_idx, blk_address)
    }

    const fn vram_address_to_tile_map_address(
        address: sm83::memory::Address,
    ) -> (usize, sm83::memory::Address) {
        let address = address - Self::TILE_MAP_BASE;
        let tile_map_idx = address as usize / core::mem::size_of::<TileMap>();
        let tile_map_address = address % core::mem::size_of::<TileMap>() as u16;
        debug_assert!(tile_map_idx < 2);
        (tile_map_idx, tile_map_address)
    }

    pub(crate) fn get_tile_map(&self, map: crate::regs::LCDC::BG_TILE_MAP::Value) -> &TileMap {
        match map {
            crate::regs::LCDC::BG_TILE_MAP::Value::HighMap => &self.tile_maps[1],
            crate::regs::LCDC::BG_TILE_MAP::Value::LowMap => &self.tile_maps[0],
        }
    }

    pub(crate) fn get_tile(
        &self,
        index: TileIndex,
        index_mode: crate::regs::LCDC::BG_AND_WINDOW_TILE_DATA::Value,
    ) -> &Tile {
        let block = (index.0 / 128) as usize;
        let index = (index.0 % 128) as usize;
        let block = match index_mode {
            crate::regs::LCDC::BG_AND_WINDOW_TILE_DATA::Value::Blocks0And1 => block,
            crate::regs::LCDC::BG_AND_WINDOW_TILE_DATA::Value::Blocks2And1 => 2 - block,
        };
        self.tile_blocks[block].get_tile(index)
    }
}

impl sm83::memory::Memory for Vram {
    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        if address < 0x9800 {
            let (blk_idx, blk_address) = Self::vram_address_to_block_address(address);
            self.tile_blocks[blk_idx].read(blk_address)
        } else {
            let (tile_map_idx, tile_map_address) = Self::vram_address_to_tile_map_address(address);
            self.tile_maps[tile_map_idx].read(tile_map_address)
        }
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        if address < 0x9800 {
            let (blk_idx, blk_address) = Self::vram_address_to_block_address(address);
            self.tile_blocks[blk_idx].write(blk_address, value)
        } else {
            let (tile_map_idx, tile_map_address) = Self::vram_address_to_tile_map_address(address);
            self.tile_maps[tile_map_idx].write(tile_map_address, value)
        }
    }
}
