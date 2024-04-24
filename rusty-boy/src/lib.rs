pub mod disassembler;
pub mod file_rom;
pub mod memory;
pub mod rom;

use crate::file_rom::FileRom;
use crate::memory::GbAddressSpace;

use ppu::{dma::DmaEngine, Color, PpuResult, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use sm83::core::Cpu;

use self::disassembler::Disassembler;

pub struct RustyBoy {
    cpu: Cpu,
    dma_engine: DmaEngine,
    address_space: GbAddressSpace,
    debug: bool,
}

impl RustyBoy {
    pub fn new_with_rom(path: &std::path::Path) -> anyhow::Result<Self> {
        let rom = FileRom::from_file(&path)?;

        const ENTRYPOINT: u16 = 0x100;

        let mut cpu = Cpu::new();
        cpu.get_mut_regs().pc_reg = ENTRYPOINT;

        Ok(Self {
            debug: false,
            cpu,
            dma_engine: DmaEngine::new(),
            address_space: GbAddressSpace::new(rom),
        })
    }

    pub fn enable_debug(&mut self) {
        self.debug = true;
    }

    fn step(&mut self) -> PpuResult {
        if self.debug {
            let disasm = Disassembler::new(self.address_space.rom.rom());
            let pc = self.cpu.get_regs().pc_reg;
            let inst = disasm.disassemble_single_inst(pc as usize).unwrap();
            let regs = self.cpu.get_regs();
            println!("{pc:#04x} {inst} -- {regs:#x?}");
        }

        let interrupts = self.address_space.interrupt_regs.active_interrupts();
        let result = self.cpu.step(&mut self.address_space, interrupts);

        let cycles = match result {
            sm83::core::ExitReason::Step(cycles)
            | sm83::core::ExitReason::Stop(cycles)
            | sm83::core::ExitReason::Halt(cycles) => cycles,
            sm83::core::ExitReason::InterruptTaken(cycles, interrupt) => {
                self.address_space.interrupt_regs.acknowledge(interrupt);
                cycles
            }
            sm83::core::ExitReason::IllegalOpcode => {
                panic!(
                    "Illegal CPU opcode at address: {}",
                    self.cpu.get_regs().pc_reg
                )
            }
        };

        let (interrupts, ppu_result) = self.address_space.ppu.run(cycles, &mut self.dma_engine);
        self.address_space.interrupt_regs.trigger(interrupts);
        self.dma_engine.run(cycles, &mut self.address_space);

        ppu_result
    }

    pub fn run_until_next_frame(
        &mut self,
    ) -> anyhow::Result<&[[Color; DISPLAY_WIDTH]; DISPLAY_HEIGHT]> {
        while PpuResult::FrameComplete != self.step() {}
        Ok(self.address_space.ppu.frame())
    }
}
