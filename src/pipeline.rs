/// cpu16 — 5-Stage Pipeline Simulation (v0.5.0)
///
/// Implements a classic in-order 5-stage pipeline:
///
///   IF  — Instruction Fetch
///   ID  — Instruction Decode + hazard detection (no register read here)
///   EX  — Execute: register read + ALU + branch resolution
///   MEM — Memory access (LOADM / STORE via cache)
///   WB  — Write-Back (register file update)
///
/// KEY DESIGN DECISION — register read in EX, not ID:
///   In a real pipeline, register reads happen in ID and forwarding paths
///   supply the correct value to EX. Without forwarding, the safest correct
///   implementation reads registers at the START of EX, after WB has already
///   committed for this cycle (WB runs first in our reverse-order tick).
///   This means a stall in ID holds the instruction until the producing
///   instruction has reached WB and committed — at which point EX reads
///   the fresh value directly from the register file. No forwarding needed.
///
/// Hazard handling:
///
///   Data hazards (RAW — register):
///     Detected in ID by comparing source registers against destination
///     registers of instructions in EX, MEM, and WB. Stall until clear.
///
///   Data hazards (RAW — flags):
///     Conditional branches stall while any flag-writing instruction is
///     in EX, MEM, or WB. Prevents CMP→JNZ reading stale flags.
///
///   Control hazards:
///     Branches resolved in EX. 2-cycle flush of wrong-path instructions.
///
///   Drain:
///     HALT in EX sets drain_mode. Pipeline ticks until HALT commits in WB.
use crate::cache::Cache;
use crate::flags::Flags;
use crate::isa::{Instruction, Opcode};
use crate::memory::Memory;

// ── Pipeline registers ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
struct IfIdReg {
    word: u16,
    next_word: u16,
    pc: u16,
    valid: bool,
}

/// ID/EX carries decoded fields only — NO register values.
/// Registers are read at the start of EX after WB has committed.
#[derive(Debug, Clone, Default)]
struct IdExReg {
    opcode: Option<Opcode>,
    dst: u8,
    src: u8,
    imm: u16,
    pc: u16,
    valid: bool,
}

#[derive(Debug, Clone, Default)]
struct ExMemReg {
    opcode: Option<Opcode>,
    dst: u8,
    src: u8,
    alu_result: u16,
    store_val: u16,
    new_flags: Option<Flags>,
    valid: bool,
    #[allow(dead_code)]
    branch_taken: bool,
    #[allow(dead_code)]
    branch_target: u16,
    is_halt: bool,
}

#[derive(Debug, Clone, Default)]
struct MemWbReg {
    opcode: Option<Opcode>,
    dst: u8,
    src: u8,
    wb_val: u16,
    wb_val2: u16,
    new_flags: Option<Flags>,
    valid: bool,
    is_halt: bool,
}

// ── Stats ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    pub cycles: u64,
    pub instructions_committed: u64,
    pub data_stall_cycles: u64,
    pub control_flush_cycles: u64,
}

impl PipelineStats {
    pub fn cpi(&self) -> f64 {
        if self.instructions_committed == 0 {
            0.0
        } else {
            self.cycles as f64 / self.instructions_committed as f64
        }
    }

    pub fn ideal_cycles(&self) -> u64 {
        self.instructions_committed.saturating_add(4)
    }

    pub fn efficiency(&self) -> f64 {
        if self.cycles == 0 {
            0.0
        } else {
            self.ideal_cycles() as f64 / self.cycles as f64 * 100.0
        }
    }
}

impl std::fmt::Display for PipelineStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "┌─────────────────────────────────────────┐")?;
        writeln!(f, "│       Pipeline Statistics               │")?;
        writeln!(f, "├─────────────────────────────────────────┤")?;
        writeln!(f, "│    Total cycles:        {:>8}         │", self.cycles)?;
        writeln!(
            f,
            "│    Instructions:        {:>8}         │",
            self.instructions_committed
        )?;
        writeln!(f, "│    CPI:                 {:>8.3}         │", self.cpi())?;
        writeln!(
            f,
            "│    Pipeline efficiency: {:>7.1}%         │",
            self.efficiency()
        )?;
        writeln!(f, "├─────────────────────────────────────────┤")?;
        writeln!(
            f,
            "│    Data stall cycles:   {:>8}         │",
            self.data_stall_cycles
        )?;
        writeln!(
            f,
            "│    Control flush cycles:{:>8}         │",
            self.control_flush_cycles
        )?;
        writeln!(f, "└─────────────────────────────────────────┘")
    }
}

// ── Pipelined CPU ─────────────────────────────────────────────────────────────

pub struct PipelinedCpu {
    pub regs: [u16; 4],
    pub pc: u16,
    pub sp: u16,
    pub flags: Flags,
    pub mem: Memory,
    pub cache: Cache,

    if_id: IfIdReg,
    id_ex: IdExReg,
    ex_mem: ExMemReg,
    mem_wb: MemWbReg,

    drain_mode: bool,
    pub halted: bool,

    pub stats: PipelineStats,
    pub print_stats: bool,
}

impl PipelinedCpu {
    pub fn new() -> Self {
        let mut cpu = Self {
            regs: [0; 4],
            pc: crate::cpu::PROG_BASE,
            sp: crate::cpu::STACK_BASE,
            flags: Flags::default(),
            mem: Memory::new(),
            cache: Cache::new(),
            if_id: IfIdReg::default(),
            id_ex: IdExReg::default(),
            ex_mem: ExMemReg::default(),
            mem_wb: MemWbReg::default(),
            drain_mode: false,
            halted: false,
            stats: PipelineStats::default(),
            print_stats: false,
        };
        cpu.flags.set_int_enable(true);
        cpu
    }

    pub fn load_program(&mut self, bytes: &[u8]) {
        self.mem.load(crate::cpu::PROG_BASE, bytes);
        self.pc = crate::cpu::PROG_BASE;
    }

    pub fn enable_stats(&mut self) {
        self.print_stats = true;
    }

    // ── Hazard detection ──────────────────────────────────────────────────────

    fn reg_in_flight(&self, reg: u8) -> bool {
        if reg == u8::MAX {
            return false;
        }
        // check_data_hazard runs AFTER stage_ex, so id_ex has already been
        // consumed — its result is now in ex_mem. Only ex_mem and mem_wb
        // hold values not yet committed to the register file.
        let stages: &[(bool, Option<Opcode>, u8)] = &[
            (self.ex_mem.valid, self.ex_mem.opcode, self.ex_mem.dst),
            (self.mem_wb.valid, self.mem_wb.opcode, self.mem_wb.dst),
        ];
        for &(valid, opcode, dst) in stages {
            if valid && opcode.is_some_and(writes_reg) && dst == reg {
                return true;
            }
        }
        false
    }

    fn flags_in_flight(&self) -> bool {
        // Same: only ex_mem and mem_wb hold uncommitted flags.
        let stages: &[(bool, Option<Opcode>)] = &[
            (self.ex_mem.valid, self.ex_mem.opcode),
            (self.mem_wb.valid, self.mem_wb.opcode),
        ];
        for &(valid, opcode) in stages {
            if valid && opcode.is_some_and(writes_flags) {
                return true;
            }
        }
        false
    }

    fn check_data_hazard(&self) -> bool {
        if !self.if_id.valid {
            return false;
        }
        let word = self.if_id.word;
        let opcode_bits = (word >> 10) as u8;
        let dst = ((word >> 8) & 0x3) as u8;
        let src = ((word >> 6) & 0x3) as u8;

        let opcode = match Opcode::try_from(opcode_bits) {
            Ok(o) => o,
            Err(_) => return false,
        };

        // Flag hazard: conditional branch needs committed flags
        if reads_flags(opcode) && self.flags_in_flight() {
            return true;
        }

        // Register hazard
        let (sa, sb, needs_b) = source_regs(opcode, dst, src);
        if needs_b {
            self.reg_in_flight(sa) || self.reg_in_flight(sb)
        } else {
            self.reg_in_flight(sa)
        }
    }

    // ── One clock tick ────────────────────────────────────────────────────────

    pub fn tick(&mut self) -> Result<(), String> {
        if self.halted {
            return Ok(());
        }

        self.stats.cycles += 1;

        // 1. WB (commit first)
        let halt_committed = self.stage_wb();

        if halt_committed {
            if self.print_stats {
                println!("\n{}", self.stats);
                println!("{}", self.cache.stats);
            }
            self.halted = true;
            return Ok(());
        }

        // 2. MEM
        self.stage_mem()?;

        // 3. EX (branch resolves here)
        let (branch_taken, _) = self.stage_ex()?;

        // 🚨 CRITICAL FIX: handle branch IMMEDIATELY
        if branch_taken {
            // Flush wrong-path instructions
            self.if_id = IfIdReg::default();
            self.id_ex = IdExReg::default();

            // Count flush penalty
            self.stats.control_flush_cycles += 2;

            // 🚨 DO NOT allow ID or IF this cycle
            return Ok(());
        }

        // 4. Hazard detection
        let stall = self.check_data_hazard();

        if stall {
            self.stats.data_stall_cycles += 1;

            // Insert bubble into EX
            self.id_ex = IdExReg::default();

            // Freeze IF/ID and PC (do nothing)
            return Ok(());
        }

        // 5. Drain mode (HALT handling)
        if self.drain_mode {
            self.stage_id()?;
            self.if_id = IfIdReg::default();
            return Ok(());
        }

        // 6. Normal flow
        self.stage_id()?;
        self.stage_if();

        Ok(())
    }

    // ── Stages ───────────────────────────────────────────────────────────────

    fn stage_if(&mut self) {
        let word = self.mem.read_word(self.pc);
        let next_word = self.mem.read_word(self.pc.wrapping_add(2));
        let fetched_pc = self.pc;

        // 2-word instructions (jump/call): advance PC past both the opcode
        // word AND the address word so the next fetch gets the right instruction.
        let opcode_bits = (word >> 10) as u8;
        let is_two_word = matches!(
            Opcode::try_from(opcode_bits),
            Ok(Opcode::Jmp | Opcode::Jz | Opcode::Jnz | Opcode::Jc | Opcode::Jn | Opcode::Call)
        );

        self.if_id = IfIdReg {
            word,
            next_word,
            pc: fetched_pc,
            valid: true,
        };

        if is_two_word {
            self.pc = self.pc.wrapping_add(4); // skip opcode + address word
        } else {
            self.pc = self.pc.wrapping_add(2); // skip opcode word only
        }
    }

    fn stage_id(&mut self) -> Result<(), String> {
        if !self.if_id.valid {
            self.id_ex = IdExReg::default();
            return Ok(());
        }
        let instr = Instruction::decode(self.if_id.word, self.if_id.next_word)?;
        // NOTE: we do NOT read register values here.
        // They are read at the start of stage_ex, after WB has committed.
        self.id_ex = IdExReg {
            opcode: Some(instr.opcode),
            dst: instr.dst,
            src: instr.src,
            imm: instr.imm,
            pc: self.if_id.pc,
            valid: true,
        };
        Ok(())
    }

    fn stage_ex(&mut self) -> Result<(bool, u16), String> {
        if !self.id_ex.valid {
            self.ex_mem = ExMemReg::default();
            return Ok((false, 0));
        }
        let opcode = match self.id_ex.opcode {
            Some(o) => o,
            None => {
                self.ex_mem = ExMemReg::default();
                return Ok((false, 0));
            }
        };

        // Read registers HERE — WB has already committed this cycle
        let a = self.regs[self.id_ex.dst as usize];
        let b = self.regs[self.id_ex.src as usize];
        let imm = self.id_ex.imm;
        let dst = self.id_ex.dst;
        let src = self.id_ex.src;

        let mut alu_result: u16 = 0;
        let mut store_val: u16 = b;
        let mut new_flags: Option<Flags> = None;
        let mut branch_taken = false;
        let mut branch_target = 0u16;
        let mut is_halt = false;

        match opcode {
            Opcode::Nop => {}
            Opcode::Load => {
                alu_result = imm;
            }
            Opcode::LoadM => {
                alu_result = b; // src = address register
            }
            Opcode::Store => {
                alu_result = a; // dst = address register
                store_val = b; // src = value register
            }
            Opcode::Mov => {
                alu_result = b;
            }
            Opcode::Add => {
                let r = a as u32 + b as u32;
                let mut f = self.flags;
                f.update_arithmetic(r, a, b, false);
                alu_result = r as u16;
                new_flags = Some(f);
            }
            Opcode::Sub => {
                let r = (a as u32).wrapping_add((!b as u32).wrapping_add(1));
                let mut f = self.flags;
                f.update_arithmetic(r, a, b, true);
                alu_result = r as u16;
                new_flags = Some(f);
            }
            Opcode::Addi => {
                let imm6 = imm & 0x3F;
                let signed: i16 = if imm6 & 0x20 != 0 {
                    (imm6 | 0xFFC0) as i16
                } else {
                    imm6 as i16
                };
                let r = (a as i32 + signed as i32) as u32;
                let mut f = self.flags;
                f.update_arithmetic(r, a, signed as u16, signed < 0);
                alu_result = r as u16;
                new_flags = Some(f);
            }
            Opcode::Mul => {
                let r = a as u32 * b as u32;
                alu_result = r as u16;
                let mut f = self.flags;
                f.update_logical(alu_result);
                new_flags = Some(f);
            }
            Opcode::Div => {
                if b == 0 {
                    return Err("Pipeline: division by zero".into());
                }
                alu_result = a / b;
                let mut f = self.flags;
                f.update_logical(alu_result);
                new_flags = Some(f);
            }
            Opcode::Mod => {
                if b == 0 {
                    return Err("Pipeline: modulo by zero".into());
                }
                alu_result = a % b;
                let mut f = self.flags;
                f.update_logical(alu_result);
                new_flags = Some(f);
            }
            Opcode::And => {
                alu_result = a & b;
                let mut f = self.flags;
                f.update_logical(alu_result);
                new_flags = Some(f);
            }
            Opcode::Or => {
                alu_result = a | b;
                let mut f = self.flags;
                f.update_logical(alu_result);
                new_flags = Some(f);
            }
            Opcode::Xor => {
                alu_result = a ^ b;
                let mut f = self.flags;
                f.update_logical(alu_result);
                new_flags = Some(f);
            }
            Opcode::Not => {
                alu_result = !a;
                let mut f = self.flags;
                f.update_logical(alu_result);
                new_flags = Some(f);
            }
            Opcode::Neg => {
                let r = (0u32).wrapping_sub(a as u32) as u16;
                let mut f = self.flags;
                f.set_zero(r == 0);
                f.set_negative(r & 0x8000 != 0);
                f.set_carry(a != 0);
                f.set_overflow(a == 0x8000);
                alu_result = r;
                new_flags = Some(f);
            }
            Opcode::Shl => {
                let shift = (imm & 0xF) as u32;
                let r = (a as u32) << shift;
                let mut f = self.flags;
                f.set_carry(r > 0xFFFF);
                alu_result = r as u16;
                f.set_zero(alu_result == 0);
                f.set_negative(alu_result & 0x8000 != 0);
                new_flags = Some(f);
            }
            Opcode::Shr => {
                let shift = (imm & 0xF) as u32;
                alu_result = a >> shift as u16;
                let mut f = self.flags;
                f.update_logical(alu_result);
                new_flags = Some(f);
            }
            Opcode::Rol => {
                let count = (imm & 0xF) as u32;
                let mut val = a;
                let mut carry = self.flags.carry();
                for _ in 0..count {
                    let nc = val & 0x8000 != 0;
                    val = (val << 1) | (carry as u16);
                    carry = nc;
                }
                let mut f = self.flags;
                f.set_carry(carry);
                f.set_zero(val == 0);
                f.set_negative(val & 0x8000 != 0);
                alu_result = val;
                new_flags = Some(f);
            }
            Opcode::Ror => {
                let count = (imm & 0xF) as u32;
                let mut val = a;
                let mut carry = self.flags.carry();
                for _ in 0..count {
                    let nc = val & 0x0001 != 0;
                    val = (val >> 1) | ((carry as u16) << 15);
                    carry = nc;
                }
                let mut f = self.flags;
                f.set_carry(carry);
                f.set_zero(val == 0);
                f.set_negative(val & 0x8000 != 0);
                alu_result = val;
                new_flags = Some(f);
            }
            Opcode::Cmp => {
                let r = (a as u32).wrapping_add((!b as u32).wrapping_add(1));
                let mut f = self.flags;
                f.update_arithmetic(r, a, b, true);
                new_flags = Some(f);
            }
            Opcode::Swap => {
                alu_result = b;
                store_val = a;
            }
            Opcode::Jmp => {
                branch_taken = true;
                branch_target = imm;
                self.pc = imm;
            }
            Opcode::Jz => {
                if self.flags.zero() {
                    branch_taken = true;
                    branch_target = imm;
                    self.pc = imm;
                }
            }
            Opcode::Jnz => {
                if !self.flags.zero() {
                    branch_taken = true;
                    branch_target = imm;
                    self.pc = imm;
                }
            }
            Opcode::Jc => {
                if self.flags.carry() {
                    branch_taken = true;
                    branch_target = imm;
                    self.pc = imm;
                }
            }
            Opcode::Jn => {
                if self.flags.negative() {
                    branch_taken = true;
                    branch_target = imm;
                    self.pc = imm;
                }
            }
            Opcode::Call => {
                let ret = self.id_ex.pc.wrapping_add(4);
                self.sp = self.sp.wrapping_sub(2);
                self.mem.write_word(self.sp, ret);
                branch_taken = true;
                branch_target = imm;
                self.pc = imm;
            }
            Opcode::Ret => {
                let ret = self.mem.read_word(self.sp);
                self.sp = self.sp.wrapping_add(2);
                branch_taken = true;
                branch_target = ret;
                self.pc = ret;
            }
            Opcode::Push => {
                self.sp = self.sp.wrapping_sub(2);
                self.mem.write_word(self.sp, a);
            }
            Opcode::Pop => {
                let v = self.mem.read_word(self.sp);
                self.sp = self.sp.wrapping_add(2);
                alu_result = v;
            }
            Opcode::Ei => {
                self.flags.set_int_enable(true);
            }
            Opcode::Di => {
                self.flags.set_int_enable(false);
            }
            Opcode::Halt => {
                self.drain_mode = true;
                is_halt = true;
            }
            Opcode::Int | Opcode::Iret => {}
        }

        self.ex_mem = ExMemReg {
            opcode: Some(opcode),
            dst,
            src,
            alu_result,
            store_val,
            new_flags,
            valid: true,
            branch_taken,
            branch_target,
            is_halt,
        };

        Ok((branch_taken, branch_target))
    }

    fn stage_mem(&mut self) -> Result<(), String> {
        if !self.ex_mem.valid {
            self.mem_wb = MemWbReg::default();
            return Ok(());
        }
        let opcode = match self.ex_mem.opcode {
            Some(o) => o,
            None => {
                self.mem_wb = MemWbReg::default();
                return Ok(());
            }
        };

        let mut wb_val = self.ex_mem.alu_result;
        let wb_val2 = self.ex_mem.store_val;

        match opcode {
            Opcode::LoadM => {
                wb_val = self.cache.read_word(self.ex_mem.alu_result, &self.mem);
            }
            Opcode::Store => {
                self.cache
                    .write_word(self.ex_mem.alu_result, self.ex_mem.store_val, &mut self.mem);
            }
            _ => {}
        }

        self.mem_wb = MemWbReg {
            opcode: Some(opcode),
            dst: self.ex_mem.dst,
            src: self.ex_mem.src,
            wb_val,
            wb_val2,
            new_flags: self.ex_mem.new_flags,
            valid: true,
            is_halt: self.ex_mem.is_halt,
        };

        Ok(())
    }

    fn stage_wb(&mut self) -> bool {
        if !self.mem_wb.valid {
            return false;
        }
        let opcode = match self.mem_wb.opcode {
            Some(o) => o,
            None => return false,
        };

        let is_halt = self.mem_wb.is_halt;

        if let Some(f) = self.mem_wb.new_flags {
            let ie = self.flags.int_enable();
            self.flags = f;
            self.flags.set_int_enable(ie);
        }

        if writes_reg(opcode) {
            self.regs[self.mem_wb.dst as usize] = self.mem_wb.wb_val;
            if matches!(opcode, Opcode::Swap) {
                self.regs[self.mem_wb.src as usize] = self.mem_wb.wb_val2;
            }
        }

        self.stats.instructions_committed += 1;
        self.mem_wb = MemWbReg::default();
        is_halt
    }

    // ── Run ───────────────────────────────────────────────────────────────────

    pub fn run(&mut self, max_cycles: u64) -> Result<(), String> {
        while !self.halted && self.stats.cycles < max_cycles {
            self.tick()?;
        }
        Ok(())
    }

    // ── Debug ─────────────────────────────────────────────────────────────────

    pub fn dump_state(&self) -> String {
        format!(
            "PC={:04X}  SP={:04X}  FLAGS={}\n  R0={:04X}  R1={:04X}  R2={:04X}  R3={:04X}\n  Cycles:{}  Committed:{}  CPI:{:.3}",
            self.pc,
            self.sp,
            self.flags,
            self.regs[0],
            self.regs[1],
            self.regs[2],
            self.regs[3],
            self.stats.cycles,
            self.stats.instructions_committed,
            self.stats.cpi(),
        )
    }

    pub fn dump_pipeline(&self) -> String {
        format!(
            "IF/ID v={} pc={:04X} | ID/EX v={} op={:?} | EX/MEM v={} op={:?} res={:04X} | MEM/WB v={} op={:?} wb={:04X}",
            self.if_id.valid,
            self.if_id.pc,
            self.id_ex.valid,
            self.id_ex.opcode,
            self.ex_mem.valid,
            self.ex_mem.opcode,
            self.ex_mem.alu_result,
            self.mem_wb.valid,
            self.mem_wb.opcode,
            self.mem_wb.wb_val,
        )
    }
}

impl Default for PipelinedCpu {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn writes_reg(op: Opcode) -> bool {
    matches!(
        op,
        Opcode::Load
            | Opcode::LoadM
            | Opcode::Mov
            | Opcode::Add
            | Opcode::Sub
            | Opcode::Addi
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::And
            | Opcode::Or
            | Opcode::Xor
            | Opcode::Not
            | Opcode::Neg
            | Opcode::Shl
            | Opcode::Shr
            | Opcode::Rol
            | Opcode::Ror
            | Opcode::Swap
            | Opcode::Pop
    )
}

fn writes_flags(op: Opcode) -> bool {
    matches!(
        op,
        Opcode::Add
            | Opcode::Sub
            | Opcode::Addi
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::And
            | Opcode::Or
            | Opcode::Xor
            | Opcode::Not
            | Opcode::Neg
            | Opcode::Shl
            | Opcode::Shr
            | Opcode::Rol
            | Opcode::Ror
            | Opcode::Cmp
    )
}

fn reads_flags(op: Opcode) -> bool {
    matches!(op, Opcode::Jz | Opcode::Jnz | Opcode::Jc | Opcode::Jn)
}

/// Which source registers does an instruction read?
/// u8::MAX = sentinel meaning "no register".
fn source_regs(op: Opcode, dst: u8, src: u8) -> (u8, u8, bool) {
    match op {
        Opcode::Add
        | Opcode::Sub
        | Opcode::Mul
        | Opcode::Div
        | Opcode::Mod
        | Opcode::And
        | Opcode::Or
        | Opcode::Xor
        | Opcode::Cmp
        | Opcode::Swap
        | Opcode::Store => (dst, src, true),

        Opcode::Not
        | Opcode::Neg
        | Opcode::Shl
        | Opcode::Shr
        | Opcode::Rol
        | Opcode::Ror
        | Opcode::Addi
        | Opcode::Push => (dst, u8::MAX, false),

        Opcode::LoadM | Opcode::Mov => (src, u8::MAX, false),

        _ => (u8::MAX, u8::MAX, false),
    }
}
