//! Container (de)muxer interface and implementations.

use alloc::vec::Vec;

pub mod avi;
pub mod dlv;

pub struct Frame {
    pub data: Vec<u8>,
    pub keyframe: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CodecId {
    Mjpeg,
    H264,
    DlvXz,
}

pub trait Container {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn frame_count(&self) -> usize;
    fn fps(&self) -> (u32, u32);
    fn codec_id(&self) -> CodecId;
    fn next_frame(&mut self) -> Option<Frame>;
    /// Seek to a decodable position and return its actual frame index.
    fn seek(&mut self, frame_idx: usize) -> usize;
}
