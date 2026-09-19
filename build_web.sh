#!/usr/bin/env bash
# Build all four games for the wasm32-unknown-unknown target and lay them
# out under web/<game>/ so they can be served by any static file host.
#
# Requires: rustup target add wasm32-unknown-unknown
set -euo pipefail
cd "$(dirname "$0")"

# Use rustup's cargo, not whichever one happens to be first on PATH.
#
# A Homebrew rust installs /opt/homebrew/bin/cargo ahead of ~/.cargo/bin,
# and that toolchain ships only the host target. `rustup target add
# wasm32-unknown-unknown` then reports success — it adds the target to the
# *rustup* toolchain — while this build keeps failing with "can't find
# crate for `core`", pointing at the one command that cannot fix it. The
# error names the target, so the toolchain is the last thing you suspect.
CARGO=cargo
if command -v rustup >/dev/null 2>&1; then
    if rustup_cargo=$(rustup which cargo 2>/dev/null) && [ -x "$rustup_cargo" ]; then
        CARGO="$rustup_cargo"
        # RUSTC too, not just cargo. Cargo resolves `rustc` from PATH unless
        # told otherwise, so rustup's cargo will happily drive Homebrew's
        # rustc — which has no wasm32 standard library — and the build fails
        # exactly as if the target were missing, while `rustup target add`
        # keeps reporting it as installed.
        if rustup_rustc=$(rustup which rustc 2>/dev/null) && [ -x "$rustup_rustc" ]; then
            export RUSTC="$rustup_rustc"
        fi
    fi
fi


GAMES=(serpent bouncer galactic_defender rally meteors sky_raider brawler)
TARGET_DIR="target/wasm32-unknown-unknown/release"

echo "[build] cargo build --release --target wasm32-unknown-unknown"
PKG_ARGS=()
for g in "${GAMES[@]}"; do PKG_ARGS+=(-p "$g"); done
"$CARGO" build --release --target wasm32-unknown-unknown "${PKG_ARGS[@]}"

for game in "${GAMES[@]}"; do
    out="web/$game"
    mkdir -p "$out"
    cp "$TARGET_DIR/$game.wasm" "$out/index.wasm"
    # rally has a hand-maintained index.html; all other games use shell.html.
    if [ "$game" != "rally" ]; then
        cp "web/shell.html" "$out/index.html"
    fi
    bytes=$(wc -c < "$out/index.wasm")
    echo "[ok] $game -> $out/index.wasm ($bytes bytes)"
done

echo
echo "Done. Serve with: python3 -m http.server -d web 8080"
