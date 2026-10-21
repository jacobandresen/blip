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
| `src/interpreter/procedure.rs` | `SUB … END SUB`, `CALL`, `LOCAL`, `EXIT SUB` |
| `src/interpreter/block.rs` | Block `IF … ELSEIF … ELSE … END IF` |
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

Procedures follow AmigaBASIC: `SUB NAME(A, B$) [STATIC] … END SUB`, entered
with `CALL NAME(1, "X")` or a bare `NAME 1, "X"`, left early with `EXIT SUB`.
They are found once at load; a missing `END SUB` or a reused name stops the
load. Normal flow steps over a body. Parameters and `LOCAL` names are saved
on entry and restored on return, so a `SUB` can recurse; every other name is
shared with the program. Arguments are typed by name: `C$` takes text.
`MOD` is floored, so `-1 MOD 8` is 7.

A line ending in `THEN` opens a block that runs to `ELSEIF … THEN`, `ELSE` or
`END IF`; the branches are found at load, like procedures. On one line,
`IF X THEN A:B ELSE C` takes either side, and `THEN 100` is a `GOTO`.

Expressions cache tokens in `Arc` slices. Each frame allows up to 50,000
statements and eight milliseconds of interpreter work. A program exceeding
100,000 statements without a `WAIT` stops. `WAIT 0` yields until the next
render frame; positive waits measure milliseconds from the command.

## Reading a game tape

Numbered `REM --- ... ---` lines mark each logical section. Reusable steps
are named `SUB`s whose parameters say what they take; the opening map gives
the control flow and names the procedures.
Section headings identify setup, input,
the frame loop, rendering, collisions, and object pools. Keep comments short
and keep existing line numbers stable so `GOTO` and `GOSUB` remain easy to follow.
Use ordinary `REM` lines for array states and what a `SUB` leaves behind in
shared names, its outputs.
`REM GAME: BUBBLER` preserves the control layout when a tape is saved
under a different name in `/USER`. The browser reads this optional identity.

```basic
1 REM FLOW: TITLE 10 -> SETUP 800 -> PLAY 100 -> RESULT 700.
999 REM --- OBJECT UPDATE - MOVE, COLLIDE, AND DRAW; SCORER GETS THE POINTS ---
1000 SUB UPDATE_OBJECT(B)
1010 LOCAL HIT
1090 END SUB
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
tools/check_basic_tapes.sh   # every tape natively, mashing keys; test/basic/*.BLIP must END
```

`BLIP_BASIC_TAPE=path` runs a tape in the native build; with `BLIP_BOT=1` a
BASIC error exits with status 1.
