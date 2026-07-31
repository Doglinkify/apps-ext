//! Platform abstraction layer.
//!
//! Two backends:
//!   * [`dlos`] — DoglinkOS-2nd: linear framebuffer + `int 0x80` syscalls.
//!   * [`sim`]  — Linux SDL2: window with the same framebuffer layout.

#[cfg(feature = "dlos")]
pub mod dlos;

#[cfg(all(feature = "sim", not(feature = "dlos")))]
pub mod sim;

use alloc::string::String;
use alloc::vec::Vec;

#[cfg(feature = "dlos")]
pub type Backend = dlos::DlosBackend;

#[cfg(all(feature = "sim", not(feature = "dlos")))]
pub type Backend = sim::SimBackend;

pub fn create_backend() -> Backend {
    #[cfg(feature = "dlos")]
    {
        dlos::DlosBackend::new()
    }
    #[cfg(all(feature = "sim", not(feature = "dlos")))]
    {
        sim::SimBackend::new()
    }
}

#[derive(Copy, Clone)]
pub struct FramebufferInfo {
    pub ptr: *mut u8,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
}

unsafe impl Send for FramebufferInfo {}
unsafe impl Sync for FramebufferInfo {}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Space,
    Q,
    Escape,
    O,
    Left,
    Right,
    Unknown(u8),
}

pub trait PlatformBackend {
    fn framebuffer(&mut self) -> FramebufferInfo;
    fn poll_key(&mut self) -> Option<Key>;
    fn ticks_ms(&self) -> u64;
    fn sleep_ms(&mut self, ms: u64);
    fn print(&mut self, msg: &str);
    fn present(&mut self) {}
    fn exit(&self) -> !;
}

pub fn load_file(path: &str) -> Result<Vec<u8>, String> {
    #[cfg(feature = "dlos")]
    {
        dlos::load_file(path)
    }
    #[cfg(all(feature = "sim", not(feature = "dlos")))]
    {
        sim::load_file(path)
    }
    #[cfg(not(any(feature = "dlos", feature = "sim")))]
    {
        let _ = path;
        Err("no platform backend enabled".into())
    }
}
