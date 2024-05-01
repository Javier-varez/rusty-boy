#![no_std]

pub mod disassembler;
pub mod joypad;
pub mod memory;

use crate::memory::GbAddressSpace;

use cartridge::Cartridge;
use ppu::{dma::DmaEngine, Color, PpuResult, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use sm83::core::Cpu;

pub struct RustyBoy<'a> {
    cpu: Cpu,
    dma_engine: DmaEngine,
    address_space: GbAddressSpace<'a>,
    debug: bool,
}

impl<'a> RustyBoy<'a> {
    pub fn new_with_cartridge(cartridge: Cartridge<'a>) -> Self {
        const ENTRYPOINT: u16 = 0x100;

        let mut cpu = Cpu::new();
        cpu.get_mut_regs().pc_reg = ENTRYPOINT;

        Self {
            debug: false,
            cpu,
            dma_engine: DmaEngine::new(),
            address_space: GbAddressSpace::new(cartridge),
        }
    }

    pub fn enable_debug(&mut self) {
        self.debug = true;
    }

    pub fn supports_battery_backed_ram(&mut self) -> bool {
        self.address_space.cartridge.has_battery()
    }

    pub fn restore_cartridge_ram(&mut self, data: &[u8]) -> Result<(), cartridge::Error> {
        self.address_space
            .cartridge
            .restore_battery_backed_ram(data)
    }

    pub fn get_cartridge_ram(&mut self) -> Option<&[u8]> {
        self.address_space.cartridge.battery_backed_ram()
    }

    fn step(&mut self, render: bool) -> PpuResult {
        if self.debug {
            let pc = self.cpu.get_regs().pc_reg;
            let inst = disassembler::disassemble_single_inst(&mut self.address_space, pc);
            let regs = self.cpu.get_regs();
            log::trace!("{pc:#04x} {inst} -- {regs:x?}");
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

        let (ppu_interrupts, ppu_result) =
            self.address_space
                .ppu
                .step(cycles, &mut self.dma_engine, render);
        let timer_interrupts = self.address_space.timer.step(cycles);
        self.dma_engine.run(cycles, &mut self.address_space);

        self.address_space
            .interrupt_regs
            .trigger(ppu_interrupts | timer_interrupts);

        ppu_result
    }

    pub fn run_until_next_frame(
        &mut self,
        render: bool,
    ) -> &[[Color; DISPLAY_WIDTH]; DISPLAY_HEIGHT] {
        while PpuResult::FrameComplete != self.step(render) {}
        self.address_space.ppu.frame()
    }

    pub fn update_keys(&mut self, state: &joypad::State) {
        self.address_space.joypad.update_buttons(state);
    }
}
