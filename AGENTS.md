# Agent guide

Keep changes small, follow the nearest game's conventions, and verify the
affected paths. Write repository scripts in Node.js `.mjs`; keep them
cross-platform and document any external tools they need. Do not add Bash or
Python wrappers. Do not publish or push unless the user asks.

## Build outputs

Do not edit generated files by hand. Web game WASM is compiled from Rust;
per-game `index.html` files are copied from `web/shell.html`. BASIC tape assets
are generated from the Rust asset sources.

- `web/<game>/index.html`, `web/<game>/index.wasm`, `web/basic-runtime.wasm`:
  `cargo build-web`
- `web/TAPE/<GAME>/ASSETS/*`: `crates/<game>/src/gfx.rs`, `tunes.rs`, and
  `build.rs`; run the package build that writes them.
- `web/<game>/icon-*`, `manifest.json`, and guide icons:
  `node tools/make_game_icons.mjs`
- `web/<game>/attract.webp`:
  `node tools/make_attract_strips.mjs <game>`
- `web/wear.css`: `node tools/make_wear.mjs`
- Brawler's BASIC pose sheets, stages, card, and poster:
  `node tools/make_brawler_art.mjs`

Changes under `web/` require a cache version bump in `web/sw.js` and new `?v=`
links for the changed files: `node tools/bump_web_versions.mjs` does both, once
per commit; then run `cargo build-web` to copy `shell.html` to the game pages.

## Start a new game

Use the smallest similar game as a model. Serpent is a compact Rust game;
BASIC tapes and their shared APIs are documented in
[`crates/blip_basic/README.md`](crates/blip_basic/README.md).

1. Pick a lowercase URL slug and uppercase tape name. Add
   `crates/<slug>` to `Cargo.toml`'s workspace members. Create its package
   manifest with workspace package metadata, `macroquad` and `blip`
   dependencies, plus `blip` as a build dependency with default features off.
2. Add `build.rs` containing `blip::build_script!();`, then implement
   `src/main.rs`, `src/gfx.rs`, `src/tunes.rs`, and `src/bot.rs`: every game
   has a native playtest bot that can reach each level (`BLIP_BOT_LEVEL`) and
   lists `PLAYTHROUGHS` named after the game's themes (see below). Add
   `src/levels.rs` for authored progression data. Keep score, lives and level
   in `blip::Session`. Keep game rules and rendering in the Rust crate;
   reusable engine behavior belongs in `crates/blip`.
3. Register the slug in `crates/blip_build/src/main.rs` so `cargo build-web`
   builds and copies its WASM. Register its title, accent, and controls in
   `web/blip_core.js` (`BLIP_GAMES`). Add its marquee colors and manual spread
   in `web/marquee.css` and `web/controls.html`.
4. If the game has a BASIC edition, add `web/TAPE/<NAME>.BLIP`; put its
   generated images and sounds under `web/TAPE/<NAME>/ASSETS/` through the
   Rust asset build. Start with `REM GAME: <NAME>`, keep numbered sections
   ordered, and use `CONTROL_TEXT$` for platform-specific control hints.
5. Add a card crop in `tools/make_game_icons.mjs` and an attract capture
   setup in `tools/make_attract_strips.mjs`. Add the game to
   `test/visual-parity.mjs`; add focused tests and place browser tests in the
   appropriate lane in `test/run-web.mjs`. Suites take the game and tape lists
   from `test/lib/games.mjs` (read from `BLIP_GAMES` and `web/TAPE`), never
   their own count. A game that reports scores also needs a Supabase
   migration adding its slug to `submit_score`.
6. Build with `cargo build-web`; generate icons and attract art; verify the
   Rust package, BASIC tape if present, web page, and relevant browser tests.
   Read [`test/visual-parity.md`](test/visual-parity.md) before judging
   screenshots. Check `rg` for other slug registries before calling the game
   integrated.

## Where to edit

| Area | Source of truth |
|---|---|
| Game rules, rendering, sound | `crates/<game>/src/` |
| Game artwork and sound generation | `crates/<game>/src/gfx.rs`, `tunes.rs` |
| Shared engine, drawing, effects, audio | `crates/blip/src/` |
| BASIC interpreter and Rust bindings | `crates/blip_basic/src/`; API guide in `crates/blip_basic/README.md` |
| BASIC game program | `web/TAPE/<GAME>.BLIP` |
| Shared BASIC assets | generated under `web/TAPE/<GAME>/ASSETS/` |
| Game shell markup, styles, behavior | `web/shell.html`, `web/shell.css`, `web/shell.js` |
| Game metadata, controls, front-end routing, fullscreen, sound | `web/blip_core.js` |
| Cabinet cards, aisle, landing page | `web/index.html` and `web/aisle.js` |
| Home Screen | `web/home.html`, `web/home.css`, `web/home.js` |
| Shared game deck | `web/deck.js`, `web/machine-deck.html` |
| Marquees and machine colors | `web/marquee.css` |
| Shared game controls and manual pages | `web/controls.html`; cabinet book in `web/index.html`, app Guide in `web/guide.*` |
| Browser test helpers | `test/lib/` |
| Browser suites | `test/*.mjs` (`node:test`) |
| Build and asset tools | `tools/*.mjs` |

The cabinet book and app Guide both read `controls.html`, `history.html`, and
`about.html`. Write shared copy once; use `data-guide="cabinet"` or
`data-guide="app"` only for content exclusive to that binding.

## Build and verify

```sh
cargo test --release -p <game>
cargo build-web
npm test
npm run test:visual
npm run test:playthrough
```

`cargo build-web` builds every Rust game and the BASIC runtime for the
`wasm32-unknown-unknown` target. Install that target once with
`rustup target add wasm32-unknown-unknown`. `npm test` runs the fast unit
suites. `npm run test:visual` checks all games in muted headless Chromium;
select one with `BLIP_VISUAL_GAME=BRAWLER npm run test:visual`. The full
browser suite is `npm run test:web`; `npm run test:all` also runs Rust tests
and rebuilds the web games. Database tests need a local Supabase instance.

Use the narrowest relevant checks while iterating, then broaden verification
for shared code. Read `package.json` for current test commands and
`test/run-web.mjs` for browser-suite lanes. Tests that start Chromium mute
audio. Native screenshots and playtests set `BLIP_SILENT=1` where supported.

For Brawler, Rust/BASIC parity captures are generated by
`node tools/compare_brawler.mjs`; select one with `--scene jade-kick`, or set
the shared clock with `--clock 8`. Review the paired images in
`test/artifacts/brawler-compare/index.html`. Verify Rust with
`cargo test --release -p brawler` and BASIC interactions with
`node --test test/brawler-basic.mjs`.

On Linux, `xvfb-run` can provide a hidden display for native playtests. It is
optional on Windows and macOS, and on Linux with an available desktop display.
The web build and browser tests do not need a visible window.

Each game's `src/bot.rs` says how to play it, built on the shared toolkit in
`crates/blip/src/bot/`: a `Pad` for the controls (`steer`, `tap_fire`,
`press_start`, ...), `grid` path finding and `predict` for bounces and
wrap-around. Keep each tactic a small named function and the module comment a
plain description of the gameplay. Its `PLAYTHROUGHS` are named runs, such as
"the snake eats food and grows into level 2", with a start level, a time limit,
settings and goals on the bot's stats; a run ends as soon as its goals are met.
Bot runs are deterministic: each playthrough has a seed, and `BLIP_BOT_SEED=n`
overrides it; check a new playthrough on several seeds before relying on it.
`npm run test:playthrough` runs every playthrough of every game, one at a
time, as a test named after it; `BLIP_PLAYTHROUGH_GAME=serpent` selects games.
Record gameplay events with `blip::bot::add`; `Session` already records
`score`, `deaths` and `level`, and `web::report_score` ends a run at game over
(call `blip::bot::finish("won")` first when the game was won).

For a Rust game's bot, run a release build with `BLIP_BOT=1`; it plays through
normal inputs and prints a `RESULT` line. Set `BLIP_BOT_LEVEL=<n>` to start at
level n, `BLIP_BOT_SHOTS=<dir>` to save frames or `BLIP_BOT_FUZZ=<seed>` to
exercise random inputs. A game's own switches are `BLIP_BOT_<NAME>`, read with
`blip::bot::num`, `flag` or `is`; see its `bot.rs`. `Session` reports the
`level` stat; a game with authored levels also calls `blip::bot::levels(n)`.
`node tools/playtest_levels.mjs [game ...]` plays every level of every game in
turn and fails on a panic or a level the bot cannot start.

## Code and review conventions

- Keep numbered BASIC tape lines unique and ascending. Group screens and
  systems into `UPDATE_*` and `DRAW_*` subs; `WAIT 0` yields a frame.
- Draw pixel text at whole-number sizes. Avoid sizes below 2 for text that
  must remain readable on a phone.
- Generate changing visual details at runtime; PNGs are still images.
- Keep CSS rules short enough to read on one screen line; `node tools/format_css.mjs <file.css>`
  expands longer one-line rules to one declaration per line; `node tools/fold_css.mjs`
  finds rules a later rule with the same selector makes redundant.
- Keep comments brief and factual. Explain measured constants or constraints,
  not the code that follows.
- Keep generated artifacts in their designated output paths and regenerate
  them from source rather than hand-editing them.
