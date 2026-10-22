# BLIP BASIC

The Rust interpreter runs the game tapes. The browser console keeps its text,
file, and editor commands in `web/basic.js` and calls Rust for shared game math.
`web/basic-language.js` parses console expressions with a small host interface
for variables, arrays, input, and Rust function calls.

## Reading the interpreter

| File | Responsibility |
| --- | --- |
| `src/main.rs` | Accept source, load its files (`load_files`), run a frame |
| `src/interpreter/mod.rs` | Program state, continuations, frame budget |
| `src/interpreter/syntax.rs` | Tokens and splitting outside strings and parentheses |
| `src/interpreter/expression.rs` | Precedence, arguments, functions, variable reads |
| `src/interpreter/statement.rs` | Command dispatch, loops, assignments |
| `src/interpreter/procedure.rs` | `SUB … END SUB`, `CALL`, `LOCAL`, `EXIT SUB`, `DEF FN` |
| `src/interpreter/block.rs` | Block `IF … ELSEIF … ELSE … END IF`, `WHILE … WEND` |
| `src/interpreter/data.rs` | `DATA`, `READ`, `RESTORE` |
| `src/interpreter/graphics.rs` | BASIC drawing commands mapped to `blip` |
| `src/interpreter/value.rs` | Values, conversions, array keys |
| `src/interpreter/color.rs` | Palette, named colors, hex colors |
| `src/shared_math.rs` | Bindings to the native games' math functions |
| `src/sounds.rs` | Every WAV by its path: effects once, music looped, decoded on first use; a tape's tunes fetched in the background |
| `src/host.rs` | Browser imports, the user's files, where /TAPE files are |

A continuation holds the next line, the next statement, and pending actions
from `IF … THEN`. Calls save one; loops remember one. `WAIT` yields without
losing it. `GOTO` discards pending actions before moving to its target.

Procedures follow AmigaBASIC: `SUB NAME(A, B$) [STATIC] … END SUB`, entered
with `CALL NAME(1, "X")` or a bare `NAME 1, "X"`, left early with `EXIT SUB`.
They are found once at load; a missing `END SUB` or a reused name stops the
load. Normal flow steps over a body. Parameters and `LOCAL` names are saved
on entry and restored on return, so a `SUB` can recurse; every other name is
shared with the program. Arguments are typed by name: `C$` takes text.
`DEF FNNAME(A, B)=expression` is a one-line function, called as `FNNAME(1, 2)`
inside any expression; its parameters shadow the program's names meanwhile.
`MOD` is floored, so `-1 MOD 8` is 7.

A line ending in `THEN` opens a block that runs to `ELSEIF … THEN`, `ELSE` or
`END IF`; the branches are found at load, like procedures. On one line,
`IF X THEN A:B ELSE C` takes either side, and `THEN 100` is a `GOTO`.

`DATA` items, numbers or quoted text, are gathered at load; `READ` takes the
next and `RESTORE [line]` rewinds. A `FOR` with nothing to do, `FOR I=1 TO 0`,
skips to its `NEXT`, as in AmigaBASIC. `WHILE X … WEND` repeats while `X` holds; `EXIT FOR`
leaves the innermost loop. `TAN`, `EXP`, `LOG`, `SGN` and `FIX` join the
functions, as in AmigaBASIC, and so do the text ones: `MID$(S$,START[,LENGTH])`
counting from 1, `LEFT$`, `RIGHT$`, `VAL`, `ASC` and `CHR$`.

Expressions cache tokens in `Arc` slices. Each frame allows up to 50,000
statements and eight milliseconds of interpreter work. A program exceeding
100,000 statements without a `WAIT` stops. `WAIT 0` yields until the next
render frame; positive waits measure milliseconds from the command.

## Reading a game tape

Every tape has the same layout, and so do the Rust games it mirrors:

| Lines | What is there |
| --- | --- |
| 1-9 | What the game is, its keys, its screens; `2 REM GAME: NAME` |
| 10 | `SETUP`: constants, then `80` music, `85` the first screen, `90` the clock |
| 100 | `MAIN LOOP`: read the keys, `UPDATE_` the screen we are on, then `DRAW_` it |
| 1000 up | One section per screen or thing: `TITLE`, `START`, `PLAY`, `OVER`, ... |

Each screen has an `UPDATE_` and a `DRAW_` `SUB` (`UPDATE_TITLE`, `DRAW_PLAY`);
`START_GAME` begins a game. Lines 85 and 165 are free for tests: 85 replaces
the first screen, 165 runs after the frame is drawn. A body indents two spaces
a level and a line holds one statement, or a short pair. `REM` lines say what
the code cannot: a rule, a measured number, what a `SUB` leaves in shared names.
`REM GAME: BUBBLER` keeps the control layout when a tape is saved under another
name in `/USER`.

A tape's files are in its own folder, `/TAPE/<GAME>/ASSETS/`, which SETUP names
once: `ASSETS$="/TAPE/BUBBLER/ASSETS/"`, then `DRAW_TEXTURE(ASSETS$+"ROBOT",...)`,
`PLAY_SFX ASSETS$+"POP.WAV"` and `PLAY_MUSIC ASSETS$+"THEME.WAV"`. A picture
without an extension is a `.PNG` and a sound a `.WAV`. When a program loads, the
runtime reads every such folder its source names; a tune (a WAV with an XM beside
it) is fetched the first time it plays. A copy's folder, `/USER/<GAME>/ASSETS/`,
holds the pictures the user changed; anything else in it comes from the tape's.

Every program starts on one 640x400 screen, a high-resolution Amiga's, and
the tapes are drawn for it. `CANVAS(width,height)` picks another size for a
game that plays better on one, as Bubbler keeps the Rust arena at 624x600. `TIME()` reads the engine clock for animation;
`CONTROL_TEXT$(keys,pad,touch)` chooses wording through Rust's controls API.
`DRAW_POLYLINE` renders an array of points through the same Rust path renderer
as Adder; see the function reference for its array layout. `DRAW_CIRCLE(x,y,r,
colour[,width])` draws a ring.

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
