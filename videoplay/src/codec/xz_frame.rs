//! DLV / LZMA2 codec — each frame is a raw LZMA2 stream that decompresses
//! to exactly `width * height * 4` bytes of BGRA.
//!
//! Uses `lzma-rust2` (pure Rust, `no_std`-friendly). The DLV file format
//! stores **raw LZMA2 streams** (not the xz container) so we can decode
//! them without pulling in SHA-2 / CRC64, which don't currently compile
//! cleanly on `x86_64-unknown-none`.

use crate::codec::{Codec, DecodeError, PixelFormat};
use crate::container::Frame;

pub struct XzFrameCodec;

impl XzFrameCodec {
    pub fn new() -> Self {
        Self
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
        let n = lzma2_decompress_into(&frame.data, out)?;
        if n != out.len() {
            return Err(alloc::format!(
                "dlv: decompressed size mismatch ({} != {})",
                n,
                out.len()
            ));
        }
        Ok(())
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
