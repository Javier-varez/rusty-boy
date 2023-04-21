use sm83_decoder_macros::generate_decoder_table;

pub enum Argument {
    ImmediateU8(u8),
    ImmediateU16(u16),
    Register(Register),
    RegisterPair(RegisterPair),
    IndirectU8(u16),
    IndirectU16(u16),
}

pub enum OpCode {
    Ld8RegReg(Register, Register),
    Ld8RegImm(Register),
    Ld8RegRegInd(Register, RegisterPair),
    Ld8RegIndReg(RegisterPair, Register),
    Ld8RegIndImm(RegisterPair),
}

// r => destination reg
// R => source reg

generate_decoder_table! {
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
            AF = 0,
            BC = 1,
            DE = 2,
            HL = 3,
            SP = 4,
            PC = 5,
        }
    }
    DECODER_TABLE: [OpCode; 256] {
        [r: Register, R: Register] "01rrrRRR" => { OpCode::Ld8RegReg(#r, #R) },
        [r: Register] "00rrr110" => { OpCode::Ld8RegImm(#r) },
        [r: Register] "01rrr110" => { OpCode::Ld8RegRegInd(#r, RegisterPair::HL) },
        [R: Register] "01110RRR" => { OpCode::Ld8RegIndReg(RegisterPair::HL, #R) },
        [] "01110110" => { OpCode::Ld8RegIndImm(RegisterPair::HL) },
    }
}
