use crate::header::{self, CartridgeHeader};

extern crate alloc;

use alloc::vec::Vec;

pub struct RomOnly {
    data: Vec<u8>,
}

impl RomOnly {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn header<'a>(&'a self) -> Result<CartridgeHeader<'a>, header::Error> {
        CartridgeHeader::new(&self.data)
    }

    pub fn read(&self, address: sm83::memory::Address) -> u8 {
        if (address as usize) < self.data.len() {
            self.data[address as usize]
        } else {
            0
        }
    }

    // Writes are ignored
    pub fn write(&mut self, _: sm83::memory::Address, _: u8) {}
}
