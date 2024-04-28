use crate::{
    decoder::{
        self, Bit, Condition, OpCode, Register, RegisterPair, RegisterPairMem, RegisterPairStack,
        ResetTarget,
    },
    interrupts::{Interrupt, Interrupts},
    memory::Memory,
};

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum Flag {
    Z = 1 << 7,
    N = 1 << 6,
    H = 1 << 5,
    C = 1 << 4,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Flags(u8);

impl Default for Flags {
    fn default() -> Self {
        Self::new()
    }
}

impl From<u8> for Flags {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<Flags> for u8 {
    fn from(flags: Flags) -> u8 {
        flags.0
    }
}

impl Flags {
    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn with(mut self, flag: Flag, value: bool) -> Self {
        if value {
            self.0 |= flag as u8;
        } else {
            self.0 &= !(flag as u8);
        }
        self
    }

    pub const fn is_flag_set(&self, flag: Flag) -> bool {
        (self.0 & flag as u8) != 0
    }
}

#[derive(Debug, Clone)]
pub struct Registers {
    pub flags: Flags,
    pub a_reg: u8,
    pub b_reg: u8,
    pub c_reg: u8,
    pub d_reg: u8,
    pub e_reg: u8,
    pub h_reg: u8,
    pub l_reg: u8,
    pub sp_reg: u16,
    pub pc_reg: u16,
    pub irq_en: bool, // IME register
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

impl Registers {
    const fn new() -> Self {
        Self {
            flags: Flags::new(),
            a_reg: 0x01,
            b_reg: 0xff,
            c_reg: 0x13,
            d_reg: 0x00,
            e_reg: 0xc1,
            h_reg: 0x84,
            l_reg: 0x03,
            sp_reg: 0xfffe,
            pc_reg: 0,
            irq_en: false,
        }
    }
}

const fn carry_bit8(a: u8, b: u8, c: u8, bit: usize) -> bool {
    assert!(bit < 8);
    let xor = a ^ b ^ c;
    (xor & (1 << bit)) != 0
}

const fn carry_bit16(a: u16, b: u16, c: u16, bit: usize) -> bool {
    assert!(bit < 16);
    let xor = a ^ b ^ c;
    (xor & (1 << bit)) != 0
}

const fn carry_bit32(a: u32, b: u32, c: u32, bit: usize) -> bool {
    assert!(bit < 32);
    let xor = a ^ b ^ c;
    (xor & (1 << bit)) != 0
}

fn add(a: u8, b: u8, carry: bool) -> (u8, Flags) {
    let a = a as u16;
    let b = b as u16;
    let carry = carry as u16;
    let result = a + b + carry;

    let flags = Flags::new()
        .with(Flag::Z, (result & 0xFF) == 0)
        .with(Flag::N, false)
        .with(Flag::H, carry_bit16(a, b, result, 4))
        .with(Flag::C, carry_bit16(a, b, result, 8));

    (result as u8, flags)
}

const fn add16(a: u16, b: u16, flags: Flags) -> (u16, Flags) {
    let a = a as u32;
    let b = b as u32;
    let result = a + b;

    let flags = flags
        .with(Flag::N, false)
        .with(Flag::H, carry_bit32(a, b, result, 12))
        .with(Flag::C, carry_bit32(a, b, result, 16));

    (result as u16, flags)
}

const fn sub(a: u8, b: u8, carry: bool) -> (u8, Flags) {
    let a = a as u16;
    let b = b as u16;
    let carry = carry as u16;
    let inv = (!(b + carry)).wrapping_add(1); // 2's compliment of a + b
    let result = a + inv;

    let flags = Flags::new()
        .with(Flag::Z, (result & 0xFF) == 0)
        .with(Flag::N, true)
        .with(Flag::H, carry_bit16(a, b, result, 4))
        .with(Flag::C, carry_bit16(a, b, result, 8));

    (result as u8, flags)
}

const fn and(a: u8, b: u8) -> (u8, Flags) {
    let result = a & b;

    let flags = Flags::new().with(Flag::Z, result == 0).with(Flag::H, true);

    (result, flags)
}

const fn or(a: u8, b: u8) -> (u8, Flags) {
    let result = a | b;
    let flags = Flags::new().with(Flag::Z, result == 0);
    (result, flags)
}

const fn xor(a: u8, b: u8) -> (u8, Flags) {
    let result = a ^ b;
    let flags = Flags::new().with(Flag::Z, result == 0);
    (result, flags)
}

const fn daa(a: u8, flags: Flags) -> (u8, Flags) {
    let mut result = a;
    let mut carry = false;

    if flags.is_flag_set(Flag::N) {
        // Adjust subtraction
        carry = flags.is_flag_set(Flag::C);
        let correction = match (flags.is_flag_set(Flag::C), flags.is_flag_set(Flag::H)) {
            (false, false) => 0,
            (true, false) => (!0x60u8).wrapping_add(1), // 2's complement of corresponding value
            (false, true) => (!0x06u8).wrapping_add(1),
            (true, true) => (!0x66u8).wrapping_add(1),
        };
        result = result.wrapping_add(correction);
    } else {
        // Adjust addition
        if ((a & 0xf) > 9) || flags.is_flag_set(Flag::H) {
            result = result.wrapping_add(0x06);
        }

        if (a > 0x99) || flags.is_flag_set(Flag::C) {
            result = result.wrapping_add(0x60);
            carry = true;
        }
    }

    let flags = flags
        .with(Flag::Z, result == 0)
        .with(Flag::H, false)
        .with(Flag::C, carry);
    (result, flags)
}

// Some variants of this instruction (rlca) always set Z to 0, but others actually compute the
// result
const fn rlc(value: u8, real_z: bool) -> (u8, Flags) {
    let carry = (value & 0x80) != 0;
    let mut shifted = value << 1;
    if carry {
        shifted |= 1;
    }
    let flags = Flags::new()
        .with(Flag::C, carry)
        .with(Flag::Z, shifted == 0 && real_z);
    (shifted, flags)
}

// Some variants of this instruction (rrca) always set Z to 0, but others actually compute the
// result
const fn rrc(value: u8, real_z: bool) -> (u8, Flags) {
    let carry = (value & 0x01) != 0;
    let mut shifted = value >> 1;
    if carry {
        shifted |= 0x80;
    }
    let flags = Flags::new()
        .with(Flag::C, carry)
        .with(Flag::Z, shifted == 0 && real_z);
    (shifted, flags)
}

const fn rl(value: u8, old_carry: bool, real_z: bool) -> (u8, Flags) {
    let mut shifted = value << 1;
    if old_carry {
        shifted |= 0x01;
    }
    let new_carry = (value & 0x80) != 0;
    let flags = Flags::new()
        .with(Flag::C, new_carry)
        .with(Flag::Z, shifted == 0 && real_z);
    (shifted, flags)
}

const fn rr(value: u8, old_carry: bool, real_z: bool) -> (u8, Flags) {
    let mut shifted = value >> 1;
    if old_carry {
        shifted |= 0x80;
    }
    let new_carry = (value & 0x01) != 0;
    let flags = Flags::new()
        .with(Flag::C, new_carry)
        .with(Flag::Z, shifted == 0 && real_z);
    (shifted, flags)
}

const fn sla(value: u8) -> (u8, Flags) {
    let shifted = value << 1;
    let new_carry = (value & 0x80) != 0;
    let flags = Flags::new()
        .with(Flag::C, new_carry)
        .with(Flag::Z, shifted == 0);
    (shifted, flags)
}

const fn sra(value: u8) -> (u8, Flags) {
    let negative = (value & 0x80) != 0;
    let new_carry = (value & 0x01) != 0;
    let shifted = (value >> 1) | if negative { 0x80 } else { 0x00 };
    let flags = Flags::new()
        .with(Flag::C, new_carry)
        .with(Flag::Z, shifted == 0);
    (shifted, flags)
}

const fn swap(value: u8) -> (u8, Flags) {
    let swapped = (value >> 4) | (value << 4);
    let flags = Flags::new().with(Flag::Z, swapped == 0);
    (swapped, flags)
}

const fn srl(value: u8) -> (u8, Flags) {
    let new_carry = (value & 0x01) != 0;
    let shifted = value >> 1;
    let flags = Flags::new()
        .with(Flag::C, new_carry)
        .with(Flag::Z, shifted == 0);
    (shifted, flags)
}

const fn bit_mask(bit: Bit) -> u8 {
    1 << (bit as u8)
}

const fn bit(bit_idx: Bit, value: u8, flags: Flags) -> Flags {
    let bit = bit_mask(bit_idx);
    let z_flag = (bit & value) == 0;
    flags
        .with(Flag::N, false)
        .with(Flag::H, true)
        .with(Flag::Z, z_flag)
}

const fn res(bit_idx: Bit, value: u8) -> u8 {
    value & !bit_mask(bit_idx)
}

const fn set(bit_idx: Bit, value: u8) -> u8 {
    value | bit_mask(bit_idx)
}

const fn sign_extend(a: u8) -> u16 {
    a as i8 as i16 as u16
}

const fn translate_reset_target(target: ResetTarget) -> u16 {
    match target {
        ResetTarget::Addr0x00 => 0x00,
        ResetTarget::Addr0x08 => 0x08,
        ResetTarget::Addr0x10 => 0x10,
        ResetTarget::Addr0x18 => 0x18,
        ResetTarget::Addr0x20 => 0x20,
        ResetTarget::Addr0x28 => 0x28,
        ResetTarget::Addr0x30 => 0x30,
        ResetTarget::Addr0x38 => 0x38,
    }
}

const fn translate_irq_target(interrupt: Interrupt) -> u16 {
    match interrupt {
        Interrupt::Vblank => 0x40,
        Interrupt::Lcd => 0x48,
        Interrupt::Timer => 0x50,
        Interrupt::Serial => 0x58,
        Interrupt::Joypad => 0x60,
    }
}

/// Clock cycles, not machine cycles
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cycles(usize);

impl Cycles {
    /// Creates a Cycles instance with the given number of cycles
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Wraps around the given maximum value
    pub fn wrap(&self, max: usize) -> Cycles {
        Self(self.0 % max)
    }
}

impl core::ops::Add<Cycles> for Cycles {
    type Output = Cycles;

    fn add(self, rhs: Cycles) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl core::ops::Sub<Cycles> for Cycles {
    type Output = Cycles;

    fn sub(self, rhs: Cycles) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl From<usize> for Cycles {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<Cycles> for usize {
    fn from(value: Cycles) -> Self {
        value.0
    }
}

pub enum ExitReason {
    Step(Cycles),
    InterruptTaken(Cycles, Interrupt),
    Stop(Cycles),
    Halt(Cycles),
    IllegalOpcode,
}

pub struct Cpu {
    regs: Registers,
    halted: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            regs: Registers::new(),
            halted: false,
        }
    }

    pub const fn get_regs(&self) -> &Registers {
        &self.regs
    }

    pub fn get_mut_regs(&mut self) -> &mut Registers {
        &mut self.regs
    }

    const fn get_flag(&self, flag: Flag) -> bool {
        self.get_regs().flags.is_flag_set(flag)
    }

    const fn get_flags(&self) -> Flags {
        self.get_regs().flags
    }

    fn set_flags(&mut self, flags: Flags) {
        self.get_mut_regs().flags = flags;
    }

    const fn get_reg(&self, reg: Register) -> u8 {
        let regs = self.get_regs();
        match reg {
            Register::A => regs.a_reg,
            Register::B => regs.b_reg,
            Register::C => regs.c_reg,
            Register::D => regs.d_reg,
            Register::E => regs.e_reg,
            Register::H => regs.h_reg,
            Register::L => regs.l_reg,
        }
    }

    fn set_reg(&mut self, reg: Register, value: u8) {
        let regs = self.get_mut_regs();
        let target = match reg {
            Register::A => &mut regs.a_reg,
            Register::B => &mut regs.b_reg,
            Register::C => &mut regs.c_reg,
            Register::D => &mut regs.d_reg,
            Register::E => &mut regs.e_reg,
            Register::H => &mut regs.h_reg,
            Register::L => &mut regs.l_reg,
        };
        *target = value;
    }

    const fn get_reg_pair(&self, reg: RegisterPair) -> u16 {
        let regs = self.get_regs();
        let (hi, lo) = match reg {
            RegisterPair::BC => (regs.b_reg, regs.c_reg),
            RegisterPair::DE => (regs.d_reg, regs.e_reg),
            RegisterPair::HL => (regs.h_reg, regs.l_reg),
            RegisterPair::SP => {
                return regs.sp_reg;
            }
        };
        ((hi as u16) << 8) | (lo as u16)
    }

    fn set_reg_pair(&mut self, reg: RegisterPair, value: u16) {
        let regs = self.get_mut_regs();
        let (hi, lo) = match reg {
            RegisterPair::BC => (&mut regs.b_reg, &mut regs.c_reg),
            RegisterPair::DE => (&mut regs.d_reg, &mut regs.e_reg),
            RegisterPair::HL => (&mut regs.h_reg, &mut regs.l_reg),
            RegisterPair::SP => {
                regs.sp_reg = value;
                return;
            }
        };
        *hi = ((value >> 8) & 0xff) as u8;
        *lo = (value & 0xff) as u8;
    }

    const fn get_reg_pair_stack(&self, reg: RegisterPairStack) -> u16 {
        let regs = self.get_regs();
        let (hi, lo) = match reg {
            RegisterPairStack::BC => (regs.b_reg, regs.c_reg),
            RegisterPairStack::DE => (regs.d_reg, regs.e_reg),
            RegisterPairStack::HL => (regs.h_reg, regs.l_reg),
            RegisterPairStack::AF => (regs.a_reg, regs.flags.0),
        };
        ((hi as u16) << 8) | (lo as u16)
    }

    fn set_reg_pair_stack(&mut self, reg: RegisterPairStack, value: u16) {
        let regs = self.get_mut_regs();
        let (hi, lo) = match reg {
            RegisterPairStack::BC => (&mut regs.b_reg, &mut regs.c_reg),
            RegisterPairStack::DE => (&mut regs.d_reg, &mut regs.e_reg),
            RegisterPairStack::HL => (&mut regs.h_reg, &mut regs.l_reg),
            RegisterPairStack::AF => {
                let (hi, lo) = (&mut regs.a_reg, &mut regs.flags);
                *hi = ((value >> 8) & 0xff) as u8;
                *lo = ((value & 0xf0) as u8).into(); // lo-bits are hardcoded to 0
                return;
            }
        };
        *hi = ((value >> 8) & 0xff) as u8;
        *lo = (value & 0xff) as u8;
    }

    fn get_reg_pair_mem(&mut self, reg: RegisterPairMem) -> u16 {
        let regs = self.get_regs();
        let (hi, lo) = match reg {
            RegisterPairMem::BC => (regs.b_reg, regs.c_reg),
            RegisterPairMem::DE => (regs.d_reg, regs.e_reg),
            RegisterPairMem::HLINC | RegisterPairMem::HLDEC => (regs.h_reg, regs.l_reg),
        };
        let value = ((hi as u16) << 8) | (lo as u16);

        match reg {
            RegisterPairMem::HLINC => {
                self.set_reg_pair(RegisterPair::HL, value.wrapping_add(1));
            }
            RegisterPairMem::HLDEC => {
                self.set_reg_pair(RegisterPair::HL, value.wrapping_sub(1));
            }
            _ => {}
        }
        value
    }

    fn step_pc(&mut self) -> u16 {
        let regs = self.get_mut_regs();
        let pc = regs.pc_reg;
        regs.pc_reg = regs.pc_reg.wrapping_add(1);
        pc
    }

    fn read_8_bit_immediate<T: Memory>(&mut self, memory: &mut T) -> u8 {
        let pc = self.step_pc();
        memory.read(pc)
    }

    fn read_16_bit_immediate<T: Memory>(&mut self, memory: &mut T) -> u16 {
        let pc = self.step_pc();
        let lo = memory.read(pc);
        let pc = self.step_pc();
        let hi = memory.read(pc);
        ((hi as u16) << 8) | (lo as u16)
    }

    fn check_condition(&self, condition: Condition) -> bool {
        match condition {
            Condition::NZ => !self.get_flag(Flag::Z),
            Condition::Z => self.get_flag(Flag::Z),
            Condition::NC => !self.get_flag(Flag::C),
            Condition::C => self.get_flag(Flag::C),
        }
    }

    fn stack_push<T: Memory>(&mut self, memory: &mut T, value: u16) {
        let sp = self.get_reg_pair(RegisterPair::SP);
        let pos = sp.wrapping_sub(1);
        memory.write(pos, (value >> 8) as u8);
        let pos = pos.wrapping_sub(1);
        memory.write(pos, (value & 0xff) as u8);
        self.set_reg_pair(RegisterPair::SP, pos);
    }

    fn stack_pop<T: Memory>(&mut self, memory: &mut T) -> u16 {
        let sp = self.get_reg_pair(RegisterPair::SP);
        let pos = sp;
        let lo = memory.read(pos);
        let pos = pos.wrapping_add(1);
        let hi = memory.read(pos);
        let pos = pos.wrapping_add(1);
        self.set_reg_pair(RegisterPair::SP, pos);
        (lo as u16) | ((hi as u16) << 8)
    }

    fn fetch_and_decode<T: Memory>(&mut self, memory: &mut T) -> OpCode {
        let pc = self.step_pc();
        let insn = memory.read(pc);

        match decoder::decode(insn) {
            OpCode::Prefix => {
                let pc = self.step_pc();
                let insn = memory.read(pc);
                decoder::decode_prefixed(insn)
            }
            opcode => opcode,
        }
    }

    pub fn step<T: Memory>(&mut self, memory: &mut T, interrupts: Interrupts) -> ExitReason {
        if self.halted && !interrupts.has_any() {
            return ExitReason::Halt(Cycles::new(4));
        }
        self.halted = false;

        if let Some(irq) =
            interrupts
                .highest_priority()
                .and_then(|irq| if self.regs.irq_en { Some(irq) } else { None })
        {
            self.regs.irq_en = false;
            let return_addr = self.get_regs().pc_reg;
            self.stack_push(memory, return_addr);
            self.get_mut_regs().pc_reg = translate_irq_target(irq);
            ExitReason::InterruptTaken(Cycles::new(20), irq)
        } else {
            let instruction = self.fetch_and_decode(memory);
            self.execute(memory, instruction)
        }
    }

    fn execute<T: Memory>(&mut self, memory: &mut T, opcode: OpCode) -> ExitReason {
        let cycles = match opcode {
            OpCode::Prefix => {
                panic!("Attempted to execute prefix opcode!");
            }
            OpCode::Halt => {
                self.halted = true;
                return ExitReason::Halt(Cycles::new(4));
            }
            OpCode::Stop => {
                return ExitReason::Stop(Cycles::new(4));
            }
            OpCode::Illegal => {
                return ExitReason::IllegalOpcode;
            }
            // All instructions below use ExitReason::Step
            OpCode::Nop => Cycles::new(4),
            OpCode::Ld8RegReg(dest, src) => {
                let value = self.get_reg(src);
                self.set_reg(dest, value);
                Cycles::new(4)
            }
            OpCode::Ld8RegImm(dest) => {
                let value = self.read_8_bit_immediate(memory);
                self.set_reg(dest, value);
                Cycles::new(8)
            }
            OpCode::Ld8RegInd(dest, src) => {
                let addr = self.get_reg_pair(src);
                let value = memory.read(addr);
                self.set_reg(dest, value);
                Cycles::new(8)
            }
            OpCode::Ld8IndReg(dest, src) => {
                let value = self.get_reg(src);
                let addr = self.get_reg_pair(dest);
                memory.write(addr, value);
                Cycles::new(8)
            }
            OpCode::Ld8IndImm(dest) => {
                let value = self.read_8_bit_immediate(memory);
                let addr = self.get_reg_pair(dest);
                memory.write(addr, value);
                Cycles::new(12)
            }
            OpCode::Ld8IndAcc(dest) => {
                let value = self.get_reg(Register::A);
                let addr = self.get_reg_pair_mem(dest);
                memory.write(addr, value);
                Cycles::new(8)
            }
            OpCode::Ld8AccInd(dest) => {
                let addr = self.get_reg_pair_mem(dest);
                let value = memory.read(addr);
                self.set_reg(Register::A, value);
                Cycles::new(8)
            }
            OpCode::Ld8ZeroPageCAcc => {
                let regs = self.get_regs();
                let addr = 0xFF00 | regs.c_reg as u16;
                memory.write(addr, regs.a_reg);
                Cycles::new(8)
            }
            OpCode::Ld8AccZeroPageC => {
                let regs = self.get_regs();
                let addr = 0xFF00 | regs.c_reg as u16;
                let value = memory.read(addr);
                self.get_mut_regs().a_reg = value;
                Cycles::new(8)
            }
            OpCode::Ld8ZeroPageImmAcc => {
                let imm = self.read_8_bit_immediate(memory);
                let addr = 0xFF00 | imm as u16;
                memory.write(addr, self.get_regs().a_reg);
                Cycles::new(12)
            }
            OpCode::Ld8AccZeroPageImm => {
                let imm = self.read_8_bit_immediate(memory);
                let addr = 0xFF00 | imm as u16;
                let value = memory.read(addr);
                self.get_mut_regs().a_reg = value;
                Cycles::new(12)
            }
            OpCode::Ld8IndImmAcc => {
                let imm = self.read_16_bit_immediate(memory);
                let addr = imm;
                memory.write(addr, self.get_regs().a_reg);
                Cycles::new(16)
            }
            OpCode::Ld8AccIndImm => {
                let imm = self.read_16_bit_immediate(memory);
                let addr = imm;
                let value = memory.read(addr);
                self.get_mut_regs().a_reg = value;
                Cycles::new(16)
            }
            OpCode::Ld16RegImm(dest) => {
                let imm = self.read_16_bit_immediate(memory);
                self.set_reg_pair(dest, imm);
                Cycles::new(12)
            }
            OpCode::Ld16IndImmSp => {
                let imm = self.read_16_bit_immediate(memory);
                let sp = self.get_reg_pair(RegisterPair::SP);
                memory.write(imm, (sp & 0xff) as u8);
                memory.write(imm.wrapping_add(1), (sp >> 8) as u8);
                Cycles::new(20)
            }
            OpCode::Ld16HlSpImm => {
                let sp = self.get_regs().sp_reg;
                let imm = self.read_8_bit_immediate(memory) as i8 as i16;
                let value = ((sp as i16).wrapping_add(imm)) as u16;
                self.set_reg_pair(RegisterPair::HL, value);

                self.set_flags(
                    Flags::new()
                        .with(Flag::Z, false)
                        .with(Flag::N, false)
                        .with(Flag::H, carry_bit16(imm as u16, sp, value, 4))
                        .with(Flag::C, carry_bit16(imm as u16, sp, value, 8)),
                );
                Cycles::new(12)
            }
            OpCode::Ld16SpHl => {
                let value = self.get_reg_pair(RegisterPair::HL);
                self.get_mut_regs().sp_reg = value;
                Cycles::new(8)
            }
            OpCode::AddRegReg(dest, src) => {
                let src_val = self.get_reg(src);
                let dest_val = self.get_reg(dest);
                let (result, flags) = add(src_val, dest_val, false);
                self.set_reg(dest, result);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::SubRegReg(dest, src) => {
                let src_val = self.get_reg(src);
                let dest_val = self.get_reg(dest);
                let (result, flags) = sub(dest_val, src_val, false);
                self.set_reg(dest, result);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::AndRegReg(dest, src) => {
                let src_val = self.get_reg(src);
                let dest_val = self.get_reg(dest);
                let (result, flags) = and(src_val, dest_val);
                self.set_reg(dest, result);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::OrRegReg(dest, src) => {
                let src_val = self.get_reg(src);
                let dest_val = self.get_reg(dest);
                let (result, flags) = or(src_val, dest_val);
                self.set_reg(dest, result);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::AdcRegReg(dest, src) => {
                let src_val = self.get_reg(src);
                let dest_val = self.get_reg(dest);
                let (result, flags) = add(src_val, dest_val, self.get_flag(Flag::C));
                self.set_reg(dest, result);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::SbcRegReg(dest, src) => {
                let src_val = self.get_reg(src);
                let dest_val = self.get_reg(dest);
                let (result, flags) = sub(dest_val, src_val, self.get_flag(Flag::C));
                self.set_reg(dest, result);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::XorRegReg(dest, src) => {
                let src_val = self.get_reg(src);
                let dest_val = self.get_reg(dest);
                let (result, flags) = xor(src_val, dest_val);
                self.set_reg(dest, result);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::CpRegReg(dest, src) => {
                let src_val = self.get_reg(src);
                let dest_val = self.get_reg(dest);
                let (_, flags) = sub(dest_val, src_val, false);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::AddRegPairRegPair(dest, src) => {
                let src_val = self.get_reg_pair(src);
                let dest_val = self.get_reg_pair(dest);
                let (result, flags) = add16(src_val, dest_val, self.get_flags());
                self.set_reg_pair(dest, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::AddAccImm => {
                let imm = self.read_8_bit_immediate(memory);
                let a = self.get_reg(Register::A);
                let (result, flags) = add(a, imm, false);
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::AdcAccImm => {
                let imm = self.read_8_bit_immediate(memory);
                let a = self.get_reg(Register::A);
                let (result, flags) = add(a, imm, self.get_flag(Flag::C));
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::SubAccImm => {
                let imm = self.read_8_bit_immediate(memory);
                let a = self.get_reg(Register::A);
                let (result, flags) = sub(a, imm, false);
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::SbcAccImm => {
                let imm = self.read_8_bit_immediate(memory);
                let a = self.get_reg(Register::A);
                let (result, flags) = sub(a, imm, self.get_flag(Flag::C));
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::AndAccImm => {
                let imm = self.read_8_bit_immediate(memory);
                let a = self.get_reg(Register::A);
                let (result, flags) = and(a, imm);
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::OrAccImm => {
                let imm = self.read_8_bit_immediate(memory);
                let a = self.get_reg(Register::A);
                let (result, flags) = or(a, imm);
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::XorAccImm => {
                let imm = self.read_8_bit_immediate(memory);
                let a = self.get_reg(Register::A);
                let (result, flags) = xor(a, imm);
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::CpAccImm => {
                let imm = self.read_8_bit_immediate(memory);
                let a = self.get_reg(Register::A);
                let (_, flags) = sub(a, imm, false);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::AddSpImm => {
                // This is somewhat special as it is signed 16-bit addition
                let imm = sign_extend(self.read_8_bit_immediate(memory));
                let sp = self.get_reg_pair(RegisterPair::SP);
                let result = sp.wrapping_add(imm);
                self.set_reg_pair(RegisterPair::SP, result);
                self.set_flags(
                    Flags::new()
                        .with(Flag::H, carry_bit16(imm, sp, result, 4))
                        .with(Flag::C, carry_bit16(imm, sp, result, 8)),
                );
                Cycles::new(16)
            }
            OpCode::AddAccHlInd => {
                let hl = self.get_reg_pair(RegisterPair::HL);
                let mem = memory.read(hl);
                let a = self.get_reg(Register::A);
                let (result, flags) = add(a, mem, false);
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::AdcAccHlInd => {
                let hl = self.get_reg_pair(RegisterPair::HL);
                let mem = memory.read(hl);
                let a = self.get_reg(Register::A);
                let (result, flags) = add(a, mem, self.get_flag(Flag::C));
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::SubAccHlInd => {
                let hl = self.get_reg_pair(RegisterPair::HL);
                let mem = memory.read(hl);
                let a = self.get_reg(Register::A);
                let (result, flags) = sub(a, mem, false);
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::SbcAccHlInd => {
                let hl = self.get_reg_pair(RegisterPair::HL);
                let mem = memory.read(hl);
                let a = self.get_reg(Register::A);
                let (result, flags) = sub(a, mem, self.get_flag(Flag::C));
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::AndAccHlInd => {
                let hl = self.get_reg_pair(RegisterPair::HL);
                let mem = memory.read(hl);
                let a = self.get_reg(Register::A);
                let (result, flags) = and(a, mem);
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::XorAccHlInd => {
                let hl = self.get_reg_pair(RegisterPair::HL);
                let mem = memory.read(hl);
                let a = self.get_reg(Register::A);
                let (result, flags) = xor(a, mem);
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::OrAccHlInd => {
                let hl = self.get_reg_pair(RegisterPair::HL);
                let mem = memory.read(hl);
                let a = self.get_reg(Register::A);
                let (result, flags) = or(a, mem);
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::CpAccHlInd => {
                let hl = self.get_reg_pair(RegisterPair::HL);
                let mem = memory.read(hl);
                let a = self.get_reg(Register::A);
                let (_, flags) = sub(a, mem, false);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::IncReg(reg) => {
                let prev = self.get_reg(reg);
                let next = self.get_reg(reg).wrapping_add(1);
                self.set_reg(reg, next);
                self.set_flags(
                    self.get_flags()
                        .with(Flag::Z, next == 0)
                        .with(Flag::N, false)
                        .with(Flag::H, carry_bit8(prev, 1, next, 4)),
                );
                Cycles::new(4)
            }
            OpCode::DecReg(reg) => {
                let minus_one = 0xff;
                let prev = self.get_reg(reg);
                let next = prev.wrapping_add(minus_one);
                self.set_reg(reg, next);
                self.set_flags(
                    self.get_flags()
                        .with(Flag::Z, next == 0u8)
                        .with(Flag::N, true)
                        .with(Flag::H, !carry_bit8(minus_one, prev, next, 4)),
                );
                Cycles::new(4)
            }
            OpCode::IncRegPair(reg) => {
                let prev = self.get_reg_pair(reg);
                let next = prev.wrapping_add(1);
                self.set_reg_pair(reg, next);
                Cycles::new(8)
            }
            OpCode::DecRegPair(reg) => {
                let prev = self.get_reg_pair(reg);
                let next = prev.wrapping_sub(1);
                self.set_reg_pair(reg, next);
                Cycles::new(8)
            }
            OpCode::IncIndHl => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let prev = memory.read(addr);
                let next = prev.wrapping_add(1);
                memory.write(addr, next);
                self.set_flags(
                    self.get_flags()
                        .with(Flag::Z, next == 0)
                        .with(Flag::N, false)
                        .with(Flag::H, carry_bit8(prev, 1, next, 4)),
                );
                Cycles::new(12)
            }
            OpCode::DecIndHl => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let minus_one = 0xff;
                let prev = memory.read(addr);
                let next = prev.wrapping_add(minus_one);
                memory.write(addr, next);
                self.set_flags(
                    self.get_flags()
                        .with(Flag::Z, next == 0u8)
                        .with(Flag::N, true)
                        .with(Flag::H, !carry_bit8(minus_one, prev, next, 4)),
                );
                Cycles::new(12)
            }
            OpCode::Daa => {
                let a = self.get_reg(Register::A);
                let (result, flags) = daa(a, self.get_flags());
                self.set_reg(Register::A, result);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::Cpl => {
                let a = self.get_reg(Register::A);
                let complement = !a;
                self.set_reg(Register::A, complement);
                self.set_flags(self.get_flags().with(Flag::N, true).with(Flag::H, true));
                Cycles::new(4)
            }
            OpCode::Scf => {
                self.set_flags(
                    self.get_flags()
                        .with(Flag::C, true)
                        .with(Flag::N, false)
                        .with(Flag::H, false),
                );
                Cycles::new(4)
            }
            OpCode::Ccf => {
                let flags = self.get_flags();
                self.set_flags(
                    flags
                        .with(Flag::C, !flags.is_flag_set(Flag::C))
                        .with(Flag::N, false)
                        .with(Flag::H, false),
                );
                Cycles::new(4)
            }
            OpCode::JrImm => {
                let target_offset = self.read_8_bit_immediate(memory) as i8 as i16;
                let pc = self.get_regs().pc_reg;
                let target = (pc as i16).wrapping_add(target_offset) as u16;
                self.get_mut_regs().pc_reg = target;
                Cycles::new(12)
            }
            OpCode::JrCondImm(condition) => {
                let target_offset = self.read_8_bit_immediate(memory) as i8 as i16;
                if self.check_condition(condition) {
                    let pc = self.get_regs().pc_reg;
                    let target = (pc as i16).wrapping_add(target_offset) as u16;
                    self.get_mut_regs().pc_reg = target;
                    Cycles::new(12)
                } else {
                    Cycles::new(8)
                }
            }
            OpCode::RetCond(condition) => {
                if self.check_condition(condition) {
                    let value = self.stack_pop(memory);
                    self.get_mut_regs().pc_reg = value;
                    Cycles::new(20)
                } else {
                    Cycles::new(8)
                }
            }
            OpCode::Ret => {
                let value = self.stack_pop(memory);
                self.get_mut_regs().pc_reg = value;
                Cycles::new(16)
            }
            OpCode::Reti => {
                let value = self.stack_pop(memory);
                let regs = self.get_mut_regs();
                regs.pc_reg = value;
                regs.irq_en = true;
                Cycles::new(16)
            }
            OpCode::JpCondImm(condition) => {
                let target = self.read_16_bit_immediate(memory);
                if self.check_condition(condition) {
                    self.get_mut_regs().pc_reg = target;
                    Cycles::new(16)
                } else {
                    Cycles::new(12)
                }
            }
            OpCode::JpImm => {
                let target = self.read_16_bit_immediate(memory);
                self.get_mut_regs().pc_reg = target;
                Cycles::new(16)
            }
            OpCode::JpHl => {
                let target = self.get_reg_pair(RegisterPair::HL);
                self.get_mut_regs().pc_reg = target;
                Cycles::new(4)
            }
            OpCode::CallCondImm(condition) => {
                let target = self.read_16_bit_immediate(memory);
                if self.check_condition(condition) {
                    let return_addr = self.get_regs().pc_reg;
                    self.stack_push(memory, return_addr);
                    self.get_mut_regs().pc_reg = target;
                    Cycles::new(24)
                } else {
                    Cycles::new(12)
                }
            }
            OpCode::CallImm => {
                let target = self.read_16_bit_immediate(memory);
                let return_addr = self.get_regs().pc_reg;
                self.stack_push(memory, return_addr);
                self.get_mut_regs().pc_reg = target;
                Cycles::new(24)
            }
            OpCode::Reset(target) => {
                let target = translate_reset_target(target);
                let return_addr = self.get_regs().pc_reg;
                self.stack_push(memory, return_addr);
                self.get_mut_regs().pc_reg = target;
                Cycles::new(16)
            }
            OpCode::Pop(reg) => {
                let value = self.stack_pop(memory);
                self.set_reg_pair_stack(reg, value);
                Cycles::new(12)
            }
            OpCode::Push(reg) => {
                let value = self.get_reg_pair_stack(reg);
                self.stack_push(memory, value);
                Cycles::new(16)
            }
            OpCode::Di => {
                self.get_mut_regs().irq_en = false;
                Cycles::new(4)
            }
            OpCode::Ei => {
                // TODO: make this delayed by 1 instruction
                self.get_mut_regs().irq_en = true;
                Cycles::new(4)
            }
            OpCode::Rlca => {
                let (shifted, flags) = rlc(self.get_reg(Register::A), false);
                self.set_reg(Register::A, shifted);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::Rrca => {
                let (shifted, flags) = rrc(self.get_reg(Register::A), false);
                self.set_reg(Register::A, shifted);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::Rla => {
                let (shifted, flags) = rl(self.get_reg(Register::A), self.get_flag(Flag::C), false);
                self.set_reg(Register::A, shifted);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::Rra => {
                let (shifted, flags) = rr(self.get_reg(Register::A), self.get_flag(Flag::C), false);
                self.set_reg(Register::A, shifted);
                self.set_flags(flags);
                Cycles::new(4)
            }
            OpCode::RlcReg(register) => {
                let (shifted, flags) = rlc(self.get_reg(register), true);
                self.set_reg(register, shifted);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::RrcReg(register) => {
                let (shifted, flags) = rrc(self.get_reg(register), true);
                self.set_reg(register, shifted);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::RlReg(register) => {
                let (shifted, flags) = rl(self.get_reg(register), self.get_flag(Flag::C), true);
                self.set_reg(register, shifted);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::RrReg(register) => {
                let (shifted, flags) = rr(self.get_reg(register), self.get_flag(Flag::C), true);
                self.set_reg(register, shifted);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::SlaReg(register) => {
                let (shifted, flags) = sla(self.get_reg(register));
                self.set_reg(register, shifted);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::SraReg(register) => {
                let (shifted, flags) = sra(self.get_reg(register));
                self.set_reg(register, shifted);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::SwapReg(register) => {
                let (shifted, flags) = swap(self.get_reg(register));
                self.set_reg(register, shifted);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::SrlReg(register) => {
                let (shifted, flags) = srl(self.get_reg(register));
                self.set_reg(register, shifted);
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::Bit(bit_idx, register) => {
                let flags = bit(bit_idx, self.get_reg(register), self.get_flags());
                self.set_flags(flags);
                Cycles::new(8)
            }
            OpCode::Res(bit_idx, register) => {
                let value = res(bit_idx, self.get_reg(register));
                self.set_reg(register, value);
                Cycles::new(8)
            }
            OpCode::Set(bit_idx, register) => {
                let value = set(bit_idx, self.get_reg(register));
                self.set_reg(register, value);
                Cycles::new(8)
            }
            OpCode::RlcHlInd => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let (shifted, flags) = rlc(memory.read(addr), true);
                memory.write(addr, shifted);
                self.set_flags(flags);
                Cycles::new(16)
            }
            OpCode::RrcHlInd => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let (shifted, flags) = rrc(memory.read(addr), true);
                memory.write(addr, shifted);
                self.set_flags(flags);
                Cycles::new(16)
            }
            OpCode::RlHlInd => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let (shifted, flags) = rl(memory.read(addr), self.get_flag(Flag::C), true);
                memory.write(addr, shifted);
                self.set_flags(flags);
                Cycles::new(16)
            }
            OpCode::RrHlInd => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let (shifted, flags) = rr(memory.read(addr), self.get_flag(Flag::C), true);
                memory.write(addr, shifted);
                self.set_flags(flags);
                Cycles::new(16)
            }
            OpCode::SlaHlInd => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let (shifted, flags) = sla(memory.read(addr));
                memory.write(addr, shifted);
                self.set_flags(flags);
                Cycles::new(16)
            }
            OpCode::SraHlInd => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let (shifted, flags) = sra(memory.read(addr));
                memory.write(addr, shifted);
                self.set_flags(flags);
                Cycles::new(16)
            }
            OpCode::SwapHlInd => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let (shifted, flags) = swap(memory.read(addr));
                memory.write(addr, shifted);
                self.set_flags(flags);
                Cycles::new(16)
            }
            OpCode::SrlHlInd => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let (shifted, flags) = srl(memory.read(addr));
                memory.write(addr, shifted);
                self.set_flags(flags);
                Cycles::new(16)
            }
            OpCode::BitHlInd(bit_idx) => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let flags = bit(bit_idx, memory.read(addr), self.get_flags());
                self.set_flags(flags);
                Cycles::new(12)
            }
            OpCode::ResHlInd(bit_idx) => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let value = res(bit_idx, memory.read(addr));
                memory.write(addr, value);
                Cycles::new(16)
            }
            OpCode::SetHlInd(bit_idx) => {
                let addr = self.get_reg_pair(RegisterPair::HL);
                let value = set(bit_idx, memory.read(addr));
                memory.write(addr, value);
                Cycles::new(16)
            }
        };
        ExitReason::Step(cycles)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    pub fn test_sign_extend() {
        assert_eq!(sign_extend(0x80u8), 0xFF80u16);
        assert_eq!(sign_extend(0x7fu8), 0x007fu16);
    }

    #[test]
    pub fn test_daa() {
        assert_eq!(daa(0x23, Flags::new()), (0x23, Flags::new()));
        assert_eq!(daa(0x29, Flags::new()), (0x29, Flags::new()));
        assert_eq!(
            daa(0x00, Flags::new()),
            (0x00, Flags::new().with(Flag::Z, true))
        );
        assert_eq!(
            daa(0x9A, Flags::new()),
            (0x00, Flags::new().with(Flag::Z, true).with(Flag::C, true))
        );

        assert_eq!(
            daa(0xB3, Flags::new()),
            (0x13, Flags::new().with(Flag::C, true))
        );

        assert_eq!(
            daa(0xBF, Flags::new()),
            (0x25, Flags::new().with(Flag::C, true))
        );
    }
}
