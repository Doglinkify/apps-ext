#!/usr/bin/env python3
"""Convert a regular video file to the DLV2 container.

DLV2 stores keyframes and LZMA2-compressed XOR deltas between frames. Frames are decoded by ffmpeg as
BGRA pixels, then compressed so the resulting file can be read
by the no-std player (see ``src/container/dlv.rs`` and ``src/codec/xz_frame.rs``).
"""

from __future__ import annotations

import argparse
import json
import lzma
import os
import struct
import subprocess
import sys
import tempfile
from fractions import Fraction
from pathlib import Path


MAGIC = b"DLV2"
HEADER_LEN = 28
U32_MAX = 0xFFFFFFFF


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="input video accepted by ffmpeg")
    parser.add_argument("-o", "--out", type=Path, help="output DLV path")
    parser.add_argument(
        "--preset",
        type=int,
        default=6,
        choices=range(0, 10),
        help="LZMA2 compression preset (default: 6)",
    )
    parser.add_argument("--keyframe-interval", type=int, default=30,
                        help="insert a keyframe every N frames (default: 30)")
    return parser.parse_args()


def run_ffprobe(path: Path) -> tuple[int, int, int, int]:
    command = [
        "ffprobe",
        "-v",
        "error",
        "-select_streams",
        "v:0",
        "-show_entries",
        "stream=width,height,avg_frame_rate,r_frame_rate",
        "-of",
        "json",
        str(path),
    ]
    try:
        result = subprocess.run(command, check=True, capture_output=True, text=True)
    except FileNotFoundError as exc:
        raise RuntimeError("ffprobe was not found; install ffmpeg first") from exc
    except subprocess.CalledProcessError as exc:
        detail = exc.stderr.strip() or "unable to read video metadata"
        raise RuntimeError(f"ffprobe failed: {detail}") from exc

    streams = json.loads(result.stdout).get("streams", [])
    if not streams:
        raise RuntimeError("input does not contain a video stream")
    stream = streams[0]
    width = int(stream.get("width") or 0)
    height = int(stream.get("height") or 0)
    if width <= 0 or height <= 0:
        raise RuntimeError("ffprobe returned an invalid video size")

    rate_text = stream.get("avg_frame_rate") or stream.get("r_frame_rate") or "0/0"
    try:
        rate = Fraction(rate_text)
    except (ValueError, ZeroDivisionError) as exc:
        raise RuntimeError(f"invalid video frame rate: {rate_text!r}") from exc
    if rate <= 0:
        raise RuntimeError("video has no usable frame rate")
    if rate.numerator > U32_MAX or rate.denominator > U32_MAX:
        raise RuntimeError("video frame rate does not fit in the DLV header")
    return width, height, rate.numerator, rate.denominator


def convert(args: argparse.Namespace) -> None:
    width, height, fps_num, fps_den = run_ffprobe(args.input)
    frame_size = width * height * 4
    output = args.out or args.input.with_suffix(".dlv")
    output.parent.mkdir(parents=True, exist_ok=True)

    ffmpeg = [
        "ffmpeg",
        "-v",
        "error",
        "-i",
        str(args.input),
        "-map",
        "0:v:0",
        "-an",
        "-sn",
        "-dn",
        "-pix_fmt",
        "bgra",
        "-f",
        "rawvideo",
        "-",
    ]
    try:
        process = subprocess.Popen(ffmpeg, stdout=subprocess.PIPE)
    except FileNotFoundError as exc:
        raise RuntimeError("ffmpeg was not found; install ffmpeg first") from exc

    frame_count = 0
    previous = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w+b", prefix=f".{output.name}.", suffix=".tmp", dir=output.parent, delete=False
        ) as temporary:
            temporary_path = Path(temporary.name)
            temporary.write(MAGIC)
            temporary.write(struct.pack("<IIIIII", width, height, fps_num, fps_den, 0, 0))

            assert process.stdout is not None
            while True:
                chunks = []
                remaining = frame_size
                while remaining:
                    chunk = process.stdout.read(remaining)
                    if not chunk:
                        break
                    chunks.append(chunk)
                    remaining -= len(chunk)
                frame = b"".join(chunks)
                if not frame:
                    break
                if len(frame) != frame_size:
                    raise RuntimeError(
                        f"ffmpeg produced a partial frame ({len(frame)} of {frame_size} bytes)"
                    )
                keyframe = previous is None or frame_count % max(1, args.keyframe_interval) == 0
                encoded = frame if keyframe else bytes(a ^ b for a, b in zip(frame, previous))
                compressed = lzma.compress(
                    encoded,
                    format=lzma.FORMAT_RAW,
                    filters=[{"id": lzma.FILTER_LZMA2, "preset": args.preset}],
                )
                if len(compressed) > U32_MAX or frame_count >= U32_MAX:
                    raise RuntimeError("DLV frame count or compressed size exceeds u32")
                temporary.write(struct.pack("<IB", len(compressed), 1 if keyframe else 0))
                temporary.write(compressed)
                previous = frame
                frame_count += 1
                if frame_count % 25 == 0:
                    print(f"encoded frame {frame_count}", file=sys.stderr)

            return_code = process.wait()
            if return_code != 0:
                raise RuntimeError(f"ffmpeg failed with exit code {return_code}")
            if frame_count == 0:
                raise RuntimeError("ffmpeg produced no video frames")
            temporary.seek(20)
            temporary.write(struct.pack("<I", frame_count))
            temporary.flush()
            os.fsync(temporary.fileno())
        os.replace(temporary_path, output)
    except Exception:
        if process.poll() is None:
            process.kill()
            process.wait()
        if "temporary_path" in locals():
            temporary_path.unlink(missing_ok=True)
        raise

    print(f"wrote {output} ({output.stat().st_size} bytes, {frame_count} frames)")


def main() -> int:
    args = parse_args()
    try:
        convert(args)
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
