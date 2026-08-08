#!/usr/bin/env python3
"""Generate a DLV2 (DoglinkOS Lossless Video) sample file.

DLV2 format (see src/container/dlv.rs):
  Header (28 bytes, all LE):
    [0..4]   "DLV2"
    [4..8]   width
    [8..12]  height
    [12..16] fps_num
    [16..20] fps_den
    [20..24] frame_count
    [24..28] reserved (0)
  Then frame_count frame entries, each:
    [0..4]   compressed_size
    [4]      flags (bit 0: keyframe)
    [5..5+N] raw LZMA2 payload (BGRA keyframe or XOR delta)

Uses raw LZMA2 streams (FORMAT_RAW + FILTER_LZMA2) so the bare-metal
target can decode with the pure-Rust `lzma-rust2` crate (no std, no sha2).
"""

from __future__ import annotations

import argparse
import lzma
import os
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

HEADER_MAGIC = b"DLV2"
HEADER_LEN = 28


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--duration", type=int, default=10, help="seconds (default 10)")
    p.add_argument("--size", default="320x240", help="WxH (default 320x240)")
    p.add_argument("--fps", type=int, default=25, help="frames per second (default 25)")
    p.add_argument(
        "--out",
        default="samples/demo.dlv",
        help="output path (default samples/demo.dlv)",
    )
    return p.parse_args()


def run(cmd: list[str]) -> None:
    print("+ " + " ".join(cmd), file=sys.stderr)
    subprocess.check_call(cmd)


def generate_raw_bgra(args: argparse.Namespace, tmpdir: Path) -> Path:
    out = tmpdir / "raw.bgra"
    src = (
        f"testsrc=duration={args.duration}:size={args.size}:rate={args.fps}"
    )
    vf = (
        f"drawtext=text='%{{pts\\:hms}}':x=10:y=10:fontcolor=white:"
        f"box=1:boxcolor=black@0.5:fontsize=20"
    )
    run(
        [
            "ffmpeg",
            "-y",
            "-f",
            "lavfi",
            "-i",
            src,
            "-vf",
            vf,
            "-pix_fmt",
            "bgra",
            "-f",
            "rawvideo",
            str(out),
        ]
    )
    return out


def build_dlv(args: argparse.Namespace, raw_path: Path, out_path: Path) -> None:
    w, h = (int(x) for x in args.size.split("x"))
    frame_size = w * h * 4
    total_frames = args.duration * args.fps

    raw = raw_path.read_bytes()
    if len(raw) < frame_size * total_frames:
        print(
            f"warning: raw stream is {len(raw)} bytes, expected "
            f"{frame_size * total_frames}; truncating frame count",
            file=sys.stderr,
        )
        total_frames = len(raw) // frame_size

    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "wb") as f:
        f.write(HEADER_MAGIC)
        f.write(struct.pack("<IIIIII", w, h, args.fps, 1, total_frames, 0))
        previous = None
        for i in range(total_frames):
            frame = raw[i * frame_size : (i + 1) * frame_size]
            keyframe = previous is None or i % 30 == 0
            encoded = frame if keyframe else bytes(a ^ b for a, b in zip(frame, previous))
            compressed = lzma.compress(
                encoded,
                format=lzma.FORMAT_RAW,
                filters=[{"id": lzma.FILTER_LZMA2, "preset": 6}],
            )
            f.write(struct.pack("<IB", len(compressed), 1 if keyframe else 0))
            f.write(compressed)
            previous = frame
            if (i + 1) % 25 == 0:
                print(f"  encoded frame {i + 1}/{total_frames}", file=sys.stderr)

    print(f"wrote {out_path} ({out_path.stat().st_size} bytes)", file=sys.stderr)


def main() -> int:
    args = parse_args()
    with tempfile.TemporaryDirectory() as td:
        tmpdir = Path(td)
        raw = generate_raw_bgra(args, tmpdir)
        build_dlv(args, raw, Path(args.out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
