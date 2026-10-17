use crate::cpu::Cpu;
use crate::{CYCLES_PER_FRAME, DISPLAY_HEIGHT, DISPLAY_WIDTH, Error};

/// Register snapshot, for debuggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuState {
    pub v: [u8; 16],
    pub i: u16,
    pub pc: u16,
    pub sp: u8,
    pub delay_timer: u8,
    pub sound_timer: u8,
}

/// A CHIP-8 machine with a ROM loaded. Owns no platform resources: time, input and output are
/// the host's job.
pub struct Chip8 {
    cpu: Cpu,
}

impl Chip8 {
    /// Loads `rom` at 0x200. `seed` drives the random opcode (CXNN); vary it between runs unless
    /// you want reproducible games.
    pub fn new(rom: &[u8], seed: u64) -> Result<Self, Error> {
        let mut cpu = Cpu::new(seed);
        cpu.load_rom(rom)?;
        Ok(Self { cpu })
    }

    /// Runs one 60 Hz frame: [`CYCLES_PER_FRAME`] instructions, then one tick of both timers.
    ///
    /// On error `pc` stays on the faulting instruction and the timers do not tick, so calling
    /// again returns the same error.
    pub fn step_frame(&mut self) -> Result<(), Error> {
        for _ in 0..CYCLES_PER_FRAME {
            self.cpu.step()?;
        }
        self.cpu.tick_timers();
        Ok(())
    }

    /// Presses or releases one of the 16 hex keys.
    ///
    /// # Panics
    ///
    /// If `key` is above 0xF.
    pub fn set_key(&mut self, key: u8, pressed: bool) {
        assert!(key < 16, "CHIP-8 has 16 keys, got {key:#X}");
        self.cpu.keys[key as usize] = pressed;
    }

    /// Row-major framebuffer, one byte per pixel: 1 lit, 0 dark.
    pub fn display(&self) -> &[u8; DISPLAY_WIDTH * DISPLAY_HEIGHT] {
        &self.cpu.display
    }

    /// The host should beep while this is non-zero.
    pub fn sound_timer(&self) -> u8 {
        self.cpu.sound_timer
    }

    pub fn cpu_state(&self) -> CpuState {
        CpuState {
            v: self.cpu.v,
            i: self.cpu.i,
            pc: self.cpu.pc,
            sp: self.cpu.sp,
            delay_timer: self.cpu.delay_timer,
            sound_timer: self.cpu.sound_timer,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MAX_ROM_SIZE;

    #[test]
    fn rejects_empty_and_oversized_roms() {
        assert_eq!(
            Chip8::new(&[], 0).err(),
            Some(Error::InvalidRomSize { len: 0 })
        );
        let too_big = [0; MAX_ROM_SIZE + 1];
        assert_eq!(
            Chip8::new(&too_big, 0).err(),
            Some(Error::InvalidRomSize {
                len: MAX_ROM_SIZE + 1
            })
        );
        assert!(Chip8::new(&[0; MAX_ROM_SIZE], 0).is_ok());
    }

    #[test]
    fn frame_runs_instructions_then_ticks_timers_once() {
        // V0 = 5; DT = V0; loop forever.
        let rom = [0x60, 0x05, 0xF0, 0x15, 0x12, 0x04];
        let mut chip8 = Chip8::new(&rom, 0).unwrap();

        chip8.step_frame().unwrap();

        let state = chip8.cpu_state();
        assert_eq!(state.v[0], 5);
        assert_eq!(state.delay_timer, 4);
        assert_eq!(state.pc, 0x204);
    }

    #[test]
    fn fault_stops_on_the_faulting_instruction_and_repeats() {
        // V0 = 3; return with an empty stack.
        let rom = [0x60, 0x03, 0x00, 0xEE];
        let mut chip8 = Chip8::new(&rom, 0).unwrap();
        let expected = Err(Error::StackUnderflow { pc: 0x202 });

        assert_eq!(chip8.step_frame(), expected);
        assert_eq!(chip8.cpu_state().pc, 0x202);
        assert_eq!(chip8.step_frame(), expected);
    }

    #[test]
    fn unknown_opcode_reports_word_and_address() {
        let mut chip8 = Chip8::new(&[0xFF, 0xFF], 0).unwrap();
        assert_eq!(
            chip8.step_frame(),
            Err(Error::UnknownOpcode {
                pc: 0x200,
                opcode: 0xFFFF
            })
        );
    }

    #[test]
    fn same_seed_gives_same_random_bytes() {
        // V0 = rand & 0xFF; loop forever.
        let rom = [0xC0, 0xFF, 0x12, 0x02];
        let random_byte = |seed| {
            let mut chip8 = Chip8::new(&rom, seed).unwrap();
            chip8.step_frame().unwrap();
            chip8.cpu_state().v[0]
        };

        assert_eq!(random_byte(7), random_byte(7));
        assert_ne!(random_byte(7), random_byte(8));
    }

    #[test]
    fn keys_reach_the_rom() {
        // Spin between 0x202 and 0x204 until key 0xA is down, then loop at 0x206.
        let rom = [0x60, 0x0A, 0xE0, 0x9E, 0x12, 0x02, 0x12, 0x06];
        let mut chip8 = Chip8::new(&rom, 0).unwrap();

        chip8.step_frame().unwrap();
        assert_eq!(chip8.cpu_state().pc, 0x204);

        chip8.set_key(0xA, true);
        chip8.step_frame().unwrap();
        assert_eq!(chip8.cpu_state().pc, 0x206);
    }

    #[test]
    #[should_panic(expected = "16 keys")]
    fn set_key_rejects_keys_past_f() {
        Chip8::new(&[0x12, 0x00], 0).unwrap().set_key(16, true);
    }
}
