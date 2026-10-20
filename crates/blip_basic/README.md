# BLIP BASIC

The Rust interpreter runs the game tapes. The browser console keeps its text,
file, and editor commands in `web/basic.js` and calls Rust for shared game math.
`web/basic-language.js` parses console expressions with a small host interface
for variables, arrays, input, and Rust function calls.

## Reading the interpreter

| File | Responsibility |
| --- | --- |
| `src/main.rs` | Load assets, accept source, run a frame |
| `src/interpreter/mod.rs` | Program state, continuations, frame budget |
| `src/interpreter/syntax.rs` | Tokens and splitting outside strings and parentheses |
| `src/interpreter/expression.rs` | Precedence, arguments, functions, variable reads |
| `src/interpreter/statement.rs` | Command dispatch, loops, assignments |
| `src/interpreter/graphics.rs` | BASIC drawing commands mapped to `blip` |
| `src/interpreter/value.rs` | Values, conversions, array keys |
| `src/interpreter/color.rs` | Palette, named colors, hex colors |
| `src/shared_math.rs` | Bindings to the native games' math functions |
| `src/music.rs` | Native jukeboxes and user music |
| `src/sounds.rs` | Generated effect bytes, lazy decoding, playback |
| `src/host.rs` | Browser imports and user files |

A continuation holds the next line, the next statement, and pending actions
from `IF … THEN`. Calls save one; loops remember one. `WAIT` yields without
losing it. `GOTO` discards pending actions before moving to its target.

Expressions cache tokens in `Arc` slices. Each frame allows up to 50,000
statements and eight milliseconds of interpreter work. A program exceeding
100,000 statements without a `WAIT` stops. `WAIT 0` yields until the next
render frame; positive waits measure milliseconds from the command.

## Reading a game tape

Numbered `REM --- ... ---` lines mark each logical section and subroutine.
The opening map gives the control flow and entry lines for each module.
Section headings identify setup, input,
the frame loop, rendering, collisions, and object pools. Keep comments short
and keep existing line numbers stable so `GOTO` and `GOSUB` remain easy to follow.
Use ordinary `REM` lines for array states and subroutine inputs or outputs;
these act as function contracts where BASIC uses global variables.
`REM GAME: BUBBLER` preserves the control layout when a tape is saved
under a different name in `/USER`. The browser reads this optional identity.

```basic
1 REM FLOW: TITLE 10 -> SETUP 800 -> PLAY 100 -> RESULT 700.
998 REM INPUT: B IS THE OBJECT SLOT; SCORER RECEIVES ITS POINTS.
999 REM --- OBJECT UPDATE - MOVE, COLLIDE, AND DRAW ---
```

`CANVAS(width,height)` selects a logical drawing resolution. A title can use
the Rust game's native dimensions, then select `CANVAS(320,200)` for its
existing playfield. `TIME()` reads the engine clock for animation;
`CONTROL_TEXT$(keys,pad,touch)` chooses wording through Rust's controls API.
`DRAW_POLYLINE` renders an array of points through the same Rust path renderer
as Adder; see the function reference for its array layout.

## Shared functions

Implement reusable game helpers in `crates/blip` first. Bind them in
`src/shared_math.rs`; BASIC and exported WebAssembly functions both call that
same Rust implementation. The browser forwards arguments to those exports.
Do not add a second JavaScript formula.

See [the function reference](../../docs/basic-functions.md) for signatures and
boundary behavior.

## Verify

```sh
cargo test --release -p blip -p blip_basic
./build_web.sh
node --test test/basic.mjs test/basic-shared-functions.mjs
```
