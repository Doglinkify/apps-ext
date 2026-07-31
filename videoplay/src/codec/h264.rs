//! H.264 codec — wraps the `openh264` C library (via the `openh264` crate).
//!
//! Only available when building with `--features codec-h264`, which implies
//! `std` and pulls in a pre-built `libopenh264` binary. On the DoglinkOS
//! target there is no H.264 hardware decoder and pure-Rust H.264 software
//! decoders are too heavy; use MJPEG-in-AVI or `.dlv` samples on bare metal.

#![cfg(feature = "codec-h264")]

use crate::codec::{Codec, DecodeError, PixelFormat};
use crate::container::Frame;

pub struct H264Codec {
    width: u32,
    height: u32,
    decoder: openh264::decoder::Decoder,
}

impl H264Codec {
    pub fn new() -> Result<Self, DecodeError> {
        let decoder = openh264::decoder::Decoder::new()
            .map_err(|e| alloc::format!("h264: failed to init openh264: {e}"))?;
        Ok(Self {
            width: 0,
            height: 0,
            decoder,
        })
    }
}

impl Codec for H264Codec {
    fn output_format(&self) -> (u32, u32, PixelFormat) {
        (self.width, self.height, PixelFormat::Bgra32)
    }

    fn decode(&mut self, frame: &Frame, out: &mut [u8]) -> Result<(), DecodeError> {
        use openh264::formats::YUVBuffer;

        self.decoder
            .decode(&frame.data)
            .map_err(|e| alloc::format!("h264: decode failed: {e}"))?;

        let yuv: YUVBuffer = self
            .decoder
            .decode_next()
            .map_err(|e| alloc::format!("h264: pull failed: {e}"))?
            .ok_or_else(|| "h264: no frame available".to_string())?;

        let (w, h) = (yuv.width(), yuv.height());
        self.width = w as u32;
        self.height = h as u32;

        if out.len() < w * h * 4 {
            return Err(alloc::format!(
                "h264: output buffer too small ({} < {})",
                out.len(),
                w * h * 4
            ));
        }

        let y_stride = yuv.y_stride();
        let u_stride = yuv.u_stride();
        let v_stride = yuv.v_stride();
        let y = yuv.y();
        let u = yuv.u();
        let v = yuv.v();
        for j in 0..h {
            for i in 0..w {
                let yv = y[j * y_stride + i];
                let uv = u[(j / 2) * u_stride + (i / 2)];
                let vv = v[(j / 2) * v_stride + (i / 2)];
                let (r, g, b) = yuv420_to_rgb(yv as i32, uv as i32 - 128, vv as i32 - 128);
                let o = (j * w + i) * 4;
                out[o] = b;
                out[o + 1] = g;
                out[o + 2] = r;
                out[o + 3] = 0xff;
            }
        }
        Ok(())
    }
}

fn yuv420_to_rgb(y: i32, u: i32, v: i32) -> (u8, u8, u8) {
    let y = y as f32;
    let u = u as f32;
    let v = v as f32;
    let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
    let g = (y - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0) as u8;
    let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;
    (r, g, b)
}
