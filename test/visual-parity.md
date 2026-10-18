# BASIC and Rust visual comparison

Run `npm run test:visual` after building the BASIC runtime and the nine Rust
games with `./build_web.sh`. The test opens each `/TAPE/*.BLIP` game and its
Rust counterpart in Chromium at the same 1000 × 900 viewport. Each game runs
in its own process so a browser teardown cannot cancel later comparisons. It
captures the title, starts the game with Space, sends the game's movement key,
then samples twelve play frames. Brawler needs three further Space presses in
Rust to pass its demo, fighter selection, and versus screen; BASIC enters the
fight on the first press, so its capture waits the same startup interval. The
test first checks that Start changed the title into gameplay, then fails if
either version stays blank, flickers to black during play, stops with a BASIC
runtime error, ignores the movement key, or draws fewer than ten WebGL frame
batches per second.

The output reports the fraction of bright pixels in each picture, how much
each picture changes under movement, and the observed WebGL draw cadence.
These measurements establish that gameplay responds and continues rendering;
they do not establish that the two versions look alike. Randomized scenes,
screen framing, and CRT treatment affect the measurements, so the saved
captures need visual review. Draw batches per second measures rendering, not
simulation ticks or audio timing.

For one game, run `BLIP_VISUAL_GAME=RALLY npm run test:visual`. Set
`BLIP_VISUAL_ARTIFACTS=1` to save title and first/last play screenshots from
both versions in `test/artifacts/visual-parity/` for side-by-side review.
The screenshots are generated evidence; the repeatable test and this guide
live in the repository.
