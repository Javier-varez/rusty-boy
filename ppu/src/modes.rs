#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    // Mode 0
    Hblank,
    // Mode 1
    Vblank,
    // Mode 2
    OamScan,
    // Mode 3
    DrawingPixels,
}
