//! Contains the core functionality of the SM83 CPU, including the actual CPU, registers
//! and internal CPU flags.

use crate::{
    decoder::{self, AddressingMode, Bit, Condition, OpCode, Register, RegisterPair, ResetTarget},
    interrupts::{Interrupt, Interrupts},
    memory::Memory,
};

/// A single CPU flag
#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum Flag {
    /// Zero flag
    Z = 1 << 7,
    /// Negative flag
    N = 1 << 6,
    /// Half-carry flag
    H = 1 << 5,
    /// Carry flag
    C = 1 << 4,
}

/// A combination of CPU flags, which are either set or unset
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
    /// Constructs a Flags value with all flags in the unset state
    pub const fn new() -> Self {
        Self(0)
    }

    /// Returns a new set of flags with the value of the given flag set or unset as requested.
    pub const fn with(mut self, flag: Flag, value: bool) -> Self {
        if value {
            self.0 |= flag as u8;
        } else {
            self.0 &= !(flag as u8);
        }
        self
    }

    /// Returns true if the given flag is set in the Flags instance.
    pub const fn is_flag_set(&self, flag: Flag) -> bool {
        (self.0 & flag as u8) != 0
    }
}

/// CPU Registers in a struct
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Registers {
    /// CPU Flags
    pub flags: Flags,
    /// The A register of the CPU
    pub a_reg: u8,
    /// The B register of the CPU
    pub b_reg: u8,
    /// The C register of the CPU
    pub c_reg: u8,
    /// The D register of the CPU
    pub d_reg: u8,
    /// The E register of the CPU
    pub e_reg: u8,
    /// The H register of the CPU
    pub h_reg: u8,
    /// The L register of the CPU
    pub l_reg: u8,
    /// The stack pointer of the CPU
    pub sp_reg: u16,
    /// The program counter of the CPU
    pub pc_reg: u16,
    /// The IME register of the CPU (set if IRQs are globally enabled)
    pub irq_en: bool,
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
    debug_assert!(bit < 8);
    let xor = a ^ b ^ c;
    (xor & (1 << bit)) != 0
}

const fn carry_bit16(a: u16, b: u16, c: u16, bit: usize) -> bool {
    debug_assert!(bit < 16);
    let xor = a ^ b ^ c;
    (xor & (1 << bit)) != 0
}

const fn carry_bit32(a: u32, b: u32, c: u32, bit: usize) -> bool {
    debug_assert!(bit < 32);
    let xor = a ^ b ^ c;
    (xor & (1 << bit)) != 0
}

#[cfg_attr(feature = "profile", inline(never))]
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

#[cfg_attr(feature = "profile", inline(never))]
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

#[cfg_attr(feature = "profile", inline(never))]
const fn sub(a: u8, b: u8, carry: bool) -> (u8, Flags) {
    let a = a as u16;
    let b = b as u16;
    let b_plus_carry = b.wrapping_add(carry as u16);
    let inv = (!b_plus_carry).wrapping_add(1); // 2's compliment of a + b
    let result = a.wrapping_add(inv);

    let flags = Flags::new()
        .with(Flag::Z, (result & 0xFF) == 0)
        .with(Flag::N, true)
        .with(Flag::H, carry_bit16(a, b, result, 4))
        .with(Flag::C, carry_bit16(a, b, result, 8));

    (result as u8, flags)
}

#[cfg_attr(feature = "profile", inline(never))]
const fn and(a: u8, b: u8) -> (u8, Flags) {
    let result = a & b;

    let flags = Flags::new().with(Flag::Z, result == 0).with(Flag::H, true);

    (result, flags)
}

#[cfg_attr(feature = "profile", inline(never))]
const fn or(a: u8, b: u8) -> (u8, Flags) {
    let result = a | b;
    let flags = Flags::new().with(Flag::Z, result == 0);
    (result, flags)
}

#[cfg_attr(feature = "profile", inline(never))]
const fn xor(a: u8, b: u8) -> (u8, Flags) {
    let result = a ^ b;
    let flags = Flags::new().with(Flag::Z, result == 0);
    (result, flags)
}

#[cfg_attr(feature = "profile", inline(never))]
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
#[cfg_attr(feature = "profile", inline(never))]
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
#[cfg_attr(feature = "profile", inline(never))]
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

#[cfg_attr(feature = "profile", inline(never))]
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

#[cfg_attr(feature = "profile", inline(never))]
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

#[cfg_attr(feature = "profile", inline(never))]
const fn sla(value: u8) -> (u8, Flags) {
    let shifted = value << 1;
    let new_carry = (value & 0x80) != 0;
    let flags = Flags::new()
        .with(Flag::C, new_carry)
        .with(Flag::Z, shifted == 0);
    (shifted, flags)
}

#[cfg_attr(feature = "profile", inline(never))]
const fn sra(value: u8) -> (u8, Flags) {
    let negative = (value & 0x80) != 0;
    let new_carry = (value & 0x01) != 0;
    let shifted = (value >> 1) | if negative { 0x80 } else { 0x00 };
    let flags = Flags::new()
        .with(Flag::C, new_carry)
        .with(Flag::Z, shifted == 0);
    (shifted, flags)
}

#[cfg_attr(feature = "profile", inline(never))]
const fn swap(value: u8) -> (u8, Flags) {
    let swapped = (value >> 4) | (value << 4);
    let flags = Flags::new().with(Flag::Z, swapped == 0);
    (swapped, flags)
}

#[cfg_attr(feature = "profile", inline(never))]
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

#[cfg_attr(feature = "profile", inline(never))]
const fn bit(bit_idx: Bit, value: u8, flags: Flags) -> Flags {
    let bit = bit_mask(bit_idx);
    let z_flag = (bit & value) == 0;
    flags
        .with(Flag::N, false)
        .with(Flag::H, true)
        .with(Flag::Z, z_flag)
}

#[cfg_attr(feature = "profile", inline(never))]
const fn res(bit_idx: Bit, value: u8) -> u8 {
    value & !bit_mask(bit_idx)
}

#[cfg_attr(feature = "profile", inline(never))]
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
#[repr(transparent)]
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

/// The exit reason of the CPU after stepping an instruction.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExitReason {
    /// The step of the instruction concluded successfully, and took the given number of clock cycles.
    Step(Cycles),
    /// An interrupt was taken, and and took the given number of clock cycles.
    InterruptTaken(Cycles, Interrupt),
    /// The CPU is stopped, and executed the given number of cycles
    Stop(Cycles),
    /// The CPU is halted, and executed the given number of cycles
    Halt(Cycles),
    /// The CPU attempted to execute an illegal OpCode.
    IllegalOpcode,
}

/// An abstraction of the CPU core
pub struct Cpu {
    regs: Registers,
    halted: bool,
}

impl Cpu {
    /// Constructs a new `Cpu` instance.
    pub fn new() -> Self {
        Self {
            regs: Registers::new(),
            halted: false,
        }
    }

    /// Queries the registers of the CPU
    pub const fn get_regs(&self) -> &Registers {
        &self.regs
    }

    /// Mutably queries the registers of the CPU
    pub fn get_mut_regs(&mut self) -> &mut Registers {
        &mut self.regs
    }

    const fn get_flag(&self, flag: Flag) -> bool {
        self.regs.flags.is_flag_set(flag)
    }

    const fn get_flags(&self) -> Flags {
        self.regs.flags
    }

    fn set_flags(&mut self, flags: Flags) {
        self.regs.flags = flags;
    }

    const fn get_reg(&self, reg: Register) -> u8 {
        match reg {
            Register::A => self.regs.a_reg,
            Register::B => self.regs.b_reg,
            Register::C => self.regs.c_reg,
            Register::D => self.regs.d_reg,
            Register::E => self.regs.e_reg,
            Register::H => self.regs.h_reg,
            Register::L => self.regs.l_reg,
        }
    }

    fn set_reg(&mut self, reg: Register, value: u8) {
        let target = match reg {
            Register::A => &mut self.regs.a_reg,
            Register::B => &mut self.regs.b_reg,
            Register::C => &mut self.regs.c_reg,
            Register::D => &mut self.regs.d_reg,
            Register::E => &mut self.regs.e_reg,
            Register::H => &mut self.regs.h_reg,
            Register::L => &mut self.regs.l_reg,
        };
        *target = value;
    }

    fn get_reg_pair(&mut self, reg: RegisterPair) -> u16 {
        let (hi, lo) = match reg {
            RegisterPair::BC => (self.regs.b_reg, self.regs.c_reg),
            RegisterPair::DE => (self.regs.d_reg, self.regs.e_reg),
            RegisterPair::HL | RegisterPair::HLINC | RegisterPair::HLDEC => {
                (self.regs.h_reg, self.regs.l_reg)
            }
            RegisterPair::SP => {
                return self.regs.sp_reg;
            }
            RegisterPair::AF => (self.regs.a_reg, self.regs.flags.0),
        };
        let value = ((hi as u16) << 8) | (lo as u16);

        match reg {
            RegisterPair::HLINC => {
                let value = value.wrapping_add(1);
                self.regs.h_reg = (value >> 8) as u8;
                self.regs.l_reg = (value & 0xFF) as u8;
            }
            RegisterPair::HLDEC => {
                let value = value.wrapping_sub(1);
                self.regs.h_reg = (value >> 8) as u8;
                self.regs.l_reg = (value & 0xFF) as u8;
            }
            _ => {}
        }
        value
    }

    fn set_reg_pair(&mut self, reg: RegisterPair, value: u16) {
        let (hi, lo) = match reg {
            RegisterPair::BC => (&mut self.regs.b_reg, &mut self.regs.c_reg),
            RegisterPair::DE => (&mut self.regs.d_reg, &mut self.regs.e_reg),
            RegisterPair::HL | RegisterPair::HLINC | RegisterPair::HLDEC => {
                (&mut self.regs.h_reg, &mut self.regs.l_reg)
            }
            RegisterPair::SP => {
                self.regs.sp_reg = value;
                return;
            }
            RegisterPair::AF => {
                let (hi, lo) = (&mut self.regs.a_reg, &mut self.regs.flags);
                *hi = ((value >> 8) & 0xff) as u8;
                *lo = ((value & 0xf0) as u8).into(); // lo-bits are hardcoded to 0
                return;
            }
        };
        *hi = ((value >> 8) & 0xff) as u8;
        *lo = (value & 0xff) as u8;
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn step_pc(&mut self) -> u16 {
        let pc = self.regs.pc_reg;
        self.regs.pc_reg = self.regs.pc_reg.wrapping_add(1);
        pc
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn read_8_bit_immediate<T: Memory>(&mut self, memory: &mut T) -> u8 {
        let pc = self.step_pc();
        memory.read(pc)
    }

    #[cfg_attr(feature = "profile", inline(never))]
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

    #[cfg_attr(feature = "profile", inline(never))]
    fn stack_push<T: Memory>(&mut self, memory: &mut T, value: u16) {
        let sp = self.regs.sp_reg;
        let pos = sp.wrapping_sub(1);
        memory.write(pos, (value >> 8) as u8);
        let pos = pos.wrapping_sub(1);
        memory.write(pos, (value & 0xff) as u8);
        self.regs.sp_reg = pos;
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn stack_pop<T: Memory>(&mut self, memory: &mut T) -> u16 {
        let sp = self.regs.sp_reg;
        let pos = sp;
        let lo = memory.read(pos);
        let pos = pos.wrapping_add(1);
        let hi = memory.read(pos);
        let pos = pos.wrapping_add(1);
        self.regs.sp_reg = pos;
        (lo as u16) | ((hi as u16) << 8)
    }

    #[cfg_attr(feature = "profile", inline(never))]
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

    /// Executes a single CPU instruction and returns from the function.
    #[cfg_attr(feature = "profile", inline(never))]
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
            let return_addr = self.regs.pc_reg;
            self.stack_push(memory, return_addr);
            self.regs.pc_reg = translate_irq_target(irq);
            ExitReason::InterruptTaken(Cycles::new(20), irq)
        } else {
            let instruction = self.fetch_and_decode(memory);
            self.execute(memory, instruction)
        }
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn load_8bit_with_addressing_mode<T: Memory>(
        &mut self,
        memory: &mut T,
        mode: AddressingMode,
    ) -> (Cycles, u8) {
        match mode {
            AddressingMode::Register(r) => (Cycles::new(0), self.get_reg(r)),
            AddressingMode::Immediate => (Cycles::new(4), self.read_8_bit_immediate(memory)),
            AddressingMode::IndirectRegister(r) => {
                let addr = self.get_reg_pair(r);
                (Cycles::new(4), memory.read(addr))
            }
            AddressingMode::IndirectZeroPageRegister(r) => {
                let addr = self.get_reg(r) as u16 | 0xFF00;
                (Cycles::new(4), memory.read(addr))
            }
            AddressingMode::IndirectZeroPageImmediate => {
                let addr = self.read_8_bit_immediate(memory) as u16 | 0xFF00;
                (Cycles::new(8), memory.read(addr))
            }
            AddressingMode::IndirectImmediate => {
                let addr = self.read_16_bit_immediate(memory);
                (Cycles::new(12), memory.read(addr))
            }
            _ => unreachable!(),
        }
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn store_8bit_with_addressing_mode<T: Memory>(
        &mut self,
        memory: &mut T,
        mode: AddressingMode,
        value: u8,
    ) -> Cycles {
        match mode {
            AddressingMode::Register(r) => {
                self.set_reg(r, value);
                Cycles::new(0)
            }
            AddressingMode::IndirectRegister(r) => {
                let addr = self.get_reg_pair(r);
                memory.write(addr, value);
                Cycles::new(4)
            }
            AddressingMode::IndirectZeroPageRegister(r) => {
                let addr = self.get_reg(r) as u16 | 0xFF00;
                memory.write(addr, value);
                Cycles::new(4)
            }
            AddressingMode::IndirectZeroPageImmediate => {
                let addr = self.read_8_bit_immediate(memory) as u16 | 0xFF00;
                memory.write(addr, value);
                Cycles::new(8)
            }
            AddressingMode::IndirectImmediate => {
                let addr = self.read_16_bit_immediate(memory);
                memory.write(addr, value);
                Cycles::new(12)
            }
            _ => unreachable!(),
        }
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn load_16bit_with_addressing_mode<T: Memory>(
        &mut self,
        memory: &mut T,
        mode: AddressingMode,
    ) -> (Cycles, u16) {
        match mode {
            AddressingMode::RegisterPair(r) => (Cycles::new(0), self.get_reg_pair(r)),
            AddressingMode::Immediate16 => (Cycles::new(8), self.read_16_bit_immediate(memory)),
            _ => unreachable!(),
        }
    }

    #[cfg_attr(feature = "profile", inline(never))]
    fn store_16bit_with_addressing_mode<T: Memory>(
        &mut self,
        memory: &mut T,
        mode: AddressingMode,
        value: u16,
    ) -> Cycles {
        match mode {
            AddressingMode::RegisterPair(r) => {
                self.set_reg_pair(r, value);
                Cycles::new(0)
            }
            AddressingMode::IndirectImmediate => {
                let addr = self.read_16_bit_immediate(memory);
                memory.write(addr, (value & 0xff) as u8);
                memory.write(addr.wrapping_add(1), (value >> 8) as u8);
                Cycles::new(16)
            }
            _ => unreachable!(),
        }
    }

    #[cfg_attr(feature = "profile", inline(never))]
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
            OpCode::Ld8(dest, src) => {
                let (cycles, value) = self.load_8bit_with_addressing_mode(memory, src);
                Cycles::new(4) + cycles + self.store_8bit_with_addressing_mode(memory, dest, value)
            }
            OpCode::Ld16(dest, src) => {
                let (cycles, value) = self.load_16bit_with_addressing_mode(memory, src);
                let cycles = Cycles::new(4)
                    + cycles
                    + self.store_16bit_with_addressing_mode(memory, dest, value);
                if cycles < Cycles::new(8) {
                    Cycles::new(8)
                } else {
                    cycles
                }
            }
            OpCode::Ld16HlSpImm => {
                let sp = self.regs.sp_reg;
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
            OpCode::Add8(dest, src) => {
                let (src_cycles, src_val) = self.load_8bit_with_addressing_mode(memory, src);
                let (dest_cycles, dest_val) = self.load_8bit_with_addressing_mode(memory, dest);

                let (result, flags) = add(src_val, dest_val, false);
                self.set_flags(flags);

                Cycles::new(4)
                    + src_cycles
                    + dest_cycles
                    + self.store_8bit_with_addressing_mode(memory, dest, result)
            }
            OpCode::Sub8(dest, src) => {
                let (src_cycles, src_val) = self.load_8bit_with_addressing_mode(memory, src);
                let (dest_cycles, dest_val) = self.load_8bit_with_addressing_mode(memory, dest);

                let (result, flags) = sub(dest_val, src_val, false);
                self.set_flags(flags);

                Cycles::new(4)
                    + src_cycles
                    + dest_cycles
                    + self.store_8bit_with_addressing_mode(memory, dest, result)
            }

            OpCode::And8(dest, src) => {
                let (src_cycles, src_val) = self.load_8bit_with_addressing_mode(memory, src);
                let (dest_cycles, dest_val) = self.load_8bit_with_addressing_mode(memory, dest);

                let (result, flags) = and(src_val, dest_val);
                self.set_flags(flags);

                Cycles::new(4)
                    + src_cycles
                    + dest_cycles
                    + self.store_8bit_with_addressing_mode(memory, dest, result)
            }
            OpCode::Or8(dest, src) => {
                let (src_cycles, src_val) = self.load_8bit_with_addressing_mode(memory, src);
                let (dest_cycles, dest_val) = self.load_8bit_with_addressing_mode(memory, dest);

                let (result, flags) = or(src_val, dest_val);
                self.set_flags(flags);

                Cycles::new(4)
                    + src_cycles
                    + dest_cycles
                    + self.store_8bit_with_addressing_mode(memory, dest, result)
            }
            OpCode::Adc8(dest, src) => {
                let (src_cycles, src_val) = self.load_8bit_with_addressing_mode(memory, src);
                let (dest_cycles, dest_val) = self.load_8bit_with_addressing_mode(memory, dest);

                let (result, flags) = add(src_val, dest_val, self.get_flag(Flag::C));
                self.set_flags(flags);

                Cycles::new(4)
                    + src_cycles
                    + dest_cycles
                    + self.store_8bit_with_addressing_mode(memory, dest, result)
            }
            OpCode::Sbc8(dest, src) => {
                let (src_cycles, src_val) = self.load_8bit_with_addressing_mode(memory, src);
                let (dest_cycles, dest_val) = self.load_8bit_with_addressing_mode(memory, dest);

                let (result, flags) = sub(dest_val, src_val, self.get_flag(Flag::C));
                self.set_flags(flags);

                Cycles::new(4)
                    + src_cycles
                    + dest_cycles
                    + self.store_8bit_with_addressing_mode(memory, dest, result)
            }
            OpCode::Xor8(dest, src) => {
                let (src_cycles, src_val) = self.load_8bit_with_addressing_mode(memory, src);
                let (dest_cycles, dest_val) = self.load_8bit_with_addressing_mode(memory, dest);

                let (result, flags) = xor(src_val, dest_val);
                self.set_flags(flags);

                Cycles::new(4)
                    + src_cycles
                    + dest_cycles
                    + self.store_8bit_with_addressing_mode(memory, dest, result)
            }
            OpCode::Cp8(dest, src) => {
                let (src_cycles, src_val) = self.load_8bit_with_addressing_mode(memory, src);
                let (dest_cycles, dest_val) = self.load_8bit_with_addressing_mode(memory, dest);

                let (_, flags) = sub(dest_val, src_val, false);
                self.set_flags(flags);

                Cycles::new(4) + src_cycles + dest_cycles
            }
            OpCode::Add16(dest, src) => {
                let (src_cycles, src_val) = self.load_16bit_with_addressing_mode(memory, src);
                let (dest_cycles, dest_val) = self.load_16bit_with_addressing_mode(memory, dest);

                let (result, flags) = add16(src_val, dest_val, self.get_flags());
                self.set_flags(flags);

                Cycles::new(8)
                    + src_cycles
                    + dest_cycles
                    + self.store_16bit_with_addressing_mode(memory, dest, result)
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
            OpCode::Inc8(reg) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, reg);

                let next = prev.wrapping_add(1);
                self.set_flags(
                    self.get_flags()
                        .with(Flag::Z, next == 0)
                        .with(Flag::N, false)
                        .with(Flag::H, carry_bit8(prev, 1, next, 4)),
                );

                Cycles::new(4)
                    + load_cycles
                    + self.store_8bit_with_addressing_mode(memory, reg, next)
            }
            OpCode::Dec8(reg) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, reg);

                const MINUS_ONE: u8 = 0xff;
                let next = prev.wrapping_add(MINUS_ONE);
                self.set_flags(
                    self.get_flags()
                        .with(Flag::Z, next == 0u8)
                        .with(Flag::N, true)
                        .with(Flag::H, !carry_bit8(MINUS_ONE, prev, next, 4)),
                );

                Cycles::new(4)
                    + load_cycles
                    + self.store_8bit_with_addressing_mode(memory, reg, next)
            }
            OpCode::Inc16(reg) => {
                let (load_cycles, prev) = self.load_16bit_with_addressing_mode(memory, reg);
                let next = prev.wrapping_add(1);
                Cycles::new(8)
                    + load_cycles
                    + self.store_16bit_with_addressing_mode(memory, reg, next)
            }
            OpCode::Dec16(reg) => {
                let (load_cycles, prev) = self.load_16bit_with_addressing_mode(memory, reg);
                let next = prev.wrapping_sub(1);
                Cycles::new(8)
                    + load_cycles
                    + self.store_16bit_with_addressing_mode(memory, reg, next)
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
            OpCode::JrImm(None) => {
                let target_offset = self.read_8_bit_immediate(memory) as i8 as i16;
                let pc = self.regs.pc_reg;
                let target = (pc as i16).wrapping_add(target_offset) as u16;
                self.regs.pc_reg = target;
                Cycles::new(12)
            }
            OpCode::JrImm(Some(condition)) => {
                let target_offset = self.read_8_bit_immediate(memory) as i8 as i16;
                if self.check_condition(condition) {
                    let pc = self.regs.pc_reg;
                    let target = (pc as i16).wrapping_add(target_offset) as u16;
                    self.regs.pc_reg = target;
                    Cycles::new(12)
                } else {
                    Cycles::new(8)
                }
            }
            OpCode::Ret(None) => {
                let value = self.stack_pop(memory);
                self.regs.pc_reg = value;
                Cycles::new(16)
            }
            OpCode::Ret(Some(condition)) => {
                if self.check_condition(condition) {
                    let value = self.stack_pop(memory);
                    self.regs.pc_reg = value;
                    Cycles::new(20)
                } else {
                    Cycles::new(8)
                }
            }
            OpCode::Reti => {
                let value = self.stack_pop(memory);
                self.regs.pc_reg = value;
                self.regs.irq_en = true;
                Cycles::new(16)
            }
            OpCode::JpImm(Some(condition)) => {
                let target = self.read_16_bit_immediate(memory);
                if self.check_condition(condition) {
                    self.regs.pc_reg = target;
                    Cycles::new(16)
                } else {
                    Cycles::new(12)
                }
            }
            OpCode::JpImm(None) => {
                let target = self.read_16_bit_immediate(memory);
                self.regs.pc_reg = target;
                Cycles::new(16)
            }
            OpCode::JpHl => {
                let target = self.get_reg_pair(RegisterPair::HL);
                self.regs.pc_reg = target;
                Cycles::new(4)
            }
            OpCode::CallImm(Some(condition)) => {
                let target = self.read_16_bit_immediate(memory);
                if self.check_condition(condition) {
                    let return_addr = self.regs.pc_reg;
                    self.stack_push(memory, return_addr);
                    self.regs.pc_reg = target;
                    Cycles::new(24)
                } else {
                    Cycles::new(12)
                }
            }
            OpCode::CallImm(None) => {
                let target = self.read_16_bit_immediate(memory);
                let return_addr = self.regs.pc_reg;
                self.stack_push(memory, return_addr);
                self.regs.pc_reg = target;
                Cycles::new(24)
            }
            OpCode::Reset(target) => {
                let target = translate_reset_target(target);
                let return_addr = self.regs.pc_reg;
                self.stack_push(memory, return_addr);
                self.regs.pc_reg = target;
                Cycles::new(16)
            }
            OpCode::Pop(reg) => {
                let value = self.stack_pop(memory);
                self.set_reg_pair(reg, value);
                Cycles::new(12)
            }
            OpCode::Push(reg) => {
                let value = self.get_reg_pair(reg);
                self.stack_push(memory, value);
                Cycles::new(16)
            }
            OpCode::Di => {
                self.regs.irq_en = false;
                Cycles::new(4)
            }
            OpCode::Ei => {
                // TODO: make this delayed by 1 instruction
                self.regs.irq_en = true;
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
            OpCode::Rlc(mode) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, mode);
                let (shifted, flags) = rlc(prev, true);
                self.set_flags(flags);
                Cycles::new(8)
                    + load_cycles
                    + self.store_8bit_with_addressing_mode(memory, mode, shifted)
            }
            OpCode::Rrc(mode) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, mode);
                let (shifted, flags) = rrc(prev, true);
                self.set_flags(flags);
                Cycles::new(8)
                    + load_cycles
                    + self.store_8bit_with_addressing_mode(memory, mode, shifted)
            }
            OpCode::Rl(mode) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, mode);
                let (shifted, flags) = rl(prev, self.get_flag(Flag::C), true);
                self.set_flags(flags);
                Cycles::new(8)
                    + load_cycles
                    + self.store_8bit_with_addressing_mode(memory, mode, shifted)
            }
            OpCode::Rr(mode) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, mode);
                let (shifted, flags) = rr(prev, self.get_flag(Flag::C), true);
                self.set_flags(flags);
                Cycles::new(8)
                    + load_cycles
                    + self.store_8bit_with_addressing_mode(memory, mode, shifted)
            }
            OpCode::Sla(mode) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, mode);
                let (shifted, flags) = sla(prev);
                self.set_flags(flags);
                Cycles::new(8)
                    + load_cycles
                    + self.store_8bit_with_addressing_mode(memory, mode, shifted)
            }
            OpCode::Sra(mode) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, mode);
                let (shifted, flags) = sra(prev);
                self.set_flags(flags);
                Cycles::new(8)
                    + load_cycles
                    + self.store_8bit_with_addressing_mode(memory, mode, shifted)
            }
            OpCode::Swap(mode) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, mode);
                let (shifted, flags) = swap(prev);
                self.set_flags(flags);
                Cycles::new(8)
                    + load_cycles
                    + self.store_8bit_with_addressing_mode(memory, mode, shifted)
            }
            OpCode::Srl(mode) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, mode);
                let (shifted, flags) = srl(prev);
                self.set_flags(flags);
                Cycles::new(8)
                    + load_cycles
                    + self.store_8bit_with_addressing_mode(memory, mode, shifted)
            }
            OpCode::Bit(bit_idx, mode) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, mode);
                let flags = bit(bit_idx, prev, self.get_flags());
                self.set_flags(flags);

                Cycles::new(8) + load_cycles
            }
            OpCode::Res(bit_idx, mode) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, mode);
                let value = res(bit_idx, prev);
                Cycles::new(8)
                    + load_cycles
                    + self.store_8bit_with_addressing_mode(memory, mode, value)
            }
            OpCode::Set(bit_idx, mode) => {
                let (load_cycles, prev) = self.load_8bit_with_addressing_mode(memory, mode);
                let value = set(bit_idx, prev);
                Cycles::new(8)
                    + load_cycles
                    + self.store_8bit_with_addressing_mode(memory, mode, value)
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
