use crate::header::{self, CartridgeHeader};

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use super::Mapper;

const ROM_BANK_SIZE: usize = 16 * 1024;
const ROM_BANK_MSB_SELECT_MASK: usize = 0x0100;
const ROM_BANK_MSB_SELECT_OFFSET: usize = 8;
const ROM_BANK_LSB_SELECT_MASK: usize = 0xFF;

const RAM_BASE: usize = 0xA000;
const RAM_BANK_SIZE: usize = 8 * 1024;
const RAM_BANK_SELECT_MASK: usize = 0x0f;

pub struct Mbc5 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    ram_enabled: bool,
    selected_rom_bank: usize,
    selected_ram_bank: usize,
}

impl Mbc5 {
    pub fn new(rom: Vec<u8>, ram_size: usize) -> Self {
        assert!(rom.len().count_ones() == 1); // ROM size must be a power of 2
        assert!(rom.len() < 8192 * 1024); // Max size of MB5 roms is 8 MiB

        Self {
            rom,
            ram: vec![0; ram_size],
            ram_enabled: false,
            selected_rom_bank: 0,
            selected_ram_bank: 0,
        }
    }

    fn read_rom(&self, address: usize) -> u8 {
        // Banks that don't exist need to read the wrapped address value.
        let address = address & (self.rom.len() - 1);
        self.rom[address]
    }

    fn read_ram(&self, address: usize) -> u8 {
        let address = address & (self.ram.len() - 1);
        self.ram[address]
    }

    fn write_ram(&mut self, address: usize, value: u8) {
        let address = address & (self.ram.len() - 1);
        self.ram[address] = value
    }
}

impl Mapper for Mbc5 {
    fn header(&self) -> Result<CartridgeHeader<'_>, header::Error> {
        CartridgeHeader::try_new(&self.rom)
    }

    fn read(&self, address: sm83::memory::Address) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                // ROM bank 0 (16 KiB)
                self.read_rom(address as usize)
            }
            0x4000..=0x7FFF => {
                // ROM bank 0 to 0x1FF (16 KiB each)
                let offset = self.selected_rom_bank * ROM_BANK_SIZE;
                self.read_rom(address as usize - ROM_BANK_SIZE + offset)
            }
            0xA000..=0xBFFF => {
                // RAM bank
                if !self.ram_enabled {
                    return 0xff;
                }

                match self.selected_ram_bank {
                    0x00..=0x0f => {
                        // RAM bank
                        let offset = self.selected_ram_bank * RAM_BANK_SIZE;
                        self.read_ram(address as usize - RAM_BASE + offset)
                    }
                    _ => unimplemented!(),
                }
            }
            _ => unimplemented!(),
        }
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        match address {
            0x0000..=0x1FFF => {
                // Ram is enabled if lower nibble is A
                self.ram_enabled = value & 0xF == 0x0A;
            }
            0x2000..=0x2FFF => {
                // Low 8-bits of ROM Bank number
                let value = value as usize;
                self.selected_rom_bank =
                    (self.selected_rom_bank & ROM_BANK_MSB_SELECT_MASK) | value;
            }
            0x3000..=0x3FFF => {
                // Bit 9 of ROM Bank number
                let value = value as usize;
                self.selected_rom_bank = (self.selected_rom_bank & ROM_BANK_LSB_SELECT_MASK)
                    | (value << ROM_BANK_MSB_SELECT_OFFSET);
            }
            0x4000..=0x5FFF => {
                // RAM Bank number
                let value = RAM_BANK_SELECT_MASK & (value as usize);
                self.selected_ram_bank = value;
            }
            0x6000..=0x7FFF => {}
            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    return;
                }

                match self.selected_ram_bank {
                    0x00..=0x0f => {
                        // RAM bank
                        let offset = self.selected_ram_bank * RAM_BANK_SIZE;
                        self.write_ram(address as usize - RAM_BASE + offset, value)
                    }
                    _ => unimplemented!(),
                }
            }
            _ => unimplemented!(),
        }
    }

    fn battery_backed_ram(&self) -> Option<&[u8]> {
        Some(&self.ram)
    }

    fn restore_battery_backed_ram(&mut self, ram: &[u8]) -> Result<(), crate::Error> {
        if ram.len() != self.ram.len() {
            return Err(crate::Error::UnexpectedRamSize {
                expected: self.ram.len(),
                actual: ram.len(),
            });
        }
        self.ram
            .iter_mut()
            .zip(ram.iter())
            .for_each(|(d, s)| *d = *s);
        Ok(())
    }
}
