extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

const ROM_BANK_SIZE: usize = 16 * 1024;
const ROM_BANK_SELECT_MASK: usize = 0x1F;

const RAM_BASE: usize = 0xA000;
const RAM_BANK_SIZE: usize = 4 * 1024;
const RAM_BANK_SELECT_MASK: usize = 0x03;

const BANK_MODE_SELECT_MASK: usize = 0x01;

#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Simple = 0,
    Advanced = 1,
}

pub struct Mbc1<'a> {
    ram: Vec<u8>,
    rom: &'a [u8],
    ram_enabled: bool,
    selected_rom_bank: usize,
    selected_ram_bank: usize,
    mode: Mode,
}

impl<'a> Mbc1<'a> {
    pub fn new(rom: &'a [u8], ram_size: usize) -> Self {
        assert!(rom.len().count_ones() == 1); // ROM size must be a power of 2
        assert!(rom.len() < 512 * 1024); // ROMs larger than 512 KiB are currently unsupported

        Self {
            rom,
            ram: vec![0; ram_size],
            ram_enabled: false,
            selected_rom_bank: 1,
            selected_ram_bank: 0,
            mode: Mode::Simple,
        }
    }

    fn read_rom(&self, address: usize) -> u8 {
        // Banks that don't exist need to read the wrapped address value.
        // TODO: This mod operation is in a hot path, move it to bank selection
        //       or make sure it is always implemented as a shift
        let address = address % self.rom.len();
        self.rom[address]
    }

    fn read_ram(&self, address: usize) -> u8 {
        if address < self.ram.len() && self.ram_enabled {
            self.ram[address]
        } else {
            0
        }
    }

    fn write_ram(&mut self, address: usize, value: u8) {
        if address < self.ram.len() && self.ram_enabled {
            self.ram[address] = value
        }
    }

    pub fn read(&self, address: sm83::memory::Address) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                // ROM bank 0 (16 KiB)
                // Note that for roms > 512 KiB, this bank is switched if is Mode::Advanced.
                // However, this is currently an unsupported ROM type.
                self.read_rom(address as usize)
            }
            0x4000..=0x7FFF => {
                // ROM bank 1 to 0x1F (16 KiB each)
                debug_assert!(self.selected_rom_bank < 0x20);
                debug_assert_ne!(self.selected_rom_bank, 0);
                let address = (self.selected_rom_bank - 1) * ROM_BANK_SIZE + address as usize;
                self.read_rom(address)
            }
            0xA000..=0xBFFF => {
                // RAM bank
                let offset = if self.mode == Mode::Advanced {
                    self.selected_ram_bank * RAM_BANK_SIZE
                } else {
                    0
                };
                self.read_ram(address as usize - RAM_BASE + offset)
            }
            _ => unimplemented!(),
        }
    }

    pub fn write(&mut self, address: sm83::memory::Address, value: u8) {
        match address {
            0x0000..=0x1FFF => {
                // Ram is enabled if lower nibble is A
                self.ram_enabled = value & 0xF == 0x0A;
            }
            0x2000..=0x3FFF => {
                // ROM Bank number
                let value = ROM_BANK_SELECT_MASK & (value as usize);
                if value == 0 {
                    self.selected_rom_bank = 1;
                } else {
                    self.selected_rom_bank = value;
                }
            }
            0x4000..=0x5FFF => {
                // RAM Bank number
                let value = RAM_BANK_SELECT_MASK & (value as usize);
                self.selected_ram_bank = value;
            }
            0x6000..=0x7FFF => {
                // Banking mode selection
                let value = BANK_MODE_SELECT_MASK & (value as usize);
                self.mode = if value != 0 {
                    Mode::Advanced
                } else {
                    Mode::Simple
                };
            }
            0xA000..=0xBFFF => {
                // RAM bank
                let offset = if self.mode == Mode::Advanced {
                    self.selected_ram_bank * RAM_BANK_SIZE
                } else {
                    0
                };
                self.write_ram(address as usize - RAM_BASE + offset, value)
            }
            _ => unimplemented!(),
        }
    }
}
