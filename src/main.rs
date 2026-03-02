use cpu16::cpu::{Cpu, CpuState};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cpu16 <binary.bin> [--debug] [--max-cycles N]");
        std::process::exit(1);
    }

    let bin_path = PathBuf::from(&args[1]);
    let debug_mode = args.contains(&"--debug".to_string());
    let max_cycles = parse_flag(&args, "--max-cycles").unwrap_or(1_000_000);

    let bytes = std::fs::read(&bin_path).unwrap_or_else(|e| {
        eprintln!("Error reading '{}': {}", bin_path.display(), e);
        std::process::exit(1);
    });

    let mut cpu = Cpu::new();
    cpu.load_program(&bytes);

    println!(
        "cpu16 — Loaded {} bytes from '{}'",
        bytes.len(),
        bin_path.display()
    );
    println!("Running (max {} cycles)...\n", max_cycles);

    if debug_mode {
        run_debug(&mut cpu, max_cycles);
    } else {
        match cpu.run(max_cycles) {
            Ok(CpuState::Halted) => {
                println!("\n─── HALT ───");
                println!("{}", cpu.dump_state());
            }
            Ok(CpuState::Running) => {
                println!("\n─── Max cycles reached ───");
                println!("{}", cpu.dump_state());
            }
            Ok(CpuState::WaitingForInterrupt) => {
                println!("\n─── Waiting for interrupt ───");
                println!("{}", cpu.dump_state());
            }
            Err(e) => {
                eprintln!("\n─── RUNTIME ERROR: {} ───", e);
                eprintln!("{}", cpu.dump_state());
                std::process::exit(1);
            }
        }
    }
}

fn run_debug(cpu: &mut Cpu, max_cycles: u64) {
    loop {
        println!("{}", cpu.dump_state());
        if cpu.cycles >= max_cycles {
            println!("Max cycles reached.");
            break;
        }

        // Step
        match cpu.step() {
            Ok(CpuState::Halted) => {
                println!("─── HALT ───");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("─── ERROR: {} ───", e);
                break;
            }
        }

        // Tiny pause to read output — in a real debugger you'd wait for input
        if cpu.cycles % 10 == 0 {
            println!("  ... {} cycles ...", cpu.cycles);
        }
    }
}

fn parse_flag(args: &[String], flag: &str) -> Option<u64> {
    args.windows(2).find(|w| w[0] == flag)?.get(1)?.parse().ok()
}
