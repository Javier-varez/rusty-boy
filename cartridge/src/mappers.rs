use crate::header::{self, CartridgeHeader, CartridgeType};

extern crate alloc;
use alloc::vec::Vec;

mod mbc1;
mod mbc3;
mod rom_only;

pub enum Mapper {
    RomOnly(rom_only::RomOnly),
    Mbc1(mbc1::Mbc1),
    Mbc1RamBattery(mbc1::Mbc1),
    Mbc3(mbc3::Mbc3),
    Mbc3RamBattery(mbc3::Mbc3),
}

impl Mapper {
    pub fn new(data: Vec<u8>) -> Result<Self, header::Error> {
        let header = CartridgeHeader::new(&data)?;
        let cartridge_type = header.cartridge_type;
        let ram_size = header.ram_size.into_usize();
        Ok(match cartridge_type {
            CartridgeType::RomOnly => Self::RomOnly(rom_only::RomOnly::new(data)),
            CartridgeType::Mbc1 | CartridgeType::Mbc1Ram => {
                Self::Mbc1(mbc1::Mbc1::new(data, ram_size.unwrap()))
            }
            CartridgeType::Mbc1RamBattery => {
                Self::Mbc1RamBattery(mbc1::Mbc1::new(data, ram_size.unwrap()))
            }
            CartridgeType::Mbc3 | CartridgeType::Mbc3Ram => {
                Self::Mbc3(mbc3::Mbc3::new(data, ram_size.unwrap()))
            }
            CartridgeType::Mbc3RamBattery => {
                Self::Mbc3RamBattery(mbc3::Mbc3::new(data, ram_size.unwrap()))
            }
            v => {
                // Other cartridge types are currently unsupported
                todo!("Cartridge support for mapper {v} is not implemented")
            }
        })
    }

    pub fn header<'a>(&'a self) -> Result<CartridgeHeader<'a>, header::Error> {
        match self {
            Self::RomOnly(rom_only) => rom_only.header(),
            Self::Mbc1(mbc1) => mbc1.header(),
            Self::Mbc1RamBattery(mbc1) => mbc1.header(),
            Self::Mbc3(mbc3) => mbc3.header(),
            Self::Mbc3RamBattery(mbc3) => mbc3.header(),
        }
    }

    pub fn read(&self, address: sm83::memory::Address) -> u8 {
        match self {
            Self::RomOnly(rom_only) => rom_only.read(address),
            Self::Mbc1(mbc1) => mbc1.read(address),
            Self::Mbc1RamBattery(mbc1) => mbc1.read(address),
            Self::Mbc3(mbc3) => mbc3.read(address),
            Self::Mbc3RamBattery(mbc3) => mbc3.read(address),
        }
    }

    pub fn write(&mut self, address: sm83::memory::Address, value: u8) {
        match self {
            Self::RomOnly(rom_only) => rom_only.write(address, value),
            Self::Mbc1(mbc1) => mbc1.write(address, value),
            Self::Mbc1RamBattery(mbc1) => mbc1.write(address, value),
            Self::Mbc3(mbc3) => mbc3.write(address, value),
            Self::Mbc3RamBattery(mbc3) => mbc3.write(address, value),
        }
    }

    pub fn battery_backed_ram(&self) -> Option<&[u8]> {
        match self {
            Self::Mbc1RamBattery(mbc1) => Some(mbc1.ram()),
            Self::Mbc3RamBattery(mbc3) => Some(mbc3.ram()),
            _ => None,
        }
    }

    pub fn restore_battery_backed_ram(&mut self, ram: &[u8]) -> Result<(), crate::Error> {
        match self {
            Self::Mbc1RamBattery(mbc1) => mbc1.restore_ram(ram),
            Self::Mbc3RamBattery(mbc3) => mbc3.restore_ram(ram),
            _ => Err(crate::Error::CartridgeHasNoRam),
        }
    }
}
