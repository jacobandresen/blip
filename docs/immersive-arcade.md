# An immersive arcade

A plan for making the arcade (`web/index.html`) feel like a room you walk
through rather than a list you step along. Five parts, built in this order:
walking and sound first, since together they change the feel the most; then
stepping up to play; then living screens; then the vertical keys.

## Where it starts

- Each machine is posed by `cardPose(offset)`, where `offset` is a whole
  number of machines from the one faced (`aisleOffset`, `--arc-x/y/z`,
  `--arc-turn`).
- An arrow press queues one step (`move`, `walkQueue`); `walkNext` takes a
  step every 2.24 s while the aisle eases over 2.72 s.
- `playMachineStep` (`web/kiosk.js`) plays two footsteps per step.
- Screens show a screenshot; the faced machine runs a live preview
  (`data-cabinet-screen`).
- Up and down repeat left and right (`moveRow`).
- A game going back to the arcade hands over its last frame
  (`blip-cabinet-return`).

## 1. Walking

- Where you stand becomes a decimal (`aislePos`, 3.4 is between the fourth
  and fifth machine). A `requestAnimationFrame` loop poses every machine at
  `i - aislePos`; `cardPose` takes fractional offsets.
- Holding an arrow speeds up to about one machine a second; letting go
  coasts to a stop at the nearest machine. A tap still moves exactly one.
- A drag moves with the finger and keeps a little momentum after; a gamepad
  stick sets the walking speed.
- A footstep every half machine walked (`playMachineStep`), a 2–3 px bob of
  the view in time with them, and parallax: the carpet drifts faster than the
  machines, the back wall slower.
- Arriving at a machine flips its marquee to the name and picks the card,
  as now.
- Reduced motion keeps whole-machine steps and no bob.
- Only `transform` and `opacity` change per frame (see the touch performance
  notes), so phones stay smooth.

## 2. Sound from the machines

- Each machine plays a quiet attract loop: short chip jingles made in
  `kiosk.js` with Web Audio, not the game tunes (megabytes each).
- Its volume falls off with distance, about `1 / (1 + d²)` for `d` machines,
  and it pans left or right with its offset (a `StereoPannerNode`, as
  `playMachineStep` uses). The faced machine is clearest.
- A room bed under it: murmur, distant coin drops, the cabinet hum
  (`cabinetHum`).
- Everything connects through `blipOut(ctx)`, so the mute switch
  (`blip-mute`) silences it; it starts on the first tap or key, as browsers
  require.

## 3. Stepping up to play

- Start, or Up a second time, zooms the whole scene over about 0.6 s until
  the cabinet's screen fills the view, then opens the game page.
- The game page's loader shows that same screen image first, so it reads as
  one continuous move: the mirror of `blip-cabinet-return`.

## 4. Living screens

- Nearby machines show a short loop of their game at 2–4 fps instead of a
  still. The frames come from the native autopilot (`BLIP_BOT_SHOTS`), made
  by a script in `tools/` into one small sprite strip per game.
- The faced machine keeps its live preview; far machines show a still.
- Light: each machine casts a soft pool of its accent colour
  (`BLIP_GAMES[slug].accent`) on the carpet, brighter as you approach; now
  and then a marquee or screen flickers.

## 5. Up and down

Nine games need no second row. Up steps up to the faced machine (the view
leans in, its controls light up; a second Up plays, see 3); Down steps back
into the aisle. This replaces `moveRow`, which repeats left and right.

## To decide

- Walking pace: today's deliberate steps, or about twice as fast?
- Machine sounds: on from the start, or only after a first key press or tap?

## Files

| Part | Files |
|---|---|
| Walking, stepping up, up and down | `web/index.html` (the aisle script and its styles) |
| Machine and room sound | `web/kiosk.js` |
| Screen loops | `tools/` (a strip maker), `web/<game>/attract-*.png`, `web/index.html` |
| The game page's first frame | `web/shell.js`, `web/shell.css` |

Each change to `web/` bumps `CACHE` in `web/sw.js`.

## Tests

- `test/shell.mjs`: the up/down test (`on the cabinet, up and down move the
  selection a row`) becomes step up and step back; its timings follow the new
  walk.
- `test/cabinet.mjs`, `test/touch.mjs`: walking by key, drag and gamepad; a
  tap still moves one machine; reduced motion steps.
- `test/layout.mjs`: nothing overlaps or leaves the screen mid-walk or while
  stepped up.
- New: per-frame layout cost while walking stays at transform and opacity
  (measured over CDP, as in the touch performance checks).
