use cpu16::assembler::cli;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: asm <source.asm> <output.bin>");
        std::process::exit(1);
    }
    let src = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    if let Err(e) = cli::run(&src, &out) {
        eprintln!("Assembler error: {}", e);
        std::process::exit(1);
    }
}