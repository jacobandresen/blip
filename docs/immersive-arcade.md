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

## 1. Walking — done

As built (`web/aisle.js`, the aisle script in `web/index.html`):

### Position

- Where you stand is a decimal, `aisle.pos` (3.4 is between the fourth and
  fifth machine), published as `data-aisle-pos` on `#game-grid`, with
  `data-aisle-moving` while the aisle moves. `idx`, the faced machine, changes
  only on arrival.
- `aisleOffset(i)` is `BlipAisle.offset`, wrapped into −4.5…4.5. `cardPose`
  blends on a fractional `delta`: `turn = clamp(delta, −1, 1) × 32`, and `z`
  eases from 40 to −90 over the first machine.
- Past 3.5 machines a cabinet is `visibility: hidden`, so its jump across the
  wrap is never seen. Not `opacity`: below 1 it flattens the cabinet's 3D
  sides. This replaces `aisle-wrap`.

### Motion

- One `requestAnimationFrame` loop (`aisleTick`) owns the aisle while it
  moves and stops when it settles. The cards' CSS transition is gone; the loop
  writes each card's `transform` inline.
- `BlipAisle.step(state, input, dt)`:
  - while a key is down, the walk ramps to 1 machine/s (0.4 s);
  - a press let go within 0.2 s is a tap: the aisle settles exactly one
    machine on (a critically damped spring, about 0.8 s, never overshooting);
  - held longer, letting go coasts to the nearest machine ahead it can reach
    without overshooting;
  - each further tap adds one machine to where the walk will stop, and a tap
    the other way turns back.

### Inputs

- Keys: `keydown` (not its repeats) is a tap and starts a hold; `keyup` ends
  it. `blur` lets go of everything.
- Drag: past 12 px sideways the aisle follows the pointer, a machine per
  0.34 × the track's width; on release the last 0.1 s of movement carries on.
  A drag short of the swipe threshold springs back. Taps on a machine or its
  screen work as before.
- The deck's stick and the pad deck hold the walk instead of repeating `move`.
- Gamepad: the arcade reads `axes[0]` itself while the pad holds the walk,
  from 0.3 to 1.5 machines/s; `pollGamepad` is unchanged.
- Clicking a neighbour walks to it (`walkTo`). Start pressed mid-walk starts
  the machine the walk arrives at.

### Feel

- A footstep each half machine walked (`playMachineStep(dir, 1)`, feet
  alternating).
- The view bobs up to 2.5 px with the steps. It is written into the cards',
  carpet's and wall's transforms, not `#jukebox-front`'s, whose transform is
  the launch zoom.
- Parallax: the carpet (`.aisle-carpet`, formerly `#game-grid::after`) moves at
  1.3× the machines, the back wall's ribs (`.aisle-wall`) at 0.5×.
- On arrival `highlight` picks the card, sets `--arc-scale`, the deck's lean
  and `fitSelectorRails`, once.

### Reduced motion

Today's behaviour, unchanged: whole-machine steps (`move`, `walkNext`), no
bob, no parallax.

### Performance

- No layout while walking; about 1 ms of style and 1 ms of layerizing a
  frame, on a desktop CPU.
- On the real GPU (GTX 1660, headless) the walk holds 60 fps, as the old CSS
  transition did. Headless software GL draws it at 30 fps, so the test checks
  main-thread cost, not frame rate.
- The lit Start cap's glow animated `text-shadow`, which forced a layout every
  frame even at rest; it is now an opacity pulse.

## 2. Sound from the machines — done, not yet heard on a phone or the kiosk

As built (`blipArcadeSound` and `BLIP_ATTRACT` in `web/kiosk.js`, enabled by
the aisle script):

### Attract loops

- Each machine has a jingle of 7 to 15 notes in its own instrument
  (`BLIP_ATTRACT`: wave, root, beat, decay, level), played through a 2.6 kHz
  low-pass to keep it warm. Rally's is two plain beeps; Meteors' a low
  heartbeat; Bubbler's a music box; the action games' saw and square riffs.
- After each jingle a machine is quiet for 6–10 s, and machines start out of
  step, so the room never plays in unison.
- A 200 ms look-ahead scheduler (`schedule`) books each note on the audio
  clock.

### Mixing

- Each machine's level is `BlipAisle.machineLevel(d)` = `1 / (1 + d²)`, its
  pan `BlipAisle.machinePan(offset)` = `clamp(offset / 2, −1, 1)`, both ramped
  over 50 ms. `place(pos)` is called from `poseCards`, every frame of a walk.
- Only machines within 2.5 play (five at once); the rest are dropped and
  disconnected.
- The room bed: a murmur (noise in a 520 Hz band, swelling at 0.13 Hz), and
  every 12–30 s a coin dropped in somewhere off to one side. `cabinetHum` is
  unchanged.
- The whole room goes through one gain (0.22, halved by `duck(true)` for
  stepping up) into `blipOut(ctx)`, so `blip-mute` silences it.
- It starts on the first key, tap or click, like the hum, and stops while
  the page is hidden. It does not wait for a credit: the machines are on
  whether or not the cabinet has a coin.

## 3. Stepping up to play — done

The zoom already existed (`openMachine`, 0.46 s, then the page loads). The
other half, as built:

- `openMachine` stores `blip-cabinet-launch` (`{slug, at, art}`, the faced
  screen's picture) beside `blip-cabinet-start`, which stays the slug the game
  page presses Start for.
- `web/shell.js` reads it on load. When it is under 15 s old, for this game,
  and the page is neither app style nor a cabinet screen, `#launch-frame`
  shows that picture where the game will draw (`fillCanvas` places it with
  `.cabinet-glass`), letterboxed as the game is. When the first frame is
  drawn (`hideLoader`) it fades out over 0.35 s.
- The zoom no longer fades to black: its last frame is the screen's picture,
  and the next page opens on it.
- The zoom stays at 0.46 s.

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
| The game page's first frame (`#launch-frame`) | `web/shell.js`, `web/shell.css` |

These files are in the table in `AGENTS.md`, marked "planned". As each part
lands, drop its mark there. When `test/aisle.test.mjs` and the performance
test exist, add them to the Tests list in `AGENTS.md` (the first to `npm test`
in `package.json`). Each change to `web/` bumps `CACHE` in `web/sw.js`.

## Done when

| Part | Done when | Status |
|---|---|---|
| 1. Walking | Unit, arcade, layout and performance tests pass; reduced motion steps as today | Done |
| 2. Sound | Sound tests pass; heard on an iPhone and the kiosk | Tests pass; listening by hand still to do |
| 3. Stepping up | The loader shows the faced screen, then the game | Done |
| 4. Living screens | Nine strips under 60 KB, loaded only nearby, cached offline | |
| 5. Up and down | The new `shell.mjs` test replaces the row test; `moveRow` is gone | |

## How to test it

Done items are marked ✓.

The walk is time-based, so the tests read state instead of waiting out
steps: `#game-grid` carries `data-aisle-pos` (rounded to 0.01) and
`data-aisle-moving` while the aisle moves.

### Unit (`npm test`, `test/aisle.test.mjs`) ✓

`web/aisle.js` with no browser:

- ✓ A tap from rest settles at exactly ±1, for every starting machine,
  including across the wrap (8 → 0, 0 → 8); it arrives within a second and
  never overshoots.
- ✓ Holding for 3 s walks 2.5–3.2 machines at 1 machine/s; letting go stops
  on the next machine ahead or the one after, never behind.
- ✓ A held walk's pace follows `speed`.
- ✓ A flick settles within a second, on a whole machine ahead.
- ✓ Two quick taps move exactly two; a tap the other way turns back.
- ✓ A slow drag past a third of a machine moves one; a short drag springs back.
- ✓ `offset` stays in −4.5…4.5 for every `pos` in steps of 0.01.
- ✓ `machineLevel(0) = 1`, `machineLevel(1) = 0.5`, decreasing; `machinePan`
  is antisymmetric and clamped.
- ✓ No `NaN` for `dt = 0` or a 2 s `dt` (a tab coming back).

### Arcade (`test/cabinet.mjs`) ✓

- ✓ A tap on `.pick-next`, an ArrowRight press and walking the whole aisle
  each move one machine at a time (the existing tests, unchanged).
- ✓ Holding ArrowRight for 3 s ends two to four machines on, on a whole
  machine; at 1.2 s the card is still the first one (picked on arrival, not
  in passing).
- ✓ A mouse drag moves `data-aisle-pos` before release and settles on the
  next machine; a 30 px drag springs back.
- ✓ A stubbed gamepad at `axes[0] = 1` walks more than 1.8× as far as at 0.4.
- ✓ Reduced motion: one press moves one, and `data-aisle-pos` is never
  fractional.
- ✓ Returning from a game still lands on its machine (the existing test).
- The bottom deck's stick and pad hold the walk too, but the arcade hides the
  bottom deck (`body.cabinet-page #kiosk-bar`), so there is nothing to test.
- `test/shell.mjs`'s title test waits for the walk to arrive before reading
  the picked card.

### Up and down (`test/shell.mjs`)

`on the cabinet, up and down move the selection a row` becomes:

- Up sets `data-stepped-up` and leaves the selection where it was.
- Down clears it.
- Up, Up opens the faced game (with a coin inserted first) and spends one
  coin.
- Left while stepped up clears it and walks one.

### Stepping up to play (`test/cabinet.mjs`) ✓

- ✓ After Start (the game's wasm held back 1.5 s), the game page shows
  `#launch-frame` with the machine's `screenshot.png`, exactly over the
  canvas, before `data-game-ready`; it is gone after, and so is the handover.
- ✓ A stale handover (20 s old), or one for another game, shows the plain
  loader.

### Sound (`test/cabinet.mjs`) ✓

Audio is checked by the graph, not by ear: `blipArcadeSound.state()` lists
the playing machines with the level and pan each is heading for.

- ✓ Before any input, no `AudioContext` exists and the room is silent.
- ✓ After a key: the faced machine at level 1 and centred, its neighbours at
  0.5 on their own sides, the ones beyond quieter, nothing past 2.5.
- ✓ After walking one machine, the levels follow; one machine comes into
  earshot and another leaves it.
- ✓ With `blip-mute`, `blipOut`'s gain is 0.
- ✓ With the page hidden the room stops, and it starts again when shown.

### Living screens

- `tools/make_attract_strips.mjs` checks its own output: nine strips, each
  the right size and under 60 KB.
- On load, the arcade requests strips only for machines with `d ≤ 2` (count
  `page.on('request')`); walking requests the next one.
- Every strip is listed in `web/sw.js`.

### Layout (`test/layout.mjs`)

- ✓ Every device's sweep also checks the arcade half a machine along, held
  there by a drag: nothing overlaps or leaves the screen.
- While stepped up: the same (part 5).

### Performance (`test/aisle-perf.mjs`, in `npm run test:web`) ✓

- ✓ At rest for 2 s on an emulated phone: no layouts.
- ✓ Holding ArrowRight for 3 s: at most three layouts (one on arrival), and
  under 8 ms of script and style a frame.
- Frame rate is checked by hand on the real GPU (see Performance in part 1):
  software GL in headless Chromium halves it whatever the page does.

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
