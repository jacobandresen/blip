# Agent guidance

## Web `.wasm` output is compiled — do not edit it directly

The `index.wasm` and `index.html` files under `web/serpent/`, `web/bouncer/`,
`web/galactic_defender/`, and `web/rally/` are **build outputs**. They are
produced by compiling Rust source with `cargo build --release --target
wasm32-unknown-unknown` and copying `target/wasm32-unknown-unknown/release/<game>.wasm`
into the game's web directory; `index.html` is a copy of `web/shell.html`.

**Do not patch `web/*/index.html` or `web/*/index.wasm` by hand.** Any manual
change will be overwritten the next time the project is built.

### Where to make changes

| What you want to change | Edit this file |
|---|---|
| Page structure and markup | `web/shell.html` |
| Styles (layout, touch controls, topbar, loading screen) | `web/shell.css` |
| JavaScript (canvas scaling, touch controls, coin handling) | `web/shell.js` |
| Shared nav bar styles (`.kiosk-bar`, `.kiosk-btn`, `.kiosk-hud`) | `web/kiosk.css` |
| Shared coin state (`getCoins`, `saveCoins`, `updateCoinsHud`) | `web/kiosk.js` |
| Kiosk / landing page | `web/index.html` |
| Wasm <-> JS bridge (`blip_spend_coin`) | `web/blip_bridge.js` |
| macroquad JS runtime (vendored, do not edit) | `web/mq_js_bundle.js` |
| Game logic, rendering, audio (Rust side) | `crates/<name>/src/main.rs` |
| Shared engine library (blip API) | `crates/blip/src/*.rs` |
| Asset generators (sprites + WAV) | `crates/blip_assets/src/<game>.rs` |
| Per-game asset build step | `crates/<name>/build.rs` |

`web/shell.html` is a static template — it loads, in order, `shell.js`,
`blip_bridge.js`, `mq_js_bundle.js`, and finally calls `load("index.wasm")` to
boot the macroquad runtime. `kiosk.css`, `kiosk.js`, `shell.css`, `shell.js`,
, `blip_bridge.js`, and `mq_js_bundle.js` are referenced with
`../` paths so they resolve from inside each game's subdirectory. `kiosk.js`
and `kiosk.css` are also loaded by `web/index.html` (without the `../` prefix)
to share the nav bar between the landing page and game pages.

### Rebuilding

```
rustup target add wasm32-unknown-unknown   # one-time
./build_web.sh                              # recompile all four games
```

### Tests

Run the test suite locally before pushing:

```
node --test test/fill-canvas.test.mjs   # unit tests (canvas geometry)
```

For native development:

```
cargo run -p serpent
cargo run -p bouncer
cargo run -p rally
cargo run -p galactic_defender
```

### Asset pipeline

Each game's `build.rs` calls `blip_assets::<game>::generate()` and writes the
returned PNG / WAV bytes into `$OUT_DIR/assets/{images,sounds}/`. The game's
`main.rs` embeds those bytes via `include_bytes!(concat!(env!("OUT_DIR"), ...))`,
so wasm builds carry every asset inside the single `index.wasm` and need no
separate preload step.

## Comments

Keep them short. A comment earns its place by saying something the code
cannot: the measured number behind a constant, the constraint that rules
out the obvious alternative, the failure that a rule is there to prevent.

- One to three lines. A block longer than that is almost always narrative.
- State the fact, not the story. "At 320px two stations fit `--cap: 32`
  and no further" — not an account of how that was discovered.
- No round-by-round history, no "this used to be X", no restating what
  the next line plainly does.
- Delete a comment that has become an anecdote rather than a reason.

The same goes for commit messages: say what changed and why, not the
path taken to get there.
