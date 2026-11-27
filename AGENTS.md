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
| The arcade room and its manual | `web/index.html`, `web/arcade.css`, `web/arcade.js`, `web/aisle.js`, `web/room.js`, `web/cabinet.js` |
| The control deck (stick, pad, buttons) | `<blip-deck>` from jsDelivr (below), `web/controller.js`, `web/machine-deck.html` |
| Home Screen and Guide | `web/home.*`, `web/guide.*` |
| Manual pages | `web/controls.html`, `history.html`, `about.html` (read by both the arcade's manual and the Guide; `data-guide="cabinet"` or `"app"` marks copy for one) |
| Tests | `test/*.mjs` (`node:test`), helpers in `test/lib/` |
| Tools | `tools/*.mjs` |

## The shared deck

The deck is the `<blip-deck>` element of the separate blip-deck repository (breadbin uses it too): it draws itself,
reports presses as `blip-input` events and is tested there. Change it there and tag a release; `game.html`,
`machine-deck.html` and `sw.js` (`DECK`) load `dist/blip-deck.js` from jsDelivr at that tag, and the service worker
caches it. To try an untagged build, run the tests with `BLIP_DECK=../blip-deck/dist/blip-deck.js`. Blip keeps the
rest: `blipBuildDeck` (`web/blip_core.js`) sets the deck up from `BLIP_GAMES` and adds the touch strip,
`web/controller.js` turns its events (and the keyboard, a gamepad and Rally's dials) into the game's keys, and
`web/blip.css` ("Around the control deck") styles what is around it and the deck's panel, through its custom
properties and `::part()`.

## Generated files

Never edit these by hand; regenerate them:

- `web/<game>/index.wasm`, `index.html`: `npm run build:web`
- `web/<game>/music/*.wav`: built from `tunes.rs` by the game's `build.rs`
- `web/<game>/icon-*`, `manifest.json`, guide icons: `node tools/make_game_icons.mjs`
- `web/<game>/attract.webp`: `node tools/make_attract_strips.mjs <game>`
- Brawler's pose sheets, card and poster: `node tools/make_brawler_art.mjs`

A change under `web/` needs `node tools/bump_web_versions.mjs` (once per commit: the service worker's cache
and the `?v=` links), then `npm run build:web` to copy `game.html` to the game pages.

## A new game

Copy the smallest similar game (Serpent is compact).

1. Add `crates/<slug>` to the workspace, with `macroquad` and `blip` dependencies and `blip` (default features
   off) as a build dependency. `build.rs` is `blip::build_script!();`.
2. Write `main.rs`, `gfx.rs`, `tunes.rs` and `bot.rs`; `levels.rs` for authored levels. Draw it in the blip look
   (Conventions). Keep score, lives and
   level in `blip::Session` (`lose_life_after` starts the respawn or game-over wait and reports the score); name the
   effects once with `blip::sounds!`.
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
node tools/fun_report.mjs [game] # fun score per game; CI fails a PR that lowers one (crates/blip/src/bot/FUN.md)
node tools/bot_bench.mjs <game>  # mean RESULT stats over seeds, to measure a bot change
```

Run the narrowest check while working, then the wider ones for shared code. `blip::testing` holds what the Rust tests
share: `seeded(n)`, taken first by a test that depends on the random numbers (they are global; tests run in parallel),
`climbs`, `never_falls` and `never_rises` for difficulty ramps, and `assert_grid` for authored maps. Browser tests are headless and
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

Natively, `BLIP_BOT=1` plays and prints a `RESULT` line; the other `BLIP_BOT_*` switches (start level, frames, random
input, reaction time, near-miss frames, …) are listed atop `crates/blip/src/bot/mod.rs`. A refactor that must not change play: the same `BLIP_BOT=1 BLIP_BOT_SEED=n` run prints the same `RESULT` line
before and after, and one that must not change the picture saves the same `BLIP_BOT_SHOTS` frames (a bot run
animates by its own clock). `node tools/playtest_levels.mjs [game ...]` plays every level and fails on a panic
or a level the bot cannot start.

Every bot run also reports fun measures (`fun_*`; [FUN.md](crates/blip/src/bot/FUN.md)): a game whose player can be hit
gives its bot a `blip::bot::near` hook with the hit boxes its collision code tests (one named source for both). A mechanic tweak carries a brief comment with
the measured reason; [FUN-LOG.md](crates/blip/src/bot/FUN-LOG.md) keeps the early takeaways and is not maintained.

## Conventions

- The blip look, in every game (`crates/blip/src/look.rs`, `frame.rs`): the shared palette (`BLIP_INK`, `BLIP_CREAM`,
  `BLIP_GOLD`, …), ink outlines on sprites (`Image::outline`), soft shadows (`draw_shadow`), `draw_hud` (coin, score,
  hearts), `draw_logo` for titles and big calls, `draw_message`/`draw_panel` for every banner and box, and
  `draw_attract_bar` on a title. Keep the arcade feel: score up front, fast readable play, space stays dark (`BLIP_NIGHT`).
- The arcade's cabinet is its game page's machine seen from farther (`arcade.js` fitDecks, gameMachine): change the
  game page's monitor or deck and the cabinet follows; starting a game walks the eye up to it (arcadeWalk) and the
  pages cross-fade. Room props (`web/room.js`) are simple CSS solids with SVG textures, kept clear of the machines by
  `test/room.test.mjs`.
- Pixel text at whole-number sizes, 2 or more where it must read on a phone.
- Generate changing details at runtime; PNGs are still images.
- CSS: few, general rules. A one-line rule fits a screen line (`node tools/format_css.mjs <file>` expands
  longer ones). Check a CSS change with `node tools/css_check.mjs`: snapshot before and after and compare
  (nothing should differ); `dead` and `prune` remove declarations that never take effect.
- Comments are brief and factual: measured constants and constraints, not what the next line does.
