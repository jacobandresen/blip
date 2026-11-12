# Agent guide

Keep changes small, follow the nearest game's conventions, and verify what you touch. Write repository
scripts in Node.js `.mjs`, cross-platform, documenting any external tool they need; no Bash or Python
wrappers. Do not publish or push unless asked.

## Layout

| Area | Where |
|---|---|
| A game: rules, drawing, sound, bot | `crates/<game>/src/` (`gfx.rs` pictures, `tunes.rs` sound, `bot.rs` playtest bot) |
| Shared engine | `crates/blip/src/` — put anything two games need here |
| Game list, controls, front ends, fullscreen, coins | `web/blip_core.js` (`BLIP_GAMES`) |
| Game page | `web/game.html` (copied to each `web/<game>/index.html`), `web/game.js` |
| Styles shared by the arcade, game pages and decks | `web/blip.css` |
| The arcade room and its manual | `web/index.html`, `web/arcade.css`, `web/arcade.js`, `web/aisle.js`, `web/cabinet.js` |
| The control deck (stick, pad, caps, input) | `web/deck.js`, `web/deck.css`, `web/machine-deck.html` |
| Home Screen and Guide | `web/home.*`, `web/guide.*` |
| Manual pages | `web/controls.html`, `history.html`, `about.html` (read by both the arcade's manual and the Guide; `data-guide="cabinet"` or `"app"` marks copy for one) |
| Tests | `test/*.mjs` (`node:test`), helpers in `test/lib/` |
| Tools | `tools/*.mjs` |

## The shared deck

`web/deck.js` and `web/deck.css` are also breadbin's pad (its `tools/blip-deck/sync.mjs` copies them). Keep them
free of blip's pages: `BlipDeck.build(panel, spec)`, `BlipDeck.bindStick` and `BlipController` take what they need
as arguments; only the short start-up block at the end of `deck.js` reads `BLIP_GAMES`.

## Generated files

Never edit these by hand; regenerate them:

- `web/<game>/index.wasm`, `index.html`: `npm run build:web`
- `web/<game>/music/*.wav`: built from `tunes.rs` by the game's `build.rs`
- `web/<game>/icon-*`, `manifest.json`, guide icons: `node tools/make_game_icons.mjs`
- `web/<game>/attract.webp`: `node tools/make_attract_strips.mjs <game>`
- Brawler's pose sheets, card and poster: `node tools/make_brawler_art.mjs`
- `web/room-tiles.svg`: `node tools/make_room_tiles.mjs`

A change under `web/` needs `node tools/bump_web_versions.mjs` (once per commit: the service worker's cache
and the `?v=` links), then `npm run build:web` to copy `game.html` to the game pages.

## A new game

Copy the smallest similar game (Serpent is compact).

1. Add `crates/<slug>` to the workspace, with `macroquad` and `blip` dependencies and `blip` (default features
   off) as a build dependency. `build.rs` is `blip::build_script!();`.
2. Write `main.rs`, `gfx.rs`, `tunes.rs` and `bot.rs`; `levels.rs` for authored levels. Keep score, lives and
   level in `blip::Session`.
3. Add it to `BLIP_GAMES` in `web/blip_core.js`; the build, the tests and the arcade read the list from there.
   Add its marquee colours to `web/blip.css` and its manual spread to `web/controls.html`.
4. Add its card crop to `tools/make_game_icons.mjs`, its attract capture to `tools/make_attract_strips.mjs`,
   and its start keys to `test/visual.mjs`.
5. Build, generate icons and attract art, and run its tests. `rg` the slug of another game to find any
   registry left.

## Tests

```sh
cargo test --release -p <game>   # rules
npm test                         # unit tests, seconds
npm run test:web                 # browser suites, two lanes
npm run test:visual              # every game starts and responds (BLIP_VISUAL_GAME=BRAWLER for one)
npm run test:playthrough         # every bot playthrough (BLIP_PLAYTHROUGH_GAME=serpent for one)
```

Run the narrowest check while working, then the wider ones for shared code. Browser tests are headless and
muted; native playtests set `BLIP_SILENT=1` and on Linux use `xvfb-run` when there is no display.
[`test/visual.md`](test/visual.md) explains what the visual check does and does not prove.

## Playtest bots

Each `bot.rs` plays its game through normal input, on the toolkit in `crates/blip/src/bot/`: a `Pad`
(`steer`, `tap_fire`, `press_start`, …), `grid` path finding and `predict`. Keep each tactic a small named
function and the module comment a plain description of the game.

`PLAYTHROUGHS` are named runs ("the snake eats food and grows into level 2") with a start level, time limit,
settings and goals on the bot's stats; a run ends when its goals are met. Runs are deterministic per seed
(`BLIP_BOT_SEED=n` overrides); try a new one on several seeds. Record events with `blip::bot::add`;
`Session` records `score`, `deaths` and `level`, and `web::report_score` ends a run (call
`blip::bot::finish("won")` first on a win).

Natively: `BLIP_BOT=1` plays and prints a `RESULT` line; `BLIP_BOT_LEVEL=<n>` starts at level n,
`BLIP_BOT_SHOTS=<dir>` saves frames, `BLIP_BOT_FUZZ=<seed>` sends random input, and a game's own switches
are `BLIP_BOT_<NAME>`. A refactor that must not change play: the same `BLIP_BOT=1 BLIP_BOT_SEED=n` run prints the same `RESULT` line
before and after. `node tools/playtest_levels.mjs [game ...]` plays every level and fails on a panic
or a level the bot cannot start.

## Conventions

- Pixel text at whole-number sizes, 2 or more where it must read on a phone.
- Generate changing details at runtime; PNGs are still images.
- CSS: few, general rules. A one-line rule fits a screen line (`node tools/format_css.mjs <file>` expands
  longer ones). Check a CSS change with `node tools/css_check.mjs`: snapshot before and after and compare
  (nothing should differ); `dead` and `prune` remove declarations that never take effect.
- Comments are brief and factual: measured constants and constraints, not what the next line does.
