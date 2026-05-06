# cpu16 — A 16-bit CPU Emulator in Rust

A fully custom 16-bit CPU emulator built from first principles in Rust. Includes a hand-designed
Instruction Set Architecture (ISA), a two-pass assembler with forward-reference resolution, a
5-stage in-order pipeline with RAW hazard detection and branch flush, a direct-mapped L1 cache
simulation, and a web-based step-through debugger.

[![CI](https://github.com/RajMandaliya/cpu16/actions/workflows/ci.yml/badge.svg)](https://github.com/RajMandaliya/cpu16/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/tests-49%20passing-brightgreen.svg)](tests/)

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

---

## 5-Stage Pipeline

```
  ┌─────┐     ┌─────┐     ┌─────┐     ┌─────┐     ┌─────┐
  │ IF  │────▶│ ID  │────▶│ EX  │────▶│ MEM │────▶│ WB  │
  │Fetch│     │Decode     │Execute    │Memory     │Write│
  └─────┘     └─────┘     └─────┘     └─────┘     └─────┘

  Hazard handling:
  • RAW data hazards   → stall (freeze IF/ID, insert NOP bubble into EX)
  • Flag hazards       → stall conditional branches until flags commit
  • Control hazards    → 2-cycle flush on taken branch (resolved in EX)

  Pipeline stats reported at HALT:
  • CPI (Cycles Per Instruction)
  • Efficiency %
  • Data stall cycles
  • Control flush cycles
```

---

## L1 Cache Simulation

```
  Direct-mapped, write-through, 16 lines

  Address mapping:
    line_index = (addr / 2) % 16
    tag        = (addr / 2) / 16

  Miss classification:
  • Cold miss     — line was empty (first access)
  • Conflict miss — valid line evicted by a different address mapping to same index

  Cache stats reported at HALT:
  • Hit rate %
  • Total reads / writes
  • Cold misses vs conflict misses
```

---

## Instruction Set (35 instructions)

| Category      | Instructions                                              | Notes                              |
|---------------|-----------------------------------------------------------|------------------------------------|
| Data move     | `LOAD`, `LOADM`, `STORE`, `MOV`                           | Immediate, register, and indirect  |
| Arithmetic    | `ADD`, `SUB`, `ADDI`, `MUL`, `DIV`, `MOD`, `NEG`         | Sets Z, C, N, V flags              |
| Logic         | `AND`, `OR`, `XOR`, `NOT`, `SHL`, `SHR`, `ROL`, `ROR`    | Bitwise; shifts/rotates set C flag |
| Compare       | `CMP`                                                     | Subtracts, sets flags, no write-back |
| Flow control  | `JMP`, `JZ`, `JNZ`, `JC`, `JN`, `CALL`, `RET`            | Conditional on flag state          |
| Stack         | `PUSH`, `POP`, `SWAP`                                     | SP grows downward from 0xFFFE      |
| Interrupts    | `INT`, `IRET`, `EI`, `DI`                                 | Software interrupts, IVT dispatch  |
| Misc          | `NOP`, `HALT`                                             |                                    |

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

## Web Debugger

A browser-based step-through debugger with an Axum HTTP backend and single-page UI.

```bash
cargo run -p debugger
# Open http://localhost:3000
```

**Features:**
- Assembly editor with 4 built-in example programs (fibonacci, factorial, countdown, flags demo)
- Step / Run / Reset controls with keyboard shortcuts: F5 (step) / F8 (run) / Escape (reset)
- Live register panel (R0–R3, PC, SP, FLAGS) with change highlighting on every step
- Pipeline stage display (IF → ID → EX → MEM → WB)
- L1 cache inspector: hit rate, cold/conflict miss breakdown, all 16 cache lines
- Memory hex dump with PC position highlighted in real time

**API endpoints:**

| Method | Endpoint      | Description                      |
|--------|---------------|----------------------------------|
| POST   | /api/load     | Assemble source and load program |
| POST   | /api/step     | Execute one instruction          |
| POST   | /api/run      | Run up to N cycles               |
| POST   | /api/reset    | Reset CPU, keep program loaded   |
| GET    | /api/state    | Full CPU state as JSON           |

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
│   ├── cache.rs          # Direct-mapped L1 cache with cold/conflict miss tracking
│   ├── pipeline.rs       # 5-stage pipeline with RAW/flag hazard detection
│   ├── assembler.rs      # Two-pass assembler: tokeniser + label resolution
│   └── assembler/
│       └── main.rs       # Assembler CLI
├── debugger/
│   ├── Cargo.toml        # Debugger crate (Axum + tower-http)
│   └── src/
│       └── main.rs       # HTTP server: /api/load, /step, /run, /reset, /state
│   └── static/
│       └── index.html    # Single-page debugger UI
├── examples/
│   ├── bubble_sort.asm   # Bubble sort — demonstrates cache conflict misses
│   ├── binary_search.asm # Binary search on a sorted array
│   ├── sieve.asm         # Sieve of Eratosthenes
│   ├── stack_calc.asm    # RPN stack calculator (dual-stack VM pattern)
│   ├── fibonacci.asm     # Fibonacci(10) = 55, iterative
│   └── factorial.asm     # Factorial(6) = 720, recursive via CALL/RET
├── tests/
│   └── integration_tests.rs  # 49 integration tests across all features
├── .github/workflows/
│   └── ci.yml            # fmt + clippy + test on every push
├── CHANGELOG.md
└── Cargo.toml            # Workspace: cpu16 + debugger
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
# 49 tests, 0 failures
```

### Assemble and run a program

```bash
# Compile .asm → .bin
cargo run --bin asm -- examples/factorial.asm factorial.bin

# Execute
cargo run --bin cpu16 -- factorial.bin

# Step-through debug mode (prints registers + flags after each instruction)
cargo run --bin cpu16 -- factorial.bin --debug

# Cap execution at N cycles (useful for catching infinite loops)
cargo run --bin cpu16 -- factorial.bin --max-cycles 1000
```

### Run the web debugger

```bash
cargo run -p debugger
# Open http://localhost:3000 in your browser
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
; Compute Fibonacci(10) — result in R0 = 55

        LOAD  R0, 0        ; a = 0
        LOAD  R1, 1        ; b = 1
        LOAD  R2, 9        ; counter

LOOP:
        MOV   R3, R1       ; tmp = b
        ADD   R1, R0       ; b = a + b
        MOV   R0, R3       ; a = tmp
        ADDI  R2, -1       ; counter--
        LOAD  R3, 0
        CMP   R2, R3
        JNZ   LOOP

        HALT               ; R0 = 55
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

### Flag effects per instruction

| Instruction | Z | C | N | V |
|-------------|---|---|---|---|
| `ADD`, `ADDI` | ✓ | ✓ | ✓ | ✓ |
| `SUB`, `CMP` | ✓ | ✓ | ✓ | ✓ |
| `MUL`, `DIV`, `MOD` | ✓ | — | ✓ | — |
| `NEG` | ✓ | ✓ | ✓ | ✓ |
| `AND`, `OR`, `XOR`, `NOT` | ✓ | — | ✓ | — |
| `SHL`, `SHR`, `ROL`, `ROR` | ✓ | ✓ | ✓ | — |
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
machine fits in a few hundred lines of Rust and can be reasoned about completely.

### Why only 4 general-purpose registers?

With a 16-bit instruction word, a 6-bit opcode leaves 10 bits for operands. Encoding two register
fields (dst, src) at 2 bits each uses 4 bits, leaving 6 bits for an immediate — enough for small
constants and offsets. Supporting 8 registers would require 3-bit register fields, leaving only
4 bits for the immediate, which is too narrow to be useful. 4 registers fits the encoding budget.

### Why stall instead of forwarding in the pipeline?

Forwarding passes results directly from EX output to EX input without waiting for WB — more
efficient but significantly more complex. Stalling is simpler, easier to verify correct, and
sufficient for cpu16's educational purpose. The stall penalty is visible in the CPI output,
making the cost of hazards measurable and concrete.

### Two-pass assembler

Pass 1 tokenises the source and records every label's address. Pass 2 emits encoded instructions,
substituting label addresses for forward references. This lets you write `JMP DONE` before
`DONE:` is defined — the classic approach used by most real assemblers.

### Little-endian memory

Consistent with the dominant convention in modern hardware (x86, ARM in LE mode). The low byte of
a 16-bit word lives at the lower address.

### Stack grows downward from 0xFFFE

Placing the stack at the top of the address space and growing it downward means the program and
stack never collide as long as the program doesn't grow into the top of memory. Code at the bottom,
stack at the top — the standard layout for flat address spaces.

### Interrupt Vector Table at 0x0000

The IVT occupies the lowest 32 bytes (16 two-byte entries). This mirrors real architectures (x86
real mode IVT, ARM exception table) where reset and interrupt vectors live at fixed low addresses.

---

## Releases

| Version | What's new |
|---------|------------|
| v0.6.0  | Web debugger — Axum backend + single-page UI with live register, pipeline, cache, and memory views |
| v0.5.0  | 5-stage pipeline simulation with RAW/flag hazard detection, branch flush, and CPI stats |
| v0.4.0  | L1 cache simulation — direct-mapped, write-through, cold/conflict miss classification |
| v0.3.0  | 5 new instructions: NEG, MOD, SWAP, ROL, ROR |
| v0.2.0  | 6 assembly example programs: bubble sort, binary search, sieve, RPN calculator, fibonacci, factorial |
| v0.1.0  | Initial release — ISA, assembler, fetch-decode-execute, interrupt handling, 18 tests |

---

## Performance

Measured on Apple M2, release build (`cargo build --release`):

| Program | Instructions | Cycles | Time |
|---------|-------------|--------|------|
| `fibonacci.asm` (n=10) | 63 | 63 | < 1 µs |
| `factorial.asm` (n=6) | 45 | 45 | < 1 µs |
| Tight loop (1M iterations) | 3,000,001 | 3,000,001 | ~12 ms |

The emulator executes approximately **250M simulated instructions/second** in release mode.

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
