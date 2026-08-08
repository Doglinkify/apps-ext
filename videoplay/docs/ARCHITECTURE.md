# Architecture

## Module layout

```
src/
├── main.rs           # entry point; feature-dispatched _start (dlos) or main (sim)
├── platform/
│   ├── mod.rs        # PlatformBackend trait, FramebufferInfo, Key
│   ├── dlos.rs       # int 0x80 syscall wrappers + DlosBackend
│   └── sim.rs        # SDL2 backend with the same FB layout
├── container/
│   ├── mod.rs        # Container trait, Frame, CodecId
│   ├── avi.rs        # RIFF/AVI demuxer
│   └── dlv.rs        # DLV custom demuxer
├── codec/
│   ├── mod.rs        # Codec trait, PixelFormat, DecodeError
│   ├── mjpeg.rs      # zune-jpeg → BGRA32
│   ├── xz_frame.rs   # LZMA2 decompress → BGRA32
│   └── h264.rs       # openh264 (std-only, behind feature)
├── ui.rs             # 5x7 bitmap font + control bar overlay
├── input.rs          # Key → Action mapping
└── player.rs         # the loop that ties everything together
```

## Two backends in one crate

Conditional compilation:
- `--features dlos` → `#![no_std]` `#![no_main]`, custom `_start`, `SpinLockedAllocator`.
- `--features sim`  → normal std binary, SDL2 window with same BGRA32 layout.

All shared code uses `core::` and `alloc::` only.

## Player loop (deadline-scheduled)

```
start_ticks = backend.ticks_ms()
for current_frame in 0..total_frames:
    deadline = start_ticks + current_frame * (1000 * fps_den / fps_num) - pause_offset
    now = backend.ticks_ms()

    # If we're more than one frame behind, skip this frame.
    if now > deadline + frame_period:
        container.next_frame()  # discard
        continue

    # If we're paused, just poll keys and re-render.
    if not playing:
        poll keys
        render UI
        continue

    # If we're early, sleep until the deadline.
    if now < deadline:
        backend.sleep_ms(deadline - now)

    # Decode + blit + render UI.
    frame = container.next_frame()
    codec.decode(frame, decode_buf)
    blit decode_buf → framebuffer (centered, letterboxed)
    ui.render(framebuffer)
    backend.present()

    # Non-blocking key poll.
    loop:
        key = backend.poll_key()
        if key is None: break
        handle key (quit / pause / seek / open)
```

## DLV format

Custom container for `no_std` targets where H.264 software decoding is
impractical. Per-frame raw LZMA2 streams (no xz container — sha2 crate
doesn't currently compile on `x86_64-unknown-none`).

```
Header (28 bytes, LE):
  [0..4]   "DLV1" or "DLV2"
  [4..8]   width
  [8..12]  height
  [12..16] fps_num
  [16..20] fps_den
  [20..24] frame_count
  [24..28] reserved (0)

DLV1 entries contain a size and an independent payload. DLV2 entries contain:
  [0..4]   compressed_size
  [4]      flags (bit 0: keyframe)
  [5..5+N] raw LZMA2 payload (BGRA keyframe or XOR delta from the prior frame)
```

Per-frame LZMA2 compression typically achieves 3-6× on real video.

## Why no H.264 on bare metal?

- `openh264` is a C library that doesn't cross-compile to `no_std`.
- Pure-Rust H.264 decoders are heavy (tens of thousands of lines).
- DoglinkOS runs under QEMU TCG (no KVM) — even MJPEG at 320x240/25fps
  uses noticeable CPU. H.264 software decode would drop frames.
