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
        if self.flags.int_enable() {
            if let Some(irq) = self.pending_irq.take() {
                self.handle_interrupt(irq);
                self.state = CpuState::Running;
                self.cycles += 6;
                return Ok(self.state);
            }
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
