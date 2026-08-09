//! DoglinkOS-2nd platform backend.
//!
//! Talks to the kernel via `int 0x80` software interrupts. Syscall numbers
//! and register conventions mirror `app-rt/src/lib.rs` in the DoglinkOS-2nd tree:
//!
//! | rax | name           | in                            | out         |
//! |-----|----------------|-------------------------------|-------------|
//! | 1   | write          | rdi=fd, rsi=ptr, rcx=len      |             |
//! | 4   | exit           |                               |             |
//! | 5   | read stdin     |                               | rcx=u8      |
//! | 7   | brk            | rdi=0 / rdi=new_brk           | rsi=brk     |
//! | 10  | getticks       |                               | rcx=usize   |
//! | 11  | info           | rdi=type                      | rcx=usize   |
//! | 12  | open           | rdi=ptr, rcx=len, r10=create  | rsi=fd      |
//! | 13  | read2          | rsi=fd, rdi=ptr, rcx=len      |             |
//! | 14  | seek           | rsi=fd, rdi=from, rcx=off     | r10=new_off |
//! | 15  | close          | rsi=fd                        |             |
//!
//! `sys_info` types (rdi):
//!   6 = fb width, 7 = fb height, 8 = fb pointer, 9 = fb pitch.
//!
//! Compiled only when `--features dlos` is on.

#![cfg(feature = "dlos")]

use crate::platform::{FramebufferInfo, Key, PlatformBackend};

use alloc::string::String;
use alloc::vec::Vec;
use core::arch::asm;
use core::fmt::{self, Write};

pub fn sys_write(fd: usize, buf: &[u8]) {
    unsafe {
        asm!(
            "int 0x80",
            in("rax") 1,
            in("rdi") fd,
            in("rsi") buf.as_ptr(),
            in("rcx") buf.len(),
        );
    }
}

pub fn sys_exit() -> ! {
    unsafe {
        asm!("int 0x80", in("rax") 4);
        unreachable!();
    }
}

pub fn sys_read_raw() -> u8 {
    let result: u64;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") 5,
            out("rcx") result,
        );
    }
    result as u8
}

pub fn sys_read_blocking() -> u8 {
    loop {
        let b = sys_read_raw();
        if b != 0xff {
            return b;
        }
        sys_sleep_ms(1);
    }
}

pub fn sys_getticks() -> usize {
    let res;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") 10,
            out("rcx") res,
        );
    }
    res
}

pub fn sys_info(tp: u64) -> Option<usize> {
    let res;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") 11,
            in("rdi") tp,
            out("rcx") res,
        );
    }
    match res {
        usize::MAX => None,
        v => Some(v),
    }
}

pub fn sys_open(name: &str, do_create: bool) -> Option<usize> {
    let res;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") 12,
            in("rdi") name.as_ptr(),
            in("rcx") name.len(),
            in("r10") do_create as usize,
            out("rsi") res,
        );
    }
    match res {
        usize::MAX => None,
        v => Some(v),
    }
}

pub fn sys_read3(fd: usize, buf: &mut [u8]) -> usize {
    let res;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") 22,
            in("rsi") fd,
            in("rdi") buf.as_mut_ptr(),
            in("rcx") buf.len(),
            out("r10") res,
        );
    }
    res
}

pub fn sys_seek(fd: usize, offset: isize, from: usize) -> usize {
    let res;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") 14,
            in("rsi") fd,
            in("rdi") from,
            in("rcx") offset,
            out("r10") res,
        );
    }
    res
}

pub const SEEK_CUR: usize = 0;
pub const SEEK_END: usize = 1;
pub const SEEK_SET: usize = 2;

pub fn sys_close(fd: usize) {
    unsafe {
        asm!(
            "int 0x80",
            in("rax") 15,
            in("rsi") fd,
        );
    }
}

pub fn print_raw(s: &str) {
    sys_write(1, s.as_bytes());
}

pub fn eprint_raw(s: &str) {
    sys_write(0, s.as_bytes());
}

pub struct PanicWriter;

impl Write for PanicWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        eprint_raw(s);
        Ok(())
    }
}

pub fn read_line(buf: &mut [u8]) -> usize {
    for (i, v) in buf.iter_mut().enumerate() {
        match sys_read_blocking() {
            b'\n' => return i,
            b'\r' => return i,
            c => *v = c,
        }
    }
    buf.len()
}

pub fn sys_sleep_ms(ms: u64) {
    let start = sys_getticks() as u64;
    while (sys_getticks() as u64).wrapping_sub(start) < ms / 10 {
        core::hint::spin_loop();
    }
}

pub fn load_file(path: &str) -> Result<Vec<u8>, String> {
    let fd = sys_open(path, false).ok_or_else(|| format!("open '{path}' failed"))?;
    let size = sys_seek(fd, 0, SEEK_END);
    sys_seek(fd, 0, SEEK_SET);
    let mut buf = vec![0u8; size];
    let mut read = 0usize;
    while read < size {
        let n = sys_read3(fd, &mut buf[read..]);
        if n == 0 {
            break;
        }
        read += n;
    }
    sys_close(fd);
    buf.truncate(read);
    Ok(buf)
}

pub struct DlosBackend {
    last_key: Option<u8>,
    // Arrow keys arrive from the terminal as multi-byte ANSI sequences. Keep
    // enough state across non-blocking polls to avoid treating their ESC
    // prefix as the quit key.
    escape_state: EscapeState,
    pending_key: Option<u8>,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum EscapeState {
    None,
    Escape,
    Csi,
}

impl DlosBackend {
    pub fn new() -> Self {
        if sys_info(8).is_none() {
            eprint_raw("videoplay: no framebuffer available from kernel\n");
            sys_exit();
        }
        Self {
            last_key: None,
            escape_state: EscapeState::None,
            pending_key: None,
        }
    }
}

impl Default for DlosBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformBackend for DlosBackend {
    fn framebuffer(&mut self) -> FramebufferInfo {
        FramebufferInfo {
            ptr: sys_info(8).unwrap_or(0) as *mut u8,
            width: sys_info(6).unwrap_or(0),
            height: sys_info(7).unwrap_or(0),
            pitch: sys_info(9).unwrap_or(0),
        }
    }

    fn poll_key(&mut self) -> Option<Key> {
        let b = self.pending_key.take().unwrap_or_else(sys_read_raw);
        if b == 0xff {
            // The kernel reports no pending input with 0xff. Treat that as
            // the end of the current key press so the next press of the same
            // key is not discarded as a duplicate.
            self.last_key = None;
            return match self.escape_state {
                // A standalone Escape key has no following byte. Delay it by
                // one poll so an ANSI arrow sequence can be recognized.
                EscapeState::Escape => {
                    self.escape_state = EscapeState::None;
                    Some(Key::Escape)
                }
                EscapeState::Csi => {
                    self.escape_state = EscapeState::None;
                    None
                }
                EscapeState::None => None,
            };
        }

        match self.escape_state {
            EscapeState::Escape => {
                if b == b'[' || b == b'O' {
                    self.escape_state = EscapeState::Csi;
                    return None;
                }
                self.escape_state = EscapeState::None;
                self.pending_key = Some(b);
                self.last_key = None;
                return Some(Key::Escape);
            }
            EscapeState::Csi => {
                self.escape_state = EscapeState::None;
                return match b {
                    b'C' => Some(Key::Right),
                    b'D' => Some(Key::Left),
                    _ => None,
                };
            }
            EscapeState::None => {}
        }

        if self.last_key == Some(b) {
            return None;
        }
        self.last_key = Some(b);
        if b == 0x1b {
            self.escape_state = EscapeState::Escape;
            return None;
        }
        Some(byte_to_key(b))
    }

    fn ticks_ms(&self) -> u64 {
        sys_getticks() as u64 * 10
    }

    fn sleep_ms(&mut self, ms: u64) {
        sys_sleep_ms(ms);
    }

    fn print(&mut self, msg: &str) {
        print_raw(msg);
    }

    fn exit(&self) -> ! {
        sys_exit();
    }
}

fn byte_to_key(b: u8) -> Key {
    match b {
        b' ' => Key::Space,
        b'q' | b'Q' => Key::Q,
        0x1b => Key::Escape,
        b'o' | b'O' => Key::O,
        _ => Key::Unknown(b),
    }
}
