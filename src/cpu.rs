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

    // Waiting for key press (for FX0A opcode)
    pub waiting_for_key: bool,
    pub key_register: usize,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    pub fn new() -> Self {
        // Standard CHIP-8 fontset
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

            waiting_for_key: false,
            key_register: 0,
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
                let x_pos = self.v[x] as usize % DISPLAY_WIDTH;
                let y_pos = self.v[y] as usize % DISPLAY_HEIGHT;
                self.v[0xF] = 0;

                for row in 0..n as usize {
                    let sprite_byte = self.memory[self.i as usize + row];
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
                let mut key_pressed = None;
                for (i, &pressed) in self.keys.iter().enumerate() {
                    if pressed {
                        key_pressed = Some(i);
                        break;
                    }
                }

                if let Some(key) = key_pressed {
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
                self.memory[self.i as usize] = value / 100;
                self.memory[self.i as usize + 1] = (value / 10) % 10;
                self.memory[self.i as usize + 2] = value % 10;
            }
            Opcode::RegDump { x } => {
                for i in 0..=x {
                    self.memory[self.i as usize + i] = self.v[i];
                }
            }
            Opcode::RegLoad { x } => {
                for i in 0..=x {
                    self.v[i] = self.memory[self.i as usize + i];
                }
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

    pub fn load_rom(&mut self, data: &[u8]) {
        let start = PROGRAM_START as usize;
        self.memory[start..start + data.len()].copy_from_slice(data);
    }
}
