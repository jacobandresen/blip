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
| Shared coin state (`getCoins`, `saveCoins`, `updateCoinsHud`), layout rule | `web/kiosk.js` |
| Bottom control deck markup (stations, stick, pad, caps) — one builder for every page | `web/deck.js` |
| Kiosk / landing page | `web/index.html` |
| Fullscreen button and the stored choice (`blip-fullscreen`), on every page | `web/kiosk.js` |
| What fullscreen hides on a game page; the idle return to the cabinet | `web/shell.css`, `web/shell.js` |
| Wasm <-> JS bridge (`blip_spend_coin`) | `web/blip_bridge.js` |
| macroquad JS runtime (vendored, do not edit) | `web/mq_js_bundle.js` |
| Game logic, rendering, audio (Rust side) | `crates/<name>/src/main.rs` |
| Shared engine library (blip API) | `crates/blip/src/*.rs` |
| Colour maths, deterministic scatter, capsules, outlined text | `crates/blip/src/{color,math,draw,font}.rs` |
| Brawler: the roster, the move table, the rules, the CPU | `crates/brawler/src/{roster,moves,rules,cpu}.rs` |
| Brawler: stages, fighter parts, poses, screens | `crates/brawler/src/draw/{stage,parts,pose,screens}.rs` |
| Brawler: screenshot scenes (`BLIP_SHOT_SCENE`) | `crates/brawler/src/shots.rs` |
| Brawler: tests, by theme (rules, anatomy, ladder, balance, ...) | `crates/brawler/src/tests/*.rs` |
| Score popups, bursts, rings (`blip::Fx`) | `crates/blip/src/fx.rs` |
| Music playback (`blip::Jukebox`) | `crates/blip/src/audio.rs` |
| Asset generators (sprites + WAV) | `crates/blip_assets/src/<game>.rs` |
| Songs written as note data (`Song`); `action: true` for the action games' mix | `crates/blip_assets/src/song.rs` |
| Effect voices, one instrument per game | `crates/blip_assets/src/cosy.rs` |
| Native playtest autopilot | `crates/blip/src/bot.rs`, `crates/<name>/src/bot.rs` |
| Per-game asset build step | `crates/<name>/build.rs` |
| Brawler's kiosk card poster and screenshot (captured from the game) | `./brawler_card.sh` |

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
cargo test --release                    # game rules (Brawler's balance, Serpent's turns, ...)
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

Music is the exception: a WAV loop is megabytes, so games depend on
`blip_assets` at runtime too and synthesise their tunes on the device through
a `blip::Jukebox` (first track at load, the rest on the title screen). Most
tunes are `song::Song` values — a melody of MIDI notes over chords, played on
the game's own instrument. To hear them all without a game:

```
cargo run --release -p blip_assets --example render_music -- /tmp/music
```

### Playtesting with the autopilot

Native builds carry a bot per game (`crates/<name>/src/bot.rs`; Bubbler has
its own, `BUBBLER_BOT=1`). It plays through the normal input path and prints
one `RESULT` line of stats at game over:

```
BLIP_BOT=1 BLIP_BOT_MAXT=300 xvfb-run -a -s "-screen 0 720x810x24" target/release/serpent
```

`BLIP_BOT_SHOTS=dir` saves a frame every `BLIP_BOT_SHOT_EVERY` seconds. Run
one game at a time.

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
