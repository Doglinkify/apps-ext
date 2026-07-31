#!/usr/bin/env bash
# Generate a demo AVI file for dlos-videoplay.
#
# Usage:
#   ./make_demo_avi.sh           # generates samples/demo-mjpeg.avi (default)
#   ./make_demo_avi.sh h264      # generates samples/demo-h264.avi
#
# Requires: ffmpeg.

set -euo pipefail

cd "$(dirname "$0")/.."
mkdir -p samples

CODEC="${1:-mjpeg}"
OUT=""
case "$CODEC" in
    mjpeg)
        OUT="samples/demo-mjpeg.avi"
        ENCODER="-c:v mjpeg -q:v 5"
        PIXFMT="-pix_fmt yuv420p"
        ;;
    h264)
        OUT="samples/demo-h264.avi"
        ENCODER="-c:v libx264 -preset ultrafast -tune animation"
        PIXFMT="-pix_fmt yuv420p"
        ;;
    *)
        echo "usage: $0 [mjpeg|h264]" >&2
        exit 2
        ;;
esac

ffmpeg -y \
    -f lavfi -i "testsrc=duration=10:size=320x240:rate=25" \
    -vf "drawtext=text='%{pts\\:hms}':x=10:y=10:fontcolor=white:box=1:boxcolor=black@0.5:fontsize=20" \
    $PIXFMT \
    $ENCODER \
    "$OUT"

echo "wrote $OUT"
ls -lh "$OUT"
