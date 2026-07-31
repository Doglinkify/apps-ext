//! MJPEG codec — each frame is an independent JPEG image.
//!
//! Uses `zune-jpeg` (pure Rust, `no_std`-friendly). Decoder yields RGB888
//! pixels; we expand to BGRA32 to match the framebuffer layout.

use crate::codec::{Codec, DecodeError, PixelFormat};
use crate::container::Frame;

use alloc::string::ToString;

pub struct MjpegCodec {
    width: u32,
    height: u32,
}

impl MjpegCodec {
    pub fn new() -> Self {
        Self { width: 0, height: 0 }
    }
}

impl Default for MjpegCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for MjpegCodec {
    fn output_format(&self) -> (u32, u32, PixelFormat) {
        (self.width, self.height, PixelFormat::Bgra32)
    }

    fn decode(&mut self, frame: &Frame, out: &mut [u8]) -> Result<(), DecodeError> {
        use zune_core::bytestream::ZCursor;
        use zune_jpeg::JpegDecoder;

        let mut dec = JpegDecoder::new(ZCursor::new(&frame.data));
        dec.decode_headers()
            .map_err(|e| alloc::format!("mjpeg: header decode failed: {e:?}"))?;
        let (w, h) = dec
            .dimensions()
            .ok_or_else(|| "mjpeg: missing dimensions".to_string())?;
        self.width = w as u32;
        self.height = h as u32;

        let pixels = dec
            .decode()
            .map_err(|e| alloc::format!("mjpeg: body decode failed: {e:?}"))?;

        let expected_in = w * h * 3;
        if pixels.len() < expected_in {
            return Err(alloc::format!(
                "mjpeg: short output ({} < {})",
                pixels.len(),
                expected_in
            ));
        }
        let expected_out = w * h * 4;
        if out.len() < expected_out {
            return Err(alloc::format!(
                "mjpeg: output buffer too small ({} < {})",
                out.len(),
                expected_out
            ));
        }

        for i in (0..(w * h)).rev() {
            let r = pixels[i * 3];
            let g = pixels[i * 3 + 1];
            let b = pixels[i * 3 + 2];
            out[i * 4] = b;
            out[i * 4 + 1] = g;
            out[i * 4 + 2] = r;
            out[i * 4 + 3] = 0xff;
        }
        Ok(())
    }
}
