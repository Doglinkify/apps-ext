//! Linux SDL2 simulation backend.
//!
//! Opens an SDL2 window with the same BGRA32 framebuffer layout that
//! DoglinkOS exposes, feeds keyboard events through the same [`Key`] enum,
//! and uses `std::thread::sleep` for timing.
//!
//! Compiled only when `--features sim` is on (and `dlos` is off).

#![cfg(all(feature = "sim", not(feature = "dlos")))]

use crate::platform::{FramebufferInfo, Key, PlatformBackend};

use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[derive(Default, Clone, Copy)]
pub struct Options {
    pub no_display: bool,
    pub max_frames: Option<u64>,
}

static OPTIONS: OnceLock<std::sync::Mutex<Options>> = OnceLock::new();

pub fn set_options(o: Options) {
    *OPTIONS
        .get_or_init(|| std::sync::Mutex::new(Options::default()))
        .lock()
        .unwrap() = o;
}

pub fn options() -> Options {
    OPTIONS
        .get_or_init(|| std::sync::Mutex::new(Options::default()))
        .lock()
        .unwrap()
        .clone()
}

pub fn load_file(path: &str) -> Result<Vec<u8>, String> {
    if !Path::new(path).exists() {
        return Err(format!("no such file: {path}"));
    }
    fs::read(path).map_err(|e| format!("read '{path}': {e}"))
}

pub struct SimBackend {
    width: usize,
    height: usize,
    buf: Vec<u8>,
    start: Instant,
    _sdl: Option<sdl2::Sdl>,
    canvas: Option<sdl2::render::Canvas<sdl2::video::Window>>,
    event_pump: Option<sdl2::EventPump>,
    key_queue: std::collections::VecDeque<Key>,
    frame_count: u64,
}

impl SimBackend {
    pub fn new() -> Self {
        let opts = options();
        let (w, h) = (640, 480);

        let mut buf = vec![0u8; w * h * 4];
        for px in buf.chunks_exact_mut(4) {
            px[0] = 0x12;
            px[1] = 0x12;
            px[2] = 0x12;
            px[3] = 0xff;
        }

        let (sdl, canvas, event_pump) = if opts.no_display {
            (None, None, None)
        } else {
            let sdl_ctx = sdl2::init().map_err(|e| e.to_string()).unwrap_or_else(|e| {
                eprintln!("videoplay: SDL init failed ({e}); falling back to headless");
                panic!();
            });
            let video = sdl_ctx.video().map_err(|e| e.to_string()).unwrap();
            let window = video
                .window("dlos-videoplay (sim)", w as u32, h as u32)
                .position_centered()
                .build()
                .unwrap();
            let canvas = window.into_canvas().software().build().unwrap();
            let pump = sdl_ctx.event_pump().unwrap();
            (Some(sdl_ctx), Some(canvas), Some(pump))
        };

        Self {
            width: w,
            height: h,
            buf,
            start: Instant::now(),
            _sdl: sdl,
            canvas,
            event_pump,
            key_queue: std::collections::VecDeque::new(),
            frame_count: 0,
        }
    }

    fn pump_events(&mut self) {
        let Some(pump) = self.event_pump.as_mut() else {
            return;
        };
        for ev in pump.poll_iter() {
            use sdl2::event::Event;
            use sdl2::keyboard::Keycode;
            match ev {
                Event::Quit { .. } => self.key_queue.push_back(Key::Q),
                Event::KeyDown {
                    keycode: Some(kc),
                    repeat: false,
                    ..
                } => match kc {
                    Keycode::Space => self.key_queue.push_back(Key::Space),
                    Keycode::Q => self.key_queue.push_back(Key::Q),
                    Keycode::Escape => self.key_queue.push_back(Key::Escape),
                    Keycode::O => self.key_queue.push_back(Key::O),
                    Keycode::Left => self.key_queue.push_back(Key::Left),
                    Keycode::Right => self.key_queue.push_back(Key::Right),
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

impl Default for SimBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformBackend for SimBackend {
    fn framebuffer(&mut self) -> FramebufferInfo {
        self.frame_count += 1;

        if let Some(max) = options().max_frames {
            if self.frame_count > max {
                std::process::exit(0);
            }
        }

        FramebufferInfo {
            ptr: self.buf.as_mut_ptr(),
            width: self.width,
            height: self.height,
            pitch: self.width * 4,
        }
    }

    fn poll_key(&mut self) -> Option<Key> {
        self.pump_events();
        self.key_queue.pop_front()
    }

    fn ticks_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    fn sleep_ms(&mut self, ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }

    fn print(&mut self, msg: &str) {
        print!("{msg}");
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }

    fn present(&mut self) {
        let Some(canvas) = self.canvas.as_mut() else {
            return;
        };
        let (cw, ch) = canvas.output_size().unwrap_or((640, 480));
        let creator = canvas.texture_creator();
        let mut texture = creator
            .create_texture_streaming(
                sdl2::pixels::PixelFormatEnum::BGRA32,
                self.width as u32,
                self.height as u32,
            )
            .unwrap();
        texture.update(None, &self.buf, self.width * 4).unwrap();
        canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
        canvas.clear();
        let vw = self.width as f32;
        let vh = self.height as f32;
        let cw = cw as f32;
        let ch = ch as f32;
        let scale = (cw / vw).min(ch / vh);
        let dw = (vw * scale) as u32;
        let dh = (vh * scale) as u32;
        let dx = ((cw as u32 - dw) / 2) as i32;
        let dy = ((ch as u32 - dh) / 2) as i32;
        let dst = sdl2::rect::Rect::new(dx, dy, dw, dh);
        canvas.copy(&texture, None, Some(dst)).ok();
        canvas.present();
    }

    fn exit(&self) -> ! {
        std::process::exit(0);
    }
}
