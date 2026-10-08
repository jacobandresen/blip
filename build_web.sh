#!/usr/bin/env bash
# Build every game for wasm32-unknown-unknown into web/<game>/, servable by
# any static host.
# Requires: rustup target add wasm32-unknown-unknown
set -euo pipefail
cd "$(dirname "$0")"

# Use rustup's cargo, not the first on PATH: Homebrew's cargo ships only the
# host target, so `rustup target add` succeeds while the build fails with
# "can't find crate for `core`".
CARGO=cargo
if command -v rustup >/dev/null 2>&1; then
    if rustup_cargo=$(rustup which cargo 2>/dev/null) && [ -x "$rustup_cargo" ]; then
        CARGO="$rustup_cargo"
        # RUSTC too: cargo takes rustc from PATH, and Homebrew's has no wasm32
        # standard library.
        if rustup_rustc=$(rustup which rustc 2>/dev/null) && [ -x "$rustup_rustc" ]; then
            export RUSTC="$rustup_rustc"
        fi
    fi
fi


GAMES=(serpent bouncer galactic_defender rally meteors sky_raider brawler bubbler adder)
TARGET_DIR="target/wasm32-unknown-unknown/release"

echo "[build] cargo build --release --target wasm32-unknown-unknown"
PKG_ARGS=()
for g in "${GAMES[@]}"; do PKG_ARGS+=(-p "$g"); done
"$CARGO" build --release --target wasm32-unknown-unknown "${PKG_ARGS[@]}"

for game in "${GAMES[@]}"; do
    out="web/$game"
    mkdir -p "$out"
    cp "$TARGET_DIR/$game.wasm" "$out/index.wasm"
    cp "web/shell.html" "$out/index.html"
    bytes=$(wc -c < "$out/index.wasm")
    echo "[ok] $game -> $out/index.wasm ($bytes bytes)"
done

echo
echo "Done. Serve with: python3 -m http.server -d web 8080"
