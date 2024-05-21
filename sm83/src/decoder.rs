//! Functions for decoding CPU instructions.
//!
//! +----+------------+-----------+------------+------------+-------------+-----------+------------+-----------+-------------+-----------+------------+------------+------------+------------+------------+---------+
//! |    |     x0     |     x1    |     x2     |     x3     |      x4     |     x5    |     x6     |     x7    |      x8     |     x9    |     xa     |     xb     |     xc     |     xd     |     xe     |    xf   |
//! | 0x |    NOP     | LD BC n16 | LD [BC] A  |   INC BC   |    INC B    |   DEC B   |  LD B n8   |    RLCA   | LD [a16] SP | ADD HL BC | LD A [BC]  |   DEC BC   |   INC C    |   DEC C    |  LD C n8   |   RRCA  |
//! | 1x |  STOP n8   | LD DE n16 | LD [DE] A  |   INC DE   |    INC D    |   DEC D   |  LD D n8   |    RLA    |    JR e8    | ADD HL DE | LD A [DE]  |   DEC DE   |   INC E    |   DEC E    |  LD E n8   |   RRA   |
//! | 2x |  JR NZ e8  | LD HL n16 | LD [HL+] A |   INC HL   |    INC H    |   DEC H   |  LD H n8   |    DAA    |   JR Z e8   | ADD HL HL | LD A [HL+] |   DEC HL   |   INC L    |   DEC L    |  LD L n8   |   CPL   |
//! | 3x |  JR NC e8  | LD SP n16 | LD [HL-] A |   INC SP   |   INC [HL]  |  DEC [HL] | LD [HL] n8 |    SCF    |   JR C e8   | ADD HL SP | LD A [HL-] |   DEC SP   |   INC A    |   DEC A    |  LD A n8   |   CCF   |
//! | 4x |   LD B B   |   LD B C  |   LD B D   |   LD B E   |    LD B H   |   LD B L  | LD B [HL]  |   LD B A  |    LD C B   |   LD C C  |   LD C D   |   LD C E   |   LD C H   |   LD C L   | LD C [HL]  |  LD C A |
//! | 5x |   LD D B   |   LD D C  |   LD D D   |   LD D E   |    LD D H   |   LD D L  | LD D [HL]  |   LD D A  |    LD E B   |   LD E C  |   LD E D   |   LD E E   |   LD E H   |   LD E L   | LD E [HL]  |  LD E A |
//! | 6x |   LD H B   |   LD H C  |   LD H D   |   LD H E   |    LD H H   |   LD H L  | LD H [HL]  |   LD H A  |    LD L B   |   LD L C  |   LD L D   |   LD L E   |   LD L H   |   LD L L   | LD L [HL]  |  LD L A |
//! | 7x | LD [HL] B  | LD [HL] C | LD [HL] D  | LD [HL] E  |  LD [HL] H  | LD [HL] L |    HALT    | LD [HL] A |    LD A B   |   LD A C  |   LD A D   |   LD A E   |   LD A H   |   LD A L   | LD A [HL]  |  LD A A |
//! | 8x |  ADD A B   |  ADD A C  |  ADD A D   |  ADD A E   |   ADD A H   |  ADD A L  | ADD A [HL] |  ADD A A  |   ADC A B   |  ADC A C  |  ADC A D   |  ADC A E   |  ADC A H   |  ADC A L   | ADC A [HL] | ADC A A |
//! | 9x |  SUB A B   |  SUB A C  |  SUB A D   |  SUB A E   |   SUB A H   |  SUB A L  | SUB A [HL] |  SUB A A  |   SBC A B   |  SBC A C  |  SBC A D   |  SBC A E   |  SBC A H   |  SBC A L   | SBC A [HL] | SBC A A |
//! | ax |  AND A B   |  AND A C  |  AND A D   |  AND A E   |   AND A H   |  AND A L  | AND A [HL] |  AND A A  |   XOR A B   |  XOR A C  |  XOR A D   |  XOR A E   |  XOR A H   |  XOR A L   | XOR A [HL] | XOR A A |
//! | bx |   OR A B   |   OR A C  |   OR A D   |   OR A E   |    OR A H   |   OR A L  | OR A [HL]  |   OR A A  |    CP A B   |   CP A C  |   CP A D   |   CP A E   |   CP A H   |   CP A L   | CP A [HL]  |  CP A A |
//! | cx |   RET NZ   |   POP BC  | JP NZ a16  |   JP a16   | CALL NZ a16 |  PUSH BC  |  ADD A n8  |  RST $00  |    RET Z    |    RET    |  JP Z a16  |   PREFIX   | CALL Z a16 |  CALL a16  |  ADC A n8  | RST $08 |
//! | dx |   RET NC   |   POP DE  | JP NC a16  | ILLEGAL_D3 | CALL NC a16 |  PUSH DE  |  SUB A n8  |  RST $10  |    RET C    |    RETI   |  JP C a16  | ILLEGAL_DB | CALL C a16 | ILLEGAL_DD |  SBC A n8  | RST $18 |
//! | ex | LDH [a8] A |   POP HL  |  LD [C] A  | ILLEGAL_E3 |  ILLEGAL_E4 |  PUSH HL  |  AND A n8  |  RST $20  |  ADD SP e8  |   JP HL   | LD [a16] A | ILLEGAL_EB | ILLEGAL_EC | ILLEGAL_ED |  XOR A n8  | RST $28 |
//! | fx | LDH A [a8] |   POP AF  |  LD A [C]  |     DI     |  ILLEGAL_F4 |  PUSH AF  |  OR A n8   |  RST $30  | LD HL SP e8 |  LD SP HL | LD A [a16] |     EI     | ILLEGAL_FC | ILLEGAL_FD |  CP A n8   | RST $38 |
//! +----+------------+-----------+------------+------------+-------------+-----------+------------+-----------+-------------+-----------+------------+------------+------------+------------+------------+---------+

use sm83_decoder_macros::generate_decoder_tables;

#[derive(Debug, Clone, Copy)]
pub enum AddressingMode {
    IndirectRegister(RegisterPairs),
    IndirectZeroPageRegister(Register),
    IndirectImmediate,
    IndirectZeroPageImmediate,
    Register(Register),
    RegisterPair(RegisterPairs),
    Immediate,
    Immediate16,
}

/// Opcodes of the CPU.
#[derive(Debug, Clone, Copy)]
pub enum OpCode {
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
    JrImm(Option<Condition>),              // jr 8-bit immediate, with or without condition
    Ret(Option<Condition>),                // ret with optional condition
    Reti,                                  // reti
    JpImm(Option<Condition>),              // jp imm16, optional condition
    JpHl,                                  // jp HL
    CallImm(Option<Condition>),            // call imm16, optional condition
    Reset(ResetTarget),                    // reset target
    Pop(RegisterPairStack),                // pop RegisterPairStack
    Push(RegisterPairStack),               // push RegisterPairStack
    AddSpImm,                              // add SP, n8
    Ld16HlSpImm,                           // ld HL, SP + n8
    Di,                                    // di
    Ei,                                    // ei
    Halt,                                  // halt
    Prefix,                                // prefix
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

#[derive(Debug, Clone, Copy)]
pub enum RegisterPairs {
    BC,
    DE,
    HL,
    SP,
    HLINC,
    HLDEC,
}

impl RegisterPair {
    pub const fn into_generalized(self) -> RegisterPairs {
        match self {
            RegisterPair::BC => RegisterPairs::BC,
            RegisterPair::DE => RegisterPairs::DE,
            RegisterPair::HL => RegisterPairs::HL,
            RegisterPair::SP => RegisterPairs::SP,
        }
    }
}

impl RegisterPairMem {
    pub const fn into_generalized(self) -> RegisterPairs {
        match self {
            RegisterPairMem::BC => RegisterPairs::BC,
            RegisterPairMem::DE => RegisterPairs::DE,
            RegisterPairMem::HLINC => RegisterPairs::HLINC,
            RegisterPairMem::HLDEC => RegisterPairs::HLDEC,
        }
    }
}

// r => destination reg
// R => source reg

generate_decoder_tables! {
    Declarations {
        Register {
            A = 7,
            B = 0,
            C = 1,
            D = 2,
            E = 3,
            H = 4,
            L = 5,
        },
        RegisterPair {
            BC = 0,
            DE = 1,
            HL = 2,
            SP = 3,
        },
        RegisterPairStack {
            BC = 0,
            DE = 1,
            HL = 2,
            AF = 3,
        },
        RegisterPairMem {
            BC = 0,
            DE = 1,
            HLINC = 2,
            HLDEC = 3,
        },
        Condition {
            NZ = 0,
            Z = 1,
            NC = 2,
            C = 3,
        },
        ResetTarget {
            Addr0x00 = 0,
            Addr0x08 = 1,
            Addr0x10 = 2,
            Addr0x18 = 3,
            Addr0x20 = 4,
            Addr0x28 = 5,
            Addr0x30 = 6,
            Addr0x38 = 7,
        },
        Bit {
            Bit0 = 0,
            Bit1 = 1,
            Bit2 = 2,
            Bit3 = 3,
            Bit4 = 4,
            Bit5 = 5,
            Bit6 = 6,
            Bit7 = 7,
        }
    }
    DECODER_TABLE: [OpCode; 256] {
        [r: Register, R: Register] "01rrrRRR" => { OpCode::Ld8(AddressingMode::Register(#r), AddressingMode::Register(#R)) },
        [r: Register] "01rrr110" => { OpCode::Ld8(AddressingMode::Register(#r), AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [R: Register] "01110RRR" => { OpCode::Ld8(AddressingMode::IndirectRegister(RegisterPair::HL.into_generalized()), AddressingMode::Register(#R)) },
        [] "01110110" => { OpCode::Halt },
        [r: Register] "00rrr110" => { OpCode::Ld8(AddressingMode::Register(#r), AddressingMode::Immediate) },
        [] "00110110" => { OpCode::Ld8(AddressingMode::IndirectRegister(RegisterPair::HL.into_generalized()), AddressingMode::Immediate) },
        [r: Register] "10000rrr" => { OpCode::Add8(AddressingMode::Register(Register::A), AddressingMode::Register(#r)) },
        [r: Register] "10010rrr" => { OpCode::Sub8(AddressingMode::Register(Register::A), AddressingMode::Register(#r)) },
        [r: Register] "10100rrr" => { OpCode::And8(AddressingMode::Register(Register::A), AddressingMode::Register(#r)) },
        [r: Register] "10110rrr" => { OpCode::Or8(AddressingMode::Register(Register::A), AddressingMode::Register(#r)) },
        [r: Register] "10001rrr" => { OpCode::Adc8(AddressingMode::Register(Register::A), AddressingMode::Register(#r)) },
        [r: Register] "10011rrr" => { OpCode::Sbc8(AddressingMode::Register(Register::A), AddressingMode::Register(#r)) },
        [r: Register] "10101rrr" => { OpCode::Xor8(AddressingMode::Register(Register::A), AddressingMode::Register(#r)) },
        [r: Register] "10111rrr" => { OpCode::Cp8(AddressingMode::Register(Register::A), AddressingMode::Register(#r)) },
        [R: Register] "00RRR100" => { OpCode::Inc8(AddressingMode::Register(#R)) },
        [R: Register] "00RRR101" => { OpCode::Dec8(AddressingMode::Register(#R)) },
        [] "11001011" => { OpCode::Prefix },
        [] "00000000" => { OpCode::Nop },
        [R: RegisterPair] "00RR0001" => { OpCode::Ld16(AddressingMode::RegisterPair((#R).into_generalized()), AddressingMode::Immediate16) },
        [R: RegisterPairMem] "00RR0010" => { OpCode::Ld8(AddressingMode::IndirectRegister((#R).into_generalized()), AddressingMode::Register(Register::A)) },
        [R: RegisterPairMem] "00RR1010" => { OpCode::Ld8(AddressingMode::Register(Register::A),AddressingMode::IndirectRegister((#R).into_generalized())) },
        [] "00001000" => { OpCode::Ld16(AddressingMode::IndirectImmediate, AddressingMode::RegisterPair(RegisterPairs::SP)) },
        [R: RegisterPair] "00RR0011" => { OpCode::Inc16(AddressingMode::RegisterPair((#R).into_generalized())) },
        [R: RegisterPair] "00RR1011" => { OpCode::Dec16(AddressingMode::RegisterPair((#R).into_generalized())) },
        [R: RegisterPair] "00RR1001" => { OpCode::Add16(AddressingMode::RegisterPair(RegisterPairs::HL), AddressingMode::RegisterPair((#R).into_generalized())) },
        [] "00000111" => { OpCode::Rlca },
        [] "00001111" => { OpCode::Rrca },
        [] "00010111" => { OpCode::Rla },
        [] "00011111" => { OpCode::Rra },
        [] "00100111" => { OpCode::Daa },
        [] "00101111" => { OpCode::Cpl },
        [] "00110111" => { OpCode::Scf },
        [] "00111111" => { OpCode::Ccf },
        [] "00011000" => { OpCode::JrImm(None) },
        [c: Condition] "001cc000" => { OpCode::JrImm(Some(#c)) },
        [] "00010000" => { OpCode::Stop },
        [] "11000110" => { OpCode::Add8(AddressingMode::Register(Register::A), AddressingMode::Immediate) },
        [] "11001110" => { OpCode::Adc8(AddressingMode::Register(Register::A), AddressingMode::Immediate) },
        [] "11010110" => { OpCode::Sub8(AddressingMode::Register(Register::A), AddressingMode::Immediate) },
        [] "11011110" => { OpCode::Sbc8(AddressingMode::Register(Register::A), AddressingMode::Immediate) },
        [] "11100110" => { OpCode::And8(AddressingMode::Register(Register::A), AddressingMode::Immediate) },
        [] "11101110" => { OpCode::Xor8(AddressingMode::Register(Register::A), AddressingMode::Immediate) },
        [] "11110110" => { OpCode::Or8(AddressingMode::Register(Register::A), AddressingMode::Immediate) },
        [] "11111110" => { OpCode::Cp8(AddressingMode::Register(Register::A), AddressingMode::Immediate) },
        [] "11001001" => { OpCode::Ret(None) },
        [c: Condition] "110cc000" => { OpCode::Ret(Some(#c)) },
        [] "11011001" => { OpCode::Reti },
        [c: Condition] "110cc010" => { OpCode::JpImm(Some(#c)) },
        [] "11000011" => { OpCode::JpImm(None) },
        [] "11101001" => { OpCode::JpHl },
        [c: Condition] "110cc100" => { OpCode::CallImm(Some(#c)) },
        [] "11001101" => { OpCode::CallImm(None) },
        [t: ResetTarget] "11ttt111" => { OpCode::Reset(#t) },
        [r: RegisterPairStack] "11rr0001" => { OpCode::Pop(#r) },
        [r: RegisterPairStack] "11rr0101" => { OpCode::Push(#r) },
        [] "11100010" => { OpCode::Ld8(AddressingMode::IndirectZeroPageRegister(Register::C), AddressingMode::Register(Register::A)) },
        [] "11110010" => { OpCode::Ld8( AddressingMode::Register(Register::A), AddressingMode::IndirectZeroPageRegister(Register::C)) },
        [] "11100000" => { OpCode::Ld8(AddressingMode::IndirectZeroPageImmediate, AddressingMode::Register(Register::A)) },
        [] "11110000" => { OpCode::Ld8(AddressingMode::Register(Register::A), AddressingMode::IndirectZeroPageImmediate) },
        [] "11101010" => { OpCode::Ld8(AddressingMode::IndirectImmediate, AddressingMode::Register(Register::A)) },
        [] "11111010" => { OpCode::Ld8(AddressingMode::Register(Register::A), AddressingMode::IndirectImmediate) },
        [] "11101000" => { OpCode::AddSpImm },
        [] "11111000" => { OpCode::Ld16HlSpImm },
        [] "11111001" => { OpCode::Ld16(AddressingMode::RegisterPair(RegisterPairs::SP), AddressingMode::RegisterPair(RegisterPairs::HL)) },
        [] "11110011" => { OpCode::Di },
        [] "11111011" => { OpCode::Ei },
        [] "00110100" => { OpCode::Inc8(AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "00110101" => { OpCode::Dec8(AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "10000110" => { OpCode::Add8(AddressingMode::Register(Register::A), AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "10001110" => { OpCode::Adc8(AddressingMode::Register(Register::A), AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "10010110" => { OpCode::Sub8(AddressingMode::Register(Register::A), AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "10011110" => { OpCode::Sbc8(AddressingMode::Register(Register::A), AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "10100110" => { OpCode::And8(AddressingMode::Register(Register::A), AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "10101110" => { OpCode::Xor8(AddressingMode::Register(Register::A), AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "10110110" => { OpCode::Or8(AddressingMode::Register(Register::A), AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "10111110" => { OpCode::Cp8(AddressingMode::Register(Register::A), AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "11010011" => { OpCode::Illegal },
        [] "11100011" => { OpCode::Illegal },
        [] "11100100" => { OpCode::Illegal },
        [] "11110100" => { OpCode::Illegal },
        [] "11011011" => { OpCode::Illegal },
        [] "11101011" => { OpCode::Illegal },
        [] "11101100" => { OpCode::Illegal },
        [] "11111100" => { OpCode::Illegal },
        [] "11011101" => { OpCode::Illegal },
        [] "11101101" => { OpCode::Illegal },
        [] "11111101" => { OpCode::Illegal },
    },
    PREFIXED_TABLE: [OpCode; 256] {
        [r: Register] "00000rrr" => { OpCode::Rlc(AddressingMode::Register(#r)) },
        [r: Register] "00001rrr" => { OpCode::Rrc(AddressingMode::Register(#r)) },
        [r: Register] "00010rrr" => { OpCode::Rl(AddressingMode::Register(#r)) },
        [r: Register] "00011rrr" => { OpCode::Rr(AddressingMode::Register(#r)) },
        [r: Register] "00100rrr" => { OpCode::Sla(AddressingMode::Register(#r)) },
        [r: Register] "00101rrr" => { OpCode::Sra(AddressingMode::Register(#r)) },
        [r: Register] "00110rrr" => { OpCode::Swap(AddressingMode::Register(#r)) },
        [r: Register] "00111rrr" => { OpCode::Srl(AddressingMode::Register(#r)) },
        [] "00000110" => { OpCode::Rlc(AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "00001110" => { OpCode::Rrc(AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "00010110" => { OpCode::Rl(AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "00011110" => { OpCode::Rr(AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "00100110" => { OpCode::Sla(AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "00101110" => { OpCode::Sra(AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "00110110" => { OpCode::Swap(AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [] "00111110" => { OpCode::Srl(AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [r: Register, b: Bit] "01bbbrrr" => { OpCode::Bit(#b, AddressingMode::Register(#r)) },
        [r: Register, b: Bit] "10bbbrrr" => { OpCode::Res(#b, AddressingMode::Register(#r)) },
        [r: Register, b: Bit] "11bbbrrr" => { OpCode::Set(#b, AddressingMode::Register(#r)) },
        [b: Bit] "01bbb110" => { OpCode::Bit(#b, AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [b: Bit] "10bbb110" => { OpCode::Res(#b, AddressingMode::IndirectRegister(RegisterPairs::HL)) },
        [b: Bit] "11bbb110" => { OpCode::Set(#b, AddressingMode::IndirectRegister(RegisterPairs::HL)) },
    },
}

/// Decodes a single instruction. May return an OpCode::Prefix value, which indicates that this
/// instruction is prefixed, and `decode_prefixed` must be invoked with the next byte in the stream
pub fn decode(byte: u8) -> OpCode {
    DECODER_TABLE[byte as usize]
}

/// Decodes a prefixed instruction by looking at the byte after the 0xCB prefix byte.
pub fn decode_prefixed(byte: u8) -> OpCode {
    PREFIXED_TABLE[byte as usize]
}
