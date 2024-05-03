#![no_std]

pub mod header;
pub mod mappers;

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use header::CartridgeHeader;
use mappers::Mapper;

use self::header::CartridgeType;

#[derive(Debug)]
pub enum Error {
    CartridgeHasNoRam,
    UnexpectedRamSize { expected: usize, actual: usize },
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub struct Cartridge {
    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    pub fn new(rom_data: Vec<u8>) -> Result<Self, header::Error> {
        let mapper = mappers::new_mapper(rom_data)?;
        Ok(Self { mapper })
    }

    pub fn header<'a>(&'a self) -> CartridgeHeader<'a> {
        self.mapper.header().unwrap()
    }

    pub fn has_battery(&self) -> bool {
        match self.header().cartridge_type {
            CartridgeType::Mbc1RamBattery
            | CartridgeType::Mbc2Battery
            | CartridgeType::RomRamBattery
            | CartridgeType::Mmm01RamBattery
            | CartridgeType::Mbc3TimerBattery
            | CartridgeType::Mbc3TimerRamBattery
            | CartridgeType::Mbc3RamBattery
            | CartridgeType::Mbc5RamBattery
            | CartridgeType::Mbc5RumbleRamBattery
            | CartridgeType::Mbc7SensorRumbleRamBattery
            | CartridgeType::Huc1RamBattery => true,
            _ => false,
        }
    }

    pub fn battery_backed_ram(&self) -> Option<&[u8]> {
        self.mapper.battery_backed_ram()
    }

    pub fn restore_battery_backed_ram(&mut self, ram: &[u8]) -> Result<(), Error> {
        self.mapper.restore_battery_backed_ram(ram)
    }

    pub fn read(&self, address: sm83::memory::Address) -> u8 {
        self.mapper.read(address)
    }

    pub fn write(&mut self, address: sm83::memory::Address, value: u8) {
        self.mapper.write(address, value)
    }
}
