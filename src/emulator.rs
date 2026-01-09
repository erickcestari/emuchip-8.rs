use crate::cpu::Cpu;
use crate::input::keycode_to_chip8;
use crate::renderer::Renderer;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::{Duration, Instant};

const CYCLES_PER_FRAME: u32 = 10;
const FRAME_DURATION: Duration = Duration::from_micros(16667); // ~60 FPS

pub struct Emulator {
    cpu: Cpu,
    renderer: Renderer,
}

impl Emulator {
    pub fn new(rom: &[u8]) -> Self {
        let mut cpu = Cpu::new();
        cpu.load_rom(rom);
        let renderer = Renderer::new();

        Self { cpu, renderer }
    }

    pub fn run(&mut self) {
        loop {
            let frame_start = Instant::now();

            if !self.handle_input() {
                break;
            }

            for _ in 0..CYCLES_PER_FRAME {
                self.cpu.step();
            }

            self.cpu.tick_timers();
            self.renderer.update_audio(self.cpu.sound_timer);
            self.renderer.draw(&self.cpu.display);

            let elapsed = frame_start.elapsed();
            if elapsed < FRAME_DURATION {
                std::thread::sleep(FRAME_DURATION - elapsed);
            }
        }
    }

    fn handle_input(&mut self) -> bool {
        for event in self.renderer.event_pump().poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return false,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if let Some(chip8_key) = keycode_to_chip8(key) {
                        self.cpu.keys[chip8_key] = true;
                    }
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    if let Some(chip8_key) = keycode_to_chip8(key) {
                        self.cpu.keys[chip8_key] = false;
                    }
                }
                _ => {}
            }
        }
        true
    }
}
