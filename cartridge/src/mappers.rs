//! Mappers are integrated circuits that control the address space of the cartridge and translate
//! memory accesses into control signals for memories (ROM and RAM), as well as internal register
//! and additional peripherals (RTC, accelerometers, etc).
//!
use crate::header::{self, CartridgeHeader, CartridgeType};

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;

mod mbc1;
mod mbc3;
mod mbc5;
mod rom_only;

/// All mappers must implement this trait. Allows accessing memory-mapped ROM, RAM and any
/// peripherals, as well as obtaining header information and other cartridge-specific functionality
pub trait Mapper {
    /// Obtains the header of the cartridge. It can fail if the rom does not contain enough data
    /// for the header or the title of the game is not a valid string.
    fn header<'a>(&'a self) -> Result<CartridgeHeader<'a>, header::Error>;

    /// Reads the given memory-mapped address of the cartridge. Panics if the address does not
    /// belong the address space of the cartridge (0x0000 to 0x8000 or 0xA000 to 0xC000).
    fn read(&self, address: sm83::memory::Address) -> u8;

    /// Writes the given memory-mapped address of the cartridge with the given value. Panics if
    /// the address does not belong the address space of the cartridge (0x0000 to 0x8000 or 0xA000 to 0xC000).
    fn write(&mut self, address: sm83::memory::Address, value: u8);

    /// Returns a slice of the RAM that is battery-backed in the cartridge.
    /// Not all cartridge types have this memory.
    fn battery_backed_ram(&self) -> Option<&[u8]> {
        None
    }

    /// Attempts to restore the battery-backed RAM from the given slice. This may be used to restore
    /// RAM from a save file after the emulator starts up.
    fn restore_battery_backed_ram(&mut self, _ram: &[u8]) -> Result<(), crate::Error> {
        Err(crate::Error::CartridgeHasNoRam)
    }
}

/// Creates a new mapper from the given ROM. The rom header is parsed to determine the required
/// mapper type. A boxed Mapper type implementing the mapper type indicated by the cartridge header
/// is returned. This method panics
pub fn new_mapper(data: Vec<u8>) -> Result<Box<dyn Mapper>, super::Error> {
    let header = CartridgeHeader::try_new(&data)?;
    let cartridge_type = header.cartridge_type;
    let ram_size = header
        .ram_size
        .into_usize()
        .ok_or(super::Error::InvalidHeader(header::Error::InvalidRamSize))?;

    Ok(match cartridge_type {
        CartridgeType::RomOnly => Box::new(rom_only::RomOnly::new(data)),
        CartridgeType::Mbc1 | CartridgeType::Mbc1Ram | CartridgeType::Mbc1RamBattery => {
            Box::new(mbc1::Mbc1::new(data, ram_size))
        }
        CartridgeType::Mbc3 | CartridgeType::Mbc3Ram | CartridgeType::Mbc3RamBattery => {
            Box::new(mbc3::Mbc3::new(data, ram_size))
        }
        CartridgeType::Mbc5 | CartridgeType::Mbc5Ram | CartridgeType::Mbc5RamBattery => {
            Box::new(mbc5::Mbc5::new(data, ram_size))
        }
        v => {
            // Other cartridge types are currently unsupported
            return Err(crate::Error::UnsupportedMapper(v));
        }
    })
}
