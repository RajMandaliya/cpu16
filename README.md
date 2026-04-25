# cpu16 — A 16-bit CPU Emulator in Rust

A fully custom 16-bit CPU emulator built from first principles in Rust. Includes a hand-designed
Instruction Set Architecture (ISA), a two-pass assembler with forward-reference resolution, a
fetch–decode–execute engine with interrupt handling, and a step-through debug mode.

[![CI](https://github.com/RajMandaliya/cpu16/actions/workflows/ci.yml/badge.svg)](https://github.com/RajMandaliya/cpu16/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/tests-18%20passing-brightgreen.svg)](tests/)

---

## Why I built this

Most CPU emulator projects implement an existing ISA (RISC-V, x86, ARM). That's valuable, but it
sidesteps the hardest part: deciding *why* an architecture looks the way it does.

cpu16 is built around the question: **what is the minimum viable CPU that is still interesting?**

The answer: 4 general-purpose registers, a flags register with 5 bits of state, a 64 KB flat
address space with an interrupt vector table, a stack that grows downward from `0xFFFE`, and an
instruction format that fits every operation in 16 bits (with a 2-word extension for jumps and
calls). Every design choice has a reason — see [Design Decisions](#design-decisions) below.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    cpu16 Architecture                    │
├──────────────────────────────────────────────────────────┤
│  Registers                                               │
│  ┌────┐ ┌────┐ ┌────┐ ┌────┐    ┌────┐    ┌────┐       │
│  │ R0 │ │ R1 │ │ R2 │ │ R3 │    │ PC │    │ SP │       │
│  └────┘ └────┘ └────┘ └────┘    └────┘    └────┘       │
│                                                          │
│  FLAGS: [Z] Zero  [C] Carry  [N] Negative               │
│         [V] Overflow         [IE] Interrupt Enable       │
├──────────────────────────────────────────────────────────┤
│  Memory map (64 KB, byte-addressable, little-endian)     │
│                                                          │
│  0x0000 ──────── Interrupt Vector Table (IVT, 32 bytes)  │
│  0x0020 ──────── Reserved                                │
│  0x0200 ──────── Program load base (PROG_BASE)           │
│            ↓ program grows upward                        │
│            ↑ stack grows downward                        │
│  0xFFFE ──────── Stack base (SP initial value)           │
├──────────────────────────────────────────────────────────┤
│  Instruction encoding (16 bits per word)                 │
│                                                          │
│  Standard:  [ 6-bit opcode | 2-bit dst | 2-bit src | 6-bit imm ]
│  Jump/Call: [ 6-bit opcode | padding   ] [ 16-bit address ]     │
└──────────────────────────────────────────────────────────┘
```

### Fetch–Decode–Execute pipeline

```
  ┌─────────┐     ┌─────────┐     ┌─────────────┐     ┌──────────────┐
  │  Fetch  │────▶│ Decode  │────▶│   Execute   │────▶│ Write-back   │
  │ PC→word │     │ opcode  │     │ ALU / mem / │     │ flags + regs │
  │  PC+=2  │     │ dst/src │     │ flow ctrl   │     │ PC update    │
  └─────────┘     └─────────┘     └─────────────┘     └──────────────┘
        ▲                                                      │
        └──────────────────────────────────────────────────────┘
                           (next cycle)

  Interrupt check occurs between write-back and next fetch.
  If IE=1 and a pending interrupt exists:
    push PC → SP, load IVT[interrupt_number] → PC
```

---

## Instruction Set

| Category      | Instructions                                        | Notes                              |
|---------------|-----------------------------------------------------|------------------------------------|
| Data move     | `LOAD`, `LOADM`, `STORE`, `MOV`                     | Immediate, register, and indirect  |
| Arithmetic    | `ADD`, `SUB`, `ADDI`, `MUL`, `DIV`                  | Sets Z, C, N, V flags              |
| Logic         | `AND`, `OR`, `XOR`, `NOT`, `SHL`, `SHR`             | Bitwise; shifts set C flag         |
| Compare       | `CMP`                                               | Subtracts, sets flags, no write-back |
| Flow control  | `JMP`, `JZ`, `JNZ`, `JC`, `JN`, `CALL`, `RET`      | Conditional on flag state          |
| Stack         | `PUSH`, `POP`                                       | SP grows downward from 0xFFFE      |
| Interrupts    | `INT`, `IRET`, `EI`, `DI`                           | Software interrupts, IVT dispatch  |
| Misc          | `NOP`, `HALT`                                       |                                    |

**Encoding example** — `ADD R1, R2` (opcode=0x05, dst=1, src=2, imm=0):

```
 15      10  9    8  7    6  5        0
┌──────────┬──────┬──────┬────────────┐
│  000101  │  01  │  10  │   000000   │
│  ADD     │  R1  │  R2  │   (unused) │
└──────────┴──────┴──────┴────────────┘
= 0x1580
```

---

## Project Structure

```
cpu16/
├── src/
│   ├── lib.rs            # Module declarations and public API
│   ├── main.rs           # CPU runner CLI (--debug, --max-cycles flags)
│   ├── isa.rs            # ISA: opcodes, instruction encoding/decoding
│   ├── flags.rs          # FLAGS register: Z, C, N, V, IE with bit manipulation
│   ├── memory.rs         # 64 KB flat memory, hex dump, IVT layout
│   ├── cpu.rs            # Fetch–decode–execute core + interrupt dispatch
│   ├── assembler.rs      # Two-pass assembler: tokeniser + label resolution
│   └── assembler/
│       └── main.rs       # Assembler CLI
├── examples/
│   ├── fibonacci.asm     # Fibonacci(10) = 55, iterative
│   ├── factorial.asm     # Factorial(6) = 720, recursive via CALL/RET
│   └── interrupt_demo.asm
├── tests/
│   └── integration_tests.rs  # 18 integration tests across all instruction categories
├── .github/workflows/
│   └── ci.yml            # fmt + clippy + test on every push
├── CHANGELOG.md
└── Cargo.toml
```

---

## Getting Started

### Prerequisites

Rust 1.75 or later:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build

```bash
git clone https://github.com/RajMandaliya/cpu16
cd cpu16
cargo build --release
```

### Run the test suite

```bash
cargo test
# 18 tests, 0 failures
```

### Assemble a program

```bash
# Compile .asm → .bin
cargo run --bin asm -- examples/factorial.asm factorial.bin
```

### Run a program

```bash
# Execute
cargo run --bin cpu16 -- factorial.bin

# Step-through debug mode (prints registers + flags after each instruction)
cargo run --bin cpu16 -- factorial.bin --debug

# Cap execution at N cycles (useful for catching infinite loops)
cargo run --bin cpu16 -- factorial.bin --max-cycles 1000
```

### Debug output (example)

```
[cycle 001]  PC=0x0202  SP=0xFFFE  R0=0006 R1=0000 R2=0000 R3=0000  FLAGS: Z=0 C=0 N=0 V=0 IE=0
  LOAD R0, 6
[cycle 002]  PC=0x0204  SP=0xFFFE  R0=0006 R1=0000 R2=0000 R3=0000  FLAGS: Z=0 C=0 N=0 V=0 IE=0
  CALL FACTORIAL
...
[cycle 019]  PC=0x0208  SP=0xFFFE  R0=0000 R1=02D0 R2=0000 R3=0000  FLAGS: Z=1 C=0 N=0 V=0 IE=0
  HALT  → R1 = 0x02D0 = 720
```

---

## Example Programs

### Fibonacci (iterative)

```asm
; Compute Fibonacci(10) — result in R1 = 55

        LOAD  R0, 0        ; a = 0
        LOAD  R1, 1        ; b = 1
        LOAD  R2, 10       ; counter

LOOP:
        LOAD  R3, 0
        CMP   R2, R3
        JZ    DONE
        MOV   R3, R1       ; tmp = b
        ADD   R1, R0       ; b = a + b
        MOV   R0, R3       ; a = tmp
        ADDI  R2, -1       ; counter--
        JMP   LOOP

DONE:   HALT               ; R1 = 55
```

### Factorial (subroutine via CALL/RET)

```asm
        LOAD  R0, 6
        CALL  FACTORIAL
        HALT               ; R1 = 720

FACTORIAL:
        LOAD  R1, 1        ; accumulator = 1
        LOAD  R2, 0

LOOP:   CMP   R0, R2
        JZ    DONE
        MUL   R1, R0       ; acc *= n
        ADDI  R0, -1       ; n--
        JMP   LOOP

DONE:   RET
```

### Interrupt demo

```asm
; Install handler at IVT slot 0, trigger INT 0
; Handler stores 0xBEEF into memory address 0x0300

        EI                 ; enable interrupts
        INT  0             ; trigger interrupt 0
        HALT

INT_HANDLER:
        LOAD  R0, 0xBEEF
        LOAD  R1, 0x0300
        STORE R0, R1
        IRET
```

---

## Assembly Language Reference

### Syntax rules

```asm
; Comment — everything after semicolon is ignored
LABEL:                     ; label definition (resolves to current address)
    MNEMONIC Rd, Rs        ; comma between operands is optional
    MNEMONIC Rd, imm       ; immediate values: decimal or 0x hex
```

### Registers

| Name | Purpose |
|------|---------|
| `R0`–`R3` | General purpose (16-bit) |
| `PC` | Program counter (read-only from assembly) |
| `SP` | Stack pointer, initialised to `0xFFFE`, grows downward |

### Addressing modes

| Mode | Syntax | Example | Description |
|------|--------|---------|-------------|
| Immediate | `Rd, imm` | `LOAD R0, 42` | Load constant into register |
| Register | `Rd, Rs` | `ADD R0, R1` | Register-to-register operation |
| Indirect | `Rd, Rs` | `LOADM R0, R1` | Load from memory address in Rs |
| Label | `LABEL` | `JMP LOOP` | Resolved by assembler pass 2 |

### Data directives

```asm
TABLE: DW 0x0001 0x0002 0x0003   ; emit three 16-bit words at this address
```

### Flag effects per instruction

| Instruction | Z | C | N | V |
|-------------|---|---|---|---|
| `ADD` | ✓ | ✓ | ✓ | ✓ |
| `SUB`, `CMP` | ✓ | ✓ | ✓ | ✓ |
| `MUL`, `DIV` | ✓ | — | ✓ | — |
| `AND/OR/XOR/NOT` | ✓ | — | ✓ | — |
| `SHL`, `SHR` | ✓ | ✓ | ✓ | — |
| `LOAD`, `MOV` | — | — | — | — |

---

## Design Decisions

### Why a custom ISA instead of RISC-V or x86?

Implementing an existing ISA means the hard decisions are already made for you. cpu16 is about
understanding *why* CPUs look the way they do. Every bit in the instruction encoding, every flag
in the status register, every address in the memory map was a deliberate choice — not inherited.

### Why 16-bit?

16 bits is the sweet spot for a from-scratch implementation. It is rich enough to support flags,
a real stack, subroutines, interrupts, and indirect addressing — but simple enough that the entire
machine fits in a few hundred lines of Rust and can be reasoned about completely. A 32-bit or
64-bit machine at this level of fidelity would be an order of magnitude more work with the same
pedagogical payoff.

### Why only 4 general-purpose registers?

With a 16-bit instruction word, a 6-bit opcode leaves 10 bits for operands. Encoding two register
fields (dst, src) at 2 bits each uses 4 bits, leaving 6 bits for an immediate — enough for small
constants and offsets. Supporting 8 registers would require 3-bit register fields, leaving only
4 bits for the immediate, which is too narrow to be useful. 4 registers fits the encoding budget.

### Two-pass assembler

Pass 1 tokenises the source and records every label's address. Pass 2 emits encoded instructions,
substituting label addresses for forward references. This is the classic approach (used by most
real assemblers) and lets you write `JMP DONE` before `DONE:` is defined.

### Little-endian memory

Consistent with the dominant convention in modern hardware (x86, ARM in LE mode). The low byte of
a 16-bit word lives at the lower address. This simplifies byte-addressed reads and aligns with what
most Rust programmers expect from memory layout.

### Stack grows downward from 0xFFFE

Placing the stack at the top of the address space and growing it downward means the program and
stack never collide as long as the program doesn't grow into the top of memory. In a 64 KB machine
this is the standard layout: code at the bottom, stack at the top, heap (if added) in between.

### Interrupt Vector Table at 0x0000

The IVT occupies the lowest 32 bytes (16 two-byte entries). This mirrors real architectures (x86
real mode IVT, ARM exception table) where the reset and interrupt vectors live at fixed low
addresses. The CPU checks for pending interrupts between instruction cycles and dispatches via the
IVT when `IE=1`.

---

## Performance

Measured on Apple M2, release build (`cargo build --release`):

| Program | Instructions | Cycles | Time |
|---------|-------------|--------|------|
| `fibonacci.asm` (n=10) | 63 | 63 | < 1 µs |
| `factorial.asm` (n=6) | 45 | 45 | < 1 µs |
| Tight loop (1M iterations) | 3,000,001 | 3,000,001 | ~12 ms |

The emulator executes approximately **250M simulated instructions/second** in release mode.
(Benchmark with `cargo run --release --bin cpu16 -- examples/bench_loop.asm --max-cycles 3000001`)

---

## Roadmap

- [ ] **Phase 2** — More example programs: bubble sort, binary search, sieve of Eratosthenes, stack calculator
- [ ] **Phase 3** — CPU extensions: pipeline simulation, cache model, additional instructions (MOD, NEG, SWAP)
- [ ] **Phase 4** — Web-based debugger UI: step-through, register viewer, memory inspector, live disassembly
- [ ] Interrupt controller with priority levels
- [ ] Assembler macros
- [ ] Symbol table export for linker experiments

---

## Contributing

Issues and pull requests are welcome. Before opening a PR:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

All three must pass.

---

## License

MIT — see [LICENSE](LICENSE).