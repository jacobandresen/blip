#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

FLAGS="-O2 --use-port=sdl3 -Wno-experimental -Wno-unused-parameter -lm -I lib"
SHELL="web/shell.html"

mkdir -p web/serpent web/bouncer web/galactic_defender web/rally

echo "[1/4] Building serpent..."
emcc $FLAGS \
    lib/blip.c games/serpent/main.c \
    --preload-file games/serpent/assets@/assets \
    --shell-file "$SHELL" \
    -o web/serpent/index.html

echo "[2/4] Building bouncer..."
emcc $FLAGS \
    lib/blip.c games/bouncer/main.c \
    --preload-file games/bouncer/assets@/assets \
    --shell-file "$SHELL" \
    -o web/bouncer/index.html

echo "[3/4] Building galactic_defender..."
emcc $FLAGS \
    lib/blip.c games/galactic_defender/main.c \
    --preload-file games/galactic_defender/assets@/assets \
    --shell-file "$SHELL" \
    -o web/galactic_defender/index.html

echo "[4/4] Building rally..."
(cd games/rally/assets && gcc generate_assets.c -o gen_assets -lm && ./gen_assets)
emcc $FLAGS \
    lib/blip.c games/rally/main.c \
    --preload-file games/rally/assets@/assets \
    --shell-file "$SHELL" \
    -o web/rally/index.html

echo "Done. Serve web/ with: python3 -m http.server -d web 8080"
