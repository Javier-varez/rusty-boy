use crate::header::{self, CartridgeHeader, CartridgeType};

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;

mod mbc1;
mod mbc3;
mod rom_only;

pub trait Mapper {
    fn header<'a>(&'a self) -> Result<CartridgeHeader<'a>, header::Error>;

    fn read(&self, address: sm83::memory::Address) -> u8;

    fn write(&mut self, address: sm83::memory::Address, value: u8);

    fn battery_backed_ram(&self) -> Option<&[u8]> {
        None
    }

    fn restore_battery_backed_ram(&mut self, _ram: &[u8]) -> Result<(), crate::Error> {
        Err(crate::Error::CartridgeHasNoRam)
    }
}

pub fn new_mapper(data: Vec<u8>) -> Result<Box<dyn Mapper>, header::Error> {
    let header = CartridgeHeader::new(&data)?;
    let cartridge_type = header.cartridge_type;
    let ram_size = header.ram_size.into_usize();
    Ok(match cartridge_type {
        CartridgeType::RomOnly => Box::new(rom_only::RomOnly::new(data)),
        CartridgeType::Mbc1 | CartridgeType::Mbc1Ram | CartridgeType::Mbc1RamBattery => {
            Box::new(mbc1::Mbc1::new(data, ram_size.unwrap()))
        }
        CartridgeType::Mbc3 | CartridgeType::Mbc3Ram | CartridgeType::Mbc3RamBattery => {
            Box::new(mbc3::Mbc3::new(data, ram_size.unwrap()))
        }
        v => {
            // Other cartridge types are currently unsupported
            todo!("Cartridge support for mapper {v} is not implemented")
        }
    })
}
