# An immersive arcade

A plan for making the arcade (`web/index.html`) feel like a room you walk
through rather than a list you step along. Five parts, built in this order:
walking and sound first, since together they change the feel the most; then
stepping up to play; then living screens; then the vertical keys. Each part
ships on its own and leaves the suite green.

## Where it starts

- Each machine is posed by `cardPose(c, delta, room)`, where `delta` is a
  whole number of machines from the one faced (`aisleOffset`, which wraps
  round the nine). `highlight` writes the pose as `--arc-x/y/z`, `--arc-turn`
  and `--arc-scale`, and a CSS transition on `#game-grid .card` eases it over
  2.72 s. A machine that crosses the wrap gets `aisle-wrap` for a frame so it
  jumps instead of sliding across the room.
- An arrow press queues one step (`move`, `walkQueue`, at most two laps);
  `walkNext` takes a step every 2.24 s, so a held key walks at under half a
  machine a second.
- A swipe walks `round(dx / (0.34 × width))` machines, decided on `pointerup`;
  nothing moves while the finger is down. The deck's stick repeats `move`
  every 150 ms after 340 ms, which only fills the queue.
- `pollGamepad` (`web/kiosk.js`) hands over digital codes only; the stick's
  analogue value is lost.
- `playMachineStep` plays two panned footsteps per step. `cabinetHum` runs
  under everything once the first key or tap unlocks audio.
- Screens show `screenshot.png` behind PRESS START (`.machine-attract`). No
  machine runs its game in the arcade; the live preview was removed, and
  `cabinet.mjs` checks that no hidden game loads.
- Start zooms the room into the faced screen over 0.46 s (`openMachine`,
  `machine-launching`), then opens the game page, which starts on a black
  loader.
- A game going back to the arcade hands over its last frame
  (`blip-cabinet-return`).
- Up and down repeat left and right (`moveRow`).

## 1. Walking

### Position

- Where you stand becomes a decimal, `aislePos` (3.4 is between the fourth
  and fifth machine), kept modulo nine. `idx` stays the faced machine:
  `Math.round(aislePos)`, updated as you pass the halfway point.
- `aisleOffset(i)` returns `i - aislePos`, wrapped into −4.5…4.5.
  `cardPose` already works on a fractional `delta` once `turn` and `z` blend
  instead of switching at zero: `turn = clamp(delta, −1, 1) × 32` and `z`
  eases from 40 to −90 over the first machine.
- A machine near the wrap (|offset| > 3.5) fades to `opacity: 0`, so its
  jump from +4.5 to −4.5 is never seen. This replaces `aisle-wrap`.

### Motion

- One `requestAnimationFrame` loop owns the aisle. It runs only while the
  aisle moves and stops when it settles. The CSS transition on the cards
  goes; the loop writes the pose every frame.
- The loop integrates a velocity `v` (machines per second) towards a target
  speed:
  - a held arrow or stick: 1 machine/s, reached in about 0.4 s;
  - nothing held: `v` decays and the aisle springs to the nearest whole
    machine in the direction it was going (critically damped, settling in
    about 0.5 s);
  - a tap (press and release inside 200 ms) sets the target to the next
    whole machine, so a tap still moves exactly one;
  - a second tap while moving adds one more machine to the target, as the
    queue does now.
- The walking maths lives in a new `web/aisle.js`, with no DOM: a
  `step(state, input, dt)` that returns the next `{pos, v, target}`. The
  page feeds it keys, pointer and gamepad; tests feed it numbers.

### Inputs

- Keys: `keydown` and `keyup` on ArrowLeft/Right and A/D set "held", with
  key repeat ignored.
- Drag: on `pointerdown` the aisle follows the finger, one machine per
  0.34 × the track's width as now. On `pointerup` the last 100 ms of
  movement becomes `v`, and the spring picks the machine it coasts to. A
  drag shorter than the swipe threshold is still a tap on a machine or its
  screen (`selectByTouch`, `activate`). `touch-action: pan-y` stays.
- Deck stick: `setDir` sets "held" instead of repeating `move`.
- Gamepad: `pollGamepad` also reports `axes[0]` past its dead zone, and the
  arcade maps it to a target speed from 0.3 to 1.5 machines/s. Other
  callers of `pollGamepad` are unchanged.

### Feel

- A footstep each half machine walked: `playMachineStep` gains a
  single-step form, panned by direction; at rest, nothing.
- The view bobs 2–3 px in time with the footsteps, on `#jukebox-front`
  (a `translateY`).
- Parallax: the carpet moves at 1.3× the machines, the back wall at 0.5×.
  Each is an oversized layer moved by `transform`, never by
  `background-position`, which repaints.
- Arriving (the spring settles) flips the marquee and picks the card, as
  `highlight` does now; passing a machine without stopping does neither.
- The deck iframe's `--preview-lean` and `fitSelectorRails` are written on
  arrival and on the frames where the faced machine changes, not every frame:
  both touch layout.

### Reduced motion

`prefers-reduced-motion: reduce` keeps today's behaviour: whole-machine
steps, no transition, no bob, no parallax. A held key repeats at the
current 2.24 s.

### Performance

Only `transform` and `opacity` change per frame (see the touch performance
notes). Nine cards, one carpet and one wall layer are the whole budget.

## 2. Sound from the machines

### Attract loops

- Each machine gets a short attract jingle, four to eight bars, made in
  `kiosk.js` with Web Audio from a few notes per game in its own
  instrument (the game tunes are megabytes each; these are a few hundred
  bytes of note data). Rally keeps a plain beep, per its "keep it simple" rule.
- A jingle repeats every 6–10 s with silence between, so the room never
  becomes a wall of music.

### Mixing

- Each machine's level is `1 / (1 + d²)` for `d = |offset|`, and its pan is
  `clamp(offset / 2, −1, 1)` (a `StereoPannerNode`). Both are ramped
  (`setTargetAtTime`, 50 ms) from the walking loop, not set.
- Only machines with `d ≤ 2.5` play; the rest are stopped and disconnected,
  which caps the graph at about five voices on a phone.
- A room bed under it: a murmur (filtered noise), a coin drop now and then
  (`playCoinInsert`'s clink, quiet, panned at random), and `cabinetHum`.
- Everything connects through `blipOut(ctx)`, so `blip-mute` silences it.
  Like the hum, it waits for `blipPowered()`, a visible page and the first
  key or tap.
- The mix and the jingle choice are pure functions in `web/aisle.js`
  (`machineLevel(d)`, `machinePan(offset)`), tested without audio.

## 3. Stepping up to play

The zoom exists (`openMachine`, 0.46 s, then the page loads). What is
missing is the other half:

- `openMachine` stores the faced screen's picture with the launch, as
  `blip-cabinet-start` (today only the slug): `{slug, at}` and the
  `screenshot.png` path it showed.
- `web/shell.js` reads it on load. When it is fresh (under 15 s, as with
  `blip-cabinet-return`), the loader shows that picture full-screen and
  fades it out as the first frame draws, so the zoom flows straight into
  the game. The mirror of `blip-cabinet-return`.
- Stretch the zoom to about 0.6 s only if the handover still reads as a cut.

## 4. Living screens

- `tools/make_attract_strips.mjs` runs each game's native autopilot
  (`BLIP_BOT=1 BLIP_BOT_SHOTS=dir BLIP_BOT_SHOT_EVERY=0.25`, one game at a
  time), takes 12–16 frames from the first minute of play, scales them to the
  screen's 290×164 and writes `web/<game>/attract.webp`, one strip per game,
  under 60 KB.
- Machines with `d ≤ 2` show the strip at 3 fps (`steps()` on a
  `transform` of the strip inside the screen's box). Farther ones keep
  `screenshot.png`, and their strips are not requested until they come near
  (`loading="lazy"` does not apply to backgrounds, so the image is set on
  approach).
- The faced machine shows its strip under PRESS START.
- Light: each machine casts a soft pool of `rgb(BLIP_GAMES[slug].accent)` on
  the carpet, an `opacity` from `1 / (1 + d²)`; now and then (every 20–40 s,
  per machine) a marquee or screen flickers for 120 ms.
- Strips are added to `ASSETS` in `web/sw.js` so the arcade works offline.

## 5. Up and down

Nine games need no second row.

- Up (ArrowUp, W, stick up, gamepad up) steps up to the faced machine: the
  view leans in about 15 % (`#jukebox-front`'s `transform`), its deck and
  Start lamp brighten, and the room sound ducks by half.
  `data-stepped-up` on `#game-grid` marks the state.
- A second Up while stepped up plays it (part 3): `activate()`.
- Down steps back into the aisle. Left or Right while stepped up steps back
  and walks.
- This replaces `moveRow`, which repeats left and right.

## To decide

- Walking pace: today's deliberate steps, or about twice as fast? The
  plan's 1 machine/s held is about twice as fast; a tap still moves one.
- Machine sounds: on from the start, or only after a first key press or tap?
  Browsers allow only the second. A kiosk could ship with
  `--autoplay-policy=no-user-gesture-required`, but the page must not
  depend on it.

## Files

| Part | Files |
|---|---|
| Walking maths, sound mix (no DOM) | `web/aisle.js` (new) |
| Walking loop, inputs, stepping up, up and down | `web/index.html` (the aisle script and its styles) |
| Analogue stick | `web/kiosk.js` (`pollGamepad`) |
| Machine and room sound, single footstep | `web/kiosk.js` |
| Screen loops | `tools/make_attract_strips.mjs`, `web/<game>/attract.webp`, `web/index.html` |
| The game page's first frame | `web/shell.js`, `web/shell.css` |

These files are in the table in `AGENTS.md`, marked "planned". As each part
lands, drop its mark there. When `test/aisle.test.mjs` and the performance
test exist, add them to the Tests list in `AGENTS.md` (the first to `npm test`
in `package.json`). Each change to `web/` bumps `CACHE` in `web/sw.js`.

## Done when

| Part | Done when |
|---|---|
| 1. Walking | Unit, arcade, layout and performance tests pass; reduced motion steps as today |
| 2. Sound | Sound tests pass; heard on an iPhone and the kiosk |
| 3. Stepping up | The loader shows the faced screen, then the game |
| 4. Living screens | Nine strips under 60 KB, loaded only nearby, cached offline |
| 5. Up and down | The new `shell.mjs` test replaces the row test; `moveRow` is gone |

## How to test it

The walk becomes time-based, so the tests stop waiting on wall-clock steps
(today's `waitFor(…, 3000)` after each key) and read state instead.
`#game-grid` carries `data-aisle-pos` (rounded to 0.01) and
`data-aisle-moving` while the loop runs; tests wait on those. Tests that
check a moment mid-walk use Playwright's `page.clock`, which also drives
`requestAnimationFrame`, to hold time still.

### Unit (`npm test`, new `test/aisle.test.mjs`)

`web/aisle.js` with no browser:

- A tap from rest settles at exactly ±1, for every starting machine,
  including across the wrap (8 → 0, 0 → 8).
- Holding for 3 s from rest covers about 2.6 machines (0.4 s ramp to
  1 machine/s); letting go settles on a whole machine ahead, never behind.
- A flick with `v = 4` settles within 0.6 s and on a whole machine.
- Two taps in quick succession move exactly two.
- Reduced motion: every `step` lands on a whole number.
- `aisleOffset` stays in −4.5…4.5 for every `pos` in steps of 0.01.
- `machineLevel(0) = 1`, `machineLevel(1) = 0.5`, decreasing; `machinePan`
  is antisymmetric and clamped.
- No `NaN` for `dt = 0` or a 2 s `dt` (a tab coming back).

### Arcade (`test/cabinet.mjs`, `test/touch.mjs`)

- A tap on `.pick-next`, an ArrowRight press and a short swipe each move
  exactly one machine; the existing tests keep passing with the new waits.
- Holding ArrowRight for 3 s (keydown, `page.clock.runFor(3000)`, keyup)
  ends between two and four machines on, on a whole machine.
- A drag: `pointerdown`, 10 `pointermove`s over half the width, then a check
  that `data-aisle-pos` has moved by the finger before `pointerup`; after
  release, it settles on a whole machine.
- Gamepad: a stubbed `navigator.getGamepads` with `axes[0] = 0.5` walks
  slower than with `1.0`.
- Deck stick held: walks while held, settles on release.
- Reduced motion (`page.emulateMedia({ reducedMotion: 'reduce' })`): one
  press moves one, with no frames where `data-aisle-pos` is fractional.
- The marquee and `.card-focused` change only on arrival, not when a
  machine is passed at speed.
- Returning from a game (`blip-cabinet-return`) still lands on its machine
  and does not start a walk.

### Up and down (`test/shell.mjs`)

`on the cabinet, up and down move the selection a row` becomes:

- Up sets `data-stepped-up` and leaves the selection where it was.
- Down clears it.
- Up, Up opens the faced game (with a coin inserted first) and spends one
  coin.
- Left while stepped up clears it and walks one.

### Stepping up to play (`test/cabinet.mjs`)

- After Start, the game page's `#loader` shows the faced machine's
  `screenshot.png` before `data-game-ready`, and hides it after.
- A stale handover (over 15 s, set by hand in `sessionStorage`) shows the
  plain loader.

### Sound (`test/cabinet.mjs`)

Audio is checked by the graph, not by ear. A test hook,
`window.blipArcadeSound`, lists the playing machines with their gain and pan
values.

- Before any input, no `AudioContext` exists.
- After a key: the faced machine is loudest, its neighbours at about half,
  pans opposite in sign, nothing past `d = 2.5`.
- After walking one machine, the levels follow.
- With `blip-mute`, `blipOut`'s gain is 0.
- With the page hidden (`visibilitychange`), the loops stop.

### Living screens

- `tools/make_attract_strips.mjs` checks its own output: nine strips, each
  the right size and under 60 KB.
- On load, the arcade requests strips only for machines with `d ≤ 2` (count
  `page.on('request')`); walking requests the next one.
- Every strip is listed in `web/sw.js`.

### Layout (`test/layout.mjs`)

- The existing device sweep also runs at `aislePos` = n + 0.5 (the clock
  held mid-walk) and while stepped up: nothing overlaps the deck or leaves
  the screen, and the faced machine's Start and coin door stay in reach.

### Performance (new test over CDP, as in the touch performance checks)

- Hold ArrowRight for 3 s on an emulated phone with
  `Emulation.setCPUThrottlingRate(4)`. `Performance.getMetrics` must show
  `LayoutCount` unchanged during the walk except on arrivals (at most one per
  machine reached), and the 95th percentile of frame intervals under 20 ms.
- The same at idle for 2 s: no layouts, and no `requestAnimationFrame`
  running (the loop has stopped).

### By hand

Things a headless browser cannot judge:

- iPhone Safari: audio starts on the first tap, the stereo pan is right
  through headphones, the drag feels attached to the finger.
- A low-end Android phone: the walk stays smooth with three strips playing.
- The kiosk with a real gamepad: the stick's speed range feels right; Up,
  Up plays.
- The real GPU, headless (see the GPU notes): the parallax layers and pools
  of light show no banding.
- Fifteen minutes in the room with sound on: the jingles never get
  tiresome, and the room is quiet enough to talk over.
