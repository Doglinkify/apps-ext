# apps-ext

这是一个 doglinkos-2nd app 仓库
This is a repository of doglinkos-2nd apps.

## Apps

| Directory | Description |
|-----------|-------------|
| `2048/` | 2048 game |
| `hello_std/` | Hello world with std |
| `timer/` | Timer app |
| `videoplay/` | Modern minimalist silent video player (MJPEG AVI + DLV/LZMA2) |

## videoplay

A modern minimalist **silent** video player for DoglinkOS-2nd. Runs on the
bare-metal framebuffer with no GPU, no audio, and no std.

Supported formats:
- AVI container with MJPEG codec
- DLV — custom container with per-frame raw LZMA2-compressed BGRA pixels

See `videoplay/README.md` for build & usage details.

### Build (bare-metal target)

```bash
cd videoplay
rustup target add x86_64-unknown-none
cargo build --release \
    --target x86_64-unknown-none \
    --no-default-features \
    --features "dlos codec-mjpeg codec-xz"
```

### Keyboard controls

- Space — pause / play
- Q or ESC — quit
- O — re-launch with new file path
- Left / Right — seek ±5 seconds
