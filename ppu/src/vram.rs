//! Implements the memory mapped interface to VRAM

/// The width of the tile
const TILE_WIDTH: usize = 8;
/// The number of lines in each tile (height)
const TILE_HEIGHT: usize = 8;

/// The number of bits used to index the tile palette.
const COLOR_BITS: usize = 2;

/// The number of bytes that make up a single tile line.
const TILE_LINE_SIZE: usize = TILE_WIDTH * COLOR_BITS / 8;

/// The number of tiles in a block
const TILES_PER_BLOCK: usize = 128;

/// The number of tile blocks in VRAM
const NUM_TILE_BLOCKS: usize = 3;

/// The number of tile maps in VRAM
const NUM_TILE_MAPS: usize = 2;

/// Represents a single line of a tile. Each byte in the u16 indicates
#[repr(C)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub(crate) struct TileLine(pub(crate) [u8; TILE_LINE_SIZE]);

impl TileLine {
    const fn new() -> Self {
        Self([0; TILE_LINE_SIZE])
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

const TILE_MAP_WIDTH: usize = 32;
const TILE_MAP_HEIGHT: usize = 32;

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TileMap(pub(crate) [[TileIndex; TILE_MAP_WIDTH]; TILE_MAP_HEIGHT]);

impl TileMap {
    const fn new() -> Self {
        Self([[TileIndex(0); TILE_MAP_WIDTH]; TILE_MAP_HEIGHT])
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
        (blk_idx, blk_address)
    }
}

impl sm83::memory::Memory for Vram {
    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        let (blk_idx, blk_address) = Self::vram_address_to_block_address(address);
        self.tile_blocks[blk_idx].read(blk_address)
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        let (blk_idx, blk_address) = Self::vram_address_to_block_address(address);
        self.tile_blocks[blk_idx].write(blk_address, value)
    }
}
