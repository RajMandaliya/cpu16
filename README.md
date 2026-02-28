# cpu16 — A 16-bit CPU Emulator in Rust

A fully custom 16-bit CPU emulator written from scratch in Rust — including a complete Instruction Set Architecture (ISA), a two-pass assembler, and a CLI emulator.

![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/rust-async--first-orange.svg)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                    cpu16 Architecture                │
├──────────────────────────────────────────────────────┤
│  Registers                                           │
│  ┌────┐ ┌────┐ ┌────┐ ┌────┐  ┌────┐  ┌────┐       │
│  │ R0 │ │ R1 │ │ R2 │ │ R3 │  │ PC │  │ SP │       │
│  └────┘ └────┘ └────┘ └────┘  └────┘  └────┘       │
│                                                      │
│  FLAGS: [Z] Zero  [C] Carry  [N] Negative            │
│         [V] Overflow  [IE] Interrupt Enable          │
├──────────────────────────────────────────────────────┤
│  Memory (64 KB, byte-addressable, little-endian)     │
│  0x0000 ── Interrupt Vector Table (IVT, 32 bytes)    │
│  0x0020 ── Reserved                                  │
│  0x0200 ── Program load address (PROG_BASE)          │
│  0xFFFE ── Stack base (grows downward)               │
├──────────────────────────────────────────────────────┤
│  Instruction Format (16 bits)                        │
│  ┌────────────┬──────┬──────┬────────────┐           │
│  │  6-bit op  │ 2-bit│ 2-bit│  6-bit imm │           │
│  │  (opcode)  │  dst │  src │  /offset   │           │
│  └────────────┴──────┴──────┴────────────┘           │
│  Jump/Call: 1st word = opcode, 2nd word = address    │
└─────────────────────────────────────────────────────-┘
```

---

## Instruction Set

| Category     | Instructions                                               |
|--------------|------------------------------------------------------------|
| Data Move    | `LOAD`, `LOADM`, `STORE`, `MOV`                            |
| Arithmetic   | `ADD`, `SUB`, `ADDI`, `MUL`, `DIV`                         |
| Logic        | `AND`, `OR`, `XOR`, `NOT`, `SHL`, `SHR`                    |
| Compare      | `CMP`                                                      |
| Flow Control | `JMP`, `JZ`, `JNZ`, `JC`, `JN`, `CALL`, `RET`             |
| Stack        | `PUSH`, `POP`                                              |
| Interrupts   | `INT`, `IRET`, `EI`, `DI`                                  |
| Misc         | `NOP`, `HALT`                                              |

---

## Project Structure

```
cpu16/
├── src/
│   ├── lib.rs            # Module declarations
│   ├── main.rs           # CPU runner CLI
│   ├── isa.rs            # Instruction set, opcodes, encoding/decoding
│   ├── flags.rs          # FLAGS register (Z, C, N, V, IE)
│   ├── memory.rs         # 64 KB flat memory with hex dump
│   ├── cpu.rs            # Fetch–Decode–Execute core, interrupt handling
│   ├── assembler.rs      # Two-pass assembler (tokeniser, label resolution)
│   └── assembler/
│       └── main.rs       # Assembler CLI
├── examples/
│   ├── fibonacci.asm     # Fibonacci(10) — iterative
│   ├── factorial.asm     # Factorial(6) with CALL/RET subroutine
│   └── interrupt_demo.asm
├── tests/
│   └── integration_tests.rs  # 18 tests covering all instruction categories
├── .github/workflows/
│   └── ci.yml            # GitHub Actions: fmt + clippy + test
└── Cargo.toml
```

---

## Getting Started

### Prerequisites

- Rust 1.75+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)

### Build

```bash
git clone https://github.com/RajMandaliya/cpu16
cd cpu16
cargo build --release
```

### Run Tests

```bash
cargo test
```

Expected output: 18 tests, all passing.

### Assemble a Program

```bash
# Assemble an .asm file into a binary
cargo run --bin asm -- examples/factorial.asm factorial.bin
```

### Run a Program

```bash
# Run a binary
cargo run --bin cpu16 -- factorial.bin

# Step-through debug mode
cargo run --bin cpu16 -- factorial.bin --debug

# Limit execution cycles
cargo run --bin cpu16 -- factorial.bin --max-cycles 500
```

---

## Example Programs

### Fibonacci

```asm
; Compute Fibonacci(10) = 55

        LOAD  R0, 0        ; a = 0
        LOAD  R1, 1        ; b = 1
        LOAD  R2, 10       ; counter

LOOP:
        LOAD  R3, 0
        CMP   R2, R3
        JZ    DONE
        MOV   R3, R1
        ADD   R1, R0       ; b = a + b
        MOV   R0, R3       ; a = old b
        ADDI  R2, -1
        JMP   LOOP
DONE:
        HALT               ; R1 = 55
```

### Factorial (with subroutine)

```asm
        LOAD  R0, 6
        CALL  FACTORIAL
        HALT               ; R1 = 720

FACTORIAL:
        LOAD  R1, 1
        LOAD  R2, 0
LOOP:   CMP   R0, R2
        JZ    DONE
        MUL   R1, R0
        ADDI  R0, -1
        JMP   LOOP
DONE:   RET
```

---

## Assembly Language Reference

### Syntax

```asm
; This is a comment
LABEL:          ; label definition (used as jump target)
    MNEMONIC Op1, Op2   ; instruction (comma optional)
```

### Registers: `R0`, `R1`, `R2`, `R3`

### Addressing Modes

| Mode            | Syntax         | Example         |
|-----------------|----------------|-----------------|
| Immediate       | `Rd, imm`      | `LOAD R0, 42`   |
| Register        | `Rd, Rs`       | `ADD R0, R1`    |
| Indirect (mem)  | `Rd, Rs`       | `LOADM R0, R1`  |
| Label (address) | `LABEL`        | `JMP LOOP`      |

### Data Directives

```asm
MY_DATA: DW 0x1234 0xABCD 42   ; emit raw 16-bit words
```

---

## Design Decisions

**Why a custom ISA instead of RISC-V/x86?**  
A custom ISA demonstrates first-principles understanding of CPU design rather than mimicking existing work.

**Why 16-bit?**  
Rich enough to be interesting (flags, stack, interrupts) but simple enough to implement completely in a few hundred lines.

**Two-pass assembler**  
Pass 1 collects all label addresses; Pass 2 emits instructions. This resolves forward references (e.g., `JMP DONE` before `DONE:` is defined).

**Little-endian memory**  
Aligns with modern convention and simplifies byte/word addressing.

---

## License

MIT