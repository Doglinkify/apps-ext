# dlos-videoplay

A modern, minimalist, **silent** video player for [DoglinkOS-2nd]. Designed
to run on the bare-metal framebuffer with no GPU, no audio, and no std.

[DoglinkOS-2nd]: https://github.com/Doglinkify/DoglinkOS-2nd

## Features

- **Two file formats**:
  - **AVI container** with MJPEG codec (H.264 with `openh264` on Linux sim only).
  - **DLV** — custom format: raw LZMA2-compressed BGRA pixels with periodic
    keyframes and XOR frame deltas (DLV2), while reading legacy DLV1 files.
    Designed for targets where H.264 software decoding is too heavy.
- **Modern minimal UI** drawn directly on the framebuffer:
  - Slim bottom control bar with progress, playhead knob, play/pause icon.
  - `MM:SS / MM:SS` time readout.
  - Top-left status label (`PLAYING`, `PAUSED`, `END`).
- **Keyboard controls**: Space (pause/play), Q/ESC (quit), O (open), ←/→ (seek 5s).

## Building for DoglinkOS

The dlos target is a bare-metal `x86_64-unknown-none` binary.

```bash
rustup target add x86_64-unknown-none
cargo build --release \
    --target x86_64-unknown-none \
    --no-default-features \
    --features "dlos codec-mjpeg codec-xz"
```

Result: `target/x86_64-unknown-none/release/videoplay` (~224 KB).

## DLV file format

Custom container for `no_std` targets. DLV1 remains supported for old files;
new files use DLV2:

```
Header (28 bytes, LE):
  [0..4]   "DLV2"
  [4..8]   width
  [8..12]  height
  [12..16] fps_num
  [16..20] fps_den
  [20..24] frame_count
  [24..28] reserved (0)

frame_count frame entries, each:
  [0..4]   compressed_size
  [4]      flags (bit 0: keyframe)
  [5..5+N] raw LZMA2 payload. Keyframes decompress to BGRA; other frames
           decompress to a bytewise XOR delta from the previous frame.
```

Uses raw LZMA2 streams (not the xz container) so the bare-metal target
can decode with the pure-Rust `lzma-rust2` crate without pulling in
SHA-2 / CRC64 (which don't currently compile on `x86_64-unknown-none`).

## License

MIT — see [`LICENSE`](LICENSE).
