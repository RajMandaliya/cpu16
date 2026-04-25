use cpu16::assembler::Assembler;
use cpu16::cpu::{Cpu, CpuState, PROG_BASE};
use cpu16::isa::{Instruction, Opcode};

/// Assemble a source string and load it into a fresh CPU.
fn asm_and_load(src: &str) -> Cpu {
    let assembler = Assembler::new(PROG_BASE);
    let output = assembler.assemble(src).expect("Assembly failed");
    let bytes: Vec<u8> = output.words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut cpu = Cpu::new();
    cpu.load_program(&bytes);
    cpu
}

// ── Arithmetic ────────────────────────────────────────────────────────────────

#[test]
fn test_load_and_add() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 5
        LOAD R1, 3
        ADD  R0, R1
        HALT
    ",
    );
    assert_eq!(cpu.run(100).unwrap(), CpuState::Halted);
    assert_eq!(cpu.regs[0], 8);
}

#[test]
fn test_subtract() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 10
        LOAD R1, 4
        SUB  R0, R1
        HALT
    ",
    );
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 6);
}

#[test]
fn test_multiply() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 7
        LOAD R1, 6
        MUL  R0, R1
        HALT
    ",
    );
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 42);
}

#[test]
fn test_divide() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 20
        LOAD R1, 4
        DIV  R0, R1
        HALT
    ",
    );
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 5);
}

#[test]
fn test_addi_positive() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 10
        ADDI R0, 5
        HALT
    ",
    );
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 15);
}

#[test]
fn test_addi_negative() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 10
        ADDI R0, -3
        HALT
    ",
    );
    cpu.run(100).unwrap();
    // ADDI sign-extends the 6-bit immediate: -3 in two's complement 6-bit = 0b111101
    // The assembler stores -3 as u16 (0xFFFD), masked to 6 bits = 0x3D = 61
    // sign-extended from 6 bits: bit5=1 → negative → 61 - 64 = -3 → R0 = 10 + (-3) = 7
    assert_eq!(cpu.regs[0], 7);
}

// ── Flags ─────────────────────────────────────────────────────────────────────

#[test]
fn test_zero_flag() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 5
        LOAD R1, 5
        SUB  R0, R1
        HALT
    ",
    );
    cpu.run(100).unwrap();
    assert!(cpu.flags.zero(), "Zero flag should be set after 5-5");
    assert_eq!(cpu.regs[0], 0);
}

#[test]
fn test_negative_flag() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 3
        LOAD R1, 5
        SUB  R0, R1
        HALT
    ",
    );
    cpu.run(100).unwrap();
    assert!(
        cpu.flags.negative(),
        "Negative flag should be set after 3-5"
    );
}

#[test]
fn test_carry_flag() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 63     ; 0x003F
        LOAD R1, 63
        ADD  R0, R1
        HALT
    ",
    );
    cpu.run(100).unwrap();
    // 63+63 = 126, no carry expected
    assert!(!cpu.flags.carry());
    assert_eq!(cpu.regs[0], 126);
}

// ── Logic ─────────────────────────────────────────────────────────────────────

#[test]
fn test_and() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 15
        LOAD R1, 6
        AND  R0, R1
        HALT
    ",
    );
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 15 & 6);
}

#[test]
fn test_or() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 12
        LOAD R1, 3
        OR   R0, R1
        HALT
    ",
    );
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 15);
}

#[test]
fn test_xor_self_is_zero() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 42
        XOR  R0, R0
        HALT
    ",
    );
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0);
    assert!(cpu.flags.zero());
}

#[test]
fn test_not() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 0
        NOT  R0
        HALT
    ",
    );
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0xFFFF);
}

// ── Control flow ──────────────────────────────────────────────────────────────

#[test]
fn test_jmp_unconditional() {
    let mut cpu = asm_and_load(
        "
        JMP  SKIP
        LOAD R0, 99       ; should be skipped
SKIP:
        LOAD R1, 42
        HALT
    ",
    );
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0, "R0 should not have been loaded");
    assert_eq!(cpu.regs[1], 42, "R1 should be 42");
}

#[test]
fn test_jz_taken() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 0
        LOAD R1, 0
        CMP  R0, R1
        JZ   SET_R2
        LOAD R2, 1
        HALT
SET_R2:
        LOAD R2, 42
        HALT
    ",
    );
    cpu.run(200).unwrap();
    assert_eq!(cpu.regs[2], 42, "JZ should be taken");
}

#[test]
fn test_jnz_not_taken_when_zero() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 5
        LOAD R1, 5
        CMP  R0, R1
        JNZ  SKIP
        LOAD R2, 7
        HALT
SKIP:
        LOAD R2, 0
        HALT
    ",
    );
    cpu.run(200).unwrap();
    assert_eq!(
        cpu.regs[2], 7,
        "JNZ should NOT be taken when Zero flag is set"
    );
}

// ── Stack & subroutines ───────────────────────────────────────────────────────

#[test]
fn test_push_pop() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 55
        PUSH R0
        LOAD R0, 0
        POP  R1
        HALT
    ",
    );
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[1], 55, "POP should restore pushed value");
    assert_eq!(cpu.regs[0], 0, "R0 should have been zeroed");
}

#[test]
fn test_call_ret() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 0
        CALL ADD_ONE
        HALT

ADD_ONE:
        ADDI R0, 1
        RET
    ",
    );
    cpu.run(200).unwrap();
    assert_eq!(cpu.regs[0], 1, "Subroutine should have incremented R0");
}

#[test]
fn test_nested_calls() {
    let mut cpu = asm_and_load(
        "
        LOAD R0, 0
        CALL OUTER
        HALT

OUTER:
        ADDI R0, 10
        CALL INNER
        ADDI R0, 10
        RET

INNER:
        ADDI R0, 1
        RET
    ",
    );
    cpu.run(300).unwrap();
    assert_eq!(cpu.regs[0], 21, "Nested calls: 0+10+1+10 = 21");
}

// ── Memory ────────────────────────────────────────────────────────────────────

#[test]
fn test_store_loadm() {
    let mut cpu = asm_and_load(
        "
        LOAD  R0, 0x0400   ; address
        LOAD  R1, 0xBEEF   ; value
        STORE R0, R1       ; mem[0x0400] = 0xBEEF
        LOAD  R2, 0
        LOADM R2, R0       ; R2 = mem[0x0400]
        HALT
    ",
    );
    cpu.run(200).unwrap();
    assert_eq!(
        cpu.regs[2], 0xBEEF,
        "Loaded value should match stored value"
    );
}

// ── Interrupts ────────────────────────────────────────────────────────────────

#[test]
fn test_software_interrupt() {
    // Write the handler address to IVT slot 0 manually, then trigger INT 0
    let mut cpu = Cpu::new();

    // Write handler address into IVT[0] = 0x0300 directly in memory
    cpu.mem.write_word(0x0000, 0x0300);

    // Handler at 0x0300: increment R3, IRET
    // ADDI R3, 1  =  encode_ri(Addi, 3, 1)
    let addi_r3_1 = ((Opcode::Addi as u16) << 10) | ((3u16) << 8) | 1;
    let iret = (Opcode::Iret as u16) << 10;
    cpu.mem.write_word(0x0300, addi_r3_1);
    cpu.mem.write_word(0x0302, iret);

    // Program at PROG_BASE: INT 0, INT 0, HALT
    let int0 = ((Opcode::Int as u16) << 10) | 0;
    let halt = (Opcode::Halt as u16) << 10;
    cpu.mem.write_word(PROG_BASE, int0);
    cpu.mem.write_word(PROG_BASE + 2, int0);
    cpu.mem.write_word(PROG_BASE + 4, halt);

    cpu.pc = PROG_BASE;
    cpu.flags.set_int_enable(true);

    cpu.run(500).unwrap();
    assert_eq!(cpu.regs[3], 2, "Interrupt handler should have run twice");
}

// ── Full programs ─────────────────────────────────────────────────────────────

#[test]
fn test_factorial_6() {
    let src = std::fs::read_to_string("examples/factorial.asm")
        .expect("Could not read examples/factorial.asm");
    let mut cpu = asm_and_load(&src);
    cpu.run(10_000).unwrap();
    assert_eq!(cpu.regs[1], 720, "6! should be 720");
}

#[test]
fn test_fibonacci_10() {
    let src = std::fs::read_to_string("examples/fibonacci.asm")
        .expect("Could not read examples/fibonacci.asm");
    let mut cpu = asm_and_load(&src);
    cpu.run(10_000).unwrap();
    assert_eq!(cpu.regs[1], 55, "Fibonacci(10) should be 55");
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests for extended instructions: NEG, MOD, SWAP, ROL, ROR  (v0.3.0)
//
// Add these test functions to your existing tests/integration_tests.rs file.
// They follow the exact same pattern as your existing tests.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_neg_positive_value() {
    // NEG(0x0005) → 0xFFFB, C=1 (source was non-zero), N=1, Z=0, V=0
    let mut cpu = Cpu::new();
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 5), // LOAD R0, 5
        Instruction::encode_ri(Opcode::Neg, 0, 0),  // NEG  R0
        Instruction::encode_ri(Opcode::Halt, 0, 0), // HALT
    ];
    cpu.load_program(to_bytes(program));
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0xFFFB);
    assert!(cpu.flags.carry()); // source was non-zero
    assert!(cpu.flags.negative()); // result is negative
    assert!(!cpu.flags.zero());
    assert!(!cpu.flags.overflow());
}

#[test]
fn test_neg_zero() {
    // NEG(0x0000) → 0x0000, C=0, Z=1, N=0, V=0
    let mut cpu = Cpu::new();
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 0), // LOAD R0, 0
        Instruction::encode_ri(Opcode::Neg, 0, 0),  // NEG  R0
        Instruction::encode_ri(Opcode::Halt, 0, 0), // HALT
    ];
    cpu.load_program(to_bytes(program));
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0x0000);
    assert!(!cpu.flags.carry()); // source was zero — no borrow
    assert!(cpu.flags.zero());
    assert!(!cpu.flags.negative());
    assert!(!cpu.flags.overflow());
}

#[test]
fn test_neg_min_int_overflow() {
    // NEG(0x8000) → 0x8000 (wraps), V=1, C=1, N=1, Z=0
    // Negating the most negative signed 16-bit integer overflows back to itself.
    let mut cpu = Cpu::new();
    // LOAD wide value 0x8000 using the sentinel mechanism
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 0x3E), // LOAD R0, <next word>
        0x8000u16,                                     // wide immediate = 0x8000
        Instruction::encode_ri(Opcode::Neg, 0, 0),     // NEG  R0
        Instruction::encode_ri(Opcode::Halt, 0, 0),    // HALT
    ];
    cpu.load_program(to_bytes(program));
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0x8000);
    assert!(cpu.flags.overflow()); // signed overflow
    assert!(cpu.flags.carry()); // source was non-zero
    assert!(cpu.flags.negative());
    assert!(!cpu.flags.zero());
}

#[test]
fn test_mod_basic() {
    // MOD(10, 3) → 1
    let mut cpu = Cpu::new();
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 10), // LOAD R0, 10
        Instruction::encode_ri(Opcode::Load, 1, 3),  // LOAD R1, 3
        Instruction::encode_rr(Opcode::Mod, 0, 1),   // MOD  R0, R1  → R0 = 10 % 3 = 1
        Instruction::encode_ri(Opcode::Halt, 0, 0),  // HALT
    ];
    cpu.load_program(to_bytes(program));
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 1);
    assert!(!cpu.flags.zero());
}

#[test]
fn test_mod_evenly_divisible() {
    // MOD(12, 4) → 0, Z=1
    let mut cpu = Cpu::new();
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 12), // LOAD R0, 12
        Instruction::encode_ri(Opcode::Load, 1, 4),  // LOAD R1, 4
        Instruction::encode_rr(Opcode::Mod, 0, 1),   // MOD  R0, R1  → R0 = 0
        Instruction::encode_ri(Opcode::Halt, 0, 0),  // HALT
    ];
    cpu.load_program(to_bytes(program));
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0);
    assert!(cpu.flags.zero());
}

#[test]
fn test_mod_by_zero_errors() {
    // MOD(5, 0) → Err
    let mut cpu = Cpu::new();
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 5), // LOAD R0, 5
        Instruction::encode_ri(Opcode::Load, 1, 0), // LOAD R1, 0
        Instruction::encode_rr(Opcode::Mod, 0, 1),  // MOD  R0, R1  → error
        Instruction::encode_ri(Opcode::Halt, 0, 0), // HALT
    ];
    cpu.load_program(to_bytes(program));
    let result = cpu.run(100);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Modulo by zero");
}

#[test]
fn test_swap_basic() {
    // SWAP R0, R1: R0=0xAAAA, R1=0x5555 → R0=0x5555, R1=0xAAAA
    let mut cpu = Cpu::new();
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 0x3E), // LOAD R0, <next>
        0xAAAAu16,
        Instruction::encode_ri(Opcode::Load, 1, 0x3E), // LOAD R1, <next>
        0x5555u16,
        Instruction::encode_rr(Opcode::Swap, 0, 1), // SWAP R0, R1
        Instruction::encode_ri(Opcode::Halt, 0, 0), // HALT
    ];
    cpu.load_program(to_bytes(program));
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0x5555);
    assert_eq!(cpu.regs[1], 0xAAAA);
}

#[test]
fn test_swap_same_register() {
    // SWAP R0, R0: value unchanged
    let mut cpu = Cpu::new();
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 42), // LOAD R0, 42
        Instruction::encode_rr(Opcode::Swap, 0, 0),  // SWAP R0, R0
        Instruction::encode_ri(Opcode::Halt, 0, 0),  // HALT
    ];
    cpu.load_program(to_bytes(program));
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 42);
}

#[test]
fn test_rol_by_1() {
    // ROL R0 by 1, C=0 initially
    // R0 = 0b1000_0000_0000_0001 = 0x8001
    // Rotate left 1: bit15 (1) → C, old C (0) → bit0
    // Result: 0b0000_0000_0000_0010 = 0x0002, new C=1
    let mut cpu = Cpu::new();
    cpu.flags.set_carry(false);
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 0x3E), // LOAD R0, <next>
        0x8001u16,
        Instruction::encode_ri(Opcode::Rol, 0, 1), // ROL  R0, 1
        Instruction::encode_ri(Opcode::Halt, 0, 0), // HALT
    ];
    cpu.load_program(to_bytes(program));
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0x0002);
    assert!(cpu.flags.carry()); // old bit15 became C
    assert!(!cpu.flags.zero());
    assert!(!cpu.flags.negative());
}

#[test]
fn test_rol_carry_enters_bit0() {
    // ROL with C=1: carry should enter bit 0
    // R0 = 0x0002 = 0b0000_0000_0000_0010, C=1
    // After ROL 1: result = 0x0005 = 0b0000_0000_0000_0101, new C=0
    let mut cpu = Cpu::new();
    cpu.flags.set_carry(true);
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 2), // LOAD R0, 2
        Instruction::encode_ri(Opcode::Rol, 0, 1),  // ROL  R0, 1
        Instruction::encode_ri(Opcode::Halt, 0, 0), // HALT
    ];
    cpu.load_program(to_bytes(program));
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0x0005);
    assert!(!cpu.flags.carry()); // old bit15 was 0
}

#[test]
fn test_ror_by_1() {
    // ROR R0 by 1, C=0 initially
    // R0 = 0x0003 = 0b0000_0000_0000_0011
    // Rotate right 1: bit0 (1) → C, old C (0) → bit15
    // Result: 0b0000_0000_0000_0001 = 0x0001, new C=1
    let mut cpu = Cpu::new();
    cpu.flags.set_carry(false);
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 3), // LOAD R0, 3
        Instruction::encode_ri(Opcode::Ror, 0, 1),  // ROR  R0, 1
        Instruction::encode_ri(Opcode::Halt, 0, 0), // HALT
    ];
    cpu.load_program(to_bytes(program));
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0x0001);
    assert!(cpu.flags.carry()); // old bit0 was 1
}

#[test]
fn test_ror_carry_enters_bit15() {
    // ROR with C=1: carry should enter bit 15 making result negative
    // R0 = 0x0002 = 0b0000_0000_0000_0010, C=1
    // After ROR 1: result = 0b1000_0000_0000_0001 = 0x8001, new C=0, N=1
    let mut cpu = Cpu::new();
    cpu.flags.set_carry(true);
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 2), // LOAD R0, 2
        Instruction::encode_ri(Opcode::Ror, 0, 1),  // ROR  R0, 1
        Instruction::encode_ri(Opcode::Halt, 0, 0), // HALT
    ];
    cpu.load_program(to_bytes(program));
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0x8001);
    assert!(!cpu.flags.carry()); // old bit0 was 0
    assert!(cpu.flags.negative()); // bit15 now set
}

#[test]
fn test_rol_ror_roundtrip() {
    // ROL then ROR by the same count should restore the original value
    // (as long as C starts at 0 and count is consistent)
    let mut cpu = Cpu::new();
    cpu.flags.set_carry(false);
    let program = &[
        Instruction::encode_ri(Opcode::Load, 0, 0x3E), // LOAD R0, <next>
        0x1234u16,
        Instruction::encode_ri(Opcode::Rol, 0, 4), // ROL  R0, 4
        Instruction::encode_ri(Opcode::Ror, 0, 4), // ROR  R0, 4
        Instruction::encode_ri(Opcode::Halt, 0, 0), // HALT
    ];
    cpu.load_program(to_bytes(program));
    cpu.run(100).unwrap();
    // The value should be restored if C was 0 throughout
    // (ROL 4 then ROR 4 is identity when carry stays 0 between operations)
    assert_eq!(cpu.regs[0], 0x1234);
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper used in tests above — convert u16 slice to byte slice
// (same pattern as your existing tests)
// ─────────────────────────────────────────────────────────────────────────────
fn to_bytes(words: &[u16]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(words.as_ptr() as *const u8, words.len() * 2) }
}
