pub mod file_rom;
pub mod memory;
pub mod rom;

use crate::file_rom::FileRom;
use crate::memory::GbAddressSpace;

use ppu::{Color, Ppu, PpuResult, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use sm83::core::{Cpu, Interrupts};

use self::memory::{Hram, Wram};

pub struct RustyBoy {
    rom: FileRom,
    cpu: Cpu,
    ppu: Ppu,
    wram: Wram,
    hram: Hram,
}

impl RustyBoy {
    pub fn new_with_rom(path: &std::path::Path) -> anyhow::Result<Self> {
        let rom = FileRom::from_file(&path)?;

        const ENTRYPOINT: u16 = 0x100;

        let ppu = Ppu::new();
        let mut cpu = Cpu::new();
        cpu.get_mut_regs().pc_reg = ENTRYPOINT;
        let wram = Box::new([0; 0x2000]);
        let hram = Box::new([0; 0x7F]);

        Ok(Self {
            cpu,
            ppu,
            rom,
            wram,
            hram,
        })
    }

    pub fn run_until_next_frame(
        &mut self,
    ) -> anyhow::Result<&[[Color; DISPLAY_WIDTH]; DISPLAY_HEIGHT]> {
        loop {
            let ppu = &mut self.ppu;
            let mut memory = GbAddressSpace {
                rom: &mut self.rom,
                wram: &mut self.wram,
                hram: &mut self.hram,
                ppu,
            };

            let result = self.cpu.step(&mut memory, Interrupts::new());
            drop(memory);
            match result {
                sm83::core::ExitReason::Step(cycles) => {
                    if let PpuResult::FrameComplete(_) = ppu.run(cycles) {
                        break;
                    }
                }
                _ => {
                    panic!("Unexpected PPU exit reason")
                }
            }
        }

        Ok(self.ppu.frame())
    }
}
