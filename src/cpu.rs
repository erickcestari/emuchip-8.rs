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

    #[test]
    fn cpu_initializes_correctly() {
        let cpu = Cpu::new();

        // Registers
        assert_eq!(cpu.v, [0; REGISTERS_COUNT]);
        assert_eq!(cpu.i, 0);
        assert_eq!(cpu.pc, PROGRAM_START);

        // Stack
        assert_eq!(cpu.stack, [0; STACK_DEPTH]);
        assert_eq!(cpu.sp, 0);

        // Timers
        assert_eq!(cpu.delay_timer, 0);
        assert_eq!(cpu.sound_timer, 0);

        // Display & keys
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

        // Insert any valid opcode (e.g. 0x00E0 = CLS)
        cpu.memory[0x200] = 0x00;
        cpu.memory[0x201] = 0xE0;

        let old_pc = cpu.pc;
        cpu.step();
        assert_eq!(cpu.pc, old_pc + 2);
    }

    #[test]
    fn opcode_clears_screen() {
        let mut cpu = Cpu::new();
        cpu.display.fill(1); // set non-zero

        cpu.execute(Opcode::ClearScreen);

        assert!(cpu.display.iter().all(|&p| p == 0));
    }

    #[test]
    fn opcode_loadimm_sets_register() {
        let mut cpu = Cpu::new();
        cpu.execute(Opcode::LoadImm { x: 3, kk: 0xAB });
        assert_eq!(cpu.v[3], 0xAB);
    }

    #[test]
    fn opcode_addimm_wraps() {
        let mut cpu = Cpu::new();
        cpu.v[2] = 250;

        cpu.execute(Opcode::AddImm { x: 2, kk: 10 });
        assert_eq!(cpu.v[2], 4); // 250 + 10 = 260 → 260 % 256 = 4
    }

    #[test]
    fn opcode_jump_changes_pc() {
        let mut cpu = Cpu::new();

        cpu.execute(Opcode::Jump { addr: 0x300 });
        assert_eq!(cpu.pc, 0x300);
    }

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
    fn opcode_return_pops_stack() {
        let mut cpu = Cpu::new();
        cpu.stack[0] = 0x222;
        cpu.sp = 1;

        cpu.execute(Opcode::Return);

        assert_eq!(cpu.sp, 0);
        assert_eq!(cpu.pc, 0x222);
    }

    #[test]
    fn timers_tick_down() {
        let mut cpu = Cpu::new();
        cpu.delay_timer = 3;
        cpu.sound_timer = 2;

        cpu.tick_timers();
        assert_eq!(cpu.delay_timer, 2);
        assert_eq!(cpu.sound_timer, 1);

        cpu.tick_timers();
        cpu.tick_timers();
        assert_eq!(cpu.delay_timer, 0);
        assert_eq!(cpu.sound_timer, 0);
    }

    #[test]
    fn opcode_or() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0xf0;
        cpu.v[1] = 0x0f;
        cpu.execute(Opcode::Or { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0xff);
    }

    #[test]
    fn opcode_and() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 0xf0;
        cpu.v[1] = 0x0f;
        cpu.execute(Opcode::And { x: 0, y: 1 });

        assert_eq!(cpu.v[0], 0x00);
    }
}
