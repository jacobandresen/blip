# Agent guidance

## Web `.wasm` output is compiled — do not edit it directly

The `index.wasm` and `index.html` files under each game's directory in `web/`
(`serpent`, `bouncer`, `galactic_defender`, `rally`, `meteors`, `sky_raider`,
`brawler`, `bubbler`, `adder`) are **build outputs**. They are
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
| The game list (`BLIP_GAMES`), coins (`getCoins`, `saveCoins`), layout rule, mute, fullscreen | `web/blip_core.js` (every page; the home screen loads only this) |
| Which front end a game returns to (`blip-frontend`, `blipFrontendUrl`), free play, a game's own app mode (`blipAppMode`) | `web/blip_core.js` |
| Cabinet parts: power lamp, coin HUD (`updateCoinsHud`), card arm, hum | `web/kiosk.js` |
| The home screen: icons, dock, Search, the long-press menu, edit mode (`blip-home-order`), Settings | `web/home.html`, `web/home.css`, `web/home.js` (nothing from the cabinet) |
| App style: a game opened from the home screen or its own icon (`data-appstyle`), with `‹ Home`, a frosted title bar and a launch screen | `web/shell.css` (the `App style` section), `web/shell.js` |
| The Home Screen and BLIP switches on the top bar (`#home-switch-btn`, `#basic-switch-btn`), arcade and game pages | `web/kiosk.js`, `web/kiosk.css` |
| The arcade's aisle of cabinets: marquee, screen, deck, Start, coin door, side art | `web/index.html` |
| A machine's marquee, the same sign on its cabinet and as its game page's top bar; each game's colours (`data-machine`) | `web/marquee.css` |
| The deck on each cabinet: the game page's own, rendered per game | `web/machine-deck.html` (builds it with `web/deck.js`) |
| Each game as its own home-screen app: icons (a small game card) and `manifest.json` | `tools/make_game_icons.mjs`, which writes `web/<game>/icon-*.png` and `manifest.json`, and the Guide's (generated, not edited by hand) |
| The "Add to Home Screen" sheet, free play and app mode on a game page | `web/shell.js`, `web/shell.css` |
| App style's clean picture: no CRT (`blip_crt`), drawn at the screen's pixel density | `web/blip_bridge.js`, `crates/blip/src/ctx.rs` (`crt_look`); natively `BLIP_CRT=0` |
| The app's Guide: the manual's chapters set as an iOS guide (also a sheet over an app-style game), an app of its own (`guide-manifest.json`, `guide-icon-*.png` from `tools/make_game_icons.mjs`) | `web/guide.html`, `web/guide.css`, `web/guide.js`; its words stay in `controls.html`, `history.html`, `about.html` |
| App mode's game page: no deck on a PC or with a gamepad (`data-bare`), the pause (`blipPause`, for the controls card, the Guide and a hidden page), the controls card on first open (`blip-seen-<slug>`), the score to beat under the title | `web/shell.js`, `web/shell.css` |

The home screen's zoom into a game ends with the icon where the game's
launch screen (`#loader` in app style) shows it: 108px, 16px above the
middle. Change one and the other must follow.
| Bottom control deck markup (stations, stick, pad, caps) — one builder for every page | `web/deck.js` |
| Kiosk / landing page | `web/index.html` |
| Fullscreen button and the stored choice (`blip-fullscreen`), on every page | `web/blip_core.js` |
| Sound on / off (`blip-mute`); `blipOut(ctx)`, where the cabinet's own sounds must connect | `web/blip_core.js` |
| What fullscreen hides on a game page; the idle return to the cabinet | `web/shell.css`, `web/shell.js` |
| Cabinet wear: scratches, burns, rings, scuffs, thumbed cards and manual | `tools/make_wear.py`, which writes `web/wear.css` (generated, not edited by hand) |
| Field manual: spreads, the three page-turn styles, the torn leaf, contents, notes pages | `web/index.html` (the manual script), `web/manual.css` |
| The book leaving its pocket, its cover swinging open and shut | `web/manual-motion.js` |
| The manual's last page, the operators (a `data-manual-sheet`: never split, its type shrinks to fit) | `web/about.html` |
| A game's manual pages: screenshot, blurb, controls, tip | `web/controls.html`: a `data-side="left"` and a `data-side="right"` section per game |
| The controls card (the "?" under the top bar, its arm and whirr): the cabinet's welcome and each game's keys (`controls`, `touchControls` in `BLIP_GAMES`) | `web/kiosk.js`, `web/kiosk.css` |
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

`web/shell.html` is a static template — it loads `blip_core.js` and `kiosk.js` in the head,
then `deck.js`, `blip_controller.js`, the high-score scripts, `shell.js`,
`mq_js_bundle.js` and `blip_bridge.js`, and finally calls `load("index.wasm")`
to boot the macroquad runtime. All of them are referenced with
`../` paths so they resolve from inside each game's subdirectory; only
`manifest.json` and `icon-180.png` are the game directory's own, so a game
saved to a home screen is its own app. `kiosk.js`
and `kiosk.css` are also loaded by `web/index.html` (without the `../` prefix)
to share the nav bar between the landing page and game pages.

### One manual, two bindings

The cabinet's book (`index.html`) and the app's Guide (`guide.js`) both read
`controls.html`, `history.html` and `about.html`. Write the words there once.
Mark what belongs to one binding only: `data-guide="cabinet"` (coins, the card
rack, the BLIP logo) is left out of the Guide, `data-guide="app"` (the Home
Screen, Add to Home Screen) out of the book. The Guide follows Apple's
[Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines)
(typography, lists and tables, navigation bars, sidebars, dark mode); text
written for it follows the [Apple Style Guide](https://support.apple.com/guide/applestyleguide/welcome/web):
sentence-case headings, "tap" and "touch and hold", "Home Screen".

### Rebuilding

```
rustup target add wasm32-unknown-unknown   # one-time
./build_web.sh                              # recompile all nine games
```

### Tests

Run the test suite locally before pushing:

```
npm test                # canvas geometry (unit)
cargo test --release    # game rules (Brawler's balance, Serpent's turns, ...)
npm run test:web        # cabinet, home screen, manual, top bar, controls, decks (after ./build_web.sh; six minutes, in two lanes: one for the tests that run a game, one for the rest)
npm run test:layout     # nothing overlaps or leaves the screen: 21 devices, notches, every page and controller (seven minutes, four processes)
npm run test:shell      # the game shell in headless Chromium (four minutes)
npm run test:scores     # high-score entry in all eight scoring games (three minutes, three processes)
npm run test:scores-db  # the score backend, against `npx supabase start` (skipped without it)
```

`test:scores` plays each built game until it ends and enters a name typed,
from the stick and by touch, against `test/lib/fake-supabase.js`. Run it
after touching `blip_scores.js`, the shell's key handling, or a game's
game-over path. A game hears keys only while the canvas has focus: anything
that takes focus (a prompt, a button) must leave it back there.

Tests that read the shell's layout hold the game's wasm back (`page.route('**/*.wasm', () => {})`) and wait for the page to go still instead of for a clock; only tests that need a running game load it.

The browser suites need Playwright and a Chromium binary (`BLIP_CHROMIUM`
names one outside `PATH`). Test files are `node:test` suites, one theme
each: `cabinet.mjs` (the card rack), `manual.mjs`, `touch.mjs` (fingers on the strip and deck), `layout.mjs` (every page on every device: overlaps, thumbs, safe areas), `topbar-layout.mjs`,
`controls.mjs`, `two-player-*.mjs` (decks and phones), `home.mjs` (the home screen,
free play, each game as an installable app).

For native development:

```
cargo run -p serpent            # or bouncer, rally, galactic_defender,
                                # meteors, sky_raider, brawler, bubbler, adder
```

### Home-screen icons

Each game's icon is a small version of its cabinet card, drawn from its
screenshot (Brawler's from `card.png`). After a screenshot changes, or a game
is added (give it a crop in `CROPS`):

```
node tools/make_game_icons.mjs
```

### Asset pipeline

Each game's `build.rs` calls `blip_assets::<game>::generate()` and writes the
returned PNG / WAV bytes into `$OUT_DIR/assets/{images,sounds}/`. The game's
`main.rs` embeds those bytes via `include_bytes!(concat!(env!("OUT_DIR"), ...))`,
so wasm builds carry every asset inside the single `index.wasm` and need no
separate preload step.

Effects are most of a game's download, so the warm ones
(`cosy::finish_warm`, low-passed at 3 kHz) are encoded at 22 kHz, and so are
Raider's and Brawler's low effects (`wav::encode_pcm16_half`, flat to 8 kHz).
A new effect goes at half rate if its energy above 10 kHz is 33 dB or more
under the rest; snaps, whistles and gunfire keep the full rate.

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

`BLIP_BOT_FUZZ=<seed>` swaps the autopilot for random key mashing, which finds
panics and stuck screens the bot's good play never reaches. Bubbler's own bot
has no fuzz mode.

## Pixel text

Draw text at whole-number sizes (`draw_text(.., 2.0, ..)`). The font is a
pixel grid: at 1.5 or 2.5 its rows come out uneven, and a phone shows a
680px game at half size, where anything under 2 cannot be read.

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
