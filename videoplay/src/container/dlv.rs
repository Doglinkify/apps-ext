//! DLV (DoglinkOS Lossless Video) container — minimal custom format.
//!
//! File layout (all LE):
//!   [0..4]   "DLV1" (independent frames) or "DLV2" (delta frames)
//!   [4..8]   width
//!   [8..12]  height
//!   [12..16] fps_num
//!   [16..20] fps_den
//!   [20..24] frame_count
//!   [24..28] reserved (0)
//!   [28..]   frame_count entries, each:
//!     [0..4]   compressed_size
//!     [4..4+N] lzma2_payload (decompresses to width*height*4 bytes of BGRA)
//!
//! Raw LZMA2 streams (not xz container) so we can decode on `no_std`
//! without sha2/CRC64.

use crate::container::{CodecId, Container, Frame};

use alloc::string::String;
use alloc::vec::Vec;

const MAGIC_V1: &[u8; 4] = b"DLV1";
const MAGIC_V2: &[u8; 4] = b"DLV2";
const HEADER_LEN: usize = 28;

pub struct Dlv {
    data: Vec<u8>,
    width: u32,
    height: u32,
    fps_num: u32,
    fps_den: u32,
    frame_count: usize,
    frame_offsets: Vec<usize>,
    keyframes: Vec<bool>,
    version: u8,
    cursor: usize,
}

impl Dlv {
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, String> {
        if data.len() < HEADER_LEN {
            return Err("dlv: file too short for header".into());
        }
        let version = if &data[0..4] == MAGIC_V1 { 1 } else if &data[0..4] == MAGIC_V2 { 2 } else {
            return Err("dlv: bad magic".into());
        };
        let width = rd_u32(&data, 4);
        let height = rd_u32(&data, 8);
        let fps_num = rd_u32(&data, 12);
        let fps_den = rd_u32(&data, 16);
        let frame_count = rd_u32(&data, 20) as usize;

        if width == 0 || height == 0 || fps_den == 0 || frame_count == 0 {
            return Err("dlv: invalid header (zero width/height/fps_den)".into());
        }

        let mut offsets = Vec::with_capacity(frame_count);
        let mut keyframes = Vec::with_capacity(frame_count);
        let mut pos = HEADER_LEN;
        for _ in 0..frame_count {
            if pos + 4 > data.len() {
                return Err("dlv: truncated frame table".into());
            }
            let comp_size = rd_u32(&data, pos) as usize;
            offsets.push(pos);
            if version == 2 {
                if pos + 5 > data.len() { return Err("dlv: truncated frame entry".into()); }
                let keyframe = data[pos + 4] & 1 != 0;
                if offsets.len() == 1 && !keyframe {
                    return Err("dlv: first frame must be a keyframe".into());
                }
                keyframes.push(keyframe);
                pos += 5 + comp_size;
            } else {
                keyframes.push(true);
                pos += 4 + comp_size;
            }
            if pos > data.len() {
                return Err("dlv: truncated frame payload".into());
            }
        }

        Ok(Dlv {
            data,
            width,
            height,
            fps_num,
            fps_den,
            frame_count,
            frame_offsets: offsets,
            keyframes,
            version,
            cursor: 0,
        })
    }
}

impl Container for Dlv {
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn frame_count(&self) -> usize {
        self.frame_count
    }
    fn fps(&self) -> (u32, u32) {
        (self.fps_num, self.fps_den)
    }
    fn codec_id(&self) -> CodecId {
        CodecId::DlvXz
    }

    fn next_frame(&mut self) -> Option<Frame> {
        if self.cursor >= self.frame_offsets.len() {
            return None;
        }
        let off = self.frame_offsets[self.cursor];
        self.cursor += 1;
        let comp_size = rd_u32(&self.data, off) as usize;
        let start = off + if self.version == 2 { 5 } else { 4 };
        let payload = &self.data[start..start + comp_size];
        Some(Frame {
            data: payload.to_vec(),
            keyframe: self.keyframes[self.cursor - 1],
        })
    }

    fn seek(&mut self, frame_idx: usize) -> usize {
        let clamped = frame_idx.min(self.frame_offsets.len().saturating_sub(1));
        let clamped = if self.version == 2 {
            (0..=clamped).rev().find(|&i| self.keyframes[i]).unwrap_or(0)
        } else { clamped };
        self.cursor = clamped;
        clamped
    }
}

fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
