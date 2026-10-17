use core::ops::Range;

use crate::opcode::Opcode;
use crate::{DISPLAY_HEIGHT, DISPLAY_WIDTH, Error, MAX_ROM_SIZE};

const MEMORY_SIZE: usize = 4096;
const REGISTERS_COUNT: usize = 16;
const STACK_DEPTH: usize = 16;
const PROGRAM_START: u16 = 0x200;

const FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

pub struct Cpu {
    // Registers
    pub v: [u8; REGISTERS_COUNT], // V0..VF
    pub i: u16,
    pub pc: u16,

    // Stack
    pub stack: [u16; STACK_DEPTH],
    pub sp: u8,

    // Timers
    pub delay_timer: u8,
    pub sound_timer: u8,

    // Memory
    pub memory: [u8; MEMORY_SIZE],

    // Display
    pub display: [u8; DISPLAY_WIDTH * DISPLAY_HEIGHT],

    // Keypad (16 keys)
    pub keys: [bool; 16],

    // SplitMix64 state behind CXNN
    rng: u64,
}

impl Cpu {
    pub fn new(seed: u64) -> Self {
        let mut cpu = Cpu {
            v: [0; REGISTERS_COUNT],
            i: 0,
            pc: PROGRAM_START,

            stack: [0; STACK_DEPTH],
            sp: 0,

            delay_timer: 0,
            sound_timer: 0,

            memory: [0; MEMORY_SIZE],

            display: [0; DISPLAY_WIDTH * DISPLAY_HEIGHT],
            keys: [false; 16],

            rng: seed,
        };

        // Load fontset at 0x000
        cpu.memory[..FONTSET.len()].copy_from_slice(&FONTSET);

        cpu
    }

    pub fn fetch_opcode(&self) -> Result<u16, Error> {
        let pc = self.pc as usize;
        match self.memory.get(pc..pc + 2) {
            Some(&[hi, lo]) => Ok(u16::from_be_bytes([hi, lo])),
            _ => Err(Error::AddressOutOfBounds {
                pc: self.pc,
                addr: self.pc,
            }),
        }
    }

    /// Runs one instruction. On error `pc` is left on the faulting instruction.
    pub fn step(&mut self) -> Result<(), Error> {
        let pc = self.pc;
        let opcode = self.fetch_opcode()?;
        let decoded = Opcode::decode(opcode).ok_or(Error::UnknownOpcode { pc, opcode })?;
        self.pc += 2;

        let result = self.execute(decoded);
        if result.is_err() {
            self.pc = pc;
        }
        result
    }

    /// Expects `pc` already past the instruction, as [`Cpu::step`] leaves it. Every fault is
    /// detected before any state changes.
    pub fn execute(&mut self, op: Opcode) -> Result<(), Error> {
        match op {
            Opcode::ClearScreen => {
                self.display.fill(0);
            }
            Opcode::Return => {
                if self.sp == 0 {
                    return Err(Error::StackUnderflow {
                        pc: self.instruction_pc(),
                    });
                }
                self.sp -= 1;
                self.pc = self.stack[self.sp as usize];
            }
            Opcode::Jump { addr } => {
                self.pc = addr;
            }
            Opcode::Call { addr } => {
                if self.sp as usize == STACK_DEPTH {
                    return Err(Error::StackOverflow {
                        pc: self.instruction_pc(),
                    });
                }
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = addr;
            }
            Opcode::LoadImm { x, kk } => {
                self.v[x] = kk;
            }
            Opcode::AddImm { x, kk } => {
                self.v[x] = self.v[x].wrapping_add(kk);
            }
            Opcode::LoadReg { x, y } => {
                self.v[x] = self.v[y];
            }
            Opcode::Or { x, y } => {
                self.v[x] |= self.v[y];
            }
            Opcode::And { x, y } => {
                self.v[x] &= self.v[y];
            }
            Opcode::Xor { x, y } => {
                self.v[x] ^= self.v[y];
            }
            Opcode::AddReg { x, y } => {
                let (result, overflow) = self.v[x].overflowing_add(self.v[y]);
                self.v[0xF] = if overflow { 1 } else { 0 };
                self.v[x] = result;
            }
            Opcode::SubReg { x, y } => {
                self.v[0xF] = if self.v[x] >= self.v[y] { 1 } else { 0 };
                self.v[x] = self.v[x].wrapping_sub(self.v[y]);
            }
            Opcode::ShiftRight { x } => {
                self.v[0xF] = self.v[x] & 0x1;
                self.v[x] = self.v[x].wrapping_shr(1);
            }
            Opcode::SubN { x, y } => {
                self.v[0xF] = if self.v[y] >= self.v[x] { 1 } else { 0 };
                self.v[x] = self.v[y].wrapping_sub(self.v[x]);
            }
            Opcode::ShiftLeft { x } => {
                self.v[0xF] = self.v[x] >> 7;
                self.v[x] = self.v[x].wrapping_shl(1);
            }
            Opcode::SkipNeqReg { x, y } => {
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
            }
            Opcode::LoadI { addr } => {
                self.i = addr;
            }
            Opcode::JumpV0 { addr } => {
                self.pc = addr.wrapping_add(self.v[0] as u16);
            }
            Opcode::Random { x, nn } => {
                self.v[x] = self.next_random() & nn;
            }
            Opcode::Draw { x, y, n } => {
                let sprite = self.i_range(n as usize)?;
                let x_pos = self.v[x] as usize % DISPLAY_WIDTH;
                let y_pos = self.v[y] as usize % DISPLAY_HEIGHT;
                self.v[0xF] = 0;

                for (row, addr) in sprite.enumerate() {
                    let sprite_byte = self.memory[addr];
                    let y_coord = (y_pos + row) % DISPLAY_HEIGHT;

                    for col in 0..8 {
                        let x_coord = (x_pos + col) % DISPLAY_WIDTH;
                        let sprite_pixel = (sprite_byte >> (7 - col)) & 1;

                        if sprite_pixel == 1 {
                            let idx = y_coord * DISPLAY_WIDTH + x_coord;
                            if self.display[idx] == 1 {
                                self.v[0xF] = 1;
                            }
                            self.display[idx] ^= 1;
                        }
                    }
                }
            }
            // Only the low nibble of VX names a key, as on the COSMAC VIP.
            Opcode::SkipKey { x } => {
                if self.keys[(self.v[x] & 0x0F) as usize] {
                    self.pc += 2;
                }
            }
            Opcode::SkipNotKey { x } => {
                if !self.keys[(self.v[x] & 0x0F) as usize] {
                    self.pc += 2;
                }
            }
            Opcode::LoadDelayTimer { x } => {
                self.v[x] = self.delay_timer;
            }
            Opcode::GetKey { x } => {
                if let Some(key) = self.keys.iter().position(|&pressed| pressed) {
                    self.v[x] = key as u8;
                } else {
                    // No key pressed, go back and wait
                    self.pc -= 2;
                }
            }
            Opcode::SetDelayTimer { x } => {
                self.delay_timer = self.v[x];
            }
            Opcode::SetSoundTimer { x } => {
                self.sound_timer = self.v[x];
            }
            Opcode::AddI { x } => {
                self.i = self.i.wrapping_add(self.v[x] as u16);
            }
            Opcode::LoadSprite { x } => {
                // Each font sprite is 5 bytes, starting at 0x000
                self.i = (self.v[x] as u16 & 0x0F) * 5;
            }
            Opcode::StoreBCD { x } => {
                let value = self.v[x];
                let dst = self.i_range(3)?;
                self.memory[dst].copy_from_slice(&[value / 100, (value / 10) % 10, value % 10]);
            }
            Opcode::RegDump { x } => {
                let dst = self.i_range(x + 1)?;
                self.memory[dst].copy_from_slice(&self.v[..=x]);
            }
            Opcode::RegLoad { x } => {
                let src = self.i_range(x + 1)?;
                self.v[..=x].copy_from_slice(&self.memory[src]);
            }
            Opcode::SkipEqImm { x, kk } => {
                if self.v[x] == kk {
                    self.pc += 2;
                }
            }
            Opcode::SkipDifImm { x, kk } => {
                if self.v[x] != kk {
                    self.pc += 2;
                }
            }
            Opcode::SkipEq { x, y } => {
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            }
        }
        Ok(())
    }

    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    pub fn load_rom(&mut self, data: &[u8]) -> Result<(), Error> {
        if data.is_empty() || data.len() > MAX_ROM_SIZE {
            return Err(Error::InvalidRomSize { len: data.len() });
        }
        let start = PROGRAM_START as usize;
        self.memory[start..start + data.len()].copy_from_slice(data);
        Ok(())
    }

    /// Address of the instruction `execute` is running.
    fn instruction_pc(&self) -> u16 {
        self.pc.wrapping_sub(2)
    }

    /// The `len` bytes at `I`, or the fault for touching memory past 0xFFF.
    fn i_range(&self, len: usize) -> Result<Range<usize>, Error> {
        let start = self.i as usize;
        if start + len > MEMORY_SIZE {
            return Err(Error::AddressOutOfBounds {
                pc: self.instruction_pc(),
                addr: self.i,
            });
        }
        Ok(start..start + len)
    }

    /// SplitMix64: tiny, and every seed, zero included, starts a full-period sequence.
    fn next_random(&mut self) -> u8 {
        self.rng = self.rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (z ^ (z >> 31)) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcode::Opcode;

    // ==================== Initialization Tests ====================

    #[test]
    fn cpu_initializes_correctly() {
        let cpu = Cpu::new(0);

        assert_eq!(cpu.v, [0; REGISTERS_COUNT]);
        assert_eq!(cpu.i, 0);
        assert_eq!(cpu.pc, PROGRAM_START);
        assert_eq!(cpu.stack, [0; STACK_DEPTH]);
        assert_eq!(cpu.sp, 0);
        assert_eq!(cpu.delay_timer, 0);
        assert_eq!(cpu.sound_timer, 0);
        assert!(cpu.display.iter().all(|&p| p == 0));
        assert!(cpu.keys.iter().all(|&k| !k));
    }

    #[test]
    fn fontset_is_loaded() {
        let cpu = Cpu::new(0);
        assert_eq!(&cpu.memory[0..FONTSET.len()], &FONTSET);
    }

    #[test]
    fn fetch_opcode_reads_correctly() {
        let mut cpu = Cpu::new(0);
        cpu.memory[0x200] = 0x12;
        cpu.memory[0x201] = 0x34;

        let op = cpu.fetch_opcode().unwrap();
        assert_eq!(op, 0x1234);
    }

    #[test]
    fn step_increments_pc() {
        let mut cpu = Cpu::new(0);
        cpu.memory[0x200] = 0x00;
        cpu.memory[0x201] = 0xE0;

        let old_pc = cpu.pc;
        cpu.step().unwrap();
        assert_eq!(cpu.pc, old_pc + 2);
    }

    // ==================== 00E0 - ClearScreen ====================

    #[test]
    fn opcode_clear_screen() {
        let mut cpu = Cpu::new(0);
        cpu.display.fill(1);

        cpu.execute(Opcode::ClearScreen).unwrap();

        assert!(cpu.display.iter().all(|&p| p == 0));
    }

    #[test]
    fn opcode_clear_screen_partial_display() {
        let mut cpu = Cpu::new(0);
        // Set some random pixels
        cpu.display[0] = 1;
        cpu.display[100] = 1;
        cpu.display[500] = 1;

        cpu.execute(Opcode::ClearScreen).unwrap();

        assert!(cpu.display.iter().all(|&p| p == 0));
    }

    // ==================== 00EE - Return ====================

    #[test]
    fn opcode_return_pops_stack() {
        let mut cpu = Cpu::new(0);
        cpu.stack[0] = 0x222;
        cpu.sp = 1;

        cpu.execute(Opcode::Return).unwrap();

        assert_eq!(cpu.sp, 0);
        assert_eq!(cpu.pc, 0x222);
    }

    #[test]
    fn opcode_return_nested_calls() {
        let mut cpu = Cpu::new(0);
        cpu.stack[0] = 0x200;
        cpu.stack[1] = 0x300;
        cpu.stack[2] = 0x400;
        cpu.sp = 3;

        cpu.execute(Opcode::Return).unwrap();
        assert_eq!(cpu.pc, 0x400);
        assert_eq!(cpu.sp, 2);

        cpu.execute(Opcode::Return).unwrap();
        assert_eq!(cpu.pc, 0x300);
        assert_eq!(cpu.sp, 1);

        cpu.execute(Opcode::Return).unwrap();
        assert_eq!(cpu.pc, 0x200);
        assert_eq!(cpu.sp, 0);
    }

    // ==================== 1NNN - Jump ====================

    #[test]
    fn opcode_jump_changes_pc() {
        let mut cpu = Cpu::new(0);

        cpu.execute(Opcode::Jump { addr: 0x300 }).unwrap();
        assert_eq!(cpu.pc, 0x300);
    }

    #[test]
    fn opcode_jump_to_start() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x500;

        cpu.execute(Opcode::Jump { addr: 0x200 }).unwrap();
        assert_eq!(cpu.pc, 0x200);
    }

    #[test]
    fn opcode_jump_to_end_of_memory() {
        let mut cpu = Cpu::new(0);

        cpu.execute(Opcode::Jump { addr: 0xFFE }).unwrap();
        assert_eq!(cpu.pc, 0xFFE);
    }

    // ==================== 2NNN - Call ====================

    #[test]
    fn opcode_call_pushes_stack_and_jumps() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x222;

        cpu.execute(Opcode::Call { addr: 0x456 }).unwrap();

        assert_eq!(cpu.stack[0], 0x222);
        assert_eq!(cpu.sp, 1);
        assert_eq!(cpu.pc, 0x456);
    }

    #[test]
    fn opcode_call_multiple_nested() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;

        cpu.execute(Opcode::Call { addr: 0x300 }).unwrap();
        assert_eq!(cpu.stack[0], 0x200);
        assert_eq!(cpu.sp, 1);
        assert_eq!(cpu.pc, 0x300);

        cpu.execute(Opcode::Call { addr: 0x400 }).unwrap();
        assert_eq!(cpu.stack[1], 0x300);
        assert_eq!(cpu.sp, 2);
        assert_eq!(cpu.pc, 0x400);
    }

    // ==================== 3XNN - SkipEqImm ====================

    #[test]
    fn opcode_skip_eq_imm_skips_when_equal() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[5] = 0x42;

        cpu.execute(Opcode::SkipEqImm { x: 5, kk: 0x42 }).unwrap();

        assert_eq!(cpu.pc, 0x202);
    }

    #[test]
    fn opcode_skip_eq_imm_no_skip_when_not_equal() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[5] = 0x42;

        cpu.execute(Opcode::SkipEqImm { x: 5, kk: 0x43 }).unwrap();

        assert_eq!(cpu.pc, 0x200);
    }

    #[test]
    fn opcode_skip_eq_imm_zero_values() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[0] = 0x00;

        cpu.execute(Opcode::SkipEqImm { x: 0, kk: 0x00 }).unwrap();

        assert_eq!(cpu.pc, 0x202);
    }

    // ==================== 4XNN - SkipDifImm ====================

    #[test]
    fn opcode_skip_dif_imm_skips_when_not_equal() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[3] = 0x10;

        cpu.execute(Opcode::SkipDifImm { x: 3, kk: 0x20 }).unwrap();

        assert_eq!(cpu.pc, 0x202);
    }

    #[test]
    fn opcode_skip_dif_imm_no_skip_when_equal() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[3] = 0x10;

        cpu.execute(Opcode::SkipDifImm { x: 3, kk: 0x10 }).unwrap();

        assert_eq!(cpu.pc, 0x200);
    }

    // ==================== 5XY0 - SkipEq ====================

    #[test]
    fn opcode_skip_eq_skips_when_registers_equal() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[1] = 0x55;
        cpu.v[2] = 0x55;

        cpu.execute(Opcode::SkipEq { x: 1, y: 2 }).unwrap();

        assert_eq!(cpu.pc, 0x202);
    }

    #[test]
    fn opcode_skip_eq_no_skip_when_registers_not_equal() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[1] = 0x55;
        cpu.v[2] = 0x66;

        cpu.execute(Opcode::SkipEq { x: 1, y: 2 }).unwrap();

        assert_eq!(cpu.pc, 0x200);
    }

    #[test]
    fn opcode_skip_eq_same_register() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[5] = 0xAB;

        cpu.execute(Opcode::SkipEq { x: 5, y: 5 }).unwrap();

        assert_eq!(cpu.pc, 0x202);
    }

    // ==================== 6XNN - LoadImm ====================

    #[test]
    fn opcode_load_imm_sets_register() {
        let mut cpu = Cpu::new(0);
        cpu.execute(Opcode::LoadImm { x: 3, kk: 0xAB }).unwrap();
        assert_eq!(cpu.v[3], 0xAB);
    }

    #[test]
    fn opcode_load_imm_all_registers() {
        let mut cpu = Cpu::new(0);

        for i in 0..16 {
            cpu.execute(Opcode::LoadImm {
                x: i,
                kk: i as u8 * 10,
            })
            .unwrap();
        }

        for i in 0..16 {
            assert_eq!(cpu.v[i], i as u8 * 10);
        }
    }

    #[test]
    fn opcode_load_imm_overwrites() {
        let mut cpu = Cpu::new(0);
        cpu.v[7] = 0xFF;

        cpu.execute(Opcode::LoadImm { x: 7, kk: 0x00 }).unwrap();

        assert_eq!(cpu.v[7], 0x00);
    }

    // ==================== 7XNN - AddImm ====================

    #[test]
    fn opcode_add_imm_basic() {
        let mut cpu = Cpu::new(0);
        cpu.v[2] = 10;

        cpu.execute(Opcode::AddImm { x: 2, kk: 5 }).unwrap();

        assert_eq!(cpu.v[2], 15);
    }

    #[test]
    fn opcode_add_imm_wraps() {
        let mut cpu = Cpu::new(0);
        cpu.v[2] = 250;

        cpu.execute(Opcode::AddImm { x: 2, kk: 10 }).unwrap();

        assert_eq!(cpu.v[2], 4); // 260 % 256 = 4
    }

    #[test]
    fn opcode_add_imm_no_carry_flag() {
        let mut cpu = Cpu::new(0);
        cpu.v[2] = 250;
        cpu.v[0xF] = 0;

        cpu.execute(Opcode::AddImm { x: 2, kk: 10 }).unwrap();

        // VF should NOT be affected by AddImm
        assert_eq!(cpu.v[0xF], 0);
    }

    #[test]
    fn opcode_add_imm_zero() {
        let mut cpu = Cpu::new(0);
        cpu.v[5] = 100;

        cpu.execute(Opcode::AddImm { x: 5, kk: 0 }).unwrap();

        assert_eq!(cpu.v[5], 100);
    }

    // ==================== 8XY0 - LoadReg ====================

    #[test]
    fn opcode_load_reg() {
        let mut cpu = Cpu::new(0);
        cpu.v[1] = 0x42;
        cpu.v[2] = 0x00;

        cpu.execute(Opcode::LoadReg { x: 2, y: 1 }).unwrap();

        assert_eq!(cpu.v[2], 0x42);
        assert_eq!(cpu.v[1], 0x42); // Source unchanged
    }

    #[test]
    fn opcode_load_reg_same_register() {
        let mut cpu = Cpu::new(0);
        cpu.v[5] = 0xAB;

        cpu.execute(Opcode::LoadReg { x: 5, y: 5 }).unwrap();

        assert_eq!(cpu.v[5], 0xAB);
    }

    // ==================== 8XY1 - Or ====================

    #[test]
    fn opcode_or() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0xF0;
        cpu.v[1] = 0x0F;

        cpu.execute(Opcode::Or { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 0xFF);
    }

    #[test]
    fn opcode_or_with_zero() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0xAB;
        cpu.v[1] = 0x00;

        cpu.execute(Opcode::Or { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 0xAB);
    }

    #[test]
    fn opcode_or_same_value() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0x55;
        cpu.v[1] = 0x55;

        cpu.execute(Opcode::Or { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 0x55);
    }

    // ==================== 8XY2 - And ====================

    #[test]
    fn opcode_and() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0xF0;
        cpu.v[1] = 0x0F;

        cpu.execute(Opcode::And { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 0x00);
    }

    #[test]
    fn opcode_and_all_ones() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0xFF;
        cpu.v[1] = 0xFF;

        cpu.execute(Opcode::And { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 0xFF);
    }

    #[test]
    fn opcode_and_partial() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0b10101010;
        cpu.v[1] = 0b11110000;

        cpu.execute(Opcode::And { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 0b10100000);
    }

    // ==================== 8XY3 - Xor ====================

    #[test]
    fn opcode_xor() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0xFF;
        cpu.v[1] = 0x0F;

        cpu.execute(Opcode::Xor { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 0xF0);
    }

    #[test]
    fn opcode_xor_same_value_gives_zero() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0xAB;
        cpu.v[1] = 0xAB;

        cpu.execute(Opcode::Xor { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 0x00);
    }

    #[test]
    fn opcode_xor_with_zero() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0x55;
        cpu.v[1] = 0x00;

        cpu.execute(Opcode::Xor { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 0x55);
    }

    // ==================== 8XY4 - AddReg ====================

    #[test]
    fn opcode_add_reg_no_overflow() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 10;
        cpu.v[1] = 20;

        cpu.execute(Opcode::AddReg { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 30);
        assert_eq!(cpu.v[0xF], 0);
    }

    #[test]
    fn opcode_add_reg_with_overflow() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 200;
        cpu.v[1] = 100;

        cpu.execute(Opcode::AddReg { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 44); // 300 % 256 = 44
        assert_eq!(cpu.v[0xF], 1);
    }

    #[test]
    fn opcode_add_reg_exact_overflow() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 255;
        cpu.v[1] = 1;

        cpu.execute(Opcode::AddReg { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 0);
        assert_eq!(cpu.v[0xF], 1);
    }

    #[test]
    fn opcode_add_reg_vf_as_operand() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 5;
        cpu.v[0xF] = 10;

        cpu.execute(Opcode::AddReg { x: 0, y: 0xF }).unwrap();

        assert_eq!(cpu.v[0], 15);
        assert_eq!(cpu.v[0xF], 0); // VF is set to carry flag
    }

    // ==================== 8XY5 - SubReg ====================

    #[test]
    fn opcode_sub_reg_no_borrow() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 30;
        cpu.v[1] = 10;

        cpu.execute(Opcode::SubReg { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 20);
        assert_eq!(cpu.v[0xF], 1); // No borrow
    }

    #[test]
    fn opcode_sub_reg_with_borrow() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 10;
        cpu.v[1] = 30;

        cpu.execute(Opcode::SubReg { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 236); // Wrapping subtraction
        assert_eq!(cpu.v[0xF], 0); // Borrow occurred
    }

    #[test]
    fn opcode_sub_reg_equal_values() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 50;
        cpu.v[1] = 50;

        cpu.execute(Opcode::SubReg { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 0);
        assert_eq!(cpu.v[0xF], 1); // VX >= VY, so no borrow
    }

    // ==================== 8XY6 - ShiftRight ====================

    #[test]
    fn opcode_shift_right_lsb_zero() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0b11110000;

        cpu.execute(Opcode::ShiftRight { x: 0 }).unwrap();

        assert_eq!(cpu.v[0], 0b01111000);
        assert_eq!(cpu.v[0xF], 0);
    }

    #[test]
    fn opcode_shift_right_lsb_one() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0b11110001;

        cpu.execute(Opcode::ShiftRight { x: 0 }).unwrap();

        assert_eq!(cpu.v[0], 0b01111000);
        assert_eq!(cpu.v[0xF], 1);
    }

    #[test]
    fn opcode_shift_right_one() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 1;

        cpu.execute(Opcode::ShiftRight { x: 0 }).unwrap();

        assert_eq!(cpu.v[0], 0);
        assert_eq!(cpu.v[0xF], 1);
    }

    // ==================== 8XY7 - SubN ====================

    #[test]
    fn opcode_subn_no_borrow() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 10;
        cpu.v[1] = 30;

        cpu.execute(Opcode::SubN { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 20); // VY - VX = 30 - 10
        assert_eq!(cpu.v[0xF], 1); // No borrow
    }

    #[test]
    fn opcode_subn_with_borrow() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 30;
        cpu.v[1] = 10;

        cpu.execute(Opcode::SubN { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 236); // Wrapping
        assert_eq!(cpu.v[0xF], 0); // Borrow
    }

    #[test]
    fn opcode_subn_equal_values() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 50;
        cpu.v[1] = 50;

        cpu.execute(Opcode::SubN { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 0);
        assert_eq!(cpu.v[0xF], 1); // VY >= VX
    }

    // ==================== 8XYE - ShiftLeft ====================

    #[test]
    fn opcode_shift_left_msb_zero() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0b00001111;

        cpu.execute(Opcode::ShiftLeft { x: 0 }).unwrap();

        assert_eq!(cpu.v[0], 0b00011110);
        assert_eq!(cpu.v[0xF], 0);
    }

    #[test]
    fn opcode_shift_left_msb_one() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0b10001111;

        cpu.execute(Opcode::ShiftLeft { x: 0 }).unwrap();

        assert_eq!(cpu.v[0], 0b00011110);
        assert_eq!(cpu.v[0xF], 1);
    }

    #[test]
    fn opcode_shift_left_0x80() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0x80;

        cpu.execute(Opcode::ShiftLeft { x: 0 }).unwrap();

        assert_eq!(cpu.v[0], 0);
        assert_eq!(cpu.v[0xF], 1);
    }

    // ==================== 9XY0 - SkipNeqReg ====================

    #[test]
    fn opcode_skip_neq_reg_skips_when_not_equal() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[1] = 0x10;
        cpu.v[2] = 0x20;

        cpu.execute(Opcode::SkipNeqReg { x: 1, y: 2 }).unwrap();

        assert_eq!(cpu.pc, 0x202);
    }

    #[test]
    fn opcode_skip_neq_reg_no_skip_when_equal() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[1] = 0x10;
        cpu.v[2] = 0x10;

        cpu.execute(Opcode::SkipNeqReg { x: 1, y: 2 }).unwrap();

        assert_eq!(cpu.pc, 0x200);
    }

    // ==================== ANNN - LoadI ====================

    #[test]
    fn opcode_load_i() {
        let mut cpu = Cpu::new(0);

        cpu.execute(Opcode::LoadI { addr: 0x500 }).unwrap();

        assert_eq!(cpu.i, 0x500);
    }

    #[test]
    fn opcode_load_i_max_value() {
        let mut cpu = Cpu::new(0);

        cpu.execute(Opcode::LoadI { addr: 0xFFF }).unwrap();

        assert_eq!(cpu.i, 0xFFF);
    }

    #[test]
    fn opcode_load_i_zero() {
        let mut cpu = Cpu::new(0);
        cpu.i = 0x500;

        cpu.execute(Opcode::LoadI { addr: 0x000 }).unwrap();

        assert_eq!(cpu.i, 0x000);
    }

    // ==================== BNNN - JumpV0 ====================

    #[test]
    fn opcode_jump_v0() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0x10;

        cpu.execute(Opcode::JumpV0 { addr: 0x300 }).unwrap();

        assert_eq!(cpu.pc, 0x310);
    }

    #[test]
    fn opcode_jump_v0_zero() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0x00;

        cpu.execute(Opcode::JumpV0 { addr: 0x400 }).unwrap();

        assert_eq!(cpu.pc, 0x400);
    }

    #[test]
    fn opcode_jump_v0_wraps() {
        let mut cpu = Cpu::new(0);
        cpu.v[0] = 0xFF;

        cpu.execute(Opcode::JumpV0 { addr: 0xFFF }).unwrap();

        // 0xFFF + 0xFF = 0x10FE: past memory, so the next fetch faults
        assert_eq!(cpu.pc, 0x10FE);
    }

    // ==================== CXNN - Random ====================

    #[test]
    fn opcode_random_mask() {
        let mut cpu = Cpu::new(0);

        // Run multiple times to test randomness with mask
        for _ in 0..100 {
            cpu.execute(Opcode::Random { x: 0, nn: 0x0F }).unwrap();
            assert!(cpu.v[0] <= 0x0F); // Should be masked to lower nibble
        }
    }

    #[test]
    fn opcode_random_zero_mask() {
        let mut cpu = Cpu::new(0);

        cpu.execute(Opcode::Random { x: 0, nn: 0x00 }).unwrap();

        assert_eq!(cpu.v[0], 0x00);
    }

    #[test]
    fn opcode_random_full_mask() {
        let mut cpu = Cpu::new(0);

        // Run multiple times - with 0xFF mask, we should get various values
        let mut seen_different = false;
        let first = {
            cpu.execute(Opcode::Random { x: 0, nn: 0xFF }).unwrap();
            cpu.v[0]
        };

        for _ in 0..100 {
            cpu.execute(Opcode::Random { x: 0, nn: 0xFF }).unwrap();
            if cpu.v[0] != first {
                seen_different = true;
                break;
            }
        }

        assert!(seen_different, "Random should produce different values");
    }

    // ==================== EX9E - SkipKey ====================

    #[test]
    fn opcode_skip_key_pressed() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[0] = 5;
        cpu.keys[5] = true;

        cpu.execute(Opcode::SkipKey { x: 0 }).unwrap();

        assert_eq!(cpu.pc, 0x202);
    }

    #[test]
    fn opcode_skip_key_not_pressed() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[0] = 5;
        cpu.keys[5] = false;

        cpu.execute(Opcode::SkipKey { x: 0 }).unwrap();

        assert_eq!(cpu.pc, 0x200);
    }

    #[test]
    fn opcode_skip_key_all_keys() {
        let mut cpu = Cpu::new(0);

        for key in 0..16u8 {
            cpu.pc = 0x200;
            cpu.v[0] = key;
            cpu.keys = [false; 16];
            cpu.keys[key as usize] = true;

            cpu.execute(Opcode::SkipKey { x: 0 }).unwrap();
            assert_eq!(cpu.pc, 0x202, "Key {} should cause skip", key);
        }
    }

    // ==================== EXA1 - SkipNotKey ====================

    #[test]
    fn opcode_skip_not_key_not_pressed() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[0] = 5;
        cpu.keys[5] = false;

        cpu.execute(Opcode::SkipNotKey { x: 0 }).unwrap();

        assert_eq!(cpu.pc, 0x202);
    }

    #[test]
    fn opcode_skip_not_key_pressed() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[0] = 5;
        cpu.keys[5] = true;

        cpu.execute(Opcode::SkipNotKey { x: 0 }).unwrap();

        assert_eq!(cpu.pc, 0x200);
    }

    // ==================== FX07 - LoadDelayTimer ====================

    #[test]
    fn opcode_load_delay_timer() {
        let mut cpu = Cpu::new(0);
        cpu.delay_timer = 60;

        cpu.execute(Opcode::LoadDelayTimer { x: 5 }).unwrap();

        assert_eq!(cpu.v[5], 60);
    }

    #[test]
    fn opcode_load_delay_timer_zero() {
        let mut cpu = Cpu::new(0);
        cpu.delay_timer = 0;
        cpu.v[5] = 0xFF;

        cpu.execute(Opcode::LoadDelayTimer { x: 5 }).unwrap();

        assert_eq!(cpu.v[5], 0);
    }

    // ==================== FX15 - SetDelayTimer ====================

    #[test]
    fn opcode_set_delay_timer() {
        let mut cpu = Cpu::new(0);
        cpu.v[3] = 120;

        cpu.execute(Opcode::SetDelayTimer { x: 3 }).unwrap();

        assert_eq!(cpu.delay_timer, 120);
    }

    #[test]
    fn opcode_set_delay_timer_zero() {
        let mut cpu = Cpu::new(0);
        cpu.delay_timer = 100;
        cpu.v[3] = 0;

        cpu.execute(Opcode::SetDelayTimer { x: 3 }).unwrap();

        assert_eq!(cpu.delay_timer, 0);
    }

    // ==================== FX18 - SetSoundTimer ====================

    #[test]
    fn opcode_set_sound_timer() {
        let mut cpu = Cpu::new(0);
        cpu.v[7] = 30;

        cpu.execute(Opcode::SetSoundTimer { x: 7 }).unwrap();

        assert_eq!(cpu.sound_timer, 30);
    }

    #[test]
    fn opcode_set_sound_timer_zero() {
        let mut cpu = Cpu::new(0);
        cpu.sound_timer = 50;
        cpu.v[7] = 0;

        cpu.execute(Opcode::SetSoundTimer { x: 7 }).unwrap();

        assert_eq!(cpu.sound_timer, 0);
    }

    // ==================== FX1E - AddI ====================

    #[test]
    fn opcode_add_i() {
        let mut cpu = Cpu::new(0);
        cpu.i = 0x100;
        cpu.v[2] = 0x50;

        cpu.execute(Opcode::AddI { x: 2 }).unwrap();

        assert_eq!(cpu.i, 0x150);
    }

    #[test]
    fn opcode_add_i_zero() {
        let mut cpu = Cpu::new(0);
        cpu.i = 0x500;
        cpu.v[0] = 0;

        cpu.execute(Opcode::AddI { x: 0 }).unwrap();

        assert_eq!(cpu.i, 0x500);
    }

    #[test]
    fn opcode_add_i_wraps() {
        let mut cpu = Cpu::new(0);
        cpu.i = 0xFFFF;
        cpu.v[0] = 1;

        cpu.execute(Opcode::AddI { x: 0 }).unwrap();

        assert_eq!(cpu.i, 0x0000); // Wrapping
    }

    // ==================== Timer Tests ====================

    #[test]
    fn timers_tick_down() {
        let mut cpu = Cpu::new(0);
        cpu.delay_timer = 3;
        cpu.sound_timer = 2;

        cpu.tick_timers();
        assert_eq!(cpu.delay_timer, 2);
        assert_eq!(cpu.sound_timer, 1);

        cpu.tick_timers();
        assert_eq!(cpu.delay_timer, 1);
        assert_eq!(cpu.sound_timer, 0);

        cpu.tick_timers();
        assert_eq!(cpu.delay_timer, 0);
        assert_eq!(cpu.sound_timer, 0);
    }

    #[test]
    fn timers_dont_underflow() {
        let mut cpu = Cpu::new(0);
        cpu.delay_timer = 0;
        cpu.sound_timer = 0;

        cpu.tick_timers();

        assert_eq!(cpu.delay_timer, 0);
        assert_eq!(cpu.sound_timer, 0);
    }

    // ==================== Integration Tests ====================

    #[test]
    fn call_and_return_sequence() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;

        // Call subroutine
        cpu.execute(Opcode::Call { addr: 0x400 }).unwrap();
        assert_eq!(cpu.pc, 0x400);
        assert_eq!(cpu.sp, 1);

        // Do some work
        cpu.execute(Opcode::LoadImm { x: 0, kk: 42 }).unwrap();

        // Return
        cpu.execute(Opcode::Return).unwrap();
        assert_eq!(cpu.pc, 0x200);
        assert_eq!(cpu.sp, 0);
        assert_eq!(cpu.v[0], 42); // Work persisted
    }

    #[test]
    fn arithmetic_sequence() {
        let mut cpu = Cpu::new(0);

        // V0 = 10
        cpu.execute(Opcode::LoadImm { x: 0, kk: 10 }).unwrap();
        // V1 = 5
        cpu.execute(Opcode::LoadImm { x: 1, kk: 5 }).unwrap();
        // V0 = V0 + V1 = 15
        cpu.execute(Opcode::AddReg { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 15);
        assert_eq!(cpu.v[0xF], 0); // No overflow

        // V0 = V0 - V1 = 10
        cpu.execute(Opcode::SubReg { x: 0, y: 1 }).unwrap();

        assert_eq!(cpu.v[0], 10);
        assert_eq!(cpu.v[0xF], 1); // No borrow
    }

    #[test]
    fn bitwise_operations_sequence() {
        let mut cpu = Cpu::new(0);

        cpu.execute(Opcode::LoadImm {
            x: 0,
            kk: 0b11110000,
        })
        .unwrap();
        cpu.execute(Opcode::LoadImm {
            x: 1,
            kk: 0b10101010,
        })
        .unwrap();

        // V2 = V0 (copy)
        cpu.execute(Opcode::LoadReg { x: 2, y: 0 }).unwrap();

        // V2 = V2 & V1
        cpu.execute(Opcode::And { x: 2, y: 1 }).unwrap();
        assert_eq!(cpu.v[2], 0b10100000);

        // V3 = V0 (copy)
        cpu.execute(Opcode::LoadReg { x: 3, y: 0 }).unwrap();

        // V3 = V3 | V1
        cpu.execute(Opcode::Or { x: 3, y: 1 }).unwrap();
        assert_eq!(cpu.v[3], 0b11111010);

        // V4 = V0 (copy)
        cpu.execute(Opcode::LoadReg { x: 4, y: 0 }).unwrap();

        // V4 = V4 ^ V1
        cpu.execute(Opcode::Xor { x: 4, y: 1 }).unwrap();
        assert_eq!(cpu.v[4], 0b01011010);
    }

    #[test]
    fn conditional_skip_sequence() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;

        cpu.execute(Opcode::LoadImm { x: 0, kk: 0x42 }).unwrap();
        cpu.execute(Opcode::LoadImm { x: 1, kk: 0x42 }).unwrap();

        // Should skip (equal)
        cpu.execute(Opcode::SkipEq { x: 0, y: 1 }).unwrap();
        assert_eq!(cpu.pc, 0x202);

        // Should not skip (still equal)
        cpu.execute(Opcode::SkipNeqReg { x: 0, y: 1 }).unwrap();
        assert_eq!(cpu.pc, 0x202);

        // Change V1
        cpu.execute(Opcode::LoadImm { x: 1, kk: 0x43 }).unwrap();

        // Should skip (not equal now)
        cpu.execute(Opcode::SkipNeqReg { x: 0, y: 1 }).unwrap();
        assert_eq!(cpu.pc, 0x204);
    }

    // ==================== DXYN - Draw ====================

    #[test]
    fn opcode_draw_xors_sprite_and_flags_collision() {
        let mut cpu = Cpu::new(0);
        cpu.i = 0x300;
        cpu.memory[0x300] = 0b1100_0000;
        cpu.v[0] = 2;
        cpu.v[1] = 3;

        cpu.execute(Opcode::Draw { x: 0, y: 1, n: 1 }).unwrap();
        assert_eq!(cpu.display[3 * DISPLAY_WIDTH + 2], 1);
        assert_eq!(cpu.display[3 * DISPLAY_WIDTH + 3], 1);
        assert_eq!(cpu.v[0xF], 0);

        cpu.execute(Opcode::Draw { x: 0, y: 1, n: 1 }).unwrap();
        assert!(cpu.display.iter().all(|&p| p == 0));
        assert_eq!(cpu.v[0xF], 1);
    }

    #[test]
    fn opcode_draw_wraps_around_the_edges() {
        let mut cpu = Cpu::new(0);
        cpu.i = 0x300;
        cpu.memory[0x300] = 0b1100_0000;
        cpu.memory[0x301] = 0b1100_0000;
        cpu.v[0] = DISPLAY_WIDTH as u8 - 1;
        cpu.v[1] = DISPLAY_HEIGHT as u8 - 1;

        cpu.execute(Opcode::Draw { x: 0, y: 1, n: 2 }).unwrap();

        let last_row = (DISPLAY_HEIGHT - 1) * DISPLAY_WIDTH;
        assert_eq!(cpu.display[last_row + DISPLAY_WIDTH - 1], 1);
        assert_eq!(cpu.display[last_row], 1);
        assert_eq!(cpu.display[DISPLAY_WIDTH - 1], 1);
        assert_eq!(cpu.display[0], 1);
    }

    #[test]
    fn opcode_draw_past_memory_faults_without_drawing() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x202;
        cpu.i = 0xFFE;
        cpu.v[0xF] = 7;

        assert_eq!(
            cpu.execute(Opcode::Draw { x: 0, y: 0, n: 3 }),
            Err(Error::AddressOutOfBounds {
                pc: 0x200,
                addr: 0xFFE
            })
        );
        assert!(cpu.display.iter().all(|&p| p == 0));
        assert_eq!(cpu.v[0xF], 7);
    }

    // ==================== EX9E / EXA1 - key nibble ====================

    #[test]
    fn opcode_skip_key_uses_low_nibble() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x200;
        cpu.v[0] = 0xF3;
        cpu.keys[3] = true;

        cpu.execute(Opcode::SkipKey { x: 0 }).unwrap();
        assert_eq!(cpu.pc, 0x202);

        cpu.execute(Opcode::SkipNotKey { x: 0 }).unwrap();
        assert_eq!(cpu.pc, 0x202);
    }

    // ==================== FX0A - GetKey ====================

    #[test]
    fn opcode_get_key_waits_until_a_key_is_down() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x202;

        cpu.execute(Opcode::GetKey { x: 4 }).unwrap();
        assert_eq!(cpu.pc, 0x200);

        cpu.pc = 0x202;
        cpu.keys[0xB] = true;
        cpu.execute(Opcode::GetKey { x: 4 }).unwrap();
        assert_eq!(cpu.pc, 0x202);
        assert_eq!(cpu.v[4], 0xB);
    }

    // ==================== FX29 - LoadSprite ====================

    #[test]
    fn opcode_load_sprite_points_at_font_glyph() {
        let mut cpu = Cpu::new(0);
        cpu.v[2] = 0xA;

        cpu.execute(Opcode::LoadSprite { x: 2 }).unwrap();

        assert_eq!(cpu.i, 50);
        assert_eq!(&cpu.memory[50..55], &FONTSET[50..55]);
    }

    // ==================== FX33 - StoreBCD ====================

    #[test]
    fn opcode_store_bcd() {
        let mut cpu = Cpu::new(0);
        cpu.i = 0x300;
        cpu.v[1] = 254;

        cpu.execute(Opcode::StoreBCD { x: 1 }).unwrap();

        assert_eq!(&cpu.memory[0x300..0x303], &[2, 5, 4]);
    }

    #[test]
    fn opcode_store_bcd_past_memory_faults() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x202;
        cpu.i = 0xFFE;

        assert_eq!(
            cpu.execute(Opcode::StoreBCD { x: 0 }),
            Err(Error::AddressOutOfBounds {
                pc: 0x200,
                addr: 0xFFE
            })
        );
    }

    // ==================== FX55 / FX65 - RegDump / RegLoad ====================

    #[test]
    fn opcode_reg_dump_and_load_round_trip_v0_to_vx() {
        let mut cpu = Cpu::new(0);
        cpu.i = 0x300;
        cpu.v[..4].copy_from_slice(&[1, 2, 3, 4]);

        cpu.execute(Opcode::RegDump { x: 2 }).unwrap();
        assert_eq!(&cpu.memory[0x300..0x304], &[1, 2, 3, 0]);

        cpu.v = [0; REGISTERS_COUNT];
        cpu.execute(Opcode::RegLoad { x: 2 }).unwrap();
        assert_eq!(&cpu.v[..4], &[1, 2, 3, 0]);
    }

    #[test]
    fn opcode_reg_dump_past_memory_faults_without_writing() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x202;
        cpu.i = 0xFFF;
        cpu.v[0] = 9;

        assert!(cpu.execute(Opcode::RegDump { x: 1 }).is_err());
        assert_eq!(cpu.memory[0xFFF], 0);
    }

    // ==================== Faults ====================

    #[test]
    fn call_past_stack_depth_faults() {
        let mut cpu = Cpu::new(0);
        for _ in 0..STACK_DEPTH {
            cpu.execute(Opcode::Call { addr: 0x300 }).unwrap();
        }
        cpu.pc = 0x302;

        assert_eq!(
            cpu.execute(Opcode::Call { addr: 0x300 }),
            Err(Error::StackOverflow { pc: 0x300 })
        );
        assert_eq!(cpu.sp as usize, STACK_DEPTH);
    }

    #[test]
    fn return_with_empty_stack_faults() {
        let mut cpu = Cpu::new(0);
        cpu.pc = 0x202;

        assert_eq!(
            cpu.execute(Opcode::Return),
            Err(Error::StackUnderflow { pc: 0x200 })
        );
    }

    #[test]
    fn step_fetching_past_memory_faults() {
        let mut cpu = Cpu::new(0);
        cpu.execute(Opcode::Jump { addr: 0xFFF }).unwrap();

        assert_eq!(
            cpu.step(),
            Err(Error::AddressOutOfBounds {
                pc: 0xFFF,
                addr: 0xFFF
            })
        );
    }

    #[test]
    fn step_rewinds_pc_on_fault() {
        let mut cpu = Cpu::new(0);
        cpu.memory[0x200..0x202].copy_from_slice(&[0x00, 0xEE]);

        assert!(cpu.step().is_err());
        assert_eq!(cpu.pc, 0x200);
    }

    #[test]
    fn load_rom_places_program_at_0x200() {
        let mut cpu = Cpu::new(0);
        cpu.load_rom(&[0xAB, 0xCD]).unwrap();
        assert_eq!(&cpu.memory[0x200..0x202], &[0xAB, 0xCD]);
    }
}
