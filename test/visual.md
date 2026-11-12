# Visual checks

Build the web games before running the browser check:

```sh
npm run build:web
npm run test:visual
```

`test/run-visual.mjs` runs each game in its own muted Chromium process.
`test/visual.mjs` opens each game in free play with no deck, captures its
title, starts it, sends one movement input, samples twelve play frames, and
checks that the pictures stay visible, respond to input, and keep rendering.
Brawler steps through its demo, fighter select, and versus screens with three
presses before play is sampled.

These are liveness and input-response checks, not pixel comparisons. Set
`BLIP_VISUAL_ARTIFACTS=1` to save the title, start, input, and play captures
under `test/artifacts/visual/` for manual review. Run one game with, for
example, `BLIP_VISUAL_GAME=BRAWLER npm run test:visual`.
