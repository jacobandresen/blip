#!/usr/bin/env bash
# Build all four games for the wasm32-unknown-unknown target and lay them
# out under web/<game>/ so they can be served by any static file host.
#
# Requires: rustup target add wasm32-unknown-unknown
set -euo pipefail
cd "$(dirname "$0")"

GAMES=(serpent bouncer galactic_defender rally)
TARGET_DIR="target/wasm32-unknown-unknown/release"

echo "[build] cargo build --release --target wasm32-unknown-unknown"
PKG_ARGS=()
for g in "${GAMES[@]}"; do PKG_ARGS+=(-p "$g"); done
cargo build --release --target wasm32-unknown-unknown "${PKG_ARGS[@]}"

for game in "${GAMES[@]}"; do
    out="web/$game"
    mkdir -p "$out"
    cp "$TARGET_DIR/$game.wasm" "$out/index.wasm"
    cp web/shell.html "$out/index.html"
    bytes=$(wc -c < "$out/index.wasm")
    echo "[ok] $game -> $out/index.wasm ($bytes bytes)"
done

echo
echo "Done. Serve with: python3 -m http.server -d web 8080"
