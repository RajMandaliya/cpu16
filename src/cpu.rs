use crate::cache::Cache;
use crate::flags::Flags;
use crate::isa::{Instruction, Opcode};
use crate::memory::Memory;

/// Interrupt vector table starts at address 0x0000.
const IVT_BASE: u16 = 0x0000;
/// Default stack base (grows downward from 0xFFFE).
pub const STACK_BASE: u16 = 0xFFFE;
/// Program entry point (programs loaded at this address).
pub const PROG_BASE: u16 = 0x0200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuState {
    Running,
    Halted,
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
    /// Main memory (source of truth)
    pub mem: Memory,
    /// L1 cache (sits in front of mem for all data reads/writes)
    pub cache: Cache,
    /// Pending interrupt number (None = no interrupt)
    pub pending_irq: Option<u8>,
    /// Execution state
    pub state: CpuState,
    /// Cycle counter
    pub cycles: u64,
    /// Whether to print cache stats at HALT
    pub print_cache_stats: bool,
}

impl Cpu {
    pub fn new() -> Self {
        let mut cpu = Self {
            regs: [0; 4],
            pc: PROG_BASE,
            sp: STACK_BASE,
            flags: Flags::default(),
            mem: Memory::new(),
            cache: Cache::new(),
            pending_irq: None,
            state: CpuState::Running,
            cycles: 0,
            print_cache_stats: false,
        };
        cpu.flags.set_int_enable(true);
        cpu
    }

    /// Enable cache stats printing at HALT.
    pub fn enable_cache_stats(&mut self) {
        self.print_cache_stats = true;
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

    // ── Memory access helpers (go through cache) ─────────────────────────────

    /// Read a word — goes through the cache.
    fn mem_read(&mut self, addr: u16) -> u16 {
        self.cache.read_word(addr, &self.mem)
    }

    /// Write a word — write-through cache.
    fn mem_write(&mut self, addr: u16, val: u16) {
        self.cache.write_word(addr, val, &mut self.mem)
    }

    // ── Fetch ────────────────────────────────────────────────────────────────
    // Instruction fetches bypass the cache — this simulates a data cache only.
    // A unified cache would count instruction fetches too; keeping them separate
    // isolates data access patterns which is more interesting to analyse.

    fn fetch_word(&mut self) -> u16 {
        let w = self.mem.read_word(self.pc);
        self.pc = self.pc.wrapping_add(2);
        w
    }

    // ── Stack helpers (go through cache) ─────────────────────────────────────

    fn push(&mut self, val: u16) {
        self.sp = self.sp.wrapping_sub(2);
        self.mem_write(self.sp, val);
    }

    fn pop(&mut self) -> u16 {
        let val = self.mem_read(self.sp);
        self.sp = self.sp.wrapping_add(2);
        val
    }

    // ── Interrupt dispatch ───────────────────────────────────────────────────

    fn handle_interrupt(&mut self, irq: u8) {
        self.flags.set_int_enable(false);
        let saved_pc = self.pc;
        let saved_flags = self.flags.0 as u16;
        self.push(saved_flags);
        self.push(saved_pc);
        let vector_addr = IVT_BASE.wrapping_add((irq as u16) * 2);
        self.pc = self.mem_read(vector_addr);
    }

    // ── Single-step execution ────────────────────────────────────────────────

    pub fn step(&mut self) -> Result<CpuState, String> {
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
        let next_word = self.mem.read_word(self.pc);
        let instr = Instruction::decode(word, next_word)?;

        let uses_next_word = matches!(
            instr.opcode,
            Opcode::Jmp | Opcode::Jz | Opcode::Jnz | Opcode::Jc | Opcode::Jn | Opcode::Call
        );

        self.execute(instr)?;
        self.cycles += 1;

        if uses_next_word
            && matches!(
                instr.opcode,
                Opcode::Jmp | Opcode::Jz | Opcode::Jnz | Opcode::Jc | Opcode::Jn
            )
        {
            // PC already set by jump
        } else if uses_next_word {
            // CALL: PC set by execute
        }

        Ok(self.state)
    }

    fn execute(&mut self, i: Instruction) -> Result<(), String> {
        let dst = i.dst as usize;
        let src = i.src as usize;

        match i.opcode {
            Opcode::Nop => {}

            // ── Data movement ────────────────────────────────────────────────
            Opcode::Load => {
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
                self.regs[dst] = self.mem_read(addr);
            }
            Opcode::Store => {
                let addr = self.regs[dst];
                self.mem_write(addr, self.regs[src]);
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
                let imm6 = i.imm & 0x3F;
                let signed: i16 = if imm6 & 0x20 != 0 {
                    (imm6 | 0xFFC0) as i16
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
            Opcode::Neg => {
                let a = self.regs[dst];
                let r = (0u32).wrapping_sub(a as u32);
                let r16 = r as u16;
                self.flags.set_zero(r16 == 0);
                self.flags.set_negative(r16 & 0x8000 != 0);
                self.flags.set_carry(a != 0);
                self.flags.set_overflow(a == 0x8000);
                self.regs[dst] = r16;
            }
            Opcode::Mod => {
                if self.regs[src] == 0 {
                    return Err("Modulo by zero".into());
                }
                let r = self.regs[dst] % self.regs[src];
                self.flags.update_logical(r);
                self.regs[dst] = r;
            }
            Opcode::Swap => {
                self.regs.swap(dst, src);
            }
            Opcode::Rol => {
                let count = (i.imm & 0xF) as u32;
                let mut val = self.regs[dst];
                let mut carry = self.flags.carry();
                for _ in 0..count {
                    let new_carry = val & 0x8000 != 0;
                    val = (val << 1) | (carry as u16);
                    carry = new_carry;
                }
                self.flags.set_carry(carry);
                self.flags.set_zero(val == 0);
                self.flags.set_negative(val & 0x8000 != 0);
                self.regs[dst] = val;
            }
            Opcode::Ror => {
                let count = (i.imm & 0xF) as u32;
                let mut val = self.regs[dst];
                let mut carry = self.flags.carry();
                for _ in 0..count {
                    let new_carry = val & 0x0001 != 0;
                    val = (val >> 1) | ((carry as u16) << 15);
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
                if self.print_cache_stats {
                    println!("\n{}", self.cache.stats);
                    println!("{}", self.cache.dump());
                }
            }
        }
        Ok(())
    }

    // ── Run until halt ───────────────────────────────────────────────────────

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