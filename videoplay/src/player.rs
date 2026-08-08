//! Player loop — drives container → codec → framebuffer pipeline at the
//! right frame rate, renders UI overlay, reacts to keyboard input.

use crate::codec::Codec;
use crate::container::Container;
use crate::input::{action_for, Action};
use crate::platform::{Backend, PlatformBackend};
use crate::ui::UiState;

use alloc::string::String;

pub fn run(
    backend: &mut Backend,
    container: &mut dyn Container,
    codec: &mut dyn Codec,
) -> Result<(), String> {
    let (vw, _vw_u32) = (container.width() as usize, container.width());
    let (vh, _vh_u32) = (container.height() as usize, container.height());
    let total_frames = container.frame_count();
    let (fps_num, fps_den) = container.fps();

    if vw == 0 || vh == 0 {
        return Err("container reported zero dimensions".into());
    }

    let mut decode_buf = vec![0u8; vw * vh * 4];

    let mut ui = UiState::new();
    ui.total_frames = total_frames;
    ui.fps_num = fps_num;
    ui.fps_den = fps_den;
    ui.status = alloc::string::String::from("PLAYING");
    ui.playing = true;

    let mut current_frame: usize = 0;
    let start_ticks = backend.ticks_ms();
    let mut paused_at_ticks: Option<u64> = None;
    let mut pause_offset: u64 = 0;

    while current_frame < total_frames {
        let frame_ms =
            (current_frame as u64 * 1000 * fps_den as u64) / (fps_num.max(1) as u64);
        let deadline = start_ticks + frame_ms - pause_offset;
        let now = backend.ticks_ms();

        if now > deadline + frame_period_ms(fps_num, fps_den) {
            // Decode skipped frames as well: DLV2 delta frames depend on the
            // previous decoded image even when it is not displayed.
            if let Some(frame) = container.next_frame() {
                let _ = codec.decode(&frame, &mut decode_buf);
            }
            current_frame += 1;
            continue;
        }

        if !ui.playing {
            let action = action_for(backend.poll_key());
            match action {
                Action::Quit => backend.exit(),
                Action::TogglePlay => {
                    ui.playing = true;
                    ui.status = String::from("PLAYING");
                    if let Some(t) = paused_at_ticks.take() {
                        pause_offset += backend.ticks_ms() - t;
                    }
                }
                Action::SeekBackward => {
                    let delta = seconds_to_frames(5, fps_num, fps_den);
                    current_frame = current_frame.saturating_sub(delta);
                    current_frame = container.seek(current_frame);
                    codec.reset();
                }
                Action::SeekForward => {
                    let delta = seconds_to_frames(5, fps_num, fps_den);
                    current_frame = (current_frame + delta).min(total_frames - 1);
                    current_frame = container.seek(current_frame);
                    codec.reset();
                }
                Action::Open => {
                    ui.status = String::from("OPEN (re-launch with new path)");
                    backend.print("\nvideoplay: 'O' pressed — re-launch with new path\n");
                    backend.exit();
                }
                Action::None => {}
            }

            ui.current_frame = current_frame;
            blit_and_render(backend, &decode_buf, vw, vh, &ui);
            backend.present();
            backend.sleep_ms(16);
            continue;
        }

        if now < deadline {
            backend.sleep_ms(deadline - now);
        }

        let frame = match container.next_frame() {
            Some(f) => f,
            None => break,
        };
        if let Err(e) = codec.decode(&frame, &mut decode_buf) {
            return Err(alloc::format!("decode error at frame {current_frame}: {e}"));
        }

        ui.current_frame = current_frame;
        blit_and_render(backend, &decode_buf, vw, vh, &ui);
        backend.present();

        loop {
            match action_for(backend.poll_key()) {
                Action::Quit => backend.exit(),
                Action::TogglePlay => {
                    ui.playing = false;
                    ui.status = String::from("PAUSED");
                    paused_at_ticks = Some(backend.ticks_ms());
                }
                Action::Open => {
                    backend.print("\nvideoplay: 'O' pressed — re-launch with new path\n");
                    backend.exit();
                }
                Action::SeekBackward => {
                    let delta = seconds_to_frames(5, fps_num, fps_den);
                    current_frame = current_frame.saturating_sub(delta);
                    current_frame = container.seek(current_frame);
                    codec.reset();
                    continue;
                }
                Action::SeekForward => {
                    let delta = seconds_to_frames(5, fps_num, fps_den);
                    current_frame = (current_frame + delta).min(total_frames - 1);
                    current_frame = container.seek(current_frame);
                    codec.reset();
                    continue;
                }
                Action::None => break,
            }
        }

        current_frame += 1;
    }

    ui.status = String::from("END");
    ui.playing = false;
    loop {
        ui.current_frame = current_frame.saturating_sub(1);
        blit_and_render(backend, &decode_buf, vw, vh, &ui);
        backend.present();
        match action_for(backend.poll_key()) {
            Action::Quit | Action::TogglePlay => backend.exit(),
            Action::Open => {
                backend.print("\nvideoplay: 'O' pressed — re-launch with new path\n");
                backend.exit();
            }
            _ => {}
        }
        backend.sleep_ms(50);
    }
}

fn frame_period_ms(fps_num: u32, fps_den: u32) -> u64 {
    if fps_num == 0 {
        return 40;
    }
    (1000 * fps_den as u64) / fps_num as u64
}

fn seconds_to_frames(sec: u32, fps_num: u32, fps_den: u32) -> usize {
    (sec as u64 * fps_num as u64 / fps_den.max(1) as u64) as usize
}

fn blit_and_render(
    backend: &mut Backend,
    decode_buf: &[u8],
    vw: usize,
    vh: usize,
    ui: &UiState,
) {
    let fb = backend.framebuffer();
    let fw = fb.width;
    let fh = fb.height;

    for y in 0..fh {
        for x in 0..fw {
            let off = y * fb.pitch + x * 4;
            unsafe {
                *(fb.ptr.add(off) as *mut u32) = 0xFF000000;
            }
        }
    }

    let scale_w = fw as f32 / vw as f32;
    let scale_h = fh as f32 / vh as f32;
    let scale = scale_w.min(scale_h);
    let out_w = (vw as f32 * scale) as usize;
    let out_h = (vh as f32 * scale) as usize;
    let dx = (fw - out_w) / 2;
    let dy = (fh - out_h) / 2;

    for y in 0..out_h {
        let sy = (y as f32 / scale) as usize;
        if sy >= vh {
            continue;
        }
        for x in 0..out_w {
            let sx = (x as f32 / scale) as usize;
            if sx >= vw {
                continue;
            }
            let src = (sy * vw + sx) * 4;
            let dst = (y + dy) * fb.pitch + (x + dx) * 4;
            if dst + 4 <= fb.pitch * fh {
                unsafe {
                    let b = *decode_buf.get_unchecked(src);
                    let g = *decode_buf.get_unchecked(src + 1);
                    let r = *decode_buf.get_unchecked(src + 2);
                    let a = *decode_buf.get_unchecked(src + 3);
                    *(fb.ptr.add(dst) as *mut u32) =
                        (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | (b as u32);
                }
            }
        }
    }

    ui.render(&fb);
}
