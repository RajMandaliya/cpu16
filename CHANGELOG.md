# Changelog

All notable changes to cpu16 are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Planned
- Phase 2: additional example programs (sort, search, sieve, stack calculator)
- Phase 3: pipeline simulation, cache model, extended instruction set
- Phase 4: web-based step-through debugger UI

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

[Unreleased]: https://github.com/RajMandaliya/cpu16/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/RajMandaliya/cpu16/releases/tag/v0.1.0