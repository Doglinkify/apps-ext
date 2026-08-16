//! Minimal RIFF/AVI demuxer.

use crate::container::{CodecId, Container, Frame};

use alloc::string::String;
use alloc::vec::Vec;

pub fn err<T>(msg: &'static str) -> Result<T, String> {
    Err(alloc::format!("avi: {msg}"))
}

#[derive(Default, Clone, Copy)]
struct AviMainHeader {
    micro_sec_per_frame: u32,
    _max_bytes_per_sec: u32,
    _padding_granularity: u32,
    _flags: u32,
    total_frames: u32,
    _initial_frames: u32,
    _streams: u32,
    _suggested_buffer_size: u32,
    width: u32,
    height: u32,
    _reserved: [u32; 4],
}

#[allow(dead_code)]
#[derive(Default, Clone, Copy)]
struct AviStreamHeader {
    _fcc_type: [u8; 4],
    fcc_handler: [u8; 4],
    _flags: u32,
    _priority: u16,
    _language: u16,
    _initial_frames: u32,
    _scale: u32,
    _rate: u32,
    _start: u32,
    length: u32,
    _suggested_buffer_size: u32,
    _quality: u32,
    _sample_size: u32,
    _left: i16,
    _top: i16,
    _right: i16,
    _bottom: i16,
}

#[allow(dead_code)]
#[derive(Default, Clone, Copy)]
struct BitmapInfoHeader {
    _size: u32,
    width: i32,
    height: i32,
    _planes: u16,
    _bit_count: u16,
    compression: [u8; 4],
    _size_image: u32,
    _x_pels_per_meter: i32,
    _y_pels_per_meter: i32,
    _clr_used: u32,
    _clr_important: u32,
}

pub struct Avi {
    data: Vec<u8>,
    main_header: AviMainHeader,
    stream_header: AviStreamHeader,
    bitmap_header: BitmapInfoHeader,
    frame_offsets: Vec<(usize, usize)>,
    cursor: usize,
}

impl Avi {
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, String> {
        if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"AVI " {
            return err("not a RIFF/AVI file");
        }

        let mut me = Avi {
            data,
            main_header: AviMainHeader::default(),
            stream_header: AviStreamHeader::default(),
            bitmap_header: BitmapInfoHeader::default(),
            frame_offsets: Vec::new(),
            cursor: 0,
        };
        me.parse_riff()?;
        if me.frame_offsets.is_empty() {
            return err("no video frames found in movi list");
        }
        Ok(me)
    }

    fn parse_riff(&mut self) -> Result<(), String> {
        let mut pos = 12usize;
        while pos + 8 <= self.data.len() {
            let fourcc = &self.data[pos..pos + 4];
            let size = u32::from_le_bytes([
                self.data[pos + 4],
                self.data[pos + 5],
                self.data[pos + 6],
                self.data[pos + 7],
            ]) as usize;
            let payload_start = pos + 8;
            let payload_end = payload_start + size;
            if payload_end > self.data.len() {
                return err("truncated RIFF chunk");
            }

            if fourcc == b"LIST" {
                let list_type = &self.data[payload_start..payload_start + 4];
                let list_payload_start = payload_start + 4;
                let list_payload_end = payload_end;
                if list_type == b"hdrl" {
                    self.parse_hdrl(list_payload_start, list_payload_end)?;
                } else if list_type == b"movi" {
                    self.parse_movi(list_payload_start, list_payload_end)?;
                }
            }

            pos = payload_end + (payload_end & 1);
        }
        Ok(())
    }

    fn parse_hdrl(&mut self, start: usize, end: usize) -> Result<(), String> {
        let mut pos = start;
        while pos + 8 <= end {
            let fourcc = &self.data[pos..pos + 4];
            let size = u32::from_le_bytes([
                self.data[pos + 4],
                self.data[pos + 5],
                self.data[pos + 6],
                self.data[pos + 7],
            ]) as usize;
            let payload_start = pos + 8;
            let payload_end = payload_start + size;
            if payload_end > end {
                return err("truncated hdrl chunk");
            }

            if fourcc == b"avih" {
                self.main_header = parse_main_header(&self.data[payload_start..payload_end]);
            } else if fourcc == b"LIST" {
                let list_type = &self.data[payload_start..payload_start + 4];
                if list_type == b"strl" {
                    self.parse_strl(payload_start + 4, payload_end)?;
                }
            }

            pos = payload_end + (payload_end & 1);
        }
        Ok(())
    }

    fn parse_strl(&mut self, start: usize, end: usize) -> Result<(), String> {
        let mut pos = start;
        let mut saw_video = false;
        while pos + 8 <= end {
            let fourcc = &self.data[pos..pos + 4];
            let size = u32::from_le_bytes([
                self.data[pos + 4],
                self.data[pos + 5],
                self.data[pos + 6],
                self.data[pos + 7],
            ]) as usize;
            let payload_start = pos + 8;
            let payload_end = payload_start + size;
            if payload_end > end {
                return err("truncated strl chunk");
            }

            if fourcc == b"strh" {
                let sh = parse_stream_header(&self.data[payload_start..payload_end]);
                if &sh._fcc_type == b"vids" {
                    self.stream_header = sh;
                    saw_video = true;
                }
            } else if fourcc == b"strf" && saw_video {
                self.bitmap_header = parse_bitmap_header(&self.data[payload_start..payload_end]);
            }

            pos = payload_end + (payload_end & 1);
        }
        Ok(())
    }

    fn parse_movi(&mut self, start: usize, end: usize) -> Result<(), String> {
        let mut pos = start;
        while pos + 8 <= end {
            let fourcc = &self.data[pos..pos + 4];
            let size = u32::from_le_bytes([
                self.data[pos + 4],
                self.data[pos + 5],
                self.data[pos + 6],
                self.data[pos + 7],
            ]) as usize;
            let payload_start = pos + 8;
            let payload_end = payload_start + size;
            if payload_end > end {
                break;
            }

            if &fourcc[2..4] == b"dc" || &fourcc[2..4] == b"db" {
                self.frame_offsets.push((payload_start, size));
            } else if fourcc == b"LIST" {
                let list_type = &self.data[payload_start..payload_start + 4];
                if list_type == b"rec " {
                    let inner_start = payload_start + 4;
                    let mut ip = inner_start;
                    while ip + 8 <= payload_end {
                        let ifcc = &self.data[ip..ip + 4];
                        let isize = u32::from_le_bytes([
                            self.data[ip + 4],
                            self.data[ip + 5],
                            self.data[ip + 6],
                            self.data[ip + 7],
                        ]) as usize;
                        let ips = ip + 8;
                        let ipe = ips + isize;
                        if ipe > payload_end {
                            break;
                        }
                        if &ifcc[2..4] == b"dc" || &ifcc[2..4] == b"db" {
                            self.frame_offsets.push((ips, isize));
                        }
                        ip = ipe + (ipe & 1);
                    }
                }
            }

            pos = payload_end + (payload_end & 1);
        }
        Ok(())
    }

    fn codec_fourcc(&self) -> &[u8; 4] {
        &self.bitmap_header.compression
    }
}

impl Container for Avi {
    fn width(&self) -> u32 {
        self.main_header.width
    }
    fn height(&self) -> u32 {
        self.main_header.height
    }
    fn frame_count(&self) -> usize {
        let from_stream = self.stream_header.length as usize;
        let from_main = self.main_header.total_frames as usize;
        let from_offsets = self.frame_offsets.len();
        from_stream.max(from_main).max(from_offsets)
    }
    fn fps(&self) -> (u32, u32) {
        let us = self.main_header.micro_sec_per_frame;
        if us == 0 {
            return (25, 1);
        }
        (1_000_000, us)
    }
    fn codec_id(&self) -> CodecId {
        match self.codec_fourcc() {
            b"MJPG" | b"JPEG" | b"jpeg" => CodecId::Mjpeg,
            b"H264" | b"h264" | b"X264" | b"x264" | b"avc1" | b"AVC1" => CodecId::H264,
            _ => CodecId::Mjpeg,
        }
    }

    fn next_frame(&mut self) -> Option<Frame> {
        if self.cursor >= self.frame_offsets.len() {
            return None;
        }
        let (off, len) = self.frame_offsets[self.cursor];
        self.cursor += 1;
        let data = self.data[off..off + len].to_vec();
        Some(Frame {
            data,
            keyframe: true,
        })
    }

    fn seek(&mut self, frame_idx: usize) -> usize {
        let clamped = frame_idx.min(self.frame_offsets.len().saturating_sub(1));
        self.cursor = clamped;
        clamped
    }
}

fn parse_main_header(b: &[u8]) -> AviMainHeader {
    if b.len() < 56 {
        return AviMainHeader::default();
    }
    AviMainHeader {
        micro_sec_per_frame: rd_u32(b, 0),
        _max_bytes_per_sec: rd_u32(b, 4),
        _padding_granularity: rd_u32(b, 8),
        _flags: rd_u32(b, 12),
        total_frames: rd_u32(b, 16),
        _initial_frames: rd_u32(b, 20),
        _streams: rd_u32(b, 24),
        _suggested_buffer_size: rd_u32(b, 28),
        width: rd_u32(b, 32),
        height: rd_u32(b, 36),
        _reserved: [rd_u32(b, 40), rd_u32(b, 44), rd_u32(b, 48), rd_u32(b, 52)],
    }
}

fn parse_stream_header(b: &[u8]) -> AviStreamHeader {
    if b.len() < 56 {
        return AviStreamHeader::default();
    }
    let mut fcc_type = [0u8; 4];
    let mut fcc_handler = [0u8; 4];
    fcc_type.copy_from_slice(&b[0..4]);
    fcc_handler.copy_from_slice(&b[4..8]);
    AviStreamHeader {
        _fcc_type: fcc_type,
        fcc_handler,
        _flags: rd_u32(b, 8),
        _priority: rd_u16(b, 12),
        _language: rd_u16(b, 14),
        _initial_frames: rd_u32(b, 16),
        _scale: rd_u32(b, 20),
        _rate: rd_u32(b, 24),
        _start: rd_u32(b, 28),
        length: rd_u32(b, 32),
        _suggested_buffer_size: rd_u32(b, 36),
        _quality: rd_u32(b, 40),
        _sample_size: rd_u32(b, 44),
        _left: rd_i16(b, 48),
        _top: rd_i16(b, 50),
        _right: rd_i16(b, 52),
        _bottom: rd_i16(b, 54),
    }
}

fn parse_bitmap_header(b: &[u8]) -> BitmapInfoHeader {
    if b.len() < 40 {
        return BitmapInfoHeader::default();
    }
    let mut compression = [0u8; 4];
    compression.copy_from_slice(&b[16..20]);
    BitmapInfoHeader {
        _size: rd_u32(b, 0),
        width: rd_i32(b, 4),
        height: rd_i32(b, 8),
        _planes: rd_u16(b, 12),
        _bit_count: rd_u16(b, 14),
        compression,
        _size_image: rd_u32(b, 20),
        _x_pels_per_meter: rd_i32(b, 24),
        _y_pels_per_meter: rd_i32(b, 28),
        _clr_used: rd_u32(b, 32),
        _clr_important: rd_u32(b, 36),
    }
}

fn rd_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn rd_i32(b: &[u8], o: usize) -> i32 {
    rd_u32(b, o) as i32
}
fn rd_u16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn rd_i16(b: &[u8], o: usize) -> i16 {
    rd_u16(b, o) as i16
}
