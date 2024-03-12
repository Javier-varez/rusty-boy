use sm83_decoder_macros::generate_decoder_table;

pub enum OpCode {
    Ld8RegReg(Register, Register),     // ld Register, Register
    Ld8RegImm(Register),               // ld Register, n8
    Ld8RegInd(Register, RegisterPair), // ld Register, [RegisterPair]
    Ld8IndReg(RegisterPair, Register), // ld [RegisterPair], Register
    Ld8IndImm(RegisterPair),           // ld [RegisterPair], n8
    Halt,                              // halt
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
        [r: Register] "01rrr110" => { OpCode::Ld8RegInd(#r, RegisterPair::HL) },
        [R: Register] "01110RRR" => { OpCode::Ld8IndReg(RegisterPair::HL, #R) },
        [] "01110110" => { OpCode::Halt },
        [r: Register] "00rrr110" => { OpCode::Ld8RegImm(#r) },
        [] "00110110" => { OpCode::Ld8IndImm(RegisterPair::HL) },
    }
}
