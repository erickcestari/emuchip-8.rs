pub mod audio;
pub mod cpu;
pub mod emulator;
pub mod input;
pub mod opcode;
pub mod renderer;

use emulator::Emulator;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom_file>", args[0]);
        std::process::exit(1);
    }

    let rom = fs::read(&args[1]).expect("Failed to read ROM file");
    let mut emulator = Emulator::new(&rom);
    emulator.run();
}
