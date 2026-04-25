use crate::flags::Flags;
use crate::isa::{Instruction, Opcode};
use crate::memory::Memory;

/// Interrupt vector table starts at address 0x0000.
/// Each entry is a 16-bit address (2 bytes). Up to 16 vectors (0x00–0x1E).
const IVT_BASE: u16 = 0x0000;
/// Default stack base (grows downward from 0xFFFE).
pub const STACK_BASE: u16 = 0xFFFE;
/// Program entry point (programs loaded at this address).
pub const PROG_BASE: u16 = 0x0200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuState {
    Running,
    Halted,
    /// Waiting for an interrupt
    WaitingForInterrupt,
}

pub struct Cpu {
    /// General-purpose registers R0–R3
    pub regs: [u16; 4],
    /// Program counter
    pub pc: u16,
    /// Stack pointer
    pub sp: u16,
    /// Flags
    pub flags: Flags,
    /// Main memory
    pub mem: Memory,
    /// Pending interrupt number (None = no interrupt)
    pub pending_irq: Option<u8>,
    /// Execution state
    pub state: CpuState,
    /// Cycle counter
    pub cycles: u64,
}

impl Cpu {
    pub fn new() -> Self {
        let mut cpu = Self {
            regs: [0; 4],
            pc: PROG_BASE,
            sp: STACK_BASE,
            flags: Flags::default(),
            mem: Memory::new(),
            pending_irq: None,
            state: CpuState::Running,
            cycles: 0,
        };
        // Enable interrupts by default
        cpu.flags.set_int_enable(true);
        cpu
    }

    /// Load a program binary at PROG_BASE.
    pub fn load_program(&mut self, bytes: &[u8]) {
        self.mem.load(PROG_BASE, bytes);
        self.pc = PROG_BASE;
    }

    /// Load a program at an arbitrary address.
    pub fn load_at(&mut self, addr: u16, bytes: &[u8]) {
        self.mem.load(addr, bytes);
    }

    /// Raise a hardware interrupt (IRQ).
    pub fn raise_irq(&mut self, irq: u8) {
        self.pending_irq = Some(irq);
    }

    // ── Fetch ────────────────────────────────────────────────────────────────

    fn fetch_word(&mut self) -> u16 {
        let w = self.mem.read_word(self.pc);
        self.pc = self.pc.wrapping_add(2);
        w
    }

    // ── Stack helpers ────────────────────────────────────────────────────────

    fn push(&mut self, val: u16) {
        self.sp = self.sp.wrapping_sub(2);
        self.mem.write_word(self.sp, val);
    }

    fn pop(&mut self) -> u16 {
        let val = self.mem.read_word(self.sp);
        self.sp = self.sp.wrapping_add(2);
        val
    }

    // ── Interrupt dispatch ───────────────────────────────────────────────────

    fn handle_interrupt(&mut self, irq: u8) {
        // Disable interrupts, save PC and FLAGS, jump to vector
        self.flags.set_int_enable(false);
        let saved_pc = self.pc;
        let saved_flags = self.flags.0 as u16;
        self.push(saved_flags);
        self.push(saved_pc);
        let vector_addr = IVT_BASE.wrapping_add((irq as u16) * 2);
        self.pc = self.mem.read_word(vector_addr);
    }

    // ── Single-step execution ────────────────────────────────────────────────

    /// Execute one instruction. Returns `Ok(CpuState)` or an error string.
    pub fn step(&mut self) -> Result<CpuState, String> {
        // Check for pending interrupt before executing next instruction
        if self.flags.int_enable()
            && let Some(irq) = self.pending_irq.take()
        {
            self.handle_interrupt(irq);
            self.state = CpuState::Running;
            self.cycles += 6;
            return Ok(self.state);
        }

        if self.state != CpuState::Running {
            return Ok(self.state);
        }

        let word = self.fetch_word();
        let next_word = self.mem.read_word(self.pc); // peek (may not be consumed)
        let instr = Instruction::decode(word, next_word)?;

        // For instructions that consume the next word, advance PC
        let uses_next_word = matches!(
            instr.opcode,
            Opcode::Jmp | Opcode::Jz | Opcode::Jnz | Opcode::Jc | Opcode::Jn | Opcode::Call
        );

        self.execute(instr)?;
        self.cycles += 1;

        // Advance PC past the address word AFTER executing (so jumps can overwrite PC freely)
        if uses_next_word
            && matches!(
                instr.opcode,
                Opcode::Jmp | Opcode::Jz | Opcode::Jnz | Opcode::Jc | Opcode::Jn
            )
        {
            // PC was already set by the jump; don't advance again
        } else if uses_next_word {
            // CALL: PC was set by execute; next_word was used for the address
            // nothing extra needed
        }

        Ok(self.state)
    }

    fn execute(&mut self, i: Instruction) -> Result<(), String> {
        let dst = i.dst as usize;
        let src = i.src as usize;

        match i.opcode {
            // ── NOP ──────────────────────────────────────────────────────────
            Opcode::Nop => {}

            // ── Data movement ────────────────────────────────────────────────
            Opcode::Load => {
                // Wide load: sentinel 0x3E means real 16-bit value follows in next word
                if i.imm == 0x3E {
                    let wide = self.mem.read_word(self.pc);
                    self.pc = self.pc.wrapping_add(2);
                    self.regs[dst] = wide;
                } else {
                    self.regs[dst] = i.imm;
                }
            }
            Opcode::LoadM => {
                let addr = self.regs[src];
                self.regs[dst] = self.mem.read_word(addr);
            }
            Opcode::Store => {
                let addr = self.regs[dst];
                self.mem.write_word(addr, self.regs[src]);
            }
            Opcode::Mov => {
                self.regs[dst] = self.regs[src];
            }

            // ── Arithmetic ───────────────────────────────────────────────────
            Opcode::Add => {
                let a = self.regs[dst];
                let b = self.regs[src];
                let r = a as u32 + b as u32;
                self.flags.update_arithmetic(r, a, b, false);
                self.regs[dst] = r as u16;
            }
            Opcode::Sub => {
                let a = self.regs[dst];
                let b = self.regs[src];
                let r = (a as u32).wrapping_add((!b as u32).wrapping_add(1));
                self.flags.update_arithmetic(r, a, b, true);
                self.regs[dst] = r as u16;
            }
            Opcode::Addi => {
                let a = self.regs[dst];
                // sign-extend 6-bit immediate: if bit 5 is set, it's negative
                let imm6 = i.imm & 0x3F;
                let signed: i16 = if imm6 & 0x20 != 0 {
                    (imm6 | 0xFFC0) as i16 // sign extend to 16 bits
                } else {
                    imm6 as i16
                };
                let r = (a as i32 + signed as i32) as u32;
                self.flags
                    .update_arithmetic(r, a, signed as u16, signed < 0);
                self.regs[dst] = r as u16;
            }
            Opcode::Mul => {
                let r = self.regs[dst] as u32 * self.regs[src] as u32;
                self.regs[dst] = r as u16;
                self.flags.update_logical(r as u16);
            }
            Opcode::Div => {
                if self.regs[src] == 0 {
                    return Err("Division by zero".into());
                }
                let r = self.regs[dst] / self.regs[src];
                self.regs[dst] = r;
                self.flags.update_logical(r);
            }

            // ── Bitwise ──────────────────────────────────────────────────────
            Opcode::And => {
                let r = self.regs[dst] & self.regs[src];
                self.flags.update_logical(r);
                self.regs[dst] = r;
            }
            Opcode::Or => {
                let r = self.regs[dst] | self.regs[src];
                self.flags.update_logical(r);
                self.regs[dst] = r;
            }
            Opcode::Xor => {
                let r = self.regs[dst] ^ self.regs[src];
                self.flags.update_logical(r);
                self.regs[dst] = r;
            }
            Opcode::Not => {
                let r = !self.regs[dst];
                self.flags.update_logical(r);
                self.regs[dst] = r;
            }
            Opcode::Shl => {
                let shift = (i.imm & 0xF) as u32;
                let r = (self.regs[dst] as u32) << shift;
                self.flags.set_carry(r > 0xFFFF);
                let r16 = r as u16;
                self.flags.set_zero(r16 == 0);
                self.flags.set_negative(r16 & 0x8000 != 0);
                self.regs[dst] = r16;
            }
            Opcode::Shr => {
                let shift = (i.imm & 0xF) as u32;
                let r = self.regs[dst] >> shift as u16;
                self.flags.update_logical(r);
                self.regs[dst] = r;
            }

            // ── Compare ──────────────────────────────────────────────────────
            Opcode::Cmp => {
                let a = self.regs[dst];
                let b = self.regs[src];
                let r = (a as u32).wrapping_add((!b as u32).wrapping_add(1));
                self.flags.update_arithmetic(r, a, b, true);
            }

            // ── Control flow ─────────────────────────────────────────────────
            Opcode::Jmp => {
                self.pc = i.imm;
            }
            Opcode::Jz => {
                if self.flags.zero() {
                    self.pc = i.imm;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            Opcode::Jnz => {
                if !self.flags.zero() {
                    self.pc = i.imm;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            Opcode::Jc => {
                if self.flags.carry() {
                    self.pc = i.imm;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            Opcode::Jn => {
                if self.flags.negative() {
                    self.pc = i.imm;
                } else {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            Opcode::Call => {
                // PC currently points at the address word; ret addr is after it
                let ret_addr = self.pc.wrapping_add(2);
                self.push(ret_addr);
                self.pc = i.imm;
            }
            Opcode::Ret => {
                self.pc = self.pop();
            }

            // ── Stack ────────────────────────────────────────────────────────
            Opcode::Push => {
                let v = self.regs[dst];
                self.push(v);
            }
            Opcode::Pop => {
                self.regs[dst] = self.pop();
            }

            // ── Interrupts ───────────────────────────────────────────────────
            Opcode::Int => {
                self.handle_interrupt(i.imm as u8);
            }
            Opcode::Iret => {
                self.pc = self.pop();
                self.flags.0 = self.pop() as u8;
            }
            Opcode::Ei => {
                self.flags.set_int_enable(true);
            }
            Opcode::Di => {
                self.flags.set_int_enable(false);
            }

            // ── Extended instructions (v0.3.0) ────────────────────────────────

            // NEG Rd — two's complement negation: Rd = 0 - Rd
            //
            // Flag behaviour (mirrors x86 NEG semantics):
            //   Z = 1  if result is zero
            //   N = 1  if result is negative (bit 15 set)
            //   C = 1  if Rd was non-zero before negation (borrow from 0)
            //   V = 1  if Rd was 0x8000 (negation of MIN_INT overflows)
            Opcode::Neg => {
                let a = self.regs[dst];
                let r = (0u32).wrapping_sub(a as u32);
                let r16 = r as u16;
                self.flags.set_zero(r16 == 0);
                self.flags.set_negative(r16 & 0x8000 != 0);
                // C = 1 if source was non-zero (there was a borrow)
                self.flags.set_carry(a != 0);
                // V = 1 only when negating 0x8000 (signed MIN overflows)
                self.flags.set_overflow(a == 0x8000);
                self.regs[dst] = r16;
            }

            // MOD Rd, Rs — unsigned modulo: Rd = Rd % Rs
            //
            // Flag behaviour:
            //   Z = 1  if result is zero (evenly divisible)
            //   N = 1  if result has bit 15 set (unusual for small moduli)
            //   C = 0  (always cleared, consistent with DIV)
            //   error  if Rs == 0 (modulo by zero)
            Opcode::Mod => {
                if self.regs[src] == 0 {
                    return Err("Modulo by zero".into());
                }
                let r = self.regs[dst] % self.regs[src];
                self.flags.update_logical(r);
                self.regs[dst] = r;
            }

            // SWAP Rd, Rs — exchange register values atomically
            //
            // Flags: unchanged (SWAP is a data movement, not arithmetic)
            // Use case: avoids needing a scratch register for a classic
            //   three-instruction swap (MOV tmp,a / MOV a,b / MOV b,tmp)
            Opcode::Swap => {
                self.regs.swap(dst, src);
            }

            // ROL Rd, imm4 — rotate left through carry by imm4 bits
            //
            // For each bit rotated:
            //   new_bit0 = old C
            //   new C    = old bit15
            // After all rotations:
            //   Z = 1 if result is zero
            //   N = 1 if result bit 15 is set
            //   C = last bit rotated out (bit 15 of the pre-rotation value)
            //
            // ROL by 1 is the classical operation. Larger counts apply it N times.
            // ROL by 0 is a no-op (flags unchanged).
            Opcode::Rol => {
                let count = (i.imm & 0xF) as u32;
                let mut val = self.regs[dst];
                let mut carry = self.flags.carry();
                for _ in 0..count {
                    let new_carry = val & 0x8000 != 0; // old bit 15 exits
                    val = (val << 1) | (carry as u16); // old carry enters bit 0
                    carry = new_carry;
                }
                self.flags.set_carry(carry);
                self.flags.set_zero(val == 0);
                self.flags.set_negative(val & 0x8000 != 0);
                self.regs[dst] = val;
            }

            // ROR Rd, imm4 — rotate right through carry by imm4 bits
            //
            // For each bit rotated:
            //   new_bit15 = old C
            //   new C     = old bit0
            // After all rotations:
            //   Z = 1 if result is zero
            //   N = 1 if result bit 15 is set (i.e. C was set before rotation)
            //   C = last bit rotated out (bit 0 of the pre-rotation value)
            //
            // ROR by 0 is a no-op (flags unchanged).
            Opcode::Ror => {
                let count = (i.imm & 0xF) as u32;
                let mut val = self.regs[dst];
                let mut carry = self.flags.carry();
                for _ in 0..count {
                    let new_carry = val & 0x0001 != 0; // old bit 0 exits
                    val = (val >> 1) | ((carry as u16) << 15); // old carry enters bit 15
                    carry = new_carry;
                }
                self.flags.set_carry(carry);
                self.flags.set_zero(val == 0);
                self.flags.set_negative(val & 0x8000 != 0);
                self.regs[dst] = val;
            }

            // ── Halt ─────────────────────────────────────────────────────────
            Opcode::Halt => {
                self.state = CpuState::Halted;
            }
        }
        Ok(())
    }

    // ── Run until halt ───────────────────────────────────────────────────────

    /// Run until HALT or `max_cycles` exceeded.
    pub fn run(&mut self, max_cycles: u64) -> Result<CpuState, String> {
        while self.state == CpuState::Running && self.cycles < max_cycles {
            self.step()?;
        }
        Ok(self.state)
    }

    // ── Debug helpers ────────────────────────────────────────────────────────

    pub fn dump_state(&self) -> String {
        format!(
            "PC={:04X}  SP={:04X}  FLAGS={}\n  R0={:04X}  R1={:04X}  R2={:04X}  R3={:04X}\n  Cycles: {}",
            self.pc,
            self.sp,
            self.flags,
            self.regs[0],
            self.regs[1],
            self.regs[2],
            self.regs[3],
            self.cycles,
        )
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}
