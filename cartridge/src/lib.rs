#![no_std]

pub mod header;
pub mod mappers;

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

pub struct Cartridge<'a> {
    header: CartridgeHeader<'a>,
    mapper: Mapper<'a>,
}

impl<'a> Cartridge<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, header::Error> {
        let header = CartridgeHeader::new(data)?;
        let mapper = Mapper::new(&header, data);
        Ok(Self { header, mapper })
    }

    pub fn header(&self) -> &CartridgeHeader {
        &self.header
    }

    pub fn has_battery(&self) -> bool {
        match self.header.cartridge_type {
            CartridgeType::Mbc1RamBattery => true,
            CartridgeType::Mbc2Battery => true,
            CartridgeType::RomRamBattery => true,
            CartridgeType::Mmm01RamBattery => true,
            CartridgeType::Mbc3TimerBattery => true,
            CartridgeType::Mbc3TimerRamBattery => true,
            CartridgeType::Mbc3RamBattery => true,
            CartridgeType::Mbc5RamBattery => true,
            CartridgeType::Mbc5RumbleRamBattery => true,
            CartridgeType::Mbc7SensorRumbleRamBattery => true,
            CartridgeType::Huc1RamBattery => true,
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
