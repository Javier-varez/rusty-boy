use crate::header::CartridgeHeader;

extern crate alloc;

use alloc::vec::Vec;

use super::Mapper;

pub struct RomOnly {
    data: Vec<u8>,
}

impl RomOnly {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Mapper for RomOnly {
    fn header(&self) -> CartridgeHeader<'_> {
        CartridgeHeader::try_new(&self.data).unwrap()
    }

    fn read(&self, address: sm83::memory::Address) -> u8 {
        if (address as usize) < self.data.len() {
            self.data[address as usize]
        } else {
            0
        }
    }

    // Writes are ignored
    fn write(&mut self, _: sm83::memory::Address, _: u8) {}
}
