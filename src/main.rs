pub mod cpu;
pub mod emulator;
pub mod emulator_sdl2;
pub mod opcode;

use std::env;
use std::fs;

use crate::emulator_sdl2::EmulatorSdl2;
use emulator::Emulator;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom_file>", args[0]);
        std::process::exit(1);
    }

    let rom = fs::read(&args[1]).expect("Failed to read ROM file");
    let mut emulator = EmulatorSdl2::new(&rom);
    emulator.run();
}
