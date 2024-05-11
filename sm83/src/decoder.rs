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

/// Opcodes of the CPU.
#[derive(Debug, Clone, Copy)]
pub enum OpCode {
    Ld8RegReg(Register, Register),                 // ld Register, Register
    Ld8RegImm(Register),                           // ld Register, n8
    Ld8RegInd(Register, RegisterPair),             // ld Register, [RegisterPair]
    Ld8IndReg(RegisterPair, Register),             // ld [RegisterPair], Register
    Ld8IndImm(RegisterPair),                       // ld [RegisterPair], n8
    Ld8IndAcc(RegisterPairMem),                    // ld [RegisterPairMem], A
    Ld8AccInd(RegisterPairMem),                    // ld A, [RegisterPairMem]
    Ld8ZeroPageCAcc,                               // ld [C], A
    Ld8AccZeroPageC,                               // ld A, [C]
    Ld8ZeroPageImmAcc,                             // ld [n8], A
    Ld8AccZeroPageImm,                             // ld A, [n8]
    Ld8IndImmAcc,                                  // ld [n16], A
    Ld8AccIndImm,                                  // ld A, [n16]
    Ld16RegImm(RegisterPair),                      // ld RegisterPair, n16
    Ld16IndImmSp,                                  // ld [a16], SP
    Ld16HlSpImm,                                   // ld HL, SP + n8
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
    AddAccImm,                                     // add A, n8
    AdcAccImm,                                     // adc A, n8
    SubAccImm,                                     // sub A, n8
    SbcAccImm,                                     // sbc A, n8
    AndAccImm,                                     // and A, n8
    XorAccImm,                                     // xor A, n8
    OrAccImm,                                      // or A, n8
    CpAccImm,                                      // cp A, n8
    AddSpImm,                                      // add SP, n8
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
    Prefix,                                        // prefix
    Nop,                                           // nop
    Daa,                                           // daa
    Cpl,                                           // cpl
    Scf,                                           // scf
    Ccf,                                           // ccf
    JrImm,                                         // jr imm8
    JrCondImm(Condition),                          // jr cond, imm8
    Stop,                                          // stop
    RetCond(Condition),                            // ret cond
    Ret,                                           // ret
    Reti,                                          // reti
    JpCondImm(Condition),                          // jp cond, imm16
    JpImm,                                         // jp imm16
    JpHl,                                          // jp HL
    CallCondImm(Condition),                        // call cond, imm16
    CallImm,                                       // call imm16
    Reset(ResetTarget),                            // reset target
    Pop(RegisterPairStack),                        // pop RegisterPairStack
    Push(RegisterPairStack),                       // push RegisterPairStack
    Di,                                            // di
    Ei,                                            // ei
    Illegal,                                       // Illegal
    Rlca,                                          // rlca
    Rrca,                                          // rrca
    Rla,                                           // rla Register
    Rra,                                           // rra Register
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
        [r: Register, R: Register] "01rrrRRR" => { OpCode::Ld8RegReg(#r, #R) },
        [r: Register] "01rrr110" => { OpCode::Ld8RegInd(#r, RegisterPair::HL) },
        [R: Register] "01110RRR" => { OpCode::Ld8IndReg(RegisterPair::HL, #R) },
        [] "01110110" => { OpCode::Halt },
        [r: Register] "00rrr110" => { OpCode::Ld8RegImm(#r) },
        [] "00110110" => { OpCode::Ld8IndImm(RegisterPair::HL) },
        [r: Register] "10000rrr" => { OpCode::AddRegReg(Register::A, #r) },
        [r: Register] "10010rrr" => { OpCode::SubRegReg(Register::A, #r) },
        [r: Register] "10100rrr" => { OpCode::AndRegReg(Register::A, #r) },
        [r: Register] "10110rrr" => { OpCode::OrRegReg(Register::A, #r) },
        [r: Register] "10001rrr" => { OpCode::AdcRegReg(Register::A, #r) },
        [r: Register] "10011rrr" => { OpCode::SbcRegReg(Register::A, #r) },
        [r: Register] "10101rrr" => { OpCode::XorRegReg(Register::A, #r) },
        [r: Register] "10111rrr" => { OpCode::CpRegReg(Register::A, #r) },
        [R: Register] "00RRR100" => { OpCode::IncReg(#R) },
        [R: Register] "00RRR101" => { OpCode::DecReg(#R) },
        [] "11001011" => { OpCode::Prefix },
        [] "00000000" => { OpCode::Nop },
        [R: RegisterPair] "00RR0001" => { OpCode::Ld16RegImm(#R) },
        [R: RegisterPairMem] "00RR0010" => { OpCode::Ld8IndAcc(#R) },
        [R: RegisterPairMem] "00RR1010" => { OpCode::Ld8AccInd(#R) },
        [] "00001000" => { OpCode::Ld16IndImmSp },
        [R: RegisterPair] "00RR0011" => { OpCode::IncRegPair(#R) },
        [R: RegisterPair] "00RR1011" => { OpCode::DecRegPair(#R) },
        [R: RegisterPair] "00RR1001" => { OpCode::AddRegPairRegPair(RegisterPair::HL, #R) },
        [] "00000111" => { OpCode::Rlca },
        [] "00001111" => { OpCode::Rrca },
        [] "00010111" => { OpCode::Rla },
        [] "00011111" => { OpCode::Rra },
        [] "00100111" => { OpCode::Daa },
        [] "00101111" => { OpCode::Cpl },
        [] "00110111" => { OpCode::Scf },
        [] "00111111" => { OpCode::Ccf },
        [] "00011000" => { OpCode::JrImm },
        [c: Condition] "001cc000" => { OpCode::JrCondImm(#c) },
        [] "00010000" => { OpCode::Stop },
        [] "11000110" => { OpCode::AddAccImm },
        [] "11001110" => { OpCode::AdcAccImm },
        [] "11010110" => { OpCode::SubAccImm },
        [] "11011110" => { OpCode::SbcAccImm },
        [] "11100110" => { OpCode::AndAccImm },
        [] "11101110" => { OpCode::XorAccImm },
        [] "11110110" => { OpCode::OrAccImm },
        [] "11111110" => { OpCode::CpAccImm },
        [c: Condition] "110cc000" => { OpCode::RetCond(#c) },
        [] "11001001" => { OpCode::Ret },
        [] "11011001" => { OpCode::Reti },
        [c: Condition] "110cc010" => { OpCode::JpCondImm(#c) },
        [] "11000011" => { OpCode::JpImm },
        [] "11101001" => { OpCode::JpHl },
        [c: Condition] "110cc100" => { OpCode::CallCondImm(#c) },
        [] "11001101" => { OpCode::CallImm },
        [t: ResetTarget] "11ttt111" => { OpCode::Reset(#t) },
        [r: RegisterPairStack] "11rr0001" => { OpCode::Pop(#r) },
        [r: RegisterPairStack] "11rr0101" => { OpCode::Push(#r) },
        [] "11100010" => { OpCode::Ld8ZeroPageCAcc },
        [] "11110010" => { OpCode::Ld8AccZeroPageC },
        [] "11100000" => { OpCode::Ld8ZeroPageImmAcc },
        [] "11110000" => { OpCode::Ld8AccZeroPageImm },
        [] "11101010" => { OpCode::Ld8IndImmAcc },
        [] "11111010" => { OpCode::Ld8AccIndImm },
        [] "11101000" => { OpCode::AddSpImm },
        [] "11111000" => { OpCode::Ld16HlSpImm },
        [] "11111001" => { OpCode::Ld16SpHl },
        [] "11110011" => { OpCode::Di },
        [] "11111011" => { OpCode::Ei },
        [] "00110100" => { OpCode::IncIndHl },
        [] "00110101" => { OpCode::DecIndHl },
        [] "10000110" => { OpCode::AddAccHlInd },
        [] "10001110" => { OpCode::AdcAccHlInd },
        [] "10010110" => { OpCode::SubAccHlInd },
        [] "10011110" => { OpCode::SbcAccHlInd },
        [] "10100110" => { OpCode::AndAccHlInd },
        [] "10101110" => { OpCode::XorAccHlInd },
        [] "10110110" => { OpCode::OrAccHlInd },
        [] "10111110" => { OpCode::CpAccHlInd },

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
        [r: Register] "00000rrr" => { OpCode::RlcReg(#r) },
        [r: Register] "00001rrr" => { OpCode::RrcReg(#r) },
        [r: Register] "00010rrr" => { OpCode::RlReg(#r) },
        [r: Register] "00011rrr" => { OpCode::RrReg(#r) },
        [r: Register] "00100rrr" => { OpCode::SlaReg(#r) },
        [r: Register] "00101rrr" => { OpCode::SraReg(#r) },
        [r: Register] "00110rrr" => { OpCode::SwapReg(#r) },
        [r: Register] "00111rrr" => { OpCode::SrlReg(#r) },
        [] "00000110" => { OpCode::RlcHlInd },
        [] "00001110" => { OpCode::RrcHlInd },
        [] "00010110" => { OpCode::RlHlInd },
        [] "00011110" => { OpCode::RrHlInd },
        [] "00100110" => { OpCode::SlaHlInd },
        [] "00101110" => { OpCode::SraHlInd },
        [] "00110110" => { OpCode::SwapHlInd },
        [] "00111110" => { OpCode::SrlHlInd },
        [r: Register, b: Bit] "01bbbrrr" => { OpCode::Bit(#b, #r) },
        [r: Register, b: Bit] "10bbbrrr" => { OpCode::Res(#b, #r) },
        [r: Register, b: Bit] "11bbbrrr" => { OpCode::Set(#b, #r) },
        [b: Bit] "01bbb110" => { OpCode::BitHlInd(#b) },
        [b: Bit] "10bbb110" => { OpCode::ResHlInd(#b) },
        [b: Bit] "11bbb110" => { OpCode::SetHlInd(#b) },
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
