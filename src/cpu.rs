use crate::opcode::Opcode;

const MEMORY_SIZE: usize = 4096;
const REGISTERS_COUNT: usize = 16;
const STACK_DEPTH: usize = 16;
const DISPLAY_WIDTH: usize = 64;
const DISPLAY_HEIGHT: usize = 32;
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
}

impl Cpu {
    pub fn new() -> Self {
        // Standard CHIP-8 fontset
        let mut cpu = Cpu {
            v: [0; REGISTERS_COUNT],
            i: 0,
            pc: PROGRAM_START as u16,

            stack: [0; STACK_DEPTH],
            sp: 0,

            delay_timer: 0,
            sound_timer: 0,

            memory: [0; MEMORY_SIZE],

            display: [0; DISPLAY_WIDTH * DISPLAY_HEIGHT],
            keys: [false; 16],
        };

        // Load fontset at 0x000
        cpu.memory[..FONTSET.len()].copy_from_slice(&FONTSET);

        cpu
    }

    pub fn fetch_opcode(&self) -> u16 {
        u16::from_be_bytes([
            self.memory[self.pc as usize],
            self.memory[self.pc as usize + 1],
        ])
    }

    pub fn step(&mut self) {
        let opcode = self.fetch_opcode();
        self.pc += 2;

        let decoded = Opcode::decode(opcode);
        self.execute(decoded);
    }

    pub fn execute(&mut self, op: Opcode) {
        match op {
            Opcode::ClearScreen => {
                self.display.fill(0);
            }
            Opcode::Return => {
                self.sp -= 1;
                self.pc = self.stack[self.sp as usize];
            }
            Opcode::Jump { addr } => {
                self.pc = addr;
            }
            Opcode::Call { addr } => {
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
            Opcode::SkipNeqReq { x, y } => {
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
                self.v[x] = rand::random::<u8>() & nn;
            }
            Opcode::Draw { x, y, n } => {
                unreachable!();
            }
            Opcode::SkipKey { x } => {
                if self.keys[self.v[x] as usize] {
                    self.pc += 2;
                }
            }
            Opcode::SkipNotKey { x } => {
                if !self.keys[self.v[x] as usize] {
                    self.pc += 2;
                }
            }
            Opcode::LoadDelayTimer { x } => {
                self.v[x] = self.delay_timer;
            }
            Opcode::GetKey { x } => {
                unreachable!()
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
                unreachable!()
            }
            Opcode::StoreBCD { x } => {
                unreachable!()
            }
            Opcode::RegDump { x } => {
                unreachable!()
            }
            Opcode::RegLoad { x } => {
                unreachable!()
            }
            Opcode::SkipEqImm { x, conditon } => {
                if self.v[x] == conditon {
                    self.pc += 2;
                }
            }
            Opcode::SkipDifImm { x, conditon } => {
                if self.v[x] != conditon {
                    self.pc += 2;
                }
            }
            Opcode::SkipEq { x, y } => {
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            }
        }
    }

    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcode::Opcode;

    // ==================== Initialization Tests ====================

    #[test]
    fn cpu_initializes_correctly() {
        let cpu = Cpu::new();

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
        let cpu = Cpu::new();
        assert_eq!(&cpu.memory[0..FONTSET.len()], &FONTSET);
    }

    #[test]
    fn fetch_opcode_reads_correctly() {
        let mut cpu = Cpu::new();
        cpu.memory[0x200] = 0x12;
        cpu.memory[0x201] = 0x34;

        let op = cpu.fetch_opcode();
        assert_eq!(op, 0x1234);
    }

    #[test]
    fn step_increments_pc() {
        let mut cpu = Cpu::new();
        cpu.memory[0x200] = 0x00;
        cpu.memory[0x201] = 0xE0;

        let old_pc = cpu.pc;
        cpu.step();
        assert_eq!(cpu.pc, old_pc + 2);
    }

    // ==================== 00E0 - ClearScreen ====================

    #[test]
    fn opcode_clear_screen() {
        let mut cpu = Cpu::new();
        cpu.display.fill(1);

        cpu.execute(Opcode::ClearScreen);

        assert!(cpu.display.iter().all(|&p| p == 0));
    }

    #[test]
    fn opcode_clear_screen_partial_display() {
        let mut cpu = Cpu::new();
        // Set some random pixels
        cpu.display[0] = 1;
        cpu.display[100] = 1;
        cpu.display[500] = 1;

        cpu.execute(Opcode::ClearScreen);

        assert!(cpu.display.iter().all(|&p| p == 0));
    }

    // ==================== 00EE - Return ====================

    #[test]
    fn opcode_return_pops_stack() {
        let mut cpu = Cpu::new();
        cpu.stack[0] = 0x222;
        cpu.sp = 1;

        cpu.execute(Opcode::Return);

        assert_eq!(cpu.sp, 0);
        assert_eq!(cpu.pc, 0x222);
    }

    #[test]
    fn opcode_return_nested_calls() {
        let mut cpu = Cpu::new();
        cpu.stack[0] = 0x200;
        cpu.stack[1] = 0x300;
        cpu.stack[2] = 0x400;
        cpu.sp = 3;

        cpu.execute(Opcode::Return);
        assert_eq!(cpu.pc, 0x400);
        assert_eq!(cpu.sp, 2);

        cpu.execute(Opcode::Return);
        assert_eq!(cpu.pc, 0x300);
        assert_eq!(cpu.sp, 1);

        cpu.execute(Opcode::Return);
        assert_eq!(cpu.pc, 0x200);
        assert_eq!(cpu.sp, 0);
    }

    // ==================== 1NNN - Jump ====================

    #[test]
    fn opcode_jump_changes_pc() {
        let mut cpu = Cpu::new();

        cpu.execute(Opcode::Jump { addr: 0x300 });
        assert_eq!(cpu.pc, 0x300);
    }

    #[test]
    fn opcode_jump_to_start() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x500;

        cpu.execute(Opcode::Jump { addr: 0x200 });
        assert_eq!(cpu.pc, 0x200);
    }

    #[test]
    fn opcode_jump_to_end_of_memory() {
        let mut cpu = Cpu::new();

        cpu.execute(Opcode::Jump { addr: 0xFFE });
        assert_eq!(cpu.pc, 0xFFE);
    }

    // ==================== 2NNN - Call ====================

    #[test]
    fn opcode_call_pushes_stack_and_jumps() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x222;

        cpu.execute(Opcode::Call { addr: 0x456 });

        assert_eq!(cpu.stack[0], 0x222);
        assert_eq!(cpu.sp, 1);
        assert_eq!(cpu.pc, 0x456);
    }

    #[test]
    fn opcode_call_multiple_nested() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;

        cpu.execute(Opcode::Call { addr: 0x300 });
        assert_eq!(cpu.stack[0], 0x200);
        assert_eq!(cpu.sp, 1);
        assert_eq!(cpu.pc, 0x300);

        cpu.execute(Opcode::Call { addr: 0x400 });
        assert_eq!(cpu.stack[1], 0x300);
        assert_eq!(cpu.sp, 2);
        assert_eq!(cpu.pc, 0x400);
    }

    // ==================== 3XNN - SkipEqImm ====================

    #[test]
    fn opcode_skip_eq_imm_skips_when_equal() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[5] = 0x42;

        cpu.execute(Opcode::SkipEqImm {
            x: 5,
            conditon: 0x42,
        });

        assert_eq!(cpu.pc, 0x202);
    }

    #[test]
    fn opcode_skip_eq_imm_no_skip_when_not_equal() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[5] = 0x42;

        cpu.execute(Opcode::SkipEqImm {
            x: 5,
            conditon: 0x43,
        });

        assert_eq!(cpu.pc, 0x200);
    }

    #[test]
    fn opcode_skip_eq_imm_zero_values() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[0] = 0x00;

        cpu.execute(Opcode::SkipEqImm {
            x: 0,
            conditon: 0x00,
        });

        assert_eq!(cpu.pc, 0x202);
    }

    // ==================== 4XNN - SkipDifImm ====================

    #[test]
    fn opcode_skip_dif_imm_skips_when_not_equal() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[3] = 0x10;

        cpu.execute(Opcode::SkipDifImm {
            x: 3,
            conditon: 0x20,
        });

        assert_eq!(cpu.pc, 0x202);
    }

    #[test]
    fn opcode_skip_dif_imm_no_skip_when_equal() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[3] = 0x10;

        cpu.execute(Opcode::SkipDifImm {
            x: 3,
            conditon: 0x10,
        });

        assert_eq!(cpu.pc, 0x200);
    }

    // ==================== 5XY0 - SkipEq ====================

    #[test]
    fn opcode_skip_eq_skips_when_registers_equal() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[1] = 0x55;
        cpu.v[2] = 0x55;

        cpu.execute(Opcode::SkipEq { x: 1, y: 2 });

        assert_eq!(cpu.pc, 0x202);
    }

    #[test]
    fn opcode_skip_eq_no_skip_when_registers_not_equal() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[1] = 0x55;
        cpu.v[2] = 0x66;

        cpu.execute(Opcode::SkipEq { x: 1, y: 2 });

        assert_eq!(cpu.pc, 0x200);
    }

    #[test]
    fn opcode_skip_eq_same_register() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[5] = 0xAB;

        cpu.execute(Opcode::SkipEq { x: 5, y: 5 });

        assert_eq!(cpu.pc, 0x202);
    }

    // ==================== 6XNN - LoadImm ====================

    #[test]
    fn opcode_load_imm_sets_register() {
        let mut cpu = Cpu::new();
        cpu.execute(Opcode::LoadImm { x: 3, kk: 0xAB });
        assert_eq!(cpu.v[3], 0xAB);
    }

    #[test]
    fn opcode_load_imm_all_registers() {
        let mut cpu = Cpu::new();

        for i in 0..16 {
            cpu.execute(Opcode::LoadImm {
                x: i,
                kk: i as u8 * 10,
            });
        }

        for i in 0..16 {
            assert_eq!(cpu.v[i], i as u8 * 10);
        }
    }

    #[test]
    fn opcode_load_imm_overwrites() {
        let mut cpu = Cpu::new();
        cpu.v[7] = 0xFF;

        cpu.execute(Opcode::LoadImm { x: 7, kk: 0x00 });

        assert_eq!(cpu.v[7], 0x00);
    }

    // ==================== 7XNN - AddImm ====================

    #[test]
    fn opcode_add_imm_basic() {
        let mut cpu = Cpu::new();
        cpu.v[2] = 10;

        cpu.execute(Opcode::AddImm { x: 2, kk: 5 });

        assert_eq!(cpu.v[2], 15);
    }

    #[test]
    fn opcode_add_imm_wraps() {
        let mut cpu = Cpu::new();
        cpu.v[2] = 250;

        cpu.execute(Opcode::AddImm { x: 2, kk: 10 });

        assert_eq!(cpu.v[2], 4); // 260 % 256 = 4
    }

    #[test]
    fn opcode_add_imm_no_carry_flag() {
        let mut cpu = Cpu::new();
        cpu.v[2] = 250;
        cpu.v[0xF] = 0;

        cpu.execute(Opcode::AddImm { x: 2, kk: 10 });

        // VF should NOT be affected by AddImm
        assert_eq!(cpu.v[0xF], 0);
    }

    #[test]
    fn opcode_add_imm_zero() {
        let mut cpu = Cpu::new();
        cpu.v[5] = 100;

        cpu.execute(Opcode::AddImm { x: 5, kk: 0 });

        assert_eq!(cpu.v[5], 100);
    }

    // ==================== 8XY0 - LoadReg ====================

    #[test]
    fn opcode_load_reg() {
        let mut cpu = Cpu::new();
        cpu.v[1] = 0x42;
        cpu.v[2] = 0x00;

        cpu.execute(Opcode::LoadReg { x: 2, y: 1 });

        assert_eq!(cpu.v[2], 0x42);
        assert_eq!(cpu.v[1], 0x42); // Source unchanged
    }

    #[test]
    fn opcode_load_reg_same_register() {
        let mut cpu = Cpu::new();
        cpu.v[5] = 0xAB;

        cpu.execute(Opcode::LoadReg { x: 5, y: 5 });

        assert_eq!(cpu.v[5], 0xAB);
    }

    // ==================== 8XY1 - Or ====================

    #[test]
    fn opcode_or() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0xF0;
        cpu.v[1] = 0x0F;

        cpu.execute(Opcode::Or { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0xFF);
    }

    #[test]
    fn opcode_or_with_zero() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0xAB;
        cpu.v[1] = 0x00;

        cpu.execute(Opcode::Or { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0xAB);
    }

    #[test]
    fn opcode_or_same_value() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0x55;
        cpu.v[1] = 0x55;

        cpu.execute(Opcode::Or { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0x55);
    }

    // ==================== 8XY2 - And ====================

    #[test]
    fn opcode_and() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0xF0;
        cpu.v[1] = 0x0F;

        cpu.execute(Opcode::And { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0x00);
    }

    #[test]
    fn opcode_and_all_ones() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0xFF;
        cpu.v[1] = 0xFF;

        cpu.execute(Opcode::And { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0xFF);
    }

    #[test]
    fn opcode_and_partial() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0b10101010;
        cpu.v[1] = 0b11110000;

        cpu.execute(Opcode::And { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0b10100000);
    }

    // ==================== 8XY3 - Xor ====================

    #[test]
    fn opcode_xor() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0xFF;
        cpu.v[1] = 0x0F;

        cpu.execute(Opcode::Xor { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0xF0);
    }

    #[test]
    fn opcode_xor_same_value_gives_zero() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0xAB;
        cpu.v[1] = 0xAB;

        cpu.execute(Opcode::Xor { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0x00);
    }

    #[test]
    fn opcode_xor_with_zero() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0x55;
        cpu.v[1] = 0x00;

        cpu.execute(Opcode::Xor { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0x55);
    }

    // ==================== 8XY4 - AddReg ====================

    #[test]
    fn opcode_add_reg_no_overflow() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 10;
        cpu.v[1] = 20;

        cpu.execute(Opcode::AddReg { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 30);
        assert_eq!(cpu.v[0xF], 0);
    }

    #[test]
    fn opcode_add_reg_with_overflow() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 200;
        cpu.v[1] = 100;

        cpu.execute(Opcode::AddReg { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 44); // 300 % 256 = 44
        assert_eq!(cpu.v[0xF], 1);
    }

    #[test]
    fn opcode_add_reg_exact_overflow() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 255;
        cpu.v[1] = 1;

        cpu.execute(Opcode::AddReg { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0);
        assert_eq!(cpu.v[0xF], 1);
    }

    #[test]
    fn opcode_add_reg_vf_as_operand() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 5;
        cpu.v[0xF] = 10;

        cpu.execute(Opcode::AddReg { x: 0, y: 0xF });

        assert_eq!(cpu.v[0], 15);
        assert_eq!(cpu.v[0xF], 0); // VF is set to carry flag
    }

    // ==================== 8XY5 - SubReg ====================

    #[test]
    fn opcode_sub_reg_no_borrow() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 30;
        cpu.v[1] = 10;

        cpu.execute(Opcode::SubReg { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 20);
        assert_eq!(cpu.v[0xF], 1); // No borrow
    }

    #[test]
    fn opcode_sub_reg_with_borrow() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 10;
        cpu.v[1] = 30;

        cpu.execute(Opcode::SubReg { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 236); // Wrapping subtraction
        assert_eq!(cpu.v[0xF], 0); // Borrow occurred
    }

    #[test]
    fn opcode_sub_reg_equal_values() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 50;
        cpu.v[1] = 50;

        cpu.execute(Opcode::SubReg { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0);
        assert_eq!(cpu.v[0xF], 1); // VX >= VY, so no borrow
    }

    // ==================== 8XY6 - ShiftRight ====================

    #[test]
    fn opcode_shift_right_lsb_zero() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0b11110000;

        cpu.execute(Opcode::ShiftRight { x: 0 });

        assert_eq!(cpu.v[0], 0b01111000);
        assert_eq!(cpu.v[0xF], 0);
    }

    #[test]
    fn opcode_shift_right_lsb_one() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0b11110001;

        cpu.execute(Opcode::ShiftRight { x: 0 });

        assert_eq!(cpu.v[0], 0b01111000);
        assert_eq!(cpu.v[0xF], 1);
    }

    #[test]
    fn opcode_shift_right_one() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 1;

        cpu.execute(Opcode::ShiftRight { x: 0 });

        assert_eq!(cpu.v[0], 0);
        assert_eq!(cpu.v[0xF], 1);
    }

    // ==================== 8XY7 - SubN ====================

    #[test]
    fn opcode_subn_no_borrow() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 10;
        cpu.v[1] = 30;

        cpu.execute(Opcode::SubN { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 20); // VY - VX = 30 - 10
        assert_eq!(cpu.v[0xF], 1); // No borrow
    }

    #[test]
    fn opcode_subn_with_borrow() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 30;
        cpu.v[1] = 10;

        cpu.execute(Opcode::SubN { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 236); // Wrapping
        assert_eq!(cpu.v[0xF], 0); // Borrow
    }

    #[test]
    fn opcode_subn_equal_values() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 50;
        cpu.v[1] = 50;

        cpu.execute(Opcode::SubN { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0);
        assert_eq!(cpu.v[0xF], 1); // VY >= VX
    }

    // ==================== 8XYE - ShiftLeft ====================

    #[test]
    fn opcode_shift_left_msb_zero() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0b00001111;

        cpu.execute(Opcode::ShiftLeft { x: 0 });

        assert_eq!(cpu.v[0], 0b00011110);
        assert_eq!(cpu.v[0xF], 0);
    }

    #[test]
    fn opcode_shift_left_msb_one() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0b10001111;

        cpu.execute(Opcode::ShiftLeft { x: 0 });

        assert_eq!(cpu.v[0], 0b00011110);
        assert_eq!(cpu.v[0xF], 1);
    }

    #[test]
    fn opcode_shift_left_0x80() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0x80;

        cpu.execute(Opcode::ShiftLeft { x: 0 });

        assert_eq!(cpu.v[0], 0);
        assert_eq!(cpu.v[0xF], 1);
    }

    // ==================== 9XY0 - SkipNeqReq ====================

    #[test]
    fn opcode_skip_neq_reg_skips_when_not_equal() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[1] = 0x10;
        cpu.v[2] = 0x20;

        cpu.execute(Opcode::SkipNeqReq { x: 1, y: 2 });

        assert_eq!(cpu.pc, 0x202);
    }

    #[test]
    fn opcode_skip_neq_reg_no_skip_when_equal() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[1] = 0x10;
        cpu.v[2] = 0x10;

        cpu.execute(Opcode::SkipNeqReq { x: 1, y: 2 });

        assert_eq!(cpu.pc, 0x200);
    }

    // ==================== ANNN - LoadI ====================

    #[test]
    fn opcode_load_i() {
        let mut cpu = Cpu::new();

        cpu.execute(Opcode::LoadI { addr: 0x500 });

        assert_eq!(cpu.i, 0x500);
    }

    #[test]
    fn opcode_load_i_max_value() {
        let mut cpu = Cpu::new();

        cpu.execute(Opcode::LoadI { addr: 0xFFF });

        assert_eq!(cpu.i, 0xFFF);
    }

    #[test]
    fn opcode_load_i_zero() {
        let mut cpu = Cpu::new();
        cpu.i = 0x500;

        cpu.execute(Opcode::LoadI { addr: 0x000 });

        assert_eq!(cpu.i, 0x000);
    }

    // ==================== BNNN - JumpV0 ====================

    #[test]
    fn opcode_jump_v0() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0x10;

        cpu.execute(Opcode::JumpV0 { addr: 0x300 });

        assert_eq!(cpu.pc, 0x310);
    }

    #[test]
    fn opcode_jump_v0_zero() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0x00;

        cpu.execute(Opcode::JumpV0 { addr: 0x400 });

        assert_eq!(cpu.pc, 0x400);
    }

    #[test]
    fn opcode_jump_v0_wraps() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0xFF;

        cpu.execute(Opcode::JumpV0 { addr: 0xFFF });

        // 0xFFF + 0xFF = 0x10FE, wraps to 0x0FE
        assert_eq!(cpu.pc, 0x10FE);
    }

    // ==================== CXNN - Random ====================

    #[test]
    fn opcode_random_mask() {
        let mut cpu = Cpu::new();

        // Run multiple times to test randomness with mask
        for _ in 0..100 {
            cpu.execute(Opcode::Random { x: 0, nn: 0x0F });
            assert!(cpu.v[0] <= 0x0F); // Should be masked to lower nibble
        }
    }

    #[test]
    fn opcode_random_zero_mask() {
        let mut cpu = Cpu::new();

        cpu.execute(Opcode::Random { x: 0, nn: 0x00 });

        assert_eq!(cpu.v[0], 0x00);
    }

    #[test]
    fn opcode_random_full_mask() {
        let mut cpu = Cpu::new();

        // Run multiple times - with 0xFF mask, we should get various values
        let mut seen_different = false;
        let first = {
            cpu.execute(Opcode::Random { x: 0, nn: 0xFF });
            cpu.v[0]
        };

        for _ in 0..100 {
            cpu.execute(Opcode::Random { x: 0, nn: 0xFF });
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
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[0] = 5;
        cpu.keys[5] = true;

        cpu.execute(Opcode::SkipKey { x: 0 });

        assert_eq!(cpu.pc, 0x202);
    }

    #[test]
    fn opcode_skip_key_not_pressed() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[0] = 5;
        cpu.keys[5] = false;

        cpu.execute(Opcode::SkipKey { x: 0 });

        assert_eq!(cpu.pc, 0x200);
    }

    #[test]
    fn opcode_skip_key_all_keys() {
        let mut cpu = Cpu::new();

        for key in 0..16u8 {
            cpu.pc = 0x200;
            cpu.v[0] = key;
            cpu.keys = [false; 16];
            cpu.keys[key as usize] = true;

            cpu.execute(Opcode::SkipKey { x: 0 });
            assert_eq!(cpu.pc, 0x202, "Key {} should cause skip", key);
        }
    }

    // ==================== EXA1 - SkipNotKey ====================

    #[test]
    fn opcode_skip_not_key_not_pressed() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[0] = 5;
        cpu.keys[5] = false;

        cpu.execute(Opcode::SkipNotKey { x: 0 });

        assert_eq!(cpu.pc, 0x202);
    }

    #[test]
    fn opcode_skip_not_key_pressed() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;
        cpu.v[0] = 5;
        cpu.keys[5] = true;

        cpu.execute(Opcode::SkipNotKey { x: 0 });

        assert_eq!(cpu.pc, 0x200);
    }

    // ==================== FX07 - LoadDelayTimer ====================

    #[test]
    fn opcode_load_delay_timer() {
        let mut cpu = Cpu::new();
        cpu.delay_timer = 60;

        cpu.execute(Opcode::LoadDelayTimer { x: 5 });

        assert_eq!(cpu.v[5], 60);
    }

    #[test]
    fn opcode_load_delay_timer_zero() {
        let mut cpu = Cpu::new();
        cpu.delay_timer = 0;
        cpu.v[5] = 0xFF;

        cpu.execute(Opcode::LoadDelayTimer { x: 5 });

        assert_eq!(cpu.v[5], 0);
    }

    // ==================== FX15 - SetDelayTimer ====================

    #[test]
    fn opcode_set_delay_timer() {
        let mut cpu = Cpu::new();
        cpu.v[3] = 120;

        cpu.execute(Opcode::SetDelayTimer { x: 3 });

        assert_eq!(cpu.delay_timer, 120);
    }

    #[test]
    fn opcode_set_delay_timer_zero() {
        let mut cpu = Cpu::new();
        cpu.delay_timer = 100;
        cpu.v[3] = 0;

        cpu.execute(Opcode::SetDelayTimer { x: 3 });

        assert_eq!(cpu.delay_timer, 0);
    }

    // ==================== FX18 - SetSoundTimer ====================

    #[test]
    fn opcode_set_sound_timer() {
        let mut cpu = Cpu::new();
        cpu.v[7] = 30;

        cpu.execute(Opcode::SetSoundTimer { x: 7 });

        assert_eq!(cpu.sound_timer, 30);
    }

    #[test]
    fn opcode_set_sound_timer_zero() {
        let mut cpu = Cpu::new();
        cpu.sound_timer = 50;
        cpu.v[7] = 0;

        cpu.execute(Opcode::SetSoundTimer { x: 7 });

        assert_eq!(cpu.sound_timer, 0);
    }

    // ==================== FX1E - AddI ====================

    #[test]
    fn opcode_add_i() {
        let mut cpu = Cpu::new();
        cpu.i = 0x100;
        cpu.v[2] = 0x50;

        cpu.execute(Opcode::AddI { x: 2 });

        assert_eq!(cpu.i, 0x150);
    }

    #[test]
    fn opcode_add_i_zero() {
        let mut cpu = Cpu::new();
        cpu.i = 0x500;
        cpu.v[0] = 0;

        cpu.execute(Opcode::AddI { x: 0 });

        assert_eq!(cpu.i, 0x500);
    }

    #[test]
    fn opcode_add_i_wraps() {
        let mut cpu = Cpu::new();
        cpu.i = 0xFFFF;
        cpu.v[0] = 1;

        cpu.execute(Opcode::AddI { x: 0 });

        assert_eq!(cpu.i, 0x0000); // Wrapping
    }

    // ==================== Timer Tests ====================

    #[test]
    fn timers_tick_down() {
        let mut cpu = Cpu::new();
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
        let mut cpu = Cpu::new();
        cpu.delay_timer = 0;
        cpu.sound_timer = 0;

        cpu.tick_timers();

        assert_eq!(cpu.delay_timer, 0);
        assert_eq!(cpu.sound_timer, 0);
    }

    // ==================== Integration Tests ====================

    #[test]
    fn call_and_return_sequence() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;

        // Call subroutine
        cpu.execute(Opcode::Call { addr: 0x400 });
        assert_eq!(cpu.pc, 0x400);
        assert_eq!(cpu.sp, 1);

        // Do some work
        cpu.execute(Opcode::LoadImm { x: 0, kk: 42 });

        // Return
        cpu.execute(Opcode::Return);
        assert_eq!(cpu.pc, 0x200);
        assert_eq!(cpu.sp, 0);
        assert_eq!(cpu.v[0], 42); // Work persisted
    }

    #[test]
    fn arithmetic_sequence() {
        let mut cpu = Cpu::new();

        // V0 = 10
        cpu.execute(Opcode::LoadImm { x: 0, kk: 10 });
        // V1 = 5
        cpu.execute(Opcode::LoadImm { x: 1, kk: 5 });
        // V0 = V0 + V1 = 15
        cpu.execute(Opcode::AddReg { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 15);
        assert_eq!(cpu.v[0xF], 0); // No overflow

        // V0 = V0 - V1 = 10
        cpu.execute(Opcode::SubReg { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 10);
        assert_eq!(cpu.v[0xF], 1); // No borrow
    }

    #[test]
    fn bitwise_operations_sequence() {
        let mut cpu = Cpu::new();

        cpu.execute(Opcode::LoadImm {
            x: 0,
            kk: 0b11110000,
        });
        cpu.execute(Opcode::LoadImm {
            x: 1,
            kk: 0b10101010,
        });

        // V2 = V0 (copy)
        cpu.execute(Opcode::LoadReg { x: 2, y: 0 });

        // V2 = V2 & V1
        cpu.execute(Opcode::And { x: 2, y: 1 });
        assert_eq!(cpu.v[2], 0b10100000);

        // V3 = V0 (copy)
        cpu.execute(Opcode::LoadReg { x: 3, y: 0 });

        // V3 = V3 | V1
        cpu.execute(Opcode::Or { x: 3, y: 1 });
        assert_eq!(cpu.v[3], 0b11111010);

        // V4 = V0 (copy)
        cpu.execute(Opcode::LoadReg { x: 4, y: 0 });

        // V4 = V4 ^ V1
        cpu.execute(Opcode::Xor { x: 4, y: 1 });
        assert_eq!(cpu.v[4], 0b01011010);
    }

    #[test]
    fn conditional_skip_sequence() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x200;

        cpu.execute(Opcode::LoadImm { x: 0, kk: 0x42 });
        cpu.execute(Opcode::LoadImm { x: 1, kk: 0x42 });

        // Should skip (equal)
        cpu.execute(Opcode::SkipEq { x: 0, y: 1 });
        assert_eq!(cpu.pc, 0x202);

        // Should not skip (still equal)
        cpu.execute(Opcode::SkipNeqReq { x: 0, y: 1 });
        assert_eq!(cpu.pc, 0x202);

        // Change V1
        cpu.execute(Opcode::LoadImm { x: 1, kk: 0x43 });

        // Should skip (not equal now)
        cpu.execute(Opcode::SkipNeqReq { x: 0, y: 1 });
        assert_eq!(cpu.pc, 0x204);
    }
}
