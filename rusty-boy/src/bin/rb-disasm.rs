use anyhow::bail;
use clap::Parser;
use std::iter::Cloned;
use std::path::PathBuf;
use std::slice::Iter;

use rusty_boy::rom::{Rom, RomHeader};

use sm83::decoder::{
    Bit, Condition, Register, RegisterPair, RegisterPairMem, RegisterPairStack, ResetTarget,
};

/// Disassembles the given ROM, producing a stream of sm83 instructions
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The ROM to disassemble
    rom_path: PathBuf,
}

struct Disassembler<'a> {
    data: &'a [u8],
    rom: Rom<'a>,
}

impl<'a> Disassembler<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            rom: Rom::new(data),
        }
    }

    fn header(&'a self) -> anyhow::Result<RomHeader<'a>> {
        Ok(self.rom.header()?)
    }

    fn entrypoint(&'a self) -> anyhow::Result<InstructionIter<Cloned<Iter<'a, u8>>>> {
        Ok(InstructionIter::new(self.header()?.entrypoint, 0x100))
    }

    fn disassemble(&self) -> anyhow::Result<InstructionIter<Cloned<Iter<'a, u8>>>> {
        let mut entrypoint = None;
        for (_addr, insn) in InstructionIter::new(self.header()?.entrypoint, 0x100) {
            match insn {
                Instruction::JpImm(imm) => {
                    entrypoint = Some(imm);
                }
                _ => {}
            }
        }

        let Some(entrypoint) = entrypoint else {
            bail!("Entrypoint not found");
        };

        Ok(InstructionIter::new(
            &self.data[entrypoint as usize..],
            entrypoint as usize,
        ))
    }
}

pub enum Instruction {
    Ld8RegReg(Register, Register),                 // ld Register, Register
    Ld8RegImm(Register, u8),                       // ld Register, n8
    Ld8RegInd(Register, RegisterPair),             // ld Register, [RegisterPair]
    Ld8IndReg(RegisterPair, Register),             // ld [RegisterPair], Register
    Ld8IndImm(RegisterPair, u8),                   // ld [RegisterPair], n8
    Ld8IndAcc(RegisterPairMem),                    // ld [RegisterPairMem], A
    Ld8AccInd(RegisterPairMem),                    // ld A, [RegisterPairMem]
    Ld8ZeroPageCAcc,                               // ld [C], A
    Ld8AccZeroPageC,                               // ld A, [C]
    Ld8ZeroPageImmAcc(u8),                         // ld [n8], A
    Ld8AccZeroPageImm(u8),                         // ld A, [n8]
    Ld8IndImmAcc(u16),                             // ld [n16], A
    Ld8AccIndImm(u16),                             // ld A, [n16]
    Ld16RegImm(RegisterPair, u16),                 // ld RegisterPair, n16
    Ld16IndImmSp(u16),                             // ld [a16], SP
    Ld16HlSpImm(i8),                               // ld HL, SP + n8
    Ld16SpHl,                                      // ld SP, HL
    Halt,                                          // halt
    AddRegReg(Register, Register),                 // add Register, Register
    SubRegReg(Register, Register),                 // sub Register, Register
    AndRegReg(Register, Register),                 // and Register, Register
    OrRegReg(Register, Register),                  // or Register, Register
    AdcRegReg(Register, Register),                 // adc Register, Register
    SbcRegReg(Register, Register),                 // sbc Register, Register
    XorRegReg(Register, Register),                 // xor Register, Register
    CpRegReg(Register, Register),                  // cp Register, Register
    AddRegPairRegPair(RegisterPair, RegisterPair), // add RegisterPair, RegisterPair
    AddAccImm(u8),                                 // add A, n8
    AdcAccImm(u8),                                 // adc A, n8
    SubAccImm(u8),                                 // sub A, n8
    SbcAccImm(u8),                                 // sbc A, n8
    AndAccImm(u8),                                 // and A, n8
    XorAccImm(u8),                                 // xor A, n8
    OrAccImm(u8),                                  // or A, n8
    CpAccImm(u8),                                  // cp A, n8
    AddSpImm(i8),                                  // add SP, n8
    AddAccHlInd,                                   // add A, [HL]
    AdcAccHlInd,                                   // adc A, [HL]
    SubAccHlInd,                                   // sub A, [HL]
    SbcAccHlInd,                                   // sbc A, [HL]
    AndAccHlInd,                                   // and A, [HL]
    XorAccHlInd,                                   // xor A, [HL]
    OrAccHlInd,                                    // or A, [HL]
    CpAccHlInd,                                    // cp A, [HL]
    IncReg(Register),                              // inc Register
    DecReg(Register),                              // dec Register
    IncRegPair(RegisterPair),                      // inc RegisterPair
    DecRegPair(RegisterPair),                      // inc RegisterPair
    IncIndHl,                                      // inc [HL]
    DecIndHl,                                      // dec [HL]
    Nop,                                           // nop
    Daa,                                           // daa
    Cpl,                                           // cpl
    Scf,                                           // scf
    Ccf,                                           // ccf
    JrImm(i8),                                     // jr imm8
    JrCondImm(Condition, i8),                      // jr cond, imm8
    Stop,                                          // stop
    RetCond(Condition),                            // ret cond
    Ret,                                           // ret
    Reti,                                          // reti
    JpCondImm(Condition, u16),                     // jp cond, imm16
    JpImm(u16),                                    // jp imm16
    JpHl,                                          // jp HL
    CallCondImm(Condition, u16),                   // call cond, imm16
    CallImm(u16),                                  // call imm16
    Reset(ResetTarget),                            // reset target
    Pop(RegisterPairStack),                        // pop RegisterPairStack
    Push(RegisterPairStack),                       // push RegisterPairStack
    Di,                                            // di
    Ei,                                            // ei
    Illegal,                                       // Illegal
    RlcReg(Register),                              // rlc Register
    RrcReg(Register),                              // rrc Register
    RlReg(Register),                               // rl Register
    RrReg(Register),                               // rr Register
    SlaReg(Register),                              // sla Register
    SraReg(Register),                              // sra Register
    SwapReg(Register),                             // swap Register
    SrlReg(Register),                              // srl Register
    Bit(Bit, Register),                            // bit #bit, Register
    Res(Bit, Register),                            // res #bit, Register
    Set(Bit, Register),                            // set #bit, Register
    RlcHlInd,                                      // rlc [HL]
    RrcHlInd,                                      // rrc [HL]
    RlHlInd,                                       // rl [HL]
    RrHlInd,                                       // rr [HL]
    SlaHlInd,                                      // sla [HL]
    SraHlInd,                                      // sra [HL]
    SwapHlInd,                                     // swap [HL]
    SrlHlInd,                                      // srl [HL]
    BitHlInd(Bit),                                 // bit #bit, [HL]
    ResHlInd(Bit),                                 // srl #bit, [HL]
    SetHlInd(Bit),                                 // srl #bit, [HL]
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
        }
    }

    fn reg_pair_stack_to_repr(reg: RegisterPairStack) -> &'static str {
        match reg {
            RegisterPairStack::BC => "BC",
            RegisterPairStack::DE => "DE",
            RegisterPairStack::HL => "HL",
            RegisterPairStack::AF => "AF",
        }
    }

    fn reg_pair_mem_to_repr(reg: RegisterPairMem) -> &'static str {
        match reg {
            RegisterPairMem::BC => "BC",
            RegisterPairMem::DE => "DE",
            RegisterPairMem::HLINC => "HL+",
            RegisterPairMem::HLDEC => "HL-",
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
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Ld8RegReg(dest, src) => {
                write!(
                    f,
                    "ld {}, {}",
                    Self::reg_to_repr(*dest),
                    Self::reg_to_repr(*src)
                )
            }
            Instruction::Ld8RegImm(dest, imm) => {
                write!(f, "ld {}, {:#x}", Self::reg_to_repr(*dest), imm)
            }
            Instruction::Ld8RegInd(dest, src) => {
                write!(
                    f,
                    "ld {}, [{}]",
                    Self::reg_to_repr(*dest),
                    Self::reg_pair_to_repr(*src)
                )
            }
            Instruction::Ld8IndReg(dest, src) => {
                write!(
                    f,
                    "ld [{}], {}",
                    Self::reg_pair_to_repr(*dest),
                    Self::reg_to_repr(*src)
                )
            }
            Instruction::Ld8IndImm(dest, imm) => {
                write!(f, "ld [{}], {}", Self::reg_pair_to_repr(*dest), imm)
            }
            Instruction::Ld8IndAcc(dest) => {
                write!(f, "ld [{}], A", Self::reg_pair_mem_to_repr(*dest))
            }
            Instruction::Ld8AccInd(src) => {
                write!(f, "ld A, [{}]", Self::reg_pair_mem_to_repr(*src))
            }
            Instruction::Ld8ZeroPageCAcc => {
                write!(f, "ld [C], A")
            }
            Instruction::Ld8AccZeroPageC => {
                write!(f, "ld A, [C]")
            }
            Instruction::Ld8ZeroPageImmAcc(imm) => {
                write!(f, "ld [{:#x}], A", (*imm as u16) | 0xFF00)
            }
            Instruction::Ld8AccZeroPageImm(imm) => {
                write!(f, "ld A, [{:#x}]", (*imm as u16) | 0xFF00)
            }
            Instruction::Ld8IndImmAcc(imm) => {
                write!(f, "ld [{:#x}], A", *imm)
            }
            Instruction::Ld8AccIndImm(imm) => {
                write!(f, "ld A, [{:#x}]", *imm)
            }
            Instruction::Ld16RegImm(dest, imm) => {
                write!(f, "ld {}, {:#x}", Self::reg_pair_to_repr(*dest), *imm)
            }
            Instruction::Ld16IndImmSp(imm) => {
                write!(f, "ld [{:#x}], SP", *imm)
            }
            Instruction::Ld16HlSpImm(imm) => {
                write!(f, "ld HL, SP + {}", *imm)
            }
            Instruction::Ld16SpHl => {
                write!(f, "ld SP, HL")
            }
            Instruction::Halt => {
                write!(f, "halt")
            }
            Instruction::AddRegReg(dest, src) => {
                write!(
                    f,
                    "add {}, {}",
                    Self::reg_to_repr(*dest),
                    Self::reg_to_repr(*src)
                )
            }
            Instruction::SubRegReg(dest, src) => {
                write!(
                    f,
                    "sub {}, {}",
                    Self::reg_to_repr(*dest),
                    Self::reg_to_repr(*src)
                )
            }
            Instruction::AndRegReg(dest, src) => {
                write!(
                    f,
                    "and {}, {}",
                    Self::reg_to_repr(*dest),
                    Self::reg_to_repr(*src)
                )
            }
            Instruction::OrRegReg(dest, src) => {
                write!(
                    f,
                    "or {}, {}",
                    Self::reg_to_repr(*dest),
                    Self::reg_to_repr(*src)
                )
            }
            Instruction::AdcRegReg(dest, src) => {
                write!(
                    f,
                    "adc {}, {}",
                    Self::reg_to_repr(*dest),
                    Self::reg_to_repr(*src)
                )
            }
            Instruction::SbcRegReg(dest, src) => {
                write!(
                    f,
                    "sbc {}, {}",
                    Self::reg_to_repr(*dest),
                    Self::reg_to_repr(*src)
                )
            }
            Instruction::XorRegReg(dest, src) => {
                write!(
                    f,
                    "xor {}, {}",
                    Self::reg_to_repr(*dest),
                    Self::reg_to_repr(*src)
                )
            }
            Instruction::CpRegReg(dest, src) => {
                write!(
                    f,
                    "cp {}, {}",
                    Self::reg_to_repr(*dest),
                    Self::reg_to_repr(*src)
                )
            }
            Instruction::AddRegPairRegPair(dest, src) => {
                write!(
                    f,
                    "add {}, {}",
                    Self::reg_pair_to_repr(*dest),
                    Self::reg_pair_to_repr(*src)
                )
            }
            Instruction::AddAccImm(imm) => {
                write!(f, "add A, {:#x}", *imm)
            }
            Instruction::AdcAccImm(imm) => {
                write!(f, "adc A, {:#x}", *imm)
            }
            Instruction::SubAccImm(imm) => {
                write!(f, "sub A, {:#x}", *imm)
            }
            Instruction::SbcAccImm(imm) => {
                write!(f, "sbc A, {:#x}", *imm)
            }
            Instruction::AndAccImm(imm) => {
                write!(f, "and A, {:#x}", *imm)
            }
            Instruction::XorAccImm(imm) => {
                write!(f, "xor A, {:#x}", *imm)
            }
            Instruction::OrAccImm(imm) => {
                write!(f, "or A, {:#x}", *imm)
            }
            Instruction::CpAccImm(imm) => {
                write!(f, "cp A, {:#x}", *imm)
            }
            Instruction::AddSpImm(imm) => {
                write!(f, "add SP, {}", *imm)
            }
            Instruction::AddAccHlInd => {
                write!(f, "add A, [HL]")
            }
            Instruction::AdcAccHlInd => {
                write!(f, "adc A, [HL]")
            }
            Instruction::SubAccHlInd => {
                write!(f, "sub A, [HL]")
            }
            Instruction::SbcAccHlInd => {
                write!(f, "sbc A, [HL]")
            }
            Instruction::AndAccHlInd => {
                write!(f, "and A, [HL]")
            }
            Instruction::XorAccHlInd => {
                write!(f, "xor A, [HL]")
            }
            Instruction::OrAccHlInd => {
                write!(f, "or A, [HL]")
            }
            Instruction::CpAccHlInd => {
                write!(f, "cp A, [HL]")
            }
            Instruction::IncReg(reg) => {
                write!(f, "inc {}", Self::reg_to_repr(*reg))
            }
            Instruction::DecReg(reg) => {
                write!(f, "dec {}", Self::reg_to_repr(*reg))
            }
            Instruction::IncRegPair(reg) => {
                write!(f, "inc {}", Self::reg_pair_to_repr(*reg))
            }
            Instruction::DecRegPair(reg) => {
                write!(f, "dec {}", Self::reg_pair_to_repr(*reg))
            }
            Instruction::IncIndHl => {
                write!(f, "inc [HL]")
            }
            Instruction::DecIndHl => {
                write!(f, "dec [HL]")
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
            Instruction::JrImm(imm) => {
                write!(f, "jr PC + {}", *imm)
            }
            Instruction::JrCondImm(condition, imm) => {
                write!(f, "jr {}, PC + {}", Self::cond_to_repr(*condition), *imm)
            }
            Instruction::Stop => {
                write!(f, "stop")
            }
            Instruction::RetCond(condition) => {
                write!(f, "ret {}", Self::cond_to_repr(*condition))
            }
            Instruction::Ret => {
                write!(f, "ret")
            }
            Instruction::Reti => {
                write!(f, "reti")
            }
            Instruction::JpCondImm(condition, imm) => {
                write!(f, "jp {}, {:#x}", Self::cond_to_repr(*condition), imm)
            }
            Instruction::JpImm(imm) => {
                write!(f, "jp {:#x}", imm)
            }
            Instruction::JpHl => {
                write!(f, "jp HL")
            }
            Instruction::CallCondImm(condition, imm) => {
                write!(f, "call {}, {:#x}", Self::cond_to_repr(*condition), imm)
            }
            Instruction::CallImm(imm) => {
                write!(f, "call {:#x}", imm)
            }
            Instruction::Reset(target) => {
                write!(f, "rst {}", Self::reset_target_to_repr(*target))
            }
            Instruction::Pop(reg) => {
                write!(f, "pop {}", Self::reg_pair_stack_to_repr(*reg))
            }
            Instruction::Push(reg) => {
                write!(f, "push {}", Self::reg_pair_stack_to_repr(*reg))
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
            Instruction::RlcReg(reg) => {
                write!(f, "rlc {}", Self::reg_to_repr(*reg))
            }
            Instruction::RrcReg(reg) => {
                write!(f, "rrc {}", Self::reg_to_repr(*reg))
            }
            Instruction::RlReg(reg) => {
                write!(f, "rl {}", Self::reg_to_repr(*reg))
            }
            Instruction::RrReg(reg) => {
                write!(f, "rr {}", Self::reg_to_repr(*reg))
            }
            Instruction::SlaReg(reg) => {
                write!(f, "sla {}", Self::reg_to_repr(*reg))
            }
            Instruction::SraReg(reg) => {
                write!(f, "sra {}", Self::reg_to_repr(*reg))
            }
            Instruction::SwapReg(reg) => {
                write!(f, "swap {}", Self::reg_to_repr(*reg))
            }
            Instruction::SrlReg(reg) => {
                write!(f, "srl {}", Self::reg_to_repr(*reg))
            }
            Instruction::Bit(bit, reg) => {
                write!(
                    f,
                    "bit {}, {}",
                    Self::bit_to_repr(*bit),
                    Self::reg_to_repr(*reg)
                )
            }
            Instruction::Res(bit, reg) => {
                write!(
                    f,
                    "res {}, {}",
                    Self::bit_to_repr(*bit),
                    Self::reg_to_repr(*reg)
                )
            }
            Instruction::Set(bit, reg) => {
                write!(
                    f,
                    "set {}, {}",
                    Self::bit_to_repr(*bit),
                    Self::reg_to_repr(*reg)
                )
            }
            Instruction::RlcHlInd => {
                write!(f, "rlc [HL]",)
            }
            Instruction::RrcHlInd => {
                write!(f, "rrc [HL]",)
            }
            Instruction::RlHlInd => {
                write!(f, "rl [HL]",)
            }
            Instruction::RrHlInd => {
                write!(f, "rr [HL]",)
            }
            Instruction::SlaHlInd => {
                write!(f, "sla [HL]",)
            }
            Instruction::SraHlInd => {
                write!(f, "sra [HL]",)
            }
            Instruction::SwapHlInd => {
                write!(f, "swap [HL]",)
            }
            Instruction::SrlHlInd => {
                write!(f, "srl [HL]",)
            }
            Instruction::BitHlInd(bit) => {
                write!(f, "bit {}, [HL]", Self::bit_to_repr(*bit))
            }
            Instruction::ResHlInd(bit) => {
                write!(f, "res {}, [HL]", Self::bit_to_repr(*bit))
            }
            Instruction::SetHlInd(bit) => {
                write!(f, "set {}, [HL]", Self::bit_to_repr(*bit))
            }
        }
    }
}

struct InstructionIter<T>
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
    pub fn next_with_address(&mut self) -> Option<(usize, u8)> {
        let next = self.iter.next()?;
        let addr = self.address;
        self.address += 1;
        Some((addr, next))
    }

    pub fn read_8_bit_imm(&mut self) -> Option<u8> {
        let next = self.iter.next()?;
        self.address += 1;
        Some(next)
    }

    pub fn read_16_bit_imm(&mut self) -> Option<u16> {
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
            sm83::decoder::OpCode::Ld8RegReg(dest, src) => Instruction::Ld8RegReg(dest, src),
            sm83::decoder::OpCode::Ld8RegImm(dest) => {
                Instruction::Ld8RegImm(dest, self.read_8_bit_imm()?)
            }
            sm83::decoder::OpCode::Ld8RegInd(dest, src) => Instruction::Ld8RegInd(dest, src),
            sm83::decoder::OpCode::Ld8IndReg(dest, src) => Instruction::Ld8IndReg(dest, src),
            sm83::decoder::OpCode::Ld8IndImm(dest) => {
                Instruction::Ld8IndImm(dest, self.read_8_bit_imm()?)
            }
            sm83::decoder::OpCode::Ld8IndAcc(dest) => Instruction::Ld8IndAcc(dest),
            sm83::decoder::OpCode::Ld8AccInd(src) => Instruction::Ld8AccInd(src),
            sm83::decoder::OpCode::Ld8ZeroPageCAcc => Instruction::Ld8ZeroPageCAcc,
            sm83::decoder::OpCode::Ld8AccZeroPageC => Instruction::Ld8AccZeroPageC,
            sm83::decoder::OpCode::Ld8ZeroPageImmAcc => {
                Instruction::Ld8ZeroPageImmAcc(self.read_8_bit_imm()?)
            }
            sm83::decoder::OpCode::Ld8AccZeroPageImm => {
                Instruction::Ld8AccZeroPageImm(self.read_8_bit_imm()?)
            }
            sm83::decoder::OpCode::Ld8IndImmAcc => {
                Instruction::Ld8IndImmAcc(self.read_16_bit_imm()?)
            }
            sm83::decoder::OpCode::Ld8AccIndImm => {
                Instruction::Ld8AccIndImm(self.read_16_bit_imm()?)
            }
            sm83::decoder::OpCode::Ld16RegImm(dest) => {
                Instruction::Ld16RegImm(dest, self.read_16_bit_imm()?)
            }
            sm83::decoder::OpCode::Ld16IndImmSp => {
                Instruction::Ld16IndImmSp(self.read_16_bit_imm()?)
            }
            sm83::decoder::OpCode::Ld16HlSpImm => {
                Instruction::Ld16HlSpImm(self.read_8_bit_imm()? as i8)
            }
            sm83::decoder::OpCode::Ld16SpHl => Instruction::Ld16SpHl,
            sm83::decoder::OpCode::Halt => Instruction::Halt,
            sm83::decoder::OpCode::AddRegReg(dest, src) => Instruction::AddRegReg(dest, src),
            sm83::decoder::OpCode::SubRegReg(dest, src) => Instruction::SubRegReg(dest, src),
            sm83::decoder::OpCode::AndRegReg(dest, src) => Instruction::AndRegReg(dest, src),
            sm83::decoder::OpCode::OrRegReg(dest, src) => Instruction::OrRegReg(dest, src),
            sm83::decoder::OpCode::AdcRegReg(dest, src) => Instruction::AdcRegReg(dest, src),
            sm83::decoder::OpCode::SbcRegReg(dest, src) => Instruction::SbcRegReg(dest, src),
            sm83::decoder::OpCode::XorRegReg(dest, src) => Instruction::XorRegReg(dest, src),
            sm83::decoder::OpCode::CpRegReg(dest, src) => Instruction::CpRegReg(dest, src),
            sm83::decoder::OpCode::AddRegPairRegPair(dest, src) => {
                Instruction::AddRegPairRegPair(dest, src)
            }
            sm83::decoder::OpCode::AddAccImm => Instruction::AddAccImm(self.read_8_bit_imm()?),
            sm83::decoder::OpCode::AdcAccImm => Instruction::AdcAccImm(self.read_8_bit_imm()?),
            sm83::decoder::OpCode::SubAccImm => Instruction::SubAccImm(self.read_8_bit_imm()?),
            sm83::decoder::OpCode::SbcAccImm => Instruction::SbcAccImm(self.read_8_bit_imm()?),
            sm83::decoder::OpCode::AndAccImm => Instruction::AndAccImm(self.read_8_bit_imm()?),
            sm83::decoder::OpCode::XorAccImm => Instruction::XorAccImm(self.read_8_bit_imm()?),
            sm83::decoder::OpCode::OrAccImm => Instruction::OrAccImm(self.read_8_bit_imm()?),
            sm83::decoder::OpCode::CpAccImm => Instruction::CpAccImm(self.read_8_bit_imm()?),
            sm83::decoder::OpCode::AddSpImm => Instruction::AddSpImm(self.read_8_bit_imm()? as i8),
            sm83::decoder::OpCode::AddAccHlInd => Instruction::AddAccHlInd,
            sm83::decoder::OpCode::AdcAccHlInd => Instruction::AdcAccHlInd,
            sm83::decoder::OpCode::SubAccHlInd => Instruction::SubAccHlInd,
            sm83::decoder::OpCode::SbcAccHlInd => Instruction::SbcAccHlInd,
            sm83::decoder::OpCode::AndAccHlInd => Instruction::AndAccHlInd,
            sm83::decoder::OpCode::XorAccHlInd => Instruction::XorAccHlInd,
            sm83::decoder::OpCode::OrAccHlInd => Instruction::OrAccHlInd,
            sm83::decoder::OpCode::CpAccHlInd => Instruction::CpAccHlInd,
            sm83::decoder::OpCode::IncReg(dest) => Instruction::IncReg(dest),
            sm83::decoder::OpCode::DecReg(dest) => Instruction::DecReg(dest),
            sm83::decoder::OpCode::IncRegPair(dest) => Instruction::IncRegPair(dest),
            sm83::decoder::OpCode::DecRegPair(dest) => Instruction::DecRegPair(dest),
            sm83::decoder::OpCode::IncIndHl => Instruction::IncIndHl,
            sm83::decoder::OpCode::DecIndHl => Instruction::DecIndHl,
            sm83::decoder::OpCode::Prefix => unreachable!(),
            sm83::decoder::OpCode::Nop => Instruction::Nop,
            sm83::decoder::OpCode::Daa => Instruction::Daa,
            sm83::decoder::OpCode::Cpl => Instruction::Cpl,
            sm83::decoder::OpCode::Scf => Instruction::Scf,
            sm83::decoder::OpCode::Ccf => Instruction::Ccf,
            sm83::decoder::OpCode::JrImm => Instruction::JrImm(self.read_8_bit_imm()? as i8),
            sm83::decoder::OpCode::JrCondImm(cond) => {
                Instruction::JrCondImm(cond, self.read_8_bit_imm()? as i8)
            }
            sm83::decoder::OpCode::Stop => Instruction::Stop,
            sm83::decoder::OpCode::RetCond(cond) => Instruction::RetCond(cond),
            sm83::decoder::OpCode::Ret => Instruction::Ret,
            sm83::decoder::OpCode::Reti => Instruction::Reti,
            sm83::decoder::OpCode::JpCondImm(cond) => {
                Instruction::JpCondImm(cond, self.read_16_bit_imm()?)
            }
            sm83::decoder::OpCode::JpImm => Instruction::JpImm(self.read_16_bit_imm()?),
            sm83::decoder::OpCode::JpHl => Instruction::JpHl,
            sm83::decoder::OpCode::CallCondImm(cond) => {
                Instruction::CallCondImm(cond, self.read_16_bit_imm()?)
            }
            sm83::decoder::OpCode::CallImm => Instruction::CallImm(self.read_16_bit_imm()?),
            sm83::decoder::OpCode::Reset(target) => Instruction::Reset(target),
            sm83::decoder::OpCode::Pop(reg) => Instruction::Pop(reg),
            sm83::decoder::OpCode::Push(reg) => Instruction::Push(reg),
            sm83::decoder::OpCode::Di => Instruction::Di,
            sm83::decoder::OpCode::Ei => Instruction::Ei,
            sm83::decoder::OpCode::Illegal => Instruction::Illegal,
            sm83::decoder::OpCode::RlcReg(reg) => Instruction::RlcReg(reg),
            sm83::decoder::OpCode::RrcReg(reg) => Instruction::RrcReg(reg),
            sm83::decoder::OpCode::RlReg(reg) => Instruction::RlReg(reg),
            sm83::decoder::OpCode::RrReg(reg) => Instruction::RrReg(reg),
            sm83::decoder::OpCode::SlaReg(reg) => Instruction::SlaReg(reg),
            sm83::decoder::OpCode::SraReg(reg) => Instruction::SraReg(reg),
            sm83::decoder::OpCode::SwapReg(reg) => Instruction::SwapReg(reg),
            sm83::decoder::OpCode::SrlReg(reg) => Instruction::SrlReg(reg),
            sm83::decoder::OpCode::Bit(bit, reg) => Instruction::Bit(bit, reg),
            sm83::decoder::OpCode::Res(bit, reg) => Instruction::Res(bit, reg),
            sm83::decoder::OpCode::Set(bit, reg) => Instruction::Set(bit, reg),
            sm83::decoder::OpCode::RlcHlInd => Instruction::RlcHlInd,
            sm83::decoder::OpCode::RrcHlInd => Instruction::RrcHlInd,
            sm83::decoder::OpCode::RlHlInd => Instruction::RlHlInd,
            sm83::decoder::OpCode::RrHlInd => Instruction::RrHlInd,
            sm83::decoder::OpCode::SlaHlInd => Instruction::SlaHlInd,
            sm83::decoder::OpCode::SraHlInd => Instruction::SraHlInd,
            sm83::decoder::OpCode::SwapHlInd => Instruction::SwapHlInd,
            sm83::decoder::OpCode::SrlHlInd => Instruction::SrlHlInd,
            sm83::decoder::OpCode::BitHlInd(bit) => Instruction::BitHlInd(bit),
            sm83::decoder::OpCode::ResHlInd(bit) => Instruction::ResHlInd(bit),
            sm83::decoder::OpCode::SetHlInd(bit) => Instruction::SetHlInd(bit),
        };
        Some((addr, insn))
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let data = std::fs::read(args.rom_path)?;
    let disassembler = Disassembler::new(&data);

    let header = disassembler.header()?;
    println!("Rom Header:");
    println!("\tTitle: \"{}\"", header.title);
    println!("\tManufacturer code: {:?}", header.manufacturer_code);
    println!("\tCGB: {:?}", header.cgb_flag);
    assert_eq!(header.rom_size % (32 * 1024), 0);
    println!("\tROM size: {} KiB", header.rom_size / 1024);
    println!("\tRAM size: {}", header.ram_size);
    println!("\tEntrypoint:");
    for (addr, insn) in disassembler.entrypoint()? {
        println!("\t\t{:#x}\t{}", addr, insn);
    }

    for (addr, insn) in disassembler.disassemble()? {
        println!("{:#x}\t{}", addr, insn);
    }

    Ok(())
}
