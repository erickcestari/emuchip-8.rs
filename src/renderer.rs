use crate::audio::Audio;
use sdl2::EventPump;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

const SCALE: u32 = 10;
const DISPLAY_WIDTH: u32 = 64;
const DISPLAY_HEIGHT: u32 = 32;
const WINDOW_WIDTH: u32 = DISPLAY_WIDTH * SCALE;
const WINDOW_HEIGHT: u32 = DISPLAY_HEIGHT * SCALE;

pub struct Renderer {
    canvas: Canvas<Window>,
    event_pump: EventPump,
    audio: Audio,
    audio_playing: bool,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        let sdl_context = sdl2::init().expect("Failed to initialize SDL2");
        let video_subsystem = sdl_context.video().expect("Failed to get video subsystem");
        let audio_subsystem = sdl_context.audio().expect("Failed to get audio subsystem");

        let window = video_subsystem
            .window("CHIP-8 Emulator", WINDOW_WIDTH, WINDOW_HEIGHT)
            .position_centered()
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

        let event_pump = sdl_context.event_pump().expect("Failed to get event pump");
        let audio = Audio::new(&audio_subsystem);

        Self {
            canvas,
            event_pump,
            audio,
            audio_playing: false,
        }
    }

    pub fn draw(&mut self, display: &[u8]) {
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();

        self.canvas.set_draw_color(Color::RGB(255, 255, 255));
        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                let idx = (y * DISPLAY_WIDTH + x) as usize;
                if display[idx] == 1 {
                    let rect = Rect::new((x * SCALE) as i32, (y * SCALE) as i32, SCALE, SCALE);
                    self.canvas.fill_rect(rect).expect("Failed to draw rect");
                }
            }
        }

        self.canvas.present();
    }

    pub fn event_pump(&mut self) -> &mut EventPump {
        &mut self.event_pump
    }

    pub fn update_audio(&mut self, sound_timer: u8) {
        if sound_timer > 0 && !self.audio_playing {
            self.audio.play();
            self.audio_playing = true;
        } else if sound_timer == 0 && self.audio_playing {
            self.audio.stop();
            self.audio_playing = false;
        }
    }
}
