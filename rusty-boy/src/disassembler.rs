use core::iter::Cloned;
use core::slice::Iter;

use cartridge::header::CartridgeHeader;

use sm83::decoder::{Bit, Condition, Register, RegisterPair, ResetTarget};

use sm83::memory::Memory;

#[derive(Debug)]
pub enum Error {
    NoEntrypoint,
    CartridgeError(cartridge::header::Error),
}

impl From<cartridge::header::Error> for Error {
    fn from(value: cartridge::header::Error) -> Self {
        Self::CartridgeError(value)
    }
}

pub struct Disassembler<'a> {
    data: &'a [u8],
}

impl<'a> Disassembler<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn header(&'a self) -> Result<CartridgeHeader<'a>, Error> {
        Ok(CartridgeHeader::try_new(self.data)?)
    }

    pub fn entrypoint(&'a self) -> Result<InstructionIter<Cloned<Iter<'a, u8>>>, Error> {
        Ok(InstructionIter::new(self.header()?.entrypoint, 0x100))
    }

    pub fn disassemble(&self) -> Result<InstructionIter<Cloned<Iter<'a, u8>>>, Error> {
        let mut entrypoint = None;
        for (_addr, insn) in InstructionIter::new(self.header()?.entrypoint, 0x100) {
            if let Instruction::JpImm(None, imm) = insn {
                entrypoint = Some(imm);
            }
        }

        let Some(entrypoint) = entrypoint else {
            return Err(Error::NoEntrypoint);
        };

        Ok(InstructionIter::new(
            &self.data[entrypoint as usize..],
            entrypoint as usize,
        ))
    }
}

pub fn disassemble_single_inst(
    memory: &mut crate::memory::GbAddressSpace,
    addr: sm83::memory::Address,
) -> Instruction {
    // An instruction is at most 3 bytes
    let data = [
        memory.read(addr),
        memory.read(addr.wrapping_add(1)),
        memory.read(addr.wrapping_add(2)),
        memory.read(addr.wrapping_add(3)),
    ];

    InstructionIter::new(&data, addr as usize)
        .next()
        .map(|e| e.1)
        .unwrap()
}

#[derive(Debug, Clone, Copy)]
pub enum AddressingMode {
    IndirectRegister(RegisterPair),
    IndirectZeroPageRegister(Register),
    IndirectImmediate(u16),
    IndirectZeroPageImmediate(u8),
    Register(Register),
    RegisterPair(RegisterPair),
    Immediate(u8),
    Immediate16(u16),
}

fn translate_addr_mode<T>(iter: &mut T, mode: sm83::decoder::AddressingMode) -> AddressingMode
where
    T: Iterator<Item = u8>,
{
    let read_u16 = |iter: &mut T| {
        let lo = iter.next().unwrap();
        let hi = iter.next().unwrap();
        (lo as u16) | ((hi as u16) << 8)
    };

    let read_u8 = |iter: &mut T| iter.next().unwrap();

    match mode {
        sm83::decoder::AddressingMode::IndirectRegister(reg_pair) => {
            AddressingMode::IndirectRegister(reg_pair)
        }
        sm83::decoder::AddressingMode::IndirectZeroPageRegister(reg) => {
            AddressingMode::IndirectZeroPageRegister(reg)
        }
        sm83::decoder::AddressingMode::IndirectImmediate => {
            AddressingMode::IndirectImmediate(read_u16(iter))
        }
        sm83::decoder::AddressingMode::IndirectZeroPageImmediate => {
            AddressingMode::IndirectZeroPageImmediate(read_u8(iter))
        }
        sm83::decoder::AddressingMode::Register(reg) => AddressingMode::Register(reg),
        sm83::decoder::AddressingMode::RegisterPair(reg) => AddressingMode::RegisterPair(reg),
        sm83::decoder::AddressingMode::Immediate => AddressingMode::Immediate(read_u8(iter)),
        sm83::decoder::AddressingMode::Immediate16 => AddressingMode::Immediate16(read_u16(iter)),
    }
}

pub enum Instruction {
    Ld8(AddressingMode, AddressingMode),   // ld 8-bit instruction
    Ld16(AddressingMode, AddressingMode),  // ld 16-bit instruction
    Add8(AddressingMode, AddressingMode),  // add 8-bit instruction
    Sub8(AddressingMode, AddressingMode),  // sub 8-bit instruction
    And8(AddressingMode, AddressingMode),  // and 8-bit instruction
    Or8(AddressingMode, AddressingMode),   // or 8-bit instruction
    Adc8(AddressingMode, AddressingMode),  // adc 8-bit instruction
    Sbc8(AddressingMode, AddressingMode),  // sbc 8-bit instruction
    Xor8(AddressingMode, AddressingMode),  // xor 8-bit instruction
    Cp8(AddressingMode, AddressingMode),   // cp 8-bit instruction
    Add16(AddressingMode, AddressingMode), // add 16-bit instruction
    Inc8(AddressingMode),                  // inc 8-bit
    Dec8(AddressingMode),                  // dec 8-bit
    Inc16(AddressingMode),                 // inc 16-bit
    Dec16(AddressingMode),                 // dec 16-bit
    JrImm(Option<Condition>, i8),          // jr 8-bit immediate, with or without condition
    Ret(Option<Condition>),                // ret with optional condition
    Reti,                                  // reti
    JpImm(Option<Condition>, u16),         // jp imm16, optional condition
    JpHl,                                  // jp HL
    CallImm(Option<Condition>, u16),       // call imm16, optional condition
    Reset(ResetTarget),                    // reset target
    Pop(RegisterPair),                     // pop
    Push(RegisterPair),                    // push
    AddSpImm(i8),                          // add SP, n8
    Ld16HlSpImm(i8),                       // ld HL, SP + n8
    Di,                                    // di
    Ei,                                    // ei
    Halt,                                  // halt
    Nop,                                   // nop
    Rlca,                                  // rlca
    Rrca,                                  // rrca
    Rla,                                   // rla
    Rra,                                   // rra
    Daa,                                   // daa
    Cpl,                                   // cpl
    Scf,                                   // scf
    Ccf,                                   // ccf
    Stop,                                  // stop
    Rlc(AddressingMode),                   // rlc
    Rrc(AddressingMode),                   // rrc
    Rl(AddressingMode),                    // rl
    Rr(AddressingMode),                    // rr
    Sla(AddressingMode),                   // sla
    Sra(AddressingMode),                   // sra
    Swap(AddressingMode),                  // swap
    Srl(AddressingMode),                   // srl
    Bit(Bit, AddressingMode),              // bit #bit
    Res(Bit, AddressingMode),              // res #bit
    Set(Bit, AddressingMode),              // set #bit
    Illegal,                               // Illegal
}

impl Instruction {
    fn reg_to_repr(reg: Register) -> &'static str {
        match reg {
            Register::A => "A",
            Register::B => "B",
            Register::C => "C",
            Register::D => "D",
            Register::E => "E",
            Register::H => "H",
            Register::L => "L",
        }
    }

    fn reg_pair_to_repr(reg: RegisterPair) -> &'static str {
        match reg {
            RegisterPair::BC => "BC",
            RegisterPair::DE => "DE",
            RegisterPair::HL => "HL",
            RegisterPair::SP => "SP",
            RegisterPair::HLINC => "HL+",
            RegisterPair::HLDEC => "HL-",
            RegisterPair::AF => "AF",
        }
    }

    fn cond_to_repr(cond: Condition) -> &'static str {
        match cond {
            Condition::Z => "Z",
            Condition::NZ => "NZ",
            Condition::C => "C",
            Condition::NC => "NC",
        }
    }

    fn reset_target_to_repr(target: ResetTarget) -> &'static str {
        match target {
            ResetTarget::Addr0x00 => "0x00",
            ResetTarget::Addr0x08 => "0x08",
            ResetTarget::Addr0x10 => "0x10",
            ResetTarget::Addr0x18 => "0x18",
            ResetTarget::Addr0x20 => "0x20",
            ResetTarget::Addr0x28 => "0x28",
            ResetTarget::Addr0x30 => "0x30",
            ResetTarget::Addr0x38 => "0x38",
        }
    }

    fn bit_to_repr(bit: Bit) -> &'static str {
        match bit {
            Bit::Bit0 => "0",
            Bit::Bit1 => "1",
            Bit::Bit2 => "2",
            Bit::Bit3 => "3",
            Bit::Bit4 => "4",
            Bit::Bit5 => "5",
            Bit::Bit6 => "6",
            Bit::Bit7 => "7",
        }
    }

    fn format_addr_mode(
        f: &mut core::fmt::Formatter<'_>,
        mode: &AddressingMode,
    ) -> core::fmt::Result {
        match mode {
            AddressingMode::IndirectRegister(reg) => {
                write!(f, "[{}]", Self::reg_pair_to_repr(*reg))
            }
            AddressingMode::IndirectZeroPageRegister(reg) => {
                write!(f, "[0xFF00 + {}]", Self::reg_to_repr(*reg))
            }
            AddressingMode::IndirectImmediate(imm) => write!(f, "[{:#x}]", imm),
            AddressingMode::IndirectZeroPageImmediate(imm) => write!(f, "[0xFF00 + {:#x}]", imm),
            AddressingMode::Register(reg) => write!(f, "{}", Self::reg_to_repr(*reg)),
            AddressingMode::RegisterPair(reg) => write!(f, "{}", Self::reg_pair_to_repr(*reg)),
            AddressingMode::Immediate(imm) => write!(f, "{:#x}", imm),
            AddressingMode::Immediate16(imm) => write!(f, "{:#x}", imm),
        }
    }
}

impl core::fmt::Display for Instruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Instruction::Ld8(dest, src) | Instruction::Ld16(dest, src) => {
                write!(f, "ld ")?;
                Self::format_addr_mode(f, dest)?;
                write!(f, " ")?;
                Self::format_addr_mode(f, src)
            }
            Instruction::Add8(dest, src) | Instruction::Add16(dest, src) => {
                write!(f, "add ")?;
                Self::format_addr_mode(f, dest)?;
                write!(f, " ")?;
                Self::format_addr_mode(f, src)
            }
            Instruction::Sub8(dest, src) => {
                write!(f, "sub ")?;
                Self::format_addr_mode(f, dest)?;
                write!(f, " ")?;
                Self::format_addr_mode(f, src)
            }
            Instruction::And8(dest, src) => {
                write!(f, "and ")?;
                Self::format_addr_mode(f, dest)?;
                write!(f, " ")?;
                Self::format_addr_mode(f, src)
            }
            Instruction::Or8(dest, src) => {
                write!(f, "or ")?;
                Self::format_addr_mode(f, dest)?;
                write!(f, " ")?;
                Self::format_addr_mode(f, src)
            }
            Instruction::Adc8(dest, src) => {
                write!(f, "adc ")?;
                Self::format_addr_mode(f, dest)?;
                write!(f, " ")?;
                Self::format_addr_mode(f, src)
            }
            Instruction::Sbc8(dest, src) => {
                write!(f, "sbc ")?;
                Self::format_addr_mode(f, dest)?;
                write!(f, " ")?;
                Self::format_addr_mode(f, src)
            }
            Instruction::Xor8(dest, src) => {
                write!(f, "xor ")?;
                Self::format_addr_mode(f, dest)?;
                write!(f, " ")?;
                Self::format_addr_mode(f, src)
            }
            Instruction::Cp8(dest, src) => {
                write!(f, "cp ")?;
                Self::format_addr_mode(f, dest)?;
                write!(f, " ")?;
                Self::format_addr_mode(f, src)
            }
            Instruction::Inc8(mode) | Instruction::Inc16(mode) => {
                write!(f, "inc ")?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::Dec8(mode) | Instruction::Dec16(mode) => {
                write!(f, "dec ")?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::Ld16HlSpImm(imm) => {
                write!(f, "ld HL, SP + {}", *imm)
            }
            Instruction::Halt => {
                write!(f, "halt")
            }
            Instruction::Nop => {
                write!(f, "nop")
            }
            Instruction::Daa => {
                write!(f, "daa")
            }
            Instruction::Cpl => {
                write!(f, "cpl")
            }
            Instruction::Scf => {
                write!(f, "scf")
            }
            Instruction::Ccf => {
                write!(f, "ccf")
            }
            Instruction::JrImm(condition, imm) => {
                if let Some(cond) = condition {
                    write!(f, "jr {}, PC + {}", Self::cond_to_repr(*cond), *imm)
                } else {
                    write!(f, "jr PC + {}", *imm)
                }
            }
            Instruction::Stop => {
                write!(f, "stop")
            }
            Instruction::Ret(cond) => {
                if let Some(cond) = cond {
                    write!(f, "ret {}", Self::cond_to_repr(*cond))
                } else {
                    write!(f, "ret ")
                }
            }
            Instruction::Reti => {
                write!(f, "reti")
            }
            Instruction::JpImm(cond, imm) => {
                if let Some(cond) = cond {
                    write!(f, "jp {}, {:#x}", Self::cond_to_repr(*cond), imm)
                } else {
                    write!(f, "jp {:#x}", imm)
                }
            }
            Instruction::JpHl => {
                write!(f, "jp HL")
            }
            Instruction::CallImm(cond, imm) => {
                if let Some(cond) = cond {
                    write!(f, "call {}, {:#x}", Self::cond_to_repr(*cond), imm)
                } else {
                    write!(f, "call {:#x}", imm)
                }
            }
            Instruction::Reset(target) => {
                write!(f, "rst {}", Self::reset_target_to_repr(*target))
            }
            Instruction::Pop(reg) => {
                write!(f, "pop {}", Self::reg_pair_to_repr(*reg))
            }
            Instruction::Push(reg) => {
                write!(f, "push {}", Self::reg_pair_to_repr(*reg))
            }
            Instruction::Di => {
                write!(f, "di")
            }
            Instruction::Ei => {
                write!(f, "ei")
            }
            Instruction::Illegal => {
                write!(f, "unk")
            }
            Instruction::Rlca => {
                write!(f, "rlca")
            }
            Instruction::Rrca => {
                write!(f, "rrca")
            }
            Instruction::Rla => {
                write!(f, "rla")
            }
            Instruction::Rra => {
                write!(f, "rra")
            }
            Instruction::Bit(bit, mode) => {
                write!(f, "bit {}, ", Self::bit_to_repr(*bit),)?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::Res(bit, mode) => {
                write!(f, "res {}, ", Self::bit_to_repr(*bit),)?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::Set(bit, mode) => {
                write!(f, "set {}, ", Self::bit_to_repr(*bit),)?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::Rlc(mode) => {
                write!(f, "rlc ")?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::Rrc(mode) => {
                write!(f, "rrc ")?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::Rl(mode) => {
                write!(f, "rl ")?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::Rr(mode) => {
                write!(f, "rr ")?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::Sla(mode) => {
                write!(f, "sla ")?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::Sra(mode) => {
                write!(f, "sra ")?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::Swap(mode) => {
                write!(f, "swap ")?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::Srl(mode) => {
                write!(f, "srl ")?;
                Self::format_addr_mode(f, mode)
            }
            Instruction::AddSpImm(imm) => {
                write!(f, "add SP, {:#x}", imm)
            }
        }
    }
}

pub struct InstructionIter<T>
where
    T: Iterator<Item = u8>,
{
    address: usize,
    iter: T,
}

impl<'a> InstructionIter<Cloned<Iter<'a, u8>>> {
    pub fn new(data: &'a [u8], base: usize) -> Self {
        let iter = data.iter().cloned();
        Self {
            iter,
            address: base,
        }
    }
}

impl<T> InstructionIter<T>
where
    T: Iterator<Item = u8>,
{
    fn next_with_address(&mut self) -> Option<(usize, u8)> {
        let next = self.iter.next()?;
        let addr = self.address;
        self.address += 1;
        Some((addr, next))
    }

    fn read_8_bit_imm(&mut self) -> Option<u8> {
        let next = self.iter.next()?;
        self.address += 1;
        Some(next)
    }

    fn read_16_bit_imm(&mut self) -> Option<u16> {
        let lo = self.iter.next()?;
        let hi = self.iter.next()?;
        self.address += 2;
        Some(lo as u16 | ((hi as u16) << 8))
    }
}

impl<T> Iterator for InstructionIter<T>
where
    T: Iterator<Item = u8>,
{
    type Item = (usize, Instruction);

    fn next(&mut self) -> Option<Self::Item> {
        let (addr, next) = self.next_with_address()?;
        let decoded = match sm83::decoder::decode(next) {
            sm83::decoder::OpCode::Prefix => {
                sm83::decoder::decode_prefixed(self.next_with_address()?.1)
            }
            val => val,
        };

        let insn = match decoded {
            sm83::decoder::OpCode::Ld8(dest, src) => Instruction::Ld8(
                translate_addr_mode(&mut self.iter, dest),
                translate_addr_mode(&mut self.iter, src),
            ),
            sm83::decoder::OpCode::Ld16(dest, src) => Instruction::Ld16(
                translate_addr_mode(&mut self.iter, dest),
                translate_addr_mode(&mut self.iter, src),
            ),
            sm83::decoder::OpCode::Add8(dest, src) => Instruction::Add8(
                translate_addr_mode(&mut self.iter, dest),
                translate_addr_mode(&mut self.iter, src),
            ),
            sm83::decoder::OpCode::Sub8(dest, src) => Instruction::Sub8(
                translate_addr_mode(&mut self.iter, dest),
                translate_addr_mode(&mut self.iter, src),
            ),
            sm83::decoder::OpCode::And8(dest, src) => Instruction::And8(
                translate_addr_mode(&mut self.iter, dest),
                translate_addr_mode(&mut self.iter, src),
            ),
            sm83::decoder::OpCode::Or8(dest, src) => Instruction::Or8(
                translate_addr_mode(&mut self.iter, dest),
                translate_addr_mode(&mut self.iter, src),
            ),
            sm83::decoder::OpCode::Adc8(dest, src) => Instruction::Adc8(
                translate_addr_mode(&mut self.iter, dest),
                translate_addr_mode(&mut self.iter, src),
            ),
            sm83::decoder::OpCode::Sbc8(dest, src) => Instruction::Sbc8(
                translate_addr_mode(&mut self.iter, dest),
                translate_addr_mode(&mut self.iter, src),
            ),
            sm83::decoder::OpCode::Xor8(dest, src) => Instruction::Xor8(
                translate_addr_mode(&mut self.iter, dest),
                translate_addr_mode(&mut self.iter, src),
            ),
            sm83::decoder::OpCode::Cp8(dest, src) => Instruction::Cp8(
                translate_addr_mode(&mut self.iter, dest),
                translate_addr_mode(&mut self.iter, src),
            ),
            sm83::decoder::OpCode::Add16(dest, src) => Instruction::Add16(
                translate_addr_mode(&mut self.iter, dest),
                translate_addr_mode(&mut self.iter, src),
            ),
            sm83::decoder::OpCode::Inc8(mode) => {
                Instruction::Inc8(translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Dec8(mode) => {
                Instruction::Dec8(translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Inc16(mode) => {
                Instruction::Inc16(translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Dec16(mode) => {
                Instruction::Dec16(translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::JrImm(cond) => {
                Instruction::JrImm(cond, self.read_8_bit_imm()? as i8)
            }
            sm83::decoder::OpCode::Ret(cond) => Instruction::Ret(cond),
            sm83::decoder::OpCode::Reti => Instruction::Reti,
            sm83::decoder::OpCode::JpImm(cond) => Instruction::JpImm(cond, self.read_16_bit_imm()?),
            sm83::decoder::OpCode::JpHl => Instruction::JpHl,
            sm83::decoder::OpCode::CallImm(cond) => {
                Instruction::CallImm(cond, self.read_16_bit_imm()?)
            }
            sm83::decoder::OpCode::Reset(target) => Instruction::Reset(target),
            sm83::decoder::OpCode::Pop(reg) => Instruction::Pop(reg),
            sm83::decoder::OpCode::Push(reg) => Instruction::Push(reg),
            sm83::decoder::OpCode::AddSpImm => Instruction::AddSpImm(self.read_8_bit_imm()? as i8),
            sm83::decoder::OpCode::Ld16HlSpImm => {
                Instruction::Ld16HlSpImm(self.read_8_bit_imm()? as i8)
            }
            sm83::decoder::OpCode::Di => Instruction::Di,
            sm83::decoder::OpCode::Ei => Instruction::Ei,
            sm83::decoder::OpCode::Halt => Instruction::Halt,
            sm83::decoder::OpCode::Prefix => unreachable!(),
            sm83::decoder::OpCode::Nop => Instruction::Nop,
            sm83::decoder::OpCode::Rlca => Instruction::Rlca,
            sm83::decoder::OpCode::Rrca => Instruction::Rrca,
            sm83::decoder::OpCode::Rla => Instruction::Rla,
            sm83::decoder::OpCode::Rra => Instruction::Rra,
            sm83::decoder::OpCode::Daa => Instruction::Daa,
            sm83::decoder::OpCode::Cpl => Instruction::Cpl,
            sm83::decoder::OpCode::Scf => Instruction::Scf,
            sm83::decoder::OpCode::Ccf => Instruction::Ccf,
            sm83::decoder::OpCode::Stop => Instruction::Stop,
            sm83::decoder::OpCode::Illegal => Instruction::Illegal,
            sm83::decoder::OpCode::Rlc(mode) => {
                Instruction::Rlc(translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Rrc(mode) => {
                Instruction::Rrc(translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Rl(mode) => {
                Instruction::Rl(translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Rr(mode) => {
                Instruction::Rr(translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Sla(mode) => {
                Instruction::Sla(translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Sra(mode) => {
                Instruction::Sra(translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Swap(mode) => {
                Instruction::Swap(translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Srl(mode) => {
                Instruction::Srl(translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Bit(bit, mode) => {
                Instruction::Bit(bit, translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Res(bit, mode) => {
                Instruction::Res(bit, translate_addr_mode(&mut self.iter, mode))
            }
            sm83::decoder::OpCode::Set(bit, mode) => {
                Instruction::Set(bit, translate_addr_mode(&mut self.iter, mode))
            }
        };
        Some((addr, insn))
    }
}
