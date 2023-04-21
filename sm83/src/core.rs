pub enum Register {
    AF = 0,
    BC = 1,
    DE = 2,
    HL = 3,
    SP = 4,
    PC = 5,
}

struct InterruptRegisters {
    iff1: bool,
    iff2: bool,
}

struct Registers {
    flags: u8,
    a_reg: u8,
    b_reg: u8,
    c_reg: u8,
    d_reg: u8,
    e_reg: u8,
    h_reg: u8,
    l_reg: u8,
    sp_reg: u16,
    pc_reg: u16,
}

struct CoreState {
    regs: Registers,
    shadow_regs: Registers,
    interrupt_regs: InterruptRegisters,
}
