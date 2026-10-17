//! CHIP-8 interpreter core with no platform code: the host feeds key state, calls
//! [`Chip8::step_frame`] at 60 Hz, draws [`Chip8::display`] and beeps while
//! [`Chip8::sound_timer`] is non-zero. `no_std` and dependency-free, so it builds for any target,
//! `wasm32-unknown-unknown` included.
//!
//! ```
//! use emuchip_8::{Chip8, DISPLAY_HEIGHT, DISPLAY_WIDTH};
//!
//! let rom = [0x12, 0x00]; // 0x200: jump to 0x200
//! let mut chip8 = Chip8::new(&rom, 42)?;
//! chip8.set_key(0x5, true);
//! chip8.step_frame()?;
//! assert_eq!(chip8.display().len(), DISPLAY_WIDTH * DISPLAY_HEIGHT);
//! # Ok::<(), emuchip_8::Error>(())
//! ```

#![no_std]

mod chip8;
mod cpu;
mod error;
mod opcode;

pub use chip8::{Chip8, CpuState};
pub use error::Error;

pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;

/// Instructions per [`Chip8::step_frame`]: 600 Hz at 60 frames per second.
pub const CYCLES_PER_FRAME: u32 = 10;

/// Bytes between the program start (0x200) and the end of the 4 KiB memory.
pub const MAX_ROM_SIZE: usize = 4096 - 0x200;
