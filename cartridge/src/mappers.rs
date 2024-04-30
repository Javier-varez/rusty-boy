use crate::header::{CartridgeHeader, CartridgeType};

mod mbc1;
mod mbc3;
mod rom_only;

pub enum Mapper<'a> {
    RomOnly(rom_only::RomOnly<'a>),
    Mbc1(mbc1::Mbc1<'a>),
    Mbc3(mbc3::Mbc3<'a>),
}

impl<'a> Mapper<'a> {
    pub fn new(header: &CartridgeHeader, data: &'a [u8]) -> Self {
        match header.cartridge_type {
            CartridgeType::RomOnly => Self::RomOnly(rom_only::RomOnly::new(data)),
            CartridgeType::Mbc1 | CartridgeType::Mbc1Ram | CartridgeType::Mbc1RamBattery => {
                Self::Mbc1(mbc1::Mbc1::new(data, header.ram_size.into_usize().unwrap()))
            }
            CartridgeType::Mbc3 | CartridgeType::Mbc3Ram | CartridgeType::Mbc3RamBattery => {
                Self::Mbc3(mbc3::Mbc3::new(data, header.ram_size.into_usize().unwrap()))
            }
            v => {
                // Other cartridge types are currently unsupported
                todo!("Cartridge support for mapper {v} is not implemented")
            }
        }
    }

    pub fn read(&self, address: sm83::memory::Address) -> u8 {
        match self {
            Self::RomOnly(rom_only) => rom_only.read(address),
            Self::Mbc1(mbc1) => mbc1.read(address),
            Self::Mbc3(mbc3) => mbc3.read(address),
        }
    }

    pub fn write(&mut self, address: sm83::memory::Address, value: u8) {
        match self {
            Self::RomOnly(rom_only) => rom_only.write(address, value),
            Self::Mbc1(mbc1) => mbc1.write(address, value),
            Self::Mbc3(mbc3) => mbc3.write(address, value),
        }
    }
}
