#!/usr/bin/env bash
# Regenerate Brawler's kiosk pictures from the game itself:
#   web/brawler/screenshot.png  one plain frame of a fight
#   web/brawler/card.png        the card poster: a flying kick over the sign
# Requires: a native release build, xvfb-run, ImageMagick (magick).
set -euo pipefail
cd "$(dirname "$0")"

cargo build --release -p brawler
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# One game at a time: each capture runs the game to a frame and exits.
shot() { # scene frame name
    BLIP_SHOT_SCENE=$1 BLIP_SCREENSHOT_OUT="$tmp/$3.png" BLIP_SCREENSHOT_FRAME=$2 \
        xvfb-run -a -s "-screen 0 960x600x24" target/release/brawler >/dev/null 2>&1
    # The capture keeps the canvas's alpha; the game is seen on black.
    magick "$tmp/$3.png" -background black -alpha remove -alpha off "$tmp/$3.png"
}
shot 0 32 kick
shot 3 150 title
shot 15 10 poster

cp "$tmp/kick.png" web/brawler/screenshot.png

# 640x720, the card frame's shape (480:540): the poster at three times its
# size, and the sign along the bottom, clear of the badges in the card's top
# corner.
magick "$tmp/poster.png" -crop 213x200+232+78 +repage -filter point -resize 640x600! "$tmp/hero.png"
magick "$tmp/title.png" -crop 640x114+0+8 +repage "$tmp/sign.png"
magick -size 640x720 xc:black \
    "$tmp/hero.png" -geometry +0+0 -composite \
    "$tmp/sign.png" -geometry +0+604 -composite \
    -stroke '#dc3c28' -strokewidth 3 -draw "line 0,600 640,600" \
    web/brawler/card.png
echo "[ok] web/brawler/screenshot.png web/brawler/card.png"
