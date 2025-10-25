use crate::joypad::Joypad;
use cartridge::Cartridge;
use ppu::Ppu;
use sm83::interrupts::InterruptRegs;
use timer::Timer;

extern crate alloc;
use alloc::boxed::Box;

use core::mem::MaybeUninit;

pub type Wram = Box<[u8; 0x2000]>;
pub type Hram = Box<[u8; 0x7f]>;

pub struct GbAddressSpace {
    pub cartridge: Cartridge,
    pub ppu: Ppu,
    pub wram: Wram,
    pub hram: Hram,
    pub interrupt_regs: InterruptRegs,
    pub joypad: Joypad,
    pub timer: Timer,

    pub sb: u8,
    pub sc: u8,
}

impl GbAddressSpace {
    pub fn new(cartridge: Cartridge) -> Self {
        let mut wram: [MaybeUninit<u8>; 0x2000] = [MaybeUninit::uninit(); 0x2000];
        let mut hram: [MaybeUninit<u8>; 0x7f] = [MaybeUninit::uninit(); 0x7f];

        for elem in wram.iter_mut() {
            elem.write(0);
        }

        for elem in hram.iter_mut() {
            elem.write(0);
        }

        Self {
            cartridge,
            ppu: Ppu::new(),
            wram: Box::new(unsafe {
                core::mem::transmute::<[MaybeUninit<u8>; 0x2000], [u8; 0x2000]>(wram)
            }),
            hram: Box::new(unsafe {
                core::mem::transmute::<[MaybeUninit<u8>; 0x7f], [u8; 0x7f]>(hram)
            }),
            interrupt_regs: InterruptRegs::new(),
            joypad: Joypad::new(),
            timer: Timer::new(),
            sb: 0,
            sc: 0,
        }
    }

    pub fn reset(&mut self) {
        self.ppu = Ppu::new();
        for elem in self.wram.iter_mut() {
            *elem = 0;
        }
        for elem in self.hram.iter_mut() {
            *elem = 0;
        }
        self.interrupt_regs = InterruptRegs::new();
        self.joypad = Joypad::new();
        self.timer = Timer::new();
        self.sb = 0;
        self.sc = 0;
    }
}

impl sm83::memory::Memory for GbAddressSpace {
    fn read(&self, address: sm83::memory::Address) -> u8 {
        match address {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.cartridge.read(address),
            0xC000..=0xDFFF => self.wram[address as usize - 0xC000],
            0xFF80..=0xFFFE => self.hram[address as usize - 0xFF80],
            0xE000..=0xFDFF => {
                // This region must not be used (according to nintendo), but unfortunately some games seem to rely on it.
                self.wram[address as usize - 0xE000]
            }
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
            0xFEA0..=0xFEFF => {
                // This region must not be used, but unfortunately some games seem to rely on it.
                0
            }
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
            0xE000..=0xFDFF => {
                // This region must not be used (according to nintendo), but unfortunately some games seem to rely on it.
                self.wram[address as usize - 0xE000] = value;
            }
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => self.ppu.write(address, value),
            0xFF00 => self.joypad.write(address, value),
            0xFF01 => self.sb = value,
            0xFF02 => {
                self.sc = value;
                self.sc &= 0x7f;
            }
            0xFF04..=0xFF07 => self.timer.write(address, value),
            0xFF0F | 0xFFFF => self.interrupt_regs.write(address, value),
            0xFF00..=0xFF3F | 0xFF4C..=0xFF7F => {
                log::trace!("Unimplemented write to I/O regs: {address:#x} = {value:#x}")
            }
            0xFEA0..=0xFEFF => {
                // This region must not be used, but unfortunately some games seem to rely on it.
            }
        }
    }
}
