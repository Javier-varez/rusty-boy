use crate::file_rom::FileRom;
use ppu::Ppu;

pub struct GbAddressSpace<'a> {
    pub rom: &'a mut FileRom,
    pub ppu: &'a mut Ppu,
}

impl<'a> sm83::memory::Memory for GbAddressSpace<'a> {
    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        match address {
            0x0000..=0x7FFF => self.rom.read(address),
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => self.ppu.read(address),
            _ => panic!("Invalid read address: {}", address),
        }
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        match address {
            0x0000..=0x7fff => self.rom.write(address, value),
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => self.ppu.write(address, value),
            _ => panic!("Invalid write address: {}, value {}", address, value),
        }
    }
}
