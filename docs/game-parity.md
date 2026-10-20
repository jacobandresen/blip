# BASIC and Rust gameplay parity

## Adder

The BASIC tape uses the native 480 × 540 pit and the same generated pit art.
Both versions draw trails through `blip::draw_polyline` and use the engine's
segment/circle collision helper, popups, and spark bursts.

The BASIC rules now follow Rust's continuous movement: 85 pixels per second,
3.2 radians per second, and a trail sample every two pixels. Gaps occupy ten
samples, start every 2.5 seconds, stay in place, and pay once. Solid trails
remain dangerous after elimination, including when they overlap a gap.

Steering is chosen for all seats before any head moves. The CPU uses the same
13 candidate headings, safety margin, turn penalty, and first unspent gap
per opposing trail. A second human can take over the second seat without
reviving it; starting with two humans also creates a CPU. Round resets retain
scores and human lives, restore CPUs, and add a CPU up to six seats.

### Verification

`test/adder-parity.mjs` compares browser interpreter state with the real Rust
rules in `crates/adder/examples/adder_parity.rs`. Scenarios cover straight movement,
both turns, joining, CPU wall avoidance and gap seeking, trail growth, wall
and body hits, unspent and spent gaps, overlapping solid trails, and game over.
Positions and angles agree within 0.003; lengths, lives, scores, and flags
must match exactly.

`test/adder-controls.mjs` checks actual keyboard and touch steering, opposite
keys cancelling, joining, two-player starts, and round progression in renamed
`/USER` copies. `test/basic.mjs` retains title/start and round-four music checks.

### Remaining differences

The title's three demonstration paths are an animation rather than a live CPU
pit. Spawns use BASIC's random stream rather than Rust's random stream, so
their layouts differ even though the spawn-clearance rules match.

## Rally

The BASIC game now keeps Rust's 480 × 540 playfield, 72-pixel paddles,
300-pixel-per-second movement, and 275-to-450 ball speed ramp. Paddle spin,
swept collisions, wall overshoot, CPU tracking, first-to-seven scoring, and
point and match pauses use the native rules and dimensions.

Rust's ball integrator lives in `crates/rally/src/physics.rs`. The browser
comparison in `test/rally-parity.mjs` runs the actual BASIC tape against that
integrator across flight, paddle edges, spin, speed caps, low frame rates,
wall/paddle corners, and both scoring boundaries. Numeric state must agree
within 0.003. Random serves and CPU errors still use different random streams.
Direct dragging on the canvas remains a difference; BASIC supports both touch
dials and physical paddle keys.

## Cassette files

`/TAPE` mounts only the inserted game's source and graphics. Inactive cassette
files cannot be read or drawn from the cache. Stop/Eject clears the mount;
`LOAD /TAPE/GAME` inserts that cassette. Games copied to `/USER` carry their
exported pictures and can run with the cassette removed. `test/basic-tape.mjs`
checks listings, swapping, ejection, inaccessible graphics, and a portable copy.

## Frame timing

Brawler's frame-counted startup, recovery, and round timer, and Defender's
frame-counted movement and firing, now use `WAIT 0`
so each update waits for the next rendered frame. A fixed 16 ms delay after
the work could skip another display frame. Full rules and CPU parity remain
unfinished; the current BASIC mode still differs from Rust's complete game.

## Other games

Bubbler's two-player controls and Rally's two touch dials have real gameplay
checks in `test/basic-game-controls.mjs`. Their renamed user copies preserve
controls through `REM GAME:` metadata. The remaining games still need the same
deterministic rule comparisons as Adder and Rally; visual smoke checks alone do not
establish gameplay parity.
