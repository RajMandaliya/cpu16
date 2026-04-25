# Changelog

All notable changes to cpu16 are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Planned
- Phase 3: pipeline simulation, cache model, extended instruction set
- Phase 4: web-based step-through debugger UI

---

## [0.2.0] — 2025-04-25

### Added

**`examples/bubble_sort.asm`**
- Sorts an 8-element integer array in-place using bubble sort
- Input `[8, 3, 7, 1, 6, 2, 5, 4]` stored at memory address `0x0300`
- Demonstrates nested loops, `LOADM`/`STORE` for indirect memory access, and in-place swap
- Expected output at `0x0300` after `HALT`: `[1, 2, 3, 4, 5, 6, 7, 8]`
- Worst-case cycle count: ~450 cycles (reverse-sorted input)

**`examples/binary_search.asm`**
- Binary search over a sorted 8-element array at `0x0300`
- Returns zero-based index in `R0`, or `0xFFFF` if target not found
- Demonstrates `CALL`/`RET` subroutine pattern and divide-and-conquer logic in assembly
- Search for `14` in `[2, 5, 8, 11, 14, 17, 20, 23]` → `R0 = 4`
- Maximum cycle count: ~24 cycles (3 bisections)

**`examples/sieve.asm`**
- Sieve of Eratosthenes finding all prime numbers up to 30
- Boolean sieve array stored at `0x0300`–`0x033C` (`sieve[i] = 1` means i is prime)
- Demonstrates stride-based memory writes, `MUL` for computing `p*p`, nested loops
- Outer loop bound is `sqrt(30) ≈ 5` — any composite n has a prime factor ≤ `sqrt(n)`
- Primes found: 2, 3, 5, 7, 11, 13, 17, 19, 23, 29
- Cycle count: ~620 cycles

**`examples/stack_calc.asm`**
- RPN (Reverse Polish Notation) calculator using a software operand stack
- Evaluates `3 4 + 2 * 7 -` → result `R0 = 7` after `HALT`
- Software operand stack lives at `0x0400`, separate from CPU hardware stack at `0xFFFE`
- Subroutines: `PUSH_OP`, `POP_OP`, `OP_ADD`, `OP_SUB`, `OP_MUL`, `OP_DIV`
- Mirrors how real stack machines work (JVM, CPython bytecode, Forth)
- Cycle count: ~180 cycles

---

## [0.1.0] — 2025-04-01

### Added

**ISA and instruction encoding (`src/isa.rs`)**
- Defined 30-instruction ISA across 8 categories: data move, arithmetic, logic, compare,
  flow control, stack, interrupts, misc
- 16-bit fixed-width instruction format: 6-bit opcode, 2-bit dst, 2-bit src, 6-bit immediate
- 2-word format for jump/call instructions (opcode word + 16-bit address word)
- Full opcode table with encoding and decoding functions

**FLAGS register (`src/flags.rs`)**
- 5-bit FLAGS register: Zero (Z), Carry (C), Negative (N), Overflow (V), Interrupt Enable (IE)
- Per-instruction flag update logic: arithmetic sets all four condition flags; logic clears V
- IE flag controlled exclusively by `EI` / `DI` instructions

**Memory subsystem (`src/memory.rs`)**
- 64 KB flat byte-addressable memory
- Little-endian 16-bit word reads and writes
- Memory map: IVT at `0x0000`–`0x001F`, PROG_BASE at `0x0200`, stack base at `0xFFFE`
- Hex dump utility for debugging memory regions

**Fetch–decode–execute core (`src/cpu.rs`)**
- Single-cycle fetch–decode–execute loop
- PC advances by 2 bytes per standard instruction, 4 bytes for jump/call instructions
- Stack operations: `PUSH` decrements SP then writes; `POP` reads then increments SP
- Subroutine support: `CALL` pushes return address and jumps; `RET` pops and restores PC
- Interrupt dispatch: between cycles, if `IE=1` and interrupt pending, push PC and load IVT entry
- `HALT` stops execution and reports final register state
- `--debug` flag: prints full register and flag state after every instruction
- `--max-cycles` flag: enforces a cycle budget (default: unlimited)

**Two-pass assembler (`src/assembler.rs`)**
- Pass 1: tokenise source, collect label → address mappings
- Pass 2: emit encoded instruction words, resolve forward references
- Supports decimal and `0x`-prefixed hexadecimal immediates
- `DW` directive for emitting raw 16-bit data words
- Comments with `;`, optional comma between operands
- Assembler CLI binary (`cargo run --bin asm`)

**Example programs (`examples/`)**
- `fibonacci.asm`: iterative Fibonacci(10), result 55 in R1
- `factorial.asm`: Factorial(6) = 720 using `CALL`/`RET` subroutine pattern
- `interrupt_demo.asm`: installs interrupt handler via IVT, triggers `INT 0`, verifies memory write

**Test suite (`tests/integration_tests.rs`)**
- 18 integration tests covering every instruction category
- Tests verify register state, flag state, and memory state post-execution
- All tests run in CI on every push

**CI (`/.github/workflows/ci.yml`)**
- `cargo fmt --check` — enforces consistent formatting
- `cargo clippy -- -D warnings` — zero-warning policy
- `cargo test` — all 18 tests must pass
- Runs on `ubuntu-latest`, Rust stable

---

## [0.0.1] — 2025-03-15 (internal prototype)

- Initial proof-of-concept: bare fetch–execute loop with 8 instructions
- No assembler — programs hand-assembled as hex arrays in test code
- No flags register — branching not yet implemented
- Not tagged or released publicly

---

[Unreleased]: https://github.com/RajMandaliya/cpu16/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/RajMandaliya/cpu16/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/RajMandaliya/cpu16/releases/tag/v0.1.0