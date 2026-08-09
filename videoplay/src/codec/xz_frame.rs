//! DLV / LZMA2 codec — each frame is a raw LZMA2 stream that decompresses
//! to exactly `width * height * 4` bytes of BGRA.
//!
//! Uses `lzma-rust2` (pure Rust, `no_std`-friendly). The DLV file format
//! stores **raw LZMA2 streams** (not the xz container) so we can decode
//! them without pulling in SHA-2 / CRC64, which don't currently compile
//! cleanly on `x86_64-unknown-none`.

use crate::codec::{Codec, DecodeError, PixelFormat};
use crate::container::Frame;
use alloc::vec::Vec;

pub struct XzFrameCodec {
    previous: Vec<u8>,
}

impl XzFrameCodec {
    pub fn new() -> Self {
        Self {
            previous: Vec::new(),
        }
    }
}

impl Default for XzFrameCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for XzFrameCodec {
    fn output_format(&self) -> (u32, u32, PixelFormat) {
        (0, 0, PixelFormat::Bgra32)
    }

    fn decode(&mut self, frame: &Frame, out: &mut [u8]) -> Result<(), DecodeError> {
        let mut decoded = vec![0u8; out.len()];
        let n = lzma2_decompress_into(&frame.data, &mut decoded)?;
        if n != out.len() {
            return Err(alloc::format!(
                "dlv: decompressed size mismatch ({} != {})",
                n,
                out.len()
            ));
        }
        if frame.keyframe {
            out.copy_from_slice(&decoded);
        } else {
            if self.previous.len() != out.len() {
                return Err("dlv: delta frame without previous frame".into());
            }
            for (dst, (delta, prev)) in out.iter_mut().zip(decoded.iter().zip(self.previous.iter()))
            {
                *dst = *delta ^ *prev;
            }
        }
        self.previous.clear();
        self.previous.extend_from_slice(out);
        Ok(())
    }

    fn reset(&mut self) {
        self.previous.clear();
    }
}

fn lzma2_decompress_into(input: &[u8], out: &mut [u8]) -> Result<usize, DecodeError> {
    use lzma_rust2::{Lzma2Reader, Read};

    const DICT_SIZE: u32 = 8 << 20;
    let mut reader = Lzma2Reader::new(input, DICT_SIZE, None);
    let mut total = 0usize;
    while total < out.len() {
        let n = reader
            .read(&mut out[total..])
            .map_err(|e| alloc::format!("dlv: LZMA2 read failed: {e:?}"))?;
        if n == 0 {
            break;
        }
        total += n;
    }
    Ok(total)
}
