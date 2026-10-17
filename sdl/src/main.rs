mod emulator_sdl2;

use std::env;
use std::fs;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use emuchip_8::Chip8;
use emulator_sdl2::EmulatorSdl2;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom_file>", args[0]);
        process::exit(1);
    }

    let rom = fs::read(&args[1]).expect("Failed to read ROM file");
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);

    let result = Chip8::new(&rom, seed).and_then(|chip8| EmulatorSdl2::new(chip8).run());
    if let Err(e) = result {
        eprintln!("{}: {e}", args[1]);
        process::exit(1);
    }
}
