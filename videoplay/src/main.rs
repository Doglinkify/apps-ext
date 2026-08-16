//! dlos-videoplay — entry point and feature dispatch.
//!
//! This crate builds in two modes:
//!
//! * `--features dlos`   — bare-metal `#![no_std]` `#![no_main]` binary that
//!   runs on DoglinkOS-2nd using `int 0x80` syscalls
//!   and the kernel-provided linear framebuffer.
//!
//! * `--features sim`    — normal Linux std binary that opens an SDL2 window
//!   emulating the same framebuffer + keyboard, used
//!   for development, CI smoke tests, and demos.
//!
//! All player logic lives in the `player`, `container`, `codec`, `ui`, and
//! `input` modules; both backends share that logic through the
//! [`platform::PlatformBackend`] trait.

#![cfg_attr(feature = "dlos", no_std)]
#![cfg_attr(feature = "dlos", no_main)]

#[cfg(not(any(feature = "sim", feature = "dlos")))]
compile_error!("Either the \"sim\" or \"dlos\" feature must be enabled.");

#[cfg(all(feature = "sim", feature = "dlos"))]
compile_error!("The \"sim\" and \"dlos\" features are mutually exclusive.");

#[cfg(all(feature = "sim", not(target_os = "linux")))]
compile_error!("The \"sim\" feature requires a Linux target OS.");

#[cfg(all(feature = "dlos", not(all(target_arch = "x86_64", target_os = "none"))))]
compile_error!("The \"dlos\" feature requires the target triple x86_64-unknown-none.");

// `alloc` is available on both targets (std re-exports it). `#[macro_use]`
// brings the `vec!` / `format!` macros into scope for all submodules.
#[cfg_attr(feature = "dlos", macro_use)]
#[cfg_attr(not(feature = "dlos"), macro_use)]
extern crate alloc;

// Bring `Box`/`String`/`Vec` into scope at the crate root so all
// modules can refer to them without `alloc::` prefixes.
#[allow(unused_imports)]
use alloc::{boxed::Box, string::String, vec::Vec};

#[cfg(feature = "dlos")]
#[global_allocator]
static ALLOCATOR: good_memory_allocator::SpinLockedAllocator =
    good_memory_allocator::SpinLockedAllocator::empty();

pub mod codec;
pub mod container;
pub mod input;
pub mod platform;
pub mod player;
pub mod ui;

// ---------------------------------------------------------------------------
// dlos entry: `_start`, panic handler, heap init
// ---------------------------------------------------------------------------

#[cfg(feature = "dlos")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    platform::dlos::eprint_raw("videoplay: panic\n");
    if let Some(loc) = info.location() {
        platform::dlos::eprint_raw("  at ");
        platform::dlos::eprint_raw(loc.file());
        platform::dlos::eprint_raw("\n");
    }
    {
        use core::fmt::Write;
        let _ = write!(platform::dlos::PanicWriter, "{}", info.message());
        platform::dlos::eprint_raw("\n");
    }
    platform::dlos::sys_exit();
}

/// Heap size: 1 GiB.
#[cfg(feature = "dlos")]
const HEAP_SIZE: usize = 1 << 30;

#[cfg(feature = "dlos")]
fn init_heap() {
    use core::arch::asm;
    unsafe {
        let old_brk: usize;
        asm!(
            "int 0x80",
            in("rax") 7,
            in("rdi") 0,
            out("rsi") old_brk,
        );
        asm!(
            "int 0x80",
            in("rax") 7,
            in("rdi") old_brk + HEAP_SIZE,
            out("rsi") _,
        );
        ALLOCATOR.init(old_brk, HEAP_SIZE);
    }
}

#[cfg(feature = "dlos")]
#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    init_heap();
    if let Err(e) = main_inner() {
        platform::dlos::eprint_raw("videoplay: error: ");
        platform::dlos::eprint_raw(&e);
        platform::dlos::eprint_raw("\n");
    }
    platform::dlos::sys_exit();
}

#[cfg(not(feature = "dlos"))]
fn main() {
    if let Err(msg) = main_inner() {
        eprintln!("videoplay: error: {msg}");
        std::process::exit(1);
    }
}

fn main_inner() -> Result<(), alloc::string::String> {
    #[cfg(feature = "dlos")]
    {
        platform::dlos::eprint_raw("videoplay: starting (");
        platform::dlos::eprint_raw(env!("CARGO_PKG_VERSION"));
        platform::dlos::eprint_raw(")\n");
    }

    #[cfg(feature = "dlos")]
    let path: alloc::string::String = {
        platform::dlos::print_raw("Video file path: ");
        let mut buf = [0u8; 256];
        let n = platform::dlos::read_line(&mut buf);
        alloc::string::String::from_utf8_lossy(&buf[..n]).into_owned()
    };

    #[cfg(not(feature = "dlos"))]
    let path: alloc::string::String = {
        use std::env;
        let mut it = env::args();
        let _ = it.next();
        let mut path = None;
        let mut no_display = false;
        let mut max_frames: Option<u64> = None;
        while let Some(a) = it.next() {
            match a.as_str() {
                "--no-display" => no_display = true,
                "--max-frames" => {
                    if let Some(v) = it.next() {
                        max_frames = v.parse().ok();
                    }
                }
                other => path = Some(other.to_string()),
            }
        }
        let p = match path {
            Some(p) => p,
            None => {
                return Err("usage: videoplay [--no-display] [--max-frames N] <file>".into());
            }
        };
        platform::sim::set_options(platform::sim::Options {
            no_display,
            max_frames,
        });
        p
    };

    let file_bytes =
        platform::load_file(&path).map_err(|e| alloc::format!("failed to open '{path}': {e}"))?;

    if file_bytes.is_empty() {
        return Err("file is empty".into());
    }

    let mut container: Box<dyn container::Container> = if file_bytes.starts_with(b"RIFF")
        && file_bytes.len() >= 12
        && &file_bytes[8..12] == b"AVI "
    {
        Box::new(container::avi::Avi::from_bytes(file_bytes)?)
    } else if file_bytes.starts_with(b"DLV1") || file_bytes.starts_with(b"DLV2") {
        Box::new(container::dlv::Dlv::from_bytes(file_bytes)?)
    } else {
        return Err("unknown file format (expected RIFF/AVI or DLV1/DLV2)".into());
    };

    let codec_id = container.codec_id();
    let mut codec: Box<dyn codec::Codec> = match codec_id {
        container::CodecId::Mjpeg => {
            #[cfg(feature = "codec-mjpeg")]
            {
                Box::new(codec::mjpeg::MjpegCodec::new())
            }
            #[cfg(not(feature = "codec-mjpeg"))]
            {
                return Err("MJPEG codec disabled at build time".into());
            }
        }
        container::CodecId::H264 => {
            #[cfg(feature = "codec-h264")]
            {
                Box::new(codec::h264::H264Codec::new()?)
            }
            #[cfg(not(feature = "codec-h264"))]
            {
                return Err(
                    "H.264 codec disabled at build time. Build with --features codec-h264 \
                     (requires std + libopenh264). On DoglinkOS this codec is not \
                     available yet — use MJPEG-in-AVI or .dlv samples instead."
                        .into(),
                );
            }
        }
        container::CodecId::DlvXz => {
            #[cfg(feature = "codec-xz")]
            {
                Box::new(codec::xz_frame::XzFrameCodec::new())
            }
            #[cfg(not(feature = "codec-xz"))]
            {
                return Err("DLV/xz codec disabled at build time".into());
            }
        }
    };

    let mut backend = platform::create_backend();
    player::run(&mut backend, &mut *container, &mut *codec)
}
