//! Implements the memory mapped interface to VRAM

extern crate alloc;

use core::mem::MaybeUninit;

use alloc::boxed::Box;

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

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        self.0[address as usize] = value;
    }

    fn read(&self, address: sm83::memory::Address) -> u8 {
        self.0[address as usize]
    }
}

// A tile line must occupy 2 bytes
static_assertions::assert_eq_size!([u8; 2], TileLine);

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

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        let (line_idx, line_address) = Self::tile_address_to_tile_line_address(address);
        self.0[line_idx].write(line_address, value)
    }

    fn read(&self, address: sm83::memory::Address) -> u8 {
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
    fn new() -> Self {
        const UNINIT: MaybeUninit<Tile> = MaybeUninit::uninit();
        let mut tile_block = [UNINIT; TILES_PER_BLOCK];
        for tile in tile_block.iter_mut() {
            tile.write(Tile::new());
        }
        Self(unsafe {
            core::mem::transmute::<[MaybeUninit<Tile>; TILES_PER_BLOCK], [Tile; TILES_PER_BLOCK]>(
                tile_block,
            )
        })
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

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        let (tile_idx, tile_address) = Self::block_address_to_tile_address(address);
        self.0[tile_idx].write(tile_address, value)
    }

    fn read(&self, address: sm83::memory::Address) -> u8 {
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
    pub fn next(self) -> Self {
        Self(self.0 + 1)
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
    fn new() -> Self {
        const UNINIT: MaybeUninit<TileIndex> = MaybeUninit::uninit();
        let mut tile_map = [[UNINIT; TILE_MAP_WIDTH]; TILE_MAP_HEIGHT];

        for row in tile_map.iter_mut() {
            for elem in row {
                elem.write(TileIndex(0));
            }
        }

        Self(unsafe {
            core::mem::transmute::<
                [[MaybeUninit<TileIndex>; TILE_MAP_WIDTH]; TILE_MAP_HEIGHT],
                [[TileIndex; TILE_MAP_WIDTH]; TILE_MAP_HEIGHT],
            >(tile_map)
        })
    }

    #[cfg_attr(feature = "profile", inline(never))]
    pub fn line(&self, line: usize) -> &[TileIndex; TILE_MAP_WIDTH] {
        let line = (line / TILE_HEIGHT) % TILE_MAP_HEIGHT;
        &self.0[line]
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        let line = address as usize / TILE_MAP_WIDTH;
        let offset = address as usize % TILE_MAP_WIDTH;
        self.0[line][offset] = TileIndex(value);
    }

    fn read(&self, address: sm83::memory::Address) -> u8 {
        let line = address as usize / TILE_MAP_WIDTH;
        let offset = address as usize % TILE_MAP_WIDTH;
        self.0[line][offset].0
    }
}

#[repr(C)]
#[derive(Clone, Eq, PartialEq, Debug)]
struct VramImpl {
    tile_blocks: [TileBlock; NUM_TILE_BLOCKS],
    tile_maps: [TileMap; NUM_TILE_MAPS],
}

// The VRAM occupies 0x2000 bytes.
static_assertions::assert_eq_size!([u8; 0x2000], VramImpl);

#[repr(C)]
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Vram(Box<VramImpl>);

impl Default for Vram {
    fn default() -> Self {
        Self::new()
    }
}

impl Vram {
    const VRAM_BASE: u16 = 0x8000;
    const TILE_MAP_BASE: u16 = 0x9800;

    pub fn new() -> Self {
        const UNINIT_TILE_BLOCK: MaybeUninit<TileBlock> = MaybeUninit::uninit();
        let mut tile_blocks = [UNINIT_TILE_BLOCK; NUM_TILE_BLOCKS];
        for tile in tile_blocks.iter_mut() {
            tile.write(TileBlock::new());
        }

        const UNINIT_TILE_MAP: MaybeUninit<TileMap> = MaybeUninit::uninit();
        let mut tile_maps = [UNINIT_TILE_MAP; NUM_TILE_MAPS];
        for tile in tile_maps.iter_mut() {
            tile.write(TileMap::new());
        }

        Self(Box::new(VramImpl {
            tile_blocks: unsafe {
                core::mem::transmute::<
                    [MaybeUninit<TileBlock>; NUM_TILE_BLOCKS],
                    [TileBlock; NUM_TILE_BLOCKS],
                >(tile_blocks)
            },
            tile_maps: unsafe {
                core::mem::transmute::<
                    [MaybeUninit<TileMap>; NUM_TILE_MAPS],
                    [TileMap; NUM_TILE_MAPS],
                >(tile_maps)
            },
        }))
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

    #[cfg_attr(feature = "profile", inline(never))]
    pub(crate) fn get_bg_tile_map(&self, map: crate::regs::LCDC::BG_TILE_MAP::Value) -> &TileMap {
        match map {
            crate::regs::LCDC::BG_TILE_MAP::Value::HighMap => &self.0.tile_maps[1],
            crate::regs::LCDC::BG_TILE_MAP::Value::LowMap => &self.0.tile_maps[0],
        }
    }

    #[cfg_attr(feature = "profile", inline(never))]
    pub(crate) fn get_win_tile_map(
        &self,
        map: crate::regs::LCDC::WINDOW_TILE_MAP::Value,
    ) -> &TileMap {
        match map {
            crate::regs::LCDC::WINDOW_TILE_MAP::Value::HighMap => &self.0.tile_maps[1],
            crate::regs::LCDC::WINDOW_TILE_MAP::Value::LowMap => &self.0.tile_maps[0],
        }
    }

    #[cfg_attr(feature = "profile", inline(never))]
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
        self.0.tile_blocks[block].get_tile(index)
    }

    #[cfg_attr(feature = "profile", inline(never))]
    pub fn read(&self, address: sm83::memory::Address) -> u8 {
        if address < 0x9800 {
            let (blk_idx, blk_address) = Self::vram_address_to_block_address(address);
            self.0.tile_blocks[blk_idx].read(blk_address)
        } else {
            let (tile_map_idx, tile_map_address) = Self::vram_address_to_tile_map_address(address);
            self.0.tile_maps[tile_map_idx].read(tile_map_address)
        }
    }

    #[cfg_attr(feature = "profile", inline(never))]
    pub fn write(&mut self, address: sm83::memory::Address, value: u8) {
        if address < 0x9800 {
            let (blk_idx, blk_address) = Self::vram_address_to_block_address(address);
            self.0.tile_blocks[blk_idx].write(blk_address, value)
        } else {
            let (tile_map_idx, tile_map_address) = Self::vram_address_to_tile_map_address(address);
            self.0.tile_maps[tile_map_idx].write(tile_map_address, value)
        }
    }
}
