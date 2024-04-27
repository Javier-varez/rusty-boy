use crate::{file_rom::FileRom, joypad::Joypad};
use ppu::Ppu;
use sm83::interrupts::InterruptRegs;

pub type Wram = Box<[u8; 0x2000]>;
pub type Hram = Box<[u8; 0x7f]>;

pub struct GbAddressSpace {
    pub rom: FileRom,
    pub ppu: Ppu,
    pub wram: Wram,
    pub hram: Hram,
    pub interrupt_regs: InterruptRegs,
    pub joypad: Joypad,
}

impl GbAddressSpace {
    pub fn new(rom: FileRom) -> Self {
        Self {
            rom,
            ppu: Ppu::new(),
            wram: Box::new([0; 0x2000]),
            hram: Box::new([0; 0x7f]),
            interrupt_regs: InterruptRegs::new(),
            joypad: Joypad::new(),
        }
    }
}

impl sm83::memory::Memory for GbAddressSpace {
    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        match address {
            0x0000..=0x7FFF => self.rom.read(address),
            0xC000..=0xDFFF => self.wram[address as usize - 0xC000],
            0xFF80..=0xFFFE => self.hram[address as usize - 0xFF80],
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => self.ppu.read(address),
            0xFF00 => self.joypad.read(address),
            0xFF0F | 0xFFFF => self.interrupt_regs.read(address),
            0xFF00..=0xFF3F | 0xFF4C..=0xFF7F => {
                println!("Unimplemented read from I/O regs: {address:#x}");
                0
            }
            _ => panic!("Invalid read address: {}", address),
        }
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        match address {
            0x0000..=0x7fff => self.rom.write(address, value),
            0xC000..=0xDFFF => {
                self.wram[address as usize - 0xC000] = value;
            }
            0xFF80..=0xFFFE => {
                self.hram[address as usize - 0xFF80] = value;
            }
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => self.ppu.write(address, value),
            0xFF00 => self.joypad.write(address, value),
            0xFF0F | 0xFFFF => self.interrupt_regs.write(address, value),
            0xFF00..=0xFF3F | 0xFF4C..=0xFF7F => {
                println!("Unimplemented write to I/O regs: {address:#x} = {value:#x}")
            }
            _ => panic!("Invalid write address: {address:#x}, value {value:#x}"),
        }
    }
}
