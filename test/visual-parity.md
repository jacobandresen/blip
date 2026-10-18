# BASIC and Rust visual comparison

Run `npm run test:visual` after building the BASIC runtime and the nine Rust
games with `./build_web.sh`. The test opens each `/TAPE/*.BLIP` game and its
Rust counterpart in Chromium at the same 1000 × 900 viewport. Each game runs
in its own process so a browser teardown cannot cancel later comparisons. It
captures the title, starts the game with Space, sends the game's rightward key,
then samples twelve play frames. Brawler needs two further Space presses to
pass its Rust demo and fighter selection. The test fails if either version
stays blank, flickers to black during play, stops with a BASIC runtime error,
or draws fewer than ten WebGL frame batches per second.

The output reports the fraction of bright pixels in each picture and the
observed WebGL draw cadence. These are diagnostics, not a pixel equality
score: moving objects, randomized scenes, and different title layouts make
exact screenshot matches unreliable. A large difference in lit area points
to a scene that needs visual review; a large cadence difference points to a
timing problem. Draw batches per second measures rendering, not simulation
ticks or audio timing.

For one game, run `BLIP_VISUAL_GAME=RALLY npm run test:visual`. Set
`BLIP_VISUAL_ARTIFACTS=1` to save title and first/last play screenshots from
both versions in `test/artifacts/visual-parity/` for side-by-side review.
The screenshots are generated evidence; the repeatable test and this guide
live in the repository.
