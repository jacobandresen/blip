# BLIP BASIC

BLIP BASIC runs numbered game tapes in Rust. The browser supplies the console,
editor, files and audio host; shared math, drawing and effects use `crates/blip`.
It is Amiga-inspired, not a full Amiga BASIC implementation.

## Source map

| File | Responsibility |
| --- | --- |
| `src/main.rs` | Load a program and run its frame loop |
| `src/interpreter/` | Syntax, expressions, statements, procedures, blocks and graphics |
| `src/shared_math.rs` | BASIC bindings to shared Rust helpers |
| `src/sounds.rs` | Load and play tape WAV files |
| `src/host.rs` | Browser imports, tape files and user files |
| `web/basic.js` | Browser console, editor and Web Audio host |

## Game tapes

Tapes use numbered sections: `SETUP` and the first screen near the top, the
input/update/draw loop around line 100, then one section per screen or system.
Keep each screen in `UPDATE_*` and `DRAW_*` procedures. `WAIT 0` yields to the
next frame. `SUB … END SUB`, `CALL`, `LOCAL`, `EXIT SUB`, `DEF FN`, block
`IF`, `WHILE`, `DATA`, `READ` and `RESTORE` are supported.

Start with `REM GAME: NAME` so copied tapes keep their control layout. Assets
live in `/TAPE/<GAME>/ASSETS/`; use `DRAW_TEXTURE`, `PLAY_SFX` and
`PLAY_MUSIC` with paths from that folder. A tune's WAV and XM files share a
name. User replacements live in `/USER/<GAME>/ASSETS/`.

Games draw on a 640 × 400 screen by default. `CANVAS(width,height)` selects a
different size. `TIME()` reads the engine clock. Use `CONTROL_TEXT$` for the
game's platform-specific control labels.

## Shared helpers

These functions call the same Rust helpers as native games. Geometry uses
32-bit floating-point precision; ordinary BASIC arithmetic remains 64-bit.

| Function | Result |
| --- | --- |
| `MIN(a,b)`, `MAX(a,b)`, `CLAMP(x,low,high)` | Bounds and limits |
| `ATN(x)`, `ATAN2(y,x)`, `ANGLE_DIFF(a,b)` | Angles in radians |
| `LERP(a,b,t)`, `SMOOTHSTEP01(t)`, `EASE_OUT_CUBIC(t)` | Interpolation and easing |
| `WRAP(x,low,high)` | Wrap into `[low,high)`; returns `low` if `high<=low` |
| `DISTANCE(x0,y0,x1,y1)` | Point distance |
| `RECTS_OVERLAP(...)` | `1` when rectangles overlap; touching edges do not count |
| `SEGMENT_CIRCLE_OVERLAP(...)`, `SEGMENT_RECT_OVERLAP(...)` | `1` when the segment touches the shape |
| `SCATTER(index,salt)` | Repeatable value in `[0,1)` |

`DRAW_POLYLINE(name,row,count,width,color,alternate)` reads x and y from
`name(row,index,0..1)` and optionally uses column 2 to mark alternate-color
segments. `DRAW_LINE`, `DRAW_CIRCLE`, `FILL_RECT` and `FILL_CIRCLE` use the
shared renderer. `FX_CLEAR`, `FX_UPDATE`, `FX_DRAW`, `FX_BURST`, `FX_POPUP`
and `FX_RING` use the shared effects layer.

## Brawler parity

Brawler has a 15-fighter, 14-match ladder in Rust and BASIC. BASIC owns its
input loop and rules; the Rust roster, move data, pose solver, stage art,
portraits and music generate the shared tape assets. The chooser shows each
fighter's special and stats; Zenith is marked **OVERPOWERED** and can still
take damage. Press Start once to open selection and again to confirm; arrows
or the BASIC pad move the highlight.

Each fighter atlas has 19 rows of six cells. Row 0 is idle; row 18 is the
opposite-leg high kick, used on alternating grounded high kicks. Missed attacks
can produce a short cosmetic stumble. The Hood has its own hip-hop score and
the rooftop poster reads “BRAWLER” and “BLIP, 2026.” Original character names
and designs are used throughout.

`node tools/make_brawler_art.mjs` captures Rust poses and stages, packages the
BASIC art and music, refreshes the poster and icons, then rebuilds Brawler's
WASM page. Use `node tools/compare_brawler.mjs --scene jade-kick --frame 0`
or `--scene jade-alt-kick --frame 0` for matching pose captures; use
`--scene mack-special --tick 0..183` for the lowrider timeline.

## Build and verify

```text
cargo test --release -p blip -p blip_basic -p brawler
cargo build-web
node --test test/basic.mjs test/basic-language.test.mjs test/basic-shared-functions.mjs test/basic-tape.mjs test/brawler-basic.mjs
node tools/check_basic_tapes.mjs
node test/run-visual.mjs --game BRAWLER
node tools/serve_web.mjs
```

Set `BLIP_BASIC_TAPE=path` to run a specific tape natively; `BLIP_BOT=1`
prints a result line and exits on BASIC errors. The tape playtest uses Xvfb on
Linux when available, otherwise the desktop display; it runs directly on
Windows and macOS. Native capture and fuzz tools set `BLIP_SILENT=1`; browser
tests use Chromium's `--mute-audio`. The web build needs no display.
