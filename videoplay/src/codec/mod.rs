//! Codec interface and implementations.

pub mod h264;
pub mod mjpeg;
pub mod xz_frame;

use crate::container::Frame;

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    Bgra32,
}

pub type DecodeError = String;

pub trait Codec {
    fn output_format(&self) -> (u32, u32, PixelFormat);
    fn decode(&mut self, frame: &Frame, out: &mut [u8]) -> Result<(), DecodeError>;
}
