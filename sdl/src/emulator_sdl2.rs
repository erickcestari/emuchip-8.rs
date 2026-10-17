use emuchip_8::{Chip8, CpuState, DISPLAY_HEIGHT, DISPLAY_WIDTH, Error};
use sdl2::AudioSubsystem;
use sdl2::EventPump;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::ttf::{Font, Sdl2TtfContext};
use sdl2::video::Window;
use std::time::{Duration, Instant};

const FRAME_DURATION: Duration = Duration::from_micros(16667); // ~60 FPS
const SCALE: u32 = 10;
const WINDOW_WIDTH: u32 = DISPLAY_WIDTH as u32 * SCALE;
const WINDOW_HEIGHT: u32 = DISPLAY_HEIGHT as u32 * SCALE;
const DEBUG_WINDOW_WIDTH: u32 = 200;
const DEBUG_WINDOW_HEIGHT: u32 = 400;

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

struct Audio {
    device: AudioDevice<SquareWave>,
}

impl Audio {
    fn new(audio_subsystem: &AudioSubsystem) -> Self {
        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: Some(256),
        };

        let device = audio_subsystem
            .open_playback(None, &desired_spec, |spec| SquareWave {
                phase_inc: 440.0 / spec.freq as f32,
                phase: 0.0,
                volume: 0.25,
            })
            .expect("Failed to open audio device");

        Self { device }
    }

    fn play(&self) {
        self.device.resume();
    }

    fn stop(&self) {
        self.device.pause();
    }
}

fn keycode_to_chip8(keycode: Keycode) -> Option<u8> {
    match keycode {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None,
    }
}

struct Renderer {
    canvas: Canvas<Window>,
    debug_canvas: Canvas<Window>,
    event_pump: EventPump,
    audio: Audio,
    audio_playing: bool,
    ttf_context: Sdl2TtfContext,
    font_path: Option<String>,
}

impl Renderer {
    fn new() -> Self {
        let sdl_context = sdl2::init().expect("Failed to initialize SDL2");
        let video_subsystem = sdl_context.video().expect("Failed to get video subsystem");
        let audio_subsystem = sdl_context.audio().expect("Failed to get audio subsystem");
        let ttf_context = sdl2::ttf::init().expect("Failed to initialize SDL2_ttf");

        let window = video_subsystem
            .window("CHIP-8 Emulator", WINDOW_WIDTH, WINDOW_HEIGHT)
            .position(100, 100)
            .build()
            .expect("Failed to create window");

        let mut canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .expect("Failed to create canvas");

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        let debug_window = video_subsystem
            .window("CHIP-8 Registers", DEBUG_WINDOW_WIDTH, DEBUG_WINDOW_HEIGHT)
            .position(100 + WINDOW_WIDTH as i32 + 20, 100)
            .build()
            .expect("Failed to create debug window");

        let mut debug_canvas = debug_window
            .into_canvas()
            .build()
            .expect("Failed to create debug canvas");

        debug_canvas.set_draw_color(Color::RGB(30, 30, 30));
        debug_canvas.clear();
        debug_canvas.present();

        let event_pump = sdl_context.event_pump().expect("Failed to get event pump");
        let audio = Audio::new(&audio_subsystem);

        let font_path = Self::find_system_font_path();

        Self {
            canvas,
            debug_canvas,
            event_pump,
            audio,
            audio_playing: false,
            ttf_context,
            font_path,
        }
    }

    fn draw(&mut self, display: &[u8]) {
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();

        self.canvas.set_draw_color(Color::RGB(255, 255, 255));
        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                let idx = y * DISPLAY_WIDTH + x;
                if display[idx] == 1 {
                    let rect = Rect::new(
                        (x as u32 * SCALE) as i32,
                        (y as u32 * SCALE) as i32,
                        SCALE,
                        SCALE,
                    );
                    self.canvas.fill_rect(rect).expect("Failed to draw rect");
                }
            }
        }

        self.canvas.present();
    }

    fn event_pump(&mut self) -> &mut EventPump {
        &mut self.event_pump
    }

    fn update_audio(&mut self, sound_timer: u8) {
        if sound_timer > 0 && !self.audio_playing {
            self.audio.play();
            self.audio_playing = true;
        } else if sound_timer == 0 && self.audio_playing {
            self.audio.stop();
            self.audio_playing = false;
        }
    }

    fn find_system_font_path() -> Option<String> {
        // Common font paths on different systems
        let font_paths = [
            // Arch Linux
            "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
            "/usr/share/fonts/TTF/Hack-Regular.ttf",
            "/usr/share/fonts/TTF/FiraCode-Regular.ttf",
            "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
            // Debian/Ubuntu
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/truetype/freefont/FreeMono.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
            // Fedora/RHEL
            "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/liberation-mono/LiberationMono-Regular.ttf",
            // macOS
            "/System/Library/Fonts/Menlo.ttc",
            "/System/Library/Fonts/Monaco.ttf",
            "/Library/Fonts/Courier New.ttf",
            // Windows (via WSL or Wine)
            "/usr/share/fonts/truetype/msttcorefonts/cour.ttf",
            "C:\\Windows\\Fonts\\consola.ttf",
        ];

        // Try hardcoded paths first
        for path in &font_paths {
            if std::path::Path::new(path).exists() {
                return Some(path.to_string());
            }
        }

        // Try using fc-match (fontconfig) to find a monospace font on Linux
        if let Ok(output) = std::process::Command::new("fc-match")
            .args(["--format=%{file}", "monospace"])
            .output()
            && output.status.success()
        {
            let font_path = String::from_utf8_lossy(&output.stdout);
            let font_path = font_path.trim();
            if !font_path.is_empty() && std::path::Path::new(font_path).exists() {
                return Some(font_path.to_string());
            }
        }

        None
    }

    fn draw_registers(&mut self, cpu_state: &CpuState) {
        self.debug_canvas.set_draw_color(Color::RGB(30, 30, 30));
        self.debug_canvas.clear();

        let font_path = match &self.font_path {
            Some(p) => p,
            None => {
                self.debug_canvas.present();
                return;
            }
        };

        let font = match self.ttf_context.load_font(font_path, 14) {
            Ok(f) => f,
            Err(_) => {
                self.debug_canvas.present();
                return;
            }
        };

        let registers = (0..16).map(|i| {
            let text = format!("V{:X}: 0x{:02X} ({:3})", i, cpu_state.v[i], cpu_state.v[i]);
            (text, Color::RGB(0, 255, 0))
        });
        let others = [
            (
                format!("I:  0x{:04X}", cpu_state.i),
                Color::RGB(0, 200, 255),
            ),
            (
                format!("PC: 0x{:04X}", cpu_state.pc),
                Color::RGB(255, 200, 0),
            ),
            (
                format!("SP: 0x{:02X}", cpu_state.sp),
                Color::RGB(255, 100, 100),
            ),
            (
                format!("DT: 0x{:02X}", cpu_state.delay_timer),
                Color::RGB(200, 100, 255),
            ),
            (
                format!("ST: 0x{:02X}", cpu_state.sound_timer),
                Color::RGB(200, 100, 255),
            ),
        ];

        let mut y_offset = 10;
        for (text, color) in registers {
            draw_text(&mut self.debug_canvas, &font, &text, color, y_offset);
            y_offset += 18;
        }

        y_offset += 10;

        for (text, color) in others {
            draw_text(&mut self.debug_canvas, &font, &text, color, y_offset);
            y_offset += 18;
        }

        self.debug_canvas.present();
    }
}

fn draw_text(canvas: &mut Canvas<Window>, font: &Font, text: &str, color: Color, y: i32) {
    let surface = font
        .render(text)
        .blended(color)
        .expect("Failed to render text");
    let texture_creator = canvas.texture_creator();
    let texture = texture_creator
        .create_texture_from_surface(&surface)
        .expect("Failed to create texture");
    let target = Rect::new(10, y, surface.width(), surface.height());
    canvas
        .copy(&texture, None, Some(target))
        .expect("Failed to copy texture");
}

pub struct EmulatorSdl2 {
    chip8: Chip8,
    renderer: Renderer,
}

impl EmulatorSdl2 {
    pub fn new(chip8: Chip8) -> Self {
        Self {
            chip8,
            renderer: Renderer::new(),
        }
    }

    /// Runs until the window closes or the ROM faults.
    pub fn run(&mut self) -> Result<(), Error> {
        while self.step_frame()? {}
        Ok(())
    }

    /// Returns false once the user asks to quit.
    fn step_frame(&mut self) -> Result<bool, Error> {
        let frame_start = Instant::now();

        // Handle input
        for event in self.renderer.event_pump().poll_iter() {
            match event {
                Event::Quit { .. } => {
                    return Ok(false);
                }
                Event::Window {
                    win_event: WindowEvent::Close,
                    ..
                } => {
                    return Ok(false);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    return Ok(false);
                }
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if let Some(chip8_key) = keycode_to_chip8(key) {
                        self.chip8.set_key(chip8_key, true);
                    }
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    if let Some(chip8_key) = keycode_to_chip8(key) {
                        self.chip8.set_key(chip8_key, false);
                    }
                }
                _ => {}
            }
        }

        // Run CPU cycles for this frame
        self.chip8.step_frame()?;

        // Update audio and render
        self.renderer.update_audio(self.chip8.sound_timer());
        self.renderer.draw(self.chip8.display());
        self.renderer.draw_registers(&self.chip8.cpu_state());

        // Frame timing
        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }

        Ok(true)
    }
}
