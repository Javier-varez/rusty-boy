//!
//! Abstraction over a physical Game Boy cartridge.
//!
//! Game Boy cartridges can contain:
//! - A ROM memory, maximum size depending on the mapper type. Typically mapped from 0x0000 to 0x8000.
//! - A RAM memory. Typically mapped from 0xA000 to 0xC000.
//! - An RTC, with memory-mapped registers. Address ranges depends on the specific cartridge type.
//!
//! ```rust,no_run
//! use cartridge::Cartridge;
//!
//! let data = std::fs::read("my_cartridge.gb").expect("Unable to read cartridge from disk");
//! let mut cartridge = Cartridge::try_new(data).expect("Cartridge is not valid");
//!
//! // Query the cartridge header with:
//! let header = cartridge.header();
//! let title = header.title;
//! println!("Cartridge title is {title}");
//!
//! // Perform memory-mapped accesses with:
//! assert_eq!(cartridge.read(0x4000u16), 0xFA);
//! cartridge.write(0xA000u16, 0x53);
//! ```
//!
#![no_std]
#![warn(missing_docs)]

pub mod header;
pub mod mappers;

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use header::CartridgeHeader;
use mappers::Mapper;

use self::header::CartridgeType;

/// A cartridge error
#[derive(Debug)]
pub enum Error {
    /// The cartridge does not have RAM memory, but an operation involving RAM memory was
    /// requested.
    CartridgeHasNoRam,

    /// The given RAM backup does not have the expected size of the actual RAM in the cartridge.
    UnexpectedRamSize {
        /// Expected RAM size in bytes
        expected: usize,
        /// Actual RAM size in bytes
        actual: usize,
    },

    /// The header of the cartridge is not valid.
    InvalidHeader(header::Error),

    /// The given mapper is not supported
    UnsupportedMapper(header::CartridgeType),
}

impl From<header::Error> for Error {
    fn from(value: header::Error) -> Self {
        Self::InvalidHeader(value)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Represents a game cartridge
pub struct Cartridge {
    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    /// Constructs a new cartridge using the given ROM data. Can fail if there was a problem
    /// reading the header.
    pub fn try_new(rom_data: Vec<u8>) -> Result<Self, Error> {
        let mapper = mappers::new_mapper(rom_data)?;
        Ok(Self { mapper })
    }

    /// Returns the header of the current cartridge. Note that the header keeps borrowed data of
    /// the cartridge.
    pub fn header<'a>(&'a self) -> CartridgeHeader<'a> {
        self.mapper.header().unwrap()
    }

    /// Returns true if the cartridge has a battery to keep RAM powered while the GameBoy is off.
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

    /// Returns a slice of the RAM that is battery-backed in the cartridge.
    /// Not all cartridge types have this memory.
    pub fn battery_backed_ram(&self) -> Option<&[u8]> {
        self.mapper.battery_backed_ram()
    }

    /// Attempts to restore the battery-backed RAM from the given slice. This may be used to restore
    /// RAM from a save file after the emulator starts up.
    pub fn restore_battery_backed_ram(&mut self, ram: &[u8]) -> Result<(), Error> {
        self.mapper.restore_battery_backed_ram(ram)
    }

    /// Reads the given memory-mapped address of the cartridge. Panics if the address does not
    /// belong the address space of the cartridge (0x0000 to 0x8000 or 0xA000 to 0xC000).
    pub fn read(&self, address: sm83::memory::Address) -> u8 {
        self.mapper.read(address)
    }

    /// Writes the given memory-mapped address of the cartridge with the given value. Panics if
    /// the address does not belong the address space of the cartridge (0x0000 to 0x8000 or 0xA000 to 0xC000).
    pub fn write(&mut self, address: sm83::memory::Address, value: u8) {
        self.mapper.write(address, value)
    }
}
