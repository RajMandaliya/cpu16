use crate::isa::Opcode;
use std::collections::HashMap;

/// Assembled output: binary words + symbol table.
pub struct AssemblyOutput {
    pub words: Vec<u16>,
    pub symbols: HashMap<String, u16>,
}

/// Two-pass assembler for cpu16 assembly language.
pub struct Assembler {
    /// Origin — address where the binary is intended to be loaded.
    origin: u16,
}

impl Assembler {
    pub fn new(origin: u16) -> Self {
        Self { origin }
    }

    pub fn assemble(&self, source: &str) -> Result<AssemblyOutput, String> {
        let lines = self.tokenize(source);

        // Pass 1 – collect labels
        let mut symbols: HashMap<String, u16> = HashMap::new();
        let mut addr = self.origin;
        for (lineno, tokens) in &lines {
            if let Some(label) = tokens.first().filter(|t| t.ends_with(':')) {
                let name = label.trim_end_matches(':').to_string();
                symbols.insert(name, addr);
                continue;
            }
            if tokens.is_empty() {
                continue;
            }
            addr = addr.wrapping_add(self.instr_size(tokens, *lineno)? * 2);
        }

        // Pass 2 – emit words
        let mut words: Vec<u16> = Vec::new();
        for (lineno, tokens) in &lines {
            if tokens.is_empty() {
                continue;
            }
            if tokens.first().map_or(false, |t| t.ends_with(':')) {
                continue;
            }
            let emitted = self.emit(tokens, &symbols, *lineno)?;
            words.extend(emitted);
        }

        Ok(AssemblyOutput { words, symbols })
    }

    fn tokenize<'a>(&self, source: &'a str) -> Vec<(usize, Vec<String>)> {
        source
            .lines()
            .enumerate()
            .filter_map(|(i, line)| {
                // Strip comments
                let line = line.split(';').next().unwrap_or("").trim();
                if line.is_empty() {
                    return None;
                }
                let tokens: Vec<String> = line
                    .split_whitespace()
                    .map(|t| t.trim_end_matches(',').to_uppercase())
                    .collect();
                if tokens.is_empty() {
                    None
                } else {
                    Some((i + 1, tokens))
                }
            })
            .collect()
    }

    /// How many 16-bit words does this instruction emit?
    fn instr_size(&self, tokens: &[String], lineno: usize) -> Result<u16, String> {
        match tokens[0].as_str() {
            "JMP" | "JZ" | "JNZ" | "JC" | "JN" | "CALL" => Ok(2),
            "DW" => Ok(tokens.len() as u16 - 1),
            // LOAD emits 2 words when the immediate doesn't fit in 6 bits (> 62)
            "LOAD" => {
                if tokens.len() >= 3 {
                    let imm = resolve_imm_simple(&tokens[2]);
                    if imm > 0x3E { Ok(2) } else { Ok(1) }
                } else {
                    Ok(1)
                }
            }
            _ => Ok(1),
        }
    }

    fn emit(
        &self,
        tokens: &[String],
        syms: &HashMap<String, u16>,
        lineno: usize,
    ) -> Result<Vec<u16>, String> {
        let mnemonic = tokens[0].as_str();

        macro_rules! reg {
            ($t:expr) => {
                parse_reg($t, lineno)?
            };
        }
        macro_rules! resolve {
            ($t:expr) => {
                resolve_imm($t, syms, lineno)?
            };
        }

        let words = match mnemonic {
            "NOP" => vec![encode_bare(Opcode::Nop)],
            "HALT" => vec![encode_bare(Opcode::Halt)],
            "RET" => vec![encode_bare(Opcode::Ret)],
            "IRET" => vec![encode_bare(Opcode::Iret)],
            "EI" => vec![encode_bare(Opcode::Ei)],
            "DI" => vec![encode_bare(Opcode::Di)],

            "LOAD" => {
                let imm = resolve!(&tokens[2]);
                if imm > 0x3E {
                    // Wide load: sentinel 0x3E in imm field, real value in next word
                    vec![encode_ri(Opcode::Load, reg!(&tokens[1]), 0x3E), imm]
                } else {
                    vec![encode_ri(Opcode::Load, reg!(&tokens[1]), imm)]
                }
            }
            "LOADM" => vec![encode_rr(Opcode::LoadM, reg!(&tokens[1]), reg!(&tokens[2]))],
            "STORE" => vec![encode_rr(Opcode::Store, reg!(&tokens[1]), reg!(&tokens[2]))],
            "MOV" => vec![encode_rr(Opcode::Mov, reg!(&tokens[1]), reg!(&tokens[2]))],

            "ADD" => vec![encode_rr(Opcode::Add, reg!(&tokens[1]), reg!(&tokens[2]))],
            "SUB" => vec![encode_rr(Opcode::Sub, reg!(&tokens[1]), reg!(&tokens[2]))],
            "ADDI" => vec![encode_ri(
                Opcode::Addi,
                reg!(&tokens[1]),
                resolve!(&tokens[2]),
            )],
            "MUL" => vec![encode_rr(Opcode::Mul, reg!(&tokens[1]), reg!(&tokens[2]))],
            "DIV" => vec![encode_rr(Opcode::Div, reg!(&tokens[1]), reg!(&tokens[2]))],

            "AND" => vec![encode_rr(Opcode::And, reg!(&tokens[1]), reg!(&tokens[2]))],
            "OR" => vec![encode_rr(Opcode::Or, reg!(&tokens[1]), reg!(&tokens[2]))],
            "XOR" => vec![encode_rr(Opcode::Xor, reg!(&tokens[1]), reg!(&tokens[2]))],
            "NOT" => vec![encode_r(Opcode::Not, reg!(&tokens[1]))],
            "SHL" => vec![encode_ri(
                Opcode::Shl,
                reg!(&tokens[1]),
                resolve!(&tokens[2]),
            )],
            "SHR" => vec![encode_ri(
                Opcode::Shr,
                reg!(&tokens[1]),
                resolve!(&tokens[2]),
            )],

            "CMP" => vec![encode_rr(Opcode::Cmp, reg!(&tokens[1]), reg!(&tokens[2]))],

            "JMP" => {
                let a = resolve!(&tokens[1]);
                vec![encode_bare(Opcode::Jmp), a]
            }
            "JZ" => {
                let a = resolve!(&tokens[1]);
                vec![encode_bare(Opcode::Jz), a]
            }
            "JNZ" => {
                let a = resolve!(&tokens[1]);
                vec![encode_bare(Opcode::Jnz), a]
            }
            "JC" => {
                let a = resolve!(&tokens[1]);
                vec![encode_bare(Opcode::Jc), a]
            }
            "JN" => {
                let a = resolve!(&tokens[1]);
                vec![encode_bare(Opcode::Jn), a]
            }
            "CALL" => {
                let a = resolve!(&tokens[1]);
                vec![encode_bare(Opcode::Call), a]
            }

            "PUSH" => vec![encode_r(Opcode::Push, reg!(&tokens[1]))],
            "POP" => vec![encode_r(Opcode::Pop, reg!(&tokens[1]))],

            "INT" => vec![encode_ri(Opcode::Int, 0, resolve!(&tokens[1]))],

            "DW" => tokens[1..]
                .iter()
                .map(|t| resolve_imm(t, syms, lineno))
                .collect::<Result<Vec<_>, _>>()?,

            other => return Err(format!("Line {}: Unknown mnemonic '{}'", lineno, other)),
        };
        Ok(words)
    }
}

// ── Encoding helpers ──────────────────────────────────────────────────────────

fn encode_bare(op: Opcode) -> u16 {
    (op as u16) << 10
}
fn encode_rr(op: Opcode, dst: u8, src: u8) -> u16 {
    ((op as u16) << 10) | ((dst as u16 & 0x3) << 8) | ((src as u16 & 0x3) << 6)
}
fn encode_r(op: Opcode, r: u8) -> u16 {
    ((op as u16) << 10) | ((r as u16 & 0x3) << 8)
}
fn encode_ri(op: Opcode, dst: u8, imm: u16) -> u16 {
    ((op as u16) << 10) | ((dst as u16 & 0x3) << 8) | (imm & 0x3F)
}

// ── Parsing helpers ───────────────────────────────────────────────────────────

/// Quick parse for pass-1 size estimation (no symbol table yet).
/// Returns the numeric value if parseable, 0 otherwise.
fn resolve_imm_simple(s: &str) -> u16 {
    let s = s.trim_end_matches(',');
    if let Some(hex) = s.to_uppercase().strip_prefix("0X") {
        return u16::from_str_radix(hex, 16).unwrap_or(0);
    }
    if let Ok(v) = s.parse::<u16>() {
        return v;
    }
    if let Ok(v) = s.parse::<i16>() {
        return v as u16;
    }
    0
}

fn parse_reg(s: &str, lineno: usize) -> Result<u8, String> {
    match s {
        "R0" => Ok(0),
        "R1" => Ok(1),
        "R2" => Ok(2),
        "R3" => Ok(3),
        _ => Err(format!("Line {}: Unknown register '{}'", lineno, s)),
    }
}

fn resolve_imm(s: &str, syms: &HashMap<String, u16>, lineno: usize) -> Result<u16, String> {
    if let Some(v) = syms.get(s) {
        return Ok(*v);
    }
    if let Some(hex) = s.strip_prefix("0X") {
        return u16::from_str_radix(hex, 16)
            .map_err(|_| format!("Line {}: Bad hex literal '{}'", lineno, s));
    }
    // Try unsigned u16 first
    if let Ok(v) = s.parse::<u16>() {
        return Ok(v);
    }
    // Try signed i16 — handles -1, -3, etc. (two's complement reinterpret)
    if let Ok(v) = s.parse::<i16>() {
        return Ok(v as u16);
    }
    Err(format!(
        "Line {}: Cannot resolve '{}' as immediate or label",
        lineno, s
    ))
}

// ── Assembler binary entry point ──────────────────────────────────────────────

pub mod cli {
    use super::*;
    use std::path::Path;

    pub fn run(src_path: &Path, out_path: &Path) -> Result<(), String> {
        let source = std::fs::read_to_string(src_path)
            .map_err(|e| format!("Cannot read '{}': {}", src_path.display(), e))?;

        let asm = Assembler::new(crate::cpu::PROG_BASE);
        let output = asm.assemble(&source)?;

        // Convert words → bytes (little-endian)
        let bytes: Vec<u8> = output.words.iter().flat_map(|w| w.to_le_bytes()).collect();

        std::fs::write(out_path, &bytes)
            .map_err(|e| format!("Cannot write '{}': {}", out_path.display(), e))?;

        println!(
            "Assembled {} words ({} bytes) → {}",
            output.words.len(),
            bytes.len(),
            out_path.display()
        );
        println!("Symbols:");
        let mut syms: Vec<_> = output.symbols.iter().collect();
        syms.sort_by_key(|(_, v)| *v);
        for (name, addr) in syms {
            println!("  {:20} = 0x{:04X}", name, addr);
        }
        Ok(())
    }
}
