#!/usr/bin/env bash
# Convierte la salida de capture.mjs en los ficheros de docs/media/.
# Requiere ImageMagick (convert), pngquant y ffmpeg.
set -euo pipefail
cd "$(dirname "$0")"
OUT=out
MEDIA=../../docs/media
mkdir -p "$MEDIA"

for f in "$OUT"/*.png; do
  n=$(basename "$f")
  convert "$f" -resize '1600x>' -strip "$MEDIA/$n"
  pngquant --quality 70-90 --force --skip-if-larger --ext .png "$MEDIA/$n" || true
done

for lang in en es; do
  d="$OUT/gif-$lang"
  ffmpeg -v error -y -f concat -safe 0 -i "$d/frames.txt" \
    -vf "fps=15,scale=900:-1:flags=lanczos,palettegen=stats_mode=diff" "$d/palette.png"
  ffmpeg -v error -y -f concat -safe 0 -i "$d/frames.txt" -i "$d/palette.png" \
    -lavfi "fps=15,scale=900:-1:flags=lanczos[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=5" \
    -loop 0 "$MEDIA/hero-$lang.gif"
done
ls -la "$MEDIA"
