use crate::cpu::Cpu;

pub const DISPLAY_WIDTH: usize = 64;
pub const DISPLAY_HEIGHT: usize = 32;
pub const CYCLES_PER_FRAME: u32 = 10;

pub struct CpuState {
    pub v: [u8; 16],
    pub i: u16,
    pub pc: u16,
    pub sp: u8,
    pub delay_timer: u8,
    pub sound_timer: u8,
}

pub trait Emulator {
    /// Create a new emulator instance with the given ROM
    fn new(rom: &[u8]) -> Self;

    /// Run a single frame (should be called at ~60 FPS)
    /// Returns false if the emulator should quit
    fn step_frame(&mut self) -> bool;

    /// Set a key state (key: 0-15, pressed: true/false)
    fn set_key(&mut self, key: u8, pressed: bool);

    /// Get the display buffer (64x32 pixels, 1 = on, 0 = off)
    fn display(&self) -> &[u8];

    /// Get the current sound timer value (beep if > 0)
    fn sound_timer(&self) -> u8;
}

/// Core emulator without any platform-specific code
pub struct Chip8 {
    cpu: Cpu,
}

impl Chip8 {
    pub fn new(rom: &[u8]) -> Self {
        let mut cpu = Cpu::new();
        cpu.load_rom(rom);
        Self { cpu }
    }

    pub fn step_frame(&mut self) {
        for _ in 0..CYCLES_PER_FRAME {
            self.cpu.step();
        }
        self.cpu.tick_timers();
    }

    pub fn set_key(&mut self, key: u8, pressed: bool) {
        if (key as usize) < 16 {
            self.cpu.keys[key as usize] = pressed;
        }
    }

    pub fn display(&self) -> &[u8] {
        &self.cpu.display
    }

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
