use cpu16::assembler::Assembler;
use cpu16::cpu::{Cpu, CpuState, PROG_BASE};
use cpu16::isa::Opcode;

/// Assemble a source string and load it into a fresh CPU.
fn asm_and_load(src: &str) -> Cpu {
    let assembler = Assembler::new(PROG_BASE);
    let output = assembler.assemble(src).expect("Assembly failed");
    let bytes: Vec<u8> = output.words.iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    let mut cpu = Cpu::new();
    cpu.load_program(&bytes);
    cpu
}

// ── Arithmetic ────────────────────────────────────────────────────────────────

#[test]
fn test_load_and_add() {
    let mut cpu = asm_and_load("
        LOAD R0, 5
        LOAD R1, 3
        ADD  R0, R1
        HALT
    ");
    assert_eq!(cpu.run(100).unwrap(), CpuState::Halted);
    assert_eq!(cpu.regs[0], 8);
}

#[test]
fn test_subtract() {
    let mut cpu = asm_and_load("
        LOAD R0, 10
        LOAD R1, 4
        SUB  R0, R1
        HALT
    ");
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 6);
}

#[test]
fn test_multiply() {
    let mut cpu = asm_and_load("
        LOAD R0, 7
        LOAD R1, 6
        MUL  R0, R1
        HALT
    ");
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 42);
}

#[test]
fn test_divide() {
    let mut cpu = asm_and_load("
        LOAD R0, 20
        LOAD R1, 4
        DIV  R0, R1
        HALT
    ");
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 5);
}

#[test]
fn test_addi_positive() {
    let mut cpu = asm_and_load("
        LOAD R0, 10
        ADDI R0, 5
        HALT
    ");
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 15);
}

#[test]
fn test_addi_negative() {
    let mut cpu = asm_and_load("
        LOAD R0, 10
        ADDI R0, -3
        HALT
    ");
    cpu.run(100).unwrap();
    // ADDI sign-extends the 6-bit immediate: -3 in two's complement 6-bit = 0b111101
    // The assembler stores -3 as u16 (0xFFFD), masked to 6 bits = 0x3D = 61
    // sign-extended from 6 bits: bit5=1 → negative → 61 - 64 = -3 → R0 = 10 + (-3) = 7
    assert_eq!(cpu.regs[0], 7);
}

// ── Flags ─────────────────────────────────────────────────────────────────────

#[test]
fn test_zero_flag() {
    let mut cpu = asm_and_load("
        LOAD R0, 5
        LOAD R1, 5
        SUB  R0, R1
        HALT
    ");
    cpu.run(100).unwrap();
    assert!(cpu.flags.zero(), "Zero flag should be set after 5-5");
    assert_eq!(cpu.regs[0], 0);
}

#[test]
fn test_negative_flag() {
    let mut cpu = asm_and_load("
        LOAD R0, 3
        LOAD R1, 5
        SUB  R0, R1
        HALT
    ");
    cpu.run(100).unwrap();
    assert!(cpu.flags.negative(), "Negative flag should be set after 3-5");
}

#[test]
fn test_carry_flag() {
    let mut cpu = asm_and_load("
        LOAD R0, 63     ; 0x003F
        LOAD R1, 63
        ADD  R0, R1
        HALT
    ");
    cpu.run(100).unwrap();
    // 63+63 = 126, no carry expected
    assert!(!cpu.flags.carry());
    assert_eq!(cpu.regs[0], 126);
}

// ── Logic ─────────────────────────────────────────────────────────────────────

#[test]
fn test_and() {
    let mut cpu = asm_and_load("
        LOAD R0, 15
        LOAD R1, 6
        AND  R0, R1
        HALT
    ");
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 15 & 6);
}

#[test]
fn test_or() {
    let mut cpu = asm_and_load("
        LOAD R0, 12
        LOAD R1, 3
        OR   R0, R1
        HALT
    ");
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 15);
}

#[test]
fn test_xor_self_is_zero() {
    let mut cpu = asm_and_load("
        LOAD R0, 42
        XOR  R0, R0
        HALT
    ");
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0);
    assert!(cpu.flags.zero());
}

#[test]
fn test_not() {
    let mut cpu = asm_and_load("
        LOAD R0, 0
        NOT  R0
        HALT
    ");
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0xFFFF);
}

// ── Control flow ──────────────────────────────────────────────────────────────

#[test]
fn test_jmp_unconditional() {
    let mut cpu = asm_and_load("
        JMP  SKIP
        LOAD R0, 99       ; should be skipped
SKIP:
        LOAD R1, 42
        HALT
    ");
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[0], 0,  "R0 should not have been loaded");
    assert_eq!(cpu.regs[1], 42, "R1 should be 42");
}

#[test]
fn test_jz_taken() {
    let mut cpu = asm_and_load("
        LOAD R0, 0
        LOAD R1, 0
        CMP  R0, R1
        JZ   SET_R2
        LOAD R2, 1
        HALT
SET_R2:
        LOAD R2, 42
        HALT
    ");
    cpu.run(200).unwrap();
    assert_eq!(cpu.regs[2], 42, "JZ should be taken");
}

#[test]
fn test_jnz_not_taken_when_zero() {
    let mut cpu = asm_and_load("
        LOAD R0, 5
        LOAD R1, 5
        CMP  R0, R1
        JNZ  SKIP
        LOAD R2, 7
        HALT
SKIP:
        LOAD R2, 0
        HALT
    ");
    cpu.run(200).unwrap();
    assert_eq!(cpu.regs[2], 7, "JNZ should NOT be taken when Zero flag is set");
}

// ── Stack & subroutines ───────────────────────────────────────────────────────

#[test]
fn test_push_pop() {
    let mut cpu = asm_and_load("
        LOAD R0, 55
        PUSH R0
        LOAD R0, 0
        POP  R1
        HALT
    ");
    cpu.run(100).unwrap();
    assert_eq!(cpu.regs[1], 55, "POP should restore pushed value");
    assert_eq!(cpu.regs[0], 0,  "R0 should have been zeroed");
}

#[test]
fn test_call_ret() {
    let mut cpu = asm_and_load("
        LOAD R0, 0
        CALL ADD_ONE
        HALT

ADD_ONE:
        ADDI R0, 1
        RET
    ");
    cpu.run(200).unwrap();
    assert_eq!(cpu.regs[0], 1, "Subroutine should have incremented R0");
}

#[test]
fn test_nested_calls() {
    let mut cpu = asm_and_load("
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
    ");
    cpu.run(300).unwrap();
    assert_eq!(cpu.regs[0], 21, "Nested calls: 0+10+1+10 = 21");
}

// ── Memory ────────────────────────────────────────────────────────────────────

#[test]
fn test_store_loadm() {
    let mut cpu = asm_and_load("
        LOAD  R0, 0x0400   ; address
        LOAD  R1, 0xBEEF   ; value
        STORE R0, R1       ; mem[0x0400] = 0xBEEF
        LOAD  R2, 0
        LOADM R2, R0       ; R2 = mem[0x0400]
        HALT
    ");
    cpu.run(200).unwrap();
    assert_eq!(cpu.regs[2], 0xBEEF, "Loaded value should match stored value");
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
    let iret      = (Opcode::Iret as u16) << 10;
    cpu.mem.write_word(0x0300, addi_r3_1);
    cpu.mem.write_word(0x0302, iret);

    // Program at PROG_BASE: INT 0, INT 0, HALT
    let int0  = ((Opcode::Int  as u16) << 10) | 0;
    let halt  = (Opcode::Halt as u16) << 10;
    cpu.mem.write_word(PROG_BASE,       int0);
    cpu.mem.write_word(PROG_BASE + 2,   int0);
    cpu.mem.write_word(PROG_BASE + 4,   halt);

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