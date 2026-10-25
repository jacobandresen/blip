# BASIC and Rust visual checks

Build the BASIC runtime and all Rust web games before running the browser
check:

```sh
cargo build-web
npm run test:visual
```

`test/run-visual.mjs` runs each game in its own muted Chromium process.
`test/visual-parity.mjs` captures each title, starts both versions, sends one
movement input, samples twelve play frames, and checks that the pictures stay
visible, respond to input, and keep rendering. Brawler's BASIC tape enters
fighter select on the first Space press and starts the match on the second;
Rust steps through its demo, fighter select, and versus screens with three
presses. The test waits for those screens before sampling play.

These are liveness and input-response checks. They do not compare pixel
similarity or prove that animation, combat timing, or physics match. Set
`BLIP_VISUAL_ARTIFACTS=1` to save title, start, input, and play captures from
both versions under `test/artifacts/visual-parity/` for manual review. Run one
game with, for example, `BLIP_VISUAL_GAME=BRAWLER npm run test:visual`.

## Brawler pose comparison

Build the native Brawler and web games, then capture matching Rust and BASIC
scenes:

```sh
cargo build --release -p brawler
cargo build-web
node tools/compare_brawler.mjs
```

The comparison captures clean 640 × 400 scenes using the same fighter roster,
stage art, and pose data. It writes paired images and a gallery to
`test/artifacts/brawler-compare/`. Choose a single scene and animation frame
with:

```sh
node tools/compare_brawler.mjs --scene jade-alt-kick --frame 3
node tools/compare_brawler.mjs --scene choose-versus
node tools/compare_brawler.mjs --scene mack-special --tick 56
node tools/compare_brawler.mjs --scene mack-special --tick 141 --screenshot-frame 1
```

`--frame` accepts 0–5. `--tick` selects a Brawler special's simulation tick
(0–183); use it with `mack-special` to inspect the lowrider's arrival, pause,
shots, and exit. `--screenshot-frame` accepts 1–5 and controls the Rust
renderer frames before its capture (default 2). The gallery is visual evidence,
not an automated quality score: review framing, anatomy, contact poses, effects,
and text at full and phone size.
