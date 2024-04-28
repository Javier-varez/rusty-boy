use crate::joypad::Joypad;
use cartridge::Cartridge;
use ppu::Ppu;
use sm83::interrupts::InterruptRegs;
use timer::Timer;

pub type Wram = Box<[u8; 0x2000]>;
pub type Hram = Box<[u8; 0x7f]>;

pub struct GbAddressSpace<'a> {
    pub cartridge: Cartridge<'a>,
    pub ppu: Ppu,
    pub wram: Wram,
    pub hram: Hram,
    pub interrupt_regs: InterruptRegs,
    pub joypad: Joypad,
    pub timer: Timer,

    pub sb: u8,
    pub sc: u8,
    pub serial_data: Vec<u8>,
}

impl<'a> GbAddressSpace<'a> {
    pub fn new(cartridge: Cartridge<'a>) -> Self {
        Self {
            cartridge,
            ppu: Ppu::new(),
            wram: Box::new([0; 0x2000]),
            hram: Box::new([0; 0x7f]),
            interrupt_regs: InterruptRegs::new(),
            joypad: Joypad::new(),
            timer: Timer::new(),
            sb: 0,
            sc: 0,
            serial_data: vec![],
        }
    }
}

impl<'a> sm83::memory::Memory for GbAddressSpace<'a> {
    fn read(&mut self, address: sm83::memory::Address) -> u8 {
        match address {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cartridge.read(address),
            0xC000..=0xDFFF => self.wram[address as usize - 0xC000],
            0xFF80..=0xFFFE => self.hram[address as usize - 0xFF80],
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => self.ppu.read(address),
            0xFF00 => self.joypad.read(address),
            0xFF01 => self.sb,
            0xFF02 => self.sc,
            0xFF04..=0xFF07 => self.timer.read(address),
            0xFF0F | 0xFFFF => self.interrupt_regs.read(address),
            0xFF00..=0xFF3F | 0xFF4C..=0xFF7F => {
                log::trace!("Unimplemented read from I/O regs: {address:#x}");
                0
            }
            _ => panic!("Invalid read address: {}", address),
        }
    }

    fn write(&mut self, address: sm83::memory::Address, value: u8) {
        match address {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cartridge.write(address, value),
            0xC000..=0xDFFF => {
                self.wram[address as usize - 0xC000] = value;
            }
            0xFF80..=0xFFFE => {
                self.hram[address as usize - 0xFF80] = value;
            }
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => self.ppu.write(address, value),
            0xFF00 => self.joypad.write(address, value),
            0xFF01 => self.sb = value,
            0xFF02 => {
                self.sc = value;
                if self.sc & 0x80 != 0 {
                    self.serial_data.push(self.sb);
                    log::info!("S: {}", String::from_utf8_lossy(&self.serial_data));
                }
                self.sc &= 0x7f;
            }
            0xFF04..=0xFF07 => self.timer.write(address, value),
            0xFF0F | 0xFFFF => self.interrupt_regs.write(address, value),
            0xFF00..=0xFF3F | 0xFF4C..=0xFF7F => {
                log::trace!("Unimplemented write to I/O regs: {address:#x} = {value:#x}")
            }
            _ => panic!("Invalid write address: {address:#x}, value {value:#x}"),
        }
    }
}
