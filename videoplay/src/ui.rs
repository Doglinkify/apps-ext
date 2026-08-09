//! Modern minimalist UI overlay.
//!
//! Drawn on top of the video frame after the codec has written the BGRA
//! pixels. Consists of:
//!   * a slim progress bar along the bottom edge,
//!   * a play/pause indicator centred on the bar,
//!   * a "current / total" time readout on the right,
//!   * a small status line in the top-left (paused / playing / opening).
//!
//! Hand-rolled primitives — no font library, no anti-aliasing. The font is
//! a tiny 5x7 bitmap for ASCII digits and the few symbols we need.

use crate::platform::FramebufferInfo;

// Color palette (modern minimal: slate background, sky-blue accent)
pub const BG: u32 = 0xFF0F172A; // slate-900, BGRA = 0x1A 0x17 0x0F 0xFF
pub const FG: u32 = 0xFFE2E8F0; // slate-200
pub const ACCENT: u32 = 0xFF60A5FA; // sky-400
pub const MUTED: u32 = 0xFF64748B; // slate-500
pub const SHADOW: u32 = 0xFF000000; // black

const GLYPH_W: usize = 5;
#[allow(dead_code)]
const GLYPH_H: usize = 7;

const fn g(rows: [u8; 7]) -> [u8; 7] {
    rows
}

const DIGITS: [[u8; 7]; 10] = [
    g([
        0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
    ]), // 0
    g([
        0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ]), // 1
    g([
        0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
    ]), // 2
    g([
        0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
    ]), // 3
    g([
        0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
    ]), // 4
    g([
        0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
    ]), // 5
    g([
        0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
    ]), // 6
    g([
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
    ]), // 7
    g([
        0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
    ]), // 8
    g([
        0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
    ]), // 9
];

const COLON: [u8; 7] = g([
    0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
]);
const SLASH: [u8; 7] = g([
    0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
]);
const SPACE: [u8; 7] = g([
    0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
]);

const LETTERS: &[(u8, [u8; 7])] = &[
    (
        b'A',
        g([
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ]),
    ),
    (
        b'B',
        g([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ]),
    ),
    (
        b'C',
        g([
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ]),
    ),
    (
        b'D',
        g([
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ]),
    ),
    (
        b'E',
        g([
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ]),
    ),
    (
        b'F',
        g([
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ]),
    ),
    (
        b'G',
        g([
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ]),
    ),
    (
        b'H',
        g([
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ]),
    ),
    (
        b'I',
        g([
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ]),
    ),
    (
        b'L',
        g([
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ]),
    ),
    (
        b'M',
        g([
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ]),
    ),
    (
        b'N',
        g([
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ]),
    ),
    (
        b'O',
        g([
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ]),
    ),
    (
        b'P',
        g([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ]),
    ),
    (
        b'R',
        g([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ]),
    ),
    (
        b'S',
        g([
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ]),
    ),
    (
        b'T',
        g([
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ]),
    ),
    (
        b'U',
        g([
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ]),
    ),
    (
        b'X',
        g([
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ]),
    ),
    (
        b'Y',
        g([
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ]),
    ),
];

fn glyph_for(c: u8) -> [u8; 7] {
    match c {
        b'0'..=b'9' => DIGITS[(c - b'0') as usize],
        b':' => COLON,
        b'/' => SLASH,
        b' ' => SPACE,
        b'A'..=b'Z' => LETTERS
            .iter()
            .find(|(lc, _)| *lc == c)
            .map(|(_, g)| *g)
            .unwrap_or(SPACE),
        _ => SPACE,
    }
}

#[inline]
fn put_pixel(fb: &FramebufferInfo, x: i32, y: i32, color: u32) {
    if x < 0 || y < 0 {
        return;
    }
    let (x, y) = (x as usize, y as usize);
    if x >= fb.width || y >= fb.height {
        return;
    }
    let off = y * fb.pitch + x * 4;
    unsafe {
        *(fb.ptr.add(off) as *mut u32) = color;
    }
}

fn fill_rect(fb: &FramebufferInfo, x: i32, y: i32, w: i32, h: i32, color: u32) {
    for j in 0..h {
        for i in 0..w {
            put_pixel(fb, x + i, y + j, color);
        }
    }
}

fn draw_text(fb: &FramebufferInfo, x: i32, y: i32, text: &str, color: u32) {
    let mut cx = x;
    for &b in text.as_bytes() {
        let g = glyph_for(b);
        for (gy, row) in g.iter().enumerate() {
            for gx in 0..GLYPH_W {
                if (row >> (4 - gx)) & 1 == 1 {
                    put_pixel(fb, cx + gx as i32, y + gy as i32, color);
                }
            }
        }
        cx += (GLYPH_W + 1) as i32;
    }
}

fn draw_play_icon(fb: &FramebufferInfo, cx: i32, cy: i32, color: u32) {
    for dy in -4i32..=4 {
        let half = (4 - dy.abs()).max(0);
        for dx in -half..=half {
            if dx >= 0 {
                put_pixel(fb, cx + dx, cy + dy, color);
            }
        }
    }
}

fn draw_pause_icon(fb: &FramebufferInfo, cx: i32, cy: i32, color: u32) {
    for dy in -4i32..=4 {
        for dx in -3..=-1 {
            put_pixel(fb, cx + dx, cy + dy, color);
        }
        for dx in 1..=3 {
            put_pixel(fb, cx + dx, cy + dy, color);
        }
    }
}

pub struct UiState {
    pub playing: bool,
    pub current_frame: usize,
    pub total_frames: usize,
    pub fps_num: u32,
    pub fps_den: u32,
    pub status: alloc::string::String,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            playing: true,
            current_frame: 0,
            total_frames: 0,
            fps_num: 25,
            fps_den: 1,
            status: alloc::string::String::new(),
        }
    }

    pub fn render(&self, fb: &FramebufferInfo) {
        let w = fb.width as i32;
        let h = fb.height as i32;

        let bar_h = 60;
        let bar_y = h - bar_h;
        fill_rect(fb, 0, bar_y, w, bar_h, BG);

        let pb_h = 4;
        let pb_y = bar_y + 14;
        let pb_x = 80;
        let pb_w = w - 160;
        fill_rect(fb, pb_x, pb_y, pb_w, pb_h, MUTED);
        let progress = if self.total_frames > 0 {
            self.current_frame as f32 / self.total_frames as f32
        } else {
            0.0
        };
        let filled = (pb_w as f32 * progress) as i32;
        fill_rect(fb, pb_x, pb_y, filled, pb_h, ACCENT);

        let knob_x = pb_x + filled - 3;
        fill_rect(fb, knob_x, pb_y - 4, 6, pb_h + 8, FG);

        let icon_cx = 40;
        let icon_cy = pb_y;
        if self.playing {
            draw_pause_icon(fb, icon_cx, icon_cy, FG);
        } else {
            draw_play_icon(fb, icon_cx, icon_cy, FG);
        }

        let cur_sec = frame_to_seconds(self.current_frame, self.fps_num, self.fps_den);
        let tot_sec = frame_to_seconds(self.total_frames, self.fps_num, self.fps_den);
        let time_str = alloc::format!(
            "{:02}:{:02} / {:02}:{:02}",
            cur_sec / 60,
            cur_sec % 60,
            tot_sec / 60,
            tot_sec % 60
        );
        let text_w = time_str.len() as i32 * (GLYPH_W as i32 + 1);
        draw_text(fb, w - 16 - text_w, pb_y - 1, &time_str, FG);

        if !self.status.is_empty() {
            let sy = 12;
            draw_text(fb, 12 + 1, sy + 1, &self.status, SHADOW);
            draw_text(fb, 12, sy, &self.status, FG);
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

fn frame_to_seconds(frame: usize, fps_num: u32, fps_den: u32) -> u32 {
    if fps_num == 0 {
        return 0;
    }
    ((frame as u64 * fps_den as u64) / fps_num as u64) as u32
}
