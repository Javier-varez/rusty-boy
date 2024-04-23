use sm83::{
    core::Cycles,
    memory::{Address, Memory},
};

use crate::oam::OAM_SIZE;

pub struct DmaEngine {
    active: bool,
    base_address: Address,
    current_element: u16,
}

impl DmaEngine {
    pub fn new() -> Self {
        Self {
            active: false,
            base_address: 0,
            current_element: 0,
        }
    }

    pub fn trigger(&mut self, base: u8) {
        self.base_address = (base as Address) << 8;
        self.current_element = 0;
        self.active = true;
    }

    pub fn run<T: Memory>(&mut self, mut cycles: Cycles, memory: &mut T) {
        const NO_CYCLES: Cycles = Cycles::new(0);
        while cycles != NO_CYCLES && self.active {
            let src_address = self.base_address + self.current_element;
            let dest_address = 0xFF00 + self.current_element;
            let value = memory.read(src_address);
            memory.write(dest_address, value);
            cycles = cycles - Cycles::new(4);

            self.current_element += 1;
            if self.current_element as usize == OAM_SIZE {
                self.active = false;
            }
        }
    }
}
