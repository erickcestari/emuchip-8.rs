use core::fmt;

use crate::MAX_ROM_SIZE;

/// Why a ROM could not be loaded or stopped running.
///
/// `pc` is the address of the faulting instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The ROM is empty or larger than [`MAX_ROM_SIZE`].
    InvalidRomSize { len: usize },
    /// The word at `pc` is not a CHIP-8 instruction.
    UnknownOpcode { pc: u16, opcode: u16 },
    /// A call nested deeper than the 16-level stack.
    StackOverflow { pc: u16 },
    /// A return with no call to return from.
    StackUnderflow { pc: u16 },
    /// An access starting at `addr` runs past the end of memory.
    AddressOutOfBounds { pc: u16, addr: u16 },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::InvalidRomSize { len } => {
                write!(f, "ROM is {len} bytes, expected 1 to {MAX_ROM_SIZE}")
            }
            Error::UnknownOpcode { pc, opcode } => {
                write!(f, "unknown opcode {opcode:#06X} at {pc:#05X}")
            }
            Error::StackOverflow { pc } => write!(f, "stack overflow at {pc:#05X}"),
            Error::StackUnderflow { pc } => write!(f, "return with empty stack at {pc:#05X}"),
            Error::AddressOutOfBounds { pc, addr } => {
                write!(f, "access at {addr:#06X} runs past memory at {pc:#05X}")
            }
        }
    }
}

impl core::error::Error for Error {}
