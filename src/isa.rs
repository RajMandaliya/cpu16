/// cpu16 — Custom 16-bit ISA
///
/// Instruction format (16 bits):
///
///  [ 6-bit opcode | 2-bit dst | 2-bit src | 6-bit immediate/offset ]
///
/// Registers: R0, R1, R2, R3
/// Special:   SP (stack pointer), PC (program counter), FLAGS

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    // Data movement
    Nop = 0x00,
    Load = 0x01,  // LOAD  Rd, imm8     — load 8-bit immediate into Rd (zero-extended)
    LoadM = 0x02, // LOADM Rd, [Rs]     — load from memory address in Rs into Rd
    Store = 0x03, // STORE [Rd], Rs     — store Rs into memory address in Rd
    Mov = 0x04,   // MOV   Rd, Rs       — copy Rs into Rd

    // Arithmetic
    Add = 0x05,  // ADD  Rd, Rs        — Rd = Rd + Rs
    Sub = 0x06,  // SUB  Rd, Rs        — Rd = Rd - Rs
    Addi = 0x07, // ADDI Rd, imm6      — Rd = Rd + imm6 (sign-extended)
    Mul = 0x08,  // MUL  Rd, Rs        — Rd = Rd * Rs (lower 16 bits)
    Div = 0x09,  // DIV  Rd, Rs        — Rd = Rd / Rs

    // Bitwise / Logic
    And = 0x0A, // AND  Rd, Rs
    Or = 0x0B,  // OR   Rd, Rs
    Xor = 0x0C, // XOR  Rd, Rs
    Not = 0x0D, // NOT  Rd
    Shl = 0x0E, // SHL  Rd, imm4      — shift left
    Shr = 0x0F, // SHR  Rd, imm4      — shift right (logical)

    // Comparison
    Cmp = 0x10, // CMP  Ra, Rb        — set flags based on Ra - Rb (no writeback)

    // Control flow
    Jmp = 0x11,  // JMP  addr          — unconditional jump (full 16-bit addr in next word)
    Jz = 0x12,   // JZ   addr          — jump if Zero flag set
    Jnz = 0x13,  // JNZ  addr          — jump if Zero flag clear
    Jc = 0x14,   // JC   addr          — jump if Carry flag set
    Jn = 0x15,   // JN   addr          — jump if Negative flag set
    Call = 0x16, // CALL addr          — push PC+2, jump to addr
    Ret = 0x17,  // RET               — pop PC

    // Stack
    Push = 0x18, // PUSH Rs
    Pop = 0x19,  // POP  Rd

    // Interrupts
    Int = 0x1A,  // INT  imm4          — software interrupt
    Iret = 0x1B, // IRET              — return from interrupt
    Ei = 0x1C,   // EI                — enable interrupts
    Di = 0x1D,   // DI                — disable interrupts

    Halt = 0x3F, // HALT
}

impl TryFrom<u8> for Opcode {
    type Error = String;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x00 => Ok(Self::Nop),
            0x01 => Ok(Self::Load),
            0x02 => Ok(Self::LoadM),
            0x03 => Ok(Self::Store),
            0x04 => Ok(Self::Mov),
            0x05 => Ok(Self::Add),
            0x06 => Ok(Self::Sub),
            0x07 => Ok(Self::Addi),
            0x08 => Ok(Self::Mul),
            0x09 => Ok(Self::Div),
            0x0A => Ok(Self::And),
            0x0B => Ok(Self::Or),
            0x0C => Ok(Self::Xor),
            0x0D => Ok(Self::Not),
            0x0E => Ok(Self::Shl),
            0x0F => Ok(Self::Shr),
            0x10 => Ok(Self::Cmp),
            0x11 => Ok(Self::Jmp),
            0x12 => Ok(Self::Jz),
            0x13 => Ok(Self::Jnz),
            0x14 => Ok(Self::Jc),
            0x15 => Ok(Self::Jn),
            0x16 => Ok(Self::Call),
            0x17 => Ok(Self::Ret),
            0x18 => Ok(Self::Push),
            0x19 => Ok(Self::Pop),
            0x1A => Ok(Self::Int),
            0x1B => Ok(Self::Iret),
            0x1C => Ok(Self::Ei),
            0x1D => Ok(Self::Di),
            0x3F => Ok(Self::Halt),
            other => Err(format!("Unknown opcode: 0x{:02X}", other)),
        }
    }
}

/// Decoded instruction (post-fetch)
#[derive(Debug, Clone, Copy)]
pub struct Instruction {
    pub opcode: Opcode,
    pub dst: u8,  // 2-bit register index
    pub src: u8,  // 2-bit register index
    pub imm: u16, // immediate / address (may come from next word)
}

impl Instruction {
    /// Pack a register+register instruction into a 16-bit word.
    pub fn encode_rr(op: Opcode, dst: u8, src: u8) -> u16 {
        ((op as u16) << 10) | ((dst as u16 & 0x3) << 8) | ((src as u16 & 0x3) << 6)
    }

    /// Pack a register+immediate instruction into a 16-bit word.
    pub fn encode_ri(op: Opcode, dst: u8, imm6: u8) -> u16 {
        ((op as u16) << 10) | ((dst as u16 & 0x3) << 8) | (imm6 as u16 & 0x3F)
    }

    /// Decode a 16-bit word (address+immediate comes from the next word).
    pub fn decode(word: u16, next_word: u16) -> Result<Self, String> {
        let opcode_bits = (word >> 10) as u8;
        let dst = ((word >> 8) & 0x3) as u8;
        let src = ((word >> 6) & 0x3) as u8;
        let imm6 = (word & 0x3F) as u16;

        let opcode = Opcode::try_from(opcode_bits)?;

        // Instructions that carry a full 16-bit address in the next word
        let imm = match opcode {
            Opcode::Jmp | Opcode::Jz | Opcode::Jnz | Opcode::Jc | Opcode::Jn | Opcode::Call => {
                next_word
            }
            _ => imm6,
        };

        Ok(Instruction {
            opcode,
            dst,
            src,
            imm,
        })
    }
}
