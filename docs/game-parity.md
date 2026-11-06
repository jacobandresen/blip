# BASIC and Rust gameplay parity

## Adder

The BASIC tape draws a 620 × 350 pit on BASIC's 640 × 400 screen, where Rust's
is 460 × 492, with the same generated pit art.
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

The BASIC court is 640 × 400 (Rust's is 480 × 540), so the tape scales the ball
by the width: Rust's 275-to-450 speed ramp in steps of 15 is 360 to 600 in
steps of 20, and a rally crosses the court in the same time. Paddle spin,
swept collisions, wall overshoot, first-to-seven scoring, and point and match
pauses follow the native rules.

The paddles are not scaled the same way: 64 pixels at 320 pixels a second, and
a CPU that tracks at 170, on a court 368 pixels tall. Measured in courts per
rally the paddles are half again as quick as Rust's and the CPU two thirds quicker (72 pixels at 300, CPU 145,
on 512), so the BASIC CPU is the harder opponent.

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

Brawler's frame-counted startup, recovery and round timer, and Defender's
frame-counted movement and firing, use `WAIT 0` so each update waits for the
next rendered frame. A fixed 16 ms delay after the work could skip another
display frame. Brawler BASIC has its own complete playable ladder and shares
the Rust roster, move table, captured poses, stages and music assets. Its combat
and CPU are implemented separately, so the focused browser and visual checks
do not establish frame-by-frame state parity with Rust.

## Other games

Bubbler's two-player controls and Rally's two touch dials have real gameplay
checks in `test/basic-game-controls.mjs`. Their renamed user copies preserve
controls through `REM GAME:` metadata. The remaining games still need the same
deterministic rule comparisons as Adder and Rally; visual smoke checks alone do not
establish gameplay parity.

The pictures follow the arcade's where the screen allows: Meteors' rock
colours (`HSV`) and glow lines, Defender's ground line and streak strip,
Raider's sea and plane shadows, Brawler's K.O. and PERFECT calls, and the
shared game-over screen (`DRAW_GAME_OVER`). Warlock's bottom panel becomes
two side panels, because BASIC's screen is wider than it is tall.
