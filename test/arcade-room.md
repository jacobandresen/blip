# Arcade room (#65)

This work continues #65 from `f891e01`: the tiled next room, illustrated mural,
carpet, automatic lite selection and release evidence. It
retains the foundation/wall/sign work from #75/#76. Native game rules and WASM
are unchanged. Repo-owner visual approval is required before closing #65.

## Follow-up after PR #78

The owner requested removal of the PLAY!/INSERT COIN starbursts and reported
that the focused attract screen does not render in desktop Safari. They also
requested more coherent mural cabinets, preserving the machines' placement
against the wall and the room's perspective.

The starburst SVGs, construction and styling have been removed. The mural now
keeps its authored aspect ratio, with a shared vanishing point and back edges
at the painted wall/floor junction. The foreground cabinets and their physical
wall/floor geometry are unchanged.

Attract strips now paint frames using background position inside the CRT,
instead of animating a separately transformed image layer. Picture selection,
decode gating and PNG recovery remain. Screen tests keep the focused overlay
visible; only its text/shadow pixels are excluded from image comparisons.
Additional desktop checks cover normal focus, hover and the close view with
natural playback, without hiding the overlay or pausing the sprite animation.

Installed SafariDriver reports Safari 27.0.1, but both attempts to create an
automation session timed out connecting to Safari. Native Safari verification
is therefore still unavailable here; Playwright WebKit results remain engine
coverage, not proof of the reported installed-Safari fix.

The original approved gallery is preserved locally at
`test/artifacts/arcade-room/approved-with-decals/index.html`. The performance,
soak and comprehensive test results under the approved-baseline headings below
describe commit `66b58ab`, source fingerprint `0264c454…`, before this follow-up.

The follow-up run passed 50/51 checks, including all twenty attract checks
and natural desktop playback for all nine machines in focus, hover and close
view in both engines. One phone mural check found the painted screen covered
by the tiles halfway through a walk. A 24px composition offset and matching
overscan coverage address that placement. The owner then requested skipping
further checks and pushing the code, so this final adjustment has not been
rerun. The follow-up TAP log is `artifacts/arcade-room/safari-followup.tap`.
The current gallery/soak evidence remains the earlier approved baseline;
new captures and a new soak have not been produced for this follow-up.

## R2: tiled next room (#69)

[R2](https://cdn.masto.host/mastodongamedevplace/media_attachments/files/113/753/045/634/490/380/original/c12205a4a0dfe15f.jpg)
shows large dark ceramic squares, fine pale joints, contiguous stepped green,
purple, red and white shapes, and uneven reflections. The original inline SVG
uses those materials with a different abstract arrangement. Ten rows of mosaic
sit above seven plain white rows; matching CSS joints continue down taller
kiosk walls. Fourteen columns at 40 SVG units define the
opening's artwork; responsive sizing preserves square faces instead of
stretching the map. The physical horizontal repeat is a whole aisle lap,
including the intervening orange wall, recorded in `--room-tiles-period`.
The opening moves at the physical wall's perspective rate, as established in
#76, rather than the earlier proposed fixed 0.5 rate.

Desktop and kiosk show the full opening as the aisle passes it. Portrait phones
retain a crop at the right side and above the neighbouring cabinet. Landscape
hides the section, following #76. Lite removes the glaze and recess shadow,
retaining the colour map and pale joints. The radiator, museum branding and
existing game artwork in R2 are intentionally omitted.

The deterministic gallery is generated locally (artifacts are ignored):

```sh
node tools/capture_arcade_room.mjs --still
node tools/capture_arcade_room.mjs --still --lite
```

Open `test/artifacts/arcade-room/index.html` for the combined room and per-layer
crops. Compare the desktop and phone images at their actual display sizes.
Screenshots support manual reference review; passing tests do not establish
fidelity. The integrated owner sign-off remains in #73.

## R3: mural and carpet (#70, #71)

The [mural/carpet reference](https://cdn.masto.host/mastodongamedevplace/media_attachments/files/113/753/045/649/075/622/original/52f47bfd32ca5c17.jpg)
shows painted cabinets with angled sides, screens, bright marquees and warm
ceiling lines. The original inline illustration uses three generic uprights
with ochre/orange/red planes and brown outlines. It includes no game logos,
characters or copied cabinet artwork. Its seamless 720px scene (360px on
phones, 1440px on tall kiosks) is shared between rendering and movement.
Copies are adjusted on layout; walking translates the mural at rate 0.3.
The scene keeps its 3:2 aspect ratio rather than stretching to the wall height.
Cabinet sides recede toward the same authored vanishing point (360,170); their
backs meet the painted wall/floor line at y=324. The center cabinet faces
forward, and the left/right cabinets expose the corresponding inward side.
Portrait scales the complete scene uniformly into the upper wall. There is
no blur or idle mural loop, and lite retains its contrast.

The carpet replaces confetti with offset red/black rectangular courses.
Its authored 240×160px repeat is shared with the walk; the 3px woven grain
divides that repeat exactly. Lite removes the grain and pool glow. The warm
step lies at the physical wall/floor junction, behind the machines. Pools
retain some cabinet accent with amber/red light.

The floor moves at the machines' rate in the 3D plane established in #76.
That decision supersedes #71's earlier proposed fixed rate of 1.3. The near
floor sweeps faster because of perspective. Texture-wrap screenshots test
the real projected floor in both directions, full and lite.

## R1/R4: entrance decals (#72)

The owner requested that both starbursts be dropped after reviewing PR #78.
This supersedes #72's decal requirement. They are removed from the markup,
runtime construction, styles and layer contract. The tall-layout neon signs
retain their established size and placement.

## Release mode and credit (#73)

Before the first room paint, explicit `?room=full|lite` takes precedence.
Otherwise a finite positive numeric hardwareConcurrency or deviceMemory value
of at most 2 selects lite. Missing, invalid, zero, negative, string and infinite
values do not trigger it. Init-script tests verify each trigger independently,
invalid-query fallback, explicit overrides and a single initial mode selection.
The history sentence lives once in `web/history.html`, shared by both guides.

## Verification

`test/arcade-room.mjs` includes every layer in shared decoration, visibility,
hit-target, motion and lite checks. Its continuous walk sampling checks
effective displacement modulo the full lap, through negative travel and
last/first transitions. The focused checks verify square cells, a substantial
crop, preserved lite palette/grout, and matching rendered ceramic pixels
after a complete navigation lap in both directions and after resize.
The image comparison tolerates eight colour levels of compositing noise, with
at most 0.1% of pixels differing more; a misplaced tile boundary fails it.

Run browser checks sequentially on a loaded host. The shared layout command
starts four browser shards; its sequential equivalent is
`node --test --test-concurrency=1 test/layout.mjs`. The `test:web` file list is
in `test/run-web.mjs`; running those files through `node --test
--test-concurrency=1` exercises the same suites with one browser job at a time.

## iPhone Safari attract report

The original iPhone report was not reproduced by desktop WebKit. The first
compatibility pass added an explicit starting transform and decode gating,
informed by [WebKit bug 260981](https://bugs.webkit.org/show_bug.cgi?id=260981).
The owner subsequently reported the focused screen failing in desktop Safari.
The follow-up removes the animated sprite transform entirely: the decoded
strip becomes the CRT frame's background, advancing through twelve positions.
The original image remains the format-selection/decoding source. Brawler's
still remains until its decoded background is ready. Native-Safari confirmation
of this revised painting path remains required.

Each attract picture offers WebP through `<picture>/<source type="image/webp">`
with an indexed PNG strip as its `<img>` fallback. A failed WebP load/decode
removes the source so PNG can recover; failure of both retains the still.
The PNGs contain the same twelve captured frames and use the same CSS timing.
Generate both formats with `tools/make_attract_strips.mjs`; `--fallbacks-only`
derives PNGs from existing WebP captures without building/running native games.
ImageMagick 7 is required. Both formats are cached for offline use.
Picture selection fetches the preferred supported format; the service worker
precaches both formats for subsequent offline fallback.

`node --test test/attract.mjs` checks changing rendered frames in portrait and
landscape, unsupported source-type selection, recovery after WebP errors,
both-format failure and delayed decoding. It also renders changing PNG frames
for every game in both orientations and engines. `test/iphone14-room.mjs` walks all
nine cabinets through nine circuits per engine, including full/lite, both
orientations, last/first transitions and rotation back to portrait. Alongside
390×844 / 844×390, it uses Playwright's smaller iPhone 14 browser viewports,
390×664 / 750×340, to exercise the space left by browser chrome. It checks
passing screen pixels against the decoded strip, changing centered frames,
and safe/unobstructed screen and Start targets. The agreed close landscape
view can leave the coin door below the frame.
Landscape reserves the bottom safe area for the home indicator, keeping the
Start button inside the usable viewport. At 844×390 with a 21px bottom inset,
its lower edge is about 363px, above the 369px safe boundary.
The 750×340 browser viewport also limits cabinet scale using Start's authored
459px position and minimum 30.4px touch height. Its lower edge is about 316.3px,
inside the room's 318px clipping edge, rather than losing the bottom border.

Chromium uses a mobile viewport, DPR 3 and native safe-area overrides. Desktop
WebKit uses a mobile viewport, DPR 3, iPhone user agent and CSS safe-area
substitutions. These are engine emulations, not a real iPhone or iOS Simulator.
The latter is unavailable on this host (`simctl` is not installed). Artifacts
are under `test/artifacts/attract/` and `test/artifacts/iphone14-room/index.html`.
The live site is unchanged until publication, outside this local verification.
Read-only live checks on 2026-10-25 found the older `arcade-room.css?v=7`
page and pre-decode-gate attract code. Its Rally WebP is byte-identical to
the local asset (SHA-256 `b2eda71988f6c66afcf66d9507b29813bf3f9b5f2e3fe63f288db9292058673a`).
This does not reproduce or establish the cause of the reported physical-iPhone
blank screen; real-device confirmation is still needed after publication.

## Touchscreen controls

On touchscreen game pages, the shared selection rule now chooses gesture
strips where supported, or the on-screen pad/buttons otherwise, including
arcade launches and legacy `?controls=stick` links. Rally keeps its existing
dials and optional split strip. Non-touch devices keep their existing front
end and query selection. The cabinet artwork still uses the machines' decks.

`node --test test/touch-controls.mjs` checks the selection across all games and
query variants, plus visible strip, pad and Rally dial configurations in
Chromium and desktop WebKit. The two-player deck test now verifies that a
legacy stick link still lets player two join through their touch buttons.
Adder's gesture strip also exposes 2P JOIN while its second station is open;
`test/adder-two-player.mjs` verifies the real game accepts it.

## Performance and kiosk evidence (approved baseline)

`tools/capture_arcade_room.mjs` can serve historical room sources with
`--baseline <commit>`, replay the same 120-move CDP touch trace with `--perf
--runs 3 --trace`, and separate one-time image loads with `--warm-screens`.
It records source fingerprints, browser, CPU, GPU renderer, counts/durations
per input and frame intervals, with compressed timeline traces. Compare cold
and warmed runs separately; counters from different browser/hardware setups
are not directly comparable.

The historical #66 revision is `87ef391`; `f891e01` is the starting revision
for this work after #68/#76. Replays use Chromium 153.0.8010.12 on Apple M2,
ANGLE Metal, with GPU compositing enabled, at 390×844. The original #66 record
is preserved in PR #75; these are fresh same-host baseline/current replays.

Each row below is three runs of the same 120-move trace. Values are ranges;
all runs have a 16.7ms median frame interval. Final current traces and the
32-image gallery share the mobile reports' `0264c454…` source fingerprint.

| Revision / image state | Layouts/input | Style recalcs/input | Style CPU ms/input | Frame p95 ms |
| --- | ---: | ---: | ---: | ---: |
| `87ef391`, cold | .0333 | 2.042–2.058 | .649–.662 | 16.7–16.8 |
| `87ef391`, warmed | 0 | 6.642–6.800 | .707–.718 | 16.7 |
| `f891e01`, cold | .0750 | 2.042–2.050 | .821–.826 | 16.8 |
| `f891e01`, warmed | .0333 | 2.042 | .831–.847 | 16.7 |
| Current, cold | .0917 | 2.042–2.058 | .782–.804 | 16.7–16.8 |
| Current, warmed | .0333 | 2.042 | .821–.895 | 16.7–16.8 |

The cold current trace includes two additional main-document layouts when
new picture strips load/finish decoding. These are bounded first-image costs,
not layout reads added to the walk loop. Warmed current and `f891e01` traces
both contain four partial layouts rooted at the existing `svg.room-wrapper`,
as it wraps/changes visibility offscreen; they take about 20–30µs each. That
prop was absent from `87ef391`, explaining its zero warmed layout count.
No new tile, mural or decal layout roots appear during steady walking.
Style work is higher than the original foundation; the table preserves that
cost rather than calling all counters unchanged. No frame-time regression was
observed in these runs. JSON metrics and compressed traces are retained under
`test/artifacts/arcade-room/performance/`.

`--soak` runs the 1080×1920 kiosk for 30 real minutes after warm-up, recording
heap once per minute. Minute 1 and the final minute use the same two-GC
procedure. A 30-second frame trace follows the final heap sample. A shorter
`--soak-minutes 1 --baseline 87ef391` run supplies an idle baseline trace, not
a 30-minute soak. Logs are under `test/artifacts/arcade-room/`.
The historical idle baseline completed with 1,801 measured frames, median
and p95 16.7ms, and zero frames over 50ms or 250ms. It uses the same Chromium
version and Apple M2 Metal renderer as the current run; its source fingerprint
matches the historical touch-trace baseline (`0baa79b9…`).

The final 30-minute run completed on 2026-10-25 from 18:29:17 to 18:59:49 UTC
(including its final trace). All 30 minute samples were visible. Minute 1 and
minute 30 used the same double-GC procedure:

| Retained measurement | Minute 1 | Minute 30 |
| --- | ---: | ---: |
| JavaScript heap, bytes | 5,226,612 | 5,202,800 |
| DOM nodes | 4,345 | 4,345 |
| Documents | 25 | 25 |
| JS event listeners | 260 | 260 |

Retained heap decreased **0.46%**, passing the <=10% increase target. Raw heap
samples stayed between 5,202,800 and 5,263,172 bytes; intermediate DOM counts
oscillated and returned to the same retained count after GC. Documents and
event-listener counts stayed constant throughout the run.

The final 30-second trace measured 1,801 frames, median/p95 **16.7ms**, zero
frames over 50ms or 250ms, and no layout events. This matches the historical
idle baseline's frame timing. The current JSON and compressed trace are in
`test/artifacts/arcade-room/soak/soak-full-f891e01_changes-30min.*` and match
source fingerprint `0264c454…`.

`test/kiosk-room.mjs` enables fullscreen through the Home Screen's settings,
returns to the cabinet, presses the actual coin door and Start, and checks
fullscreen retention through a simulated two-minute game idle return and
subsequent normal user activation. This timer test is separate from the
real-time 30-minute memory soak. Neither proves days-long kiosk stability.

Screenshot review found that decorative machine-deck iframes inherited the
saved fullscreen preference, hiding their controls on a non-touch kiosk.
`blip_core.js` now excludes `data-deck-preview` documents when restoring that
preference; game pages still restore it normally. The kiosk suite checks all
nine physical deck controls in Chromium and WebKit, their painted screenshots,
and that the parent's saved fullscreen preference remains enabled.

## Verification log (approved baseline)

The serialized full web run (the 28 files registered in `test/run-web.mjs`,
plus `test/shell.mjs`) finished with 484/497 passing. Its log is
`test/artifacts/arcade-room/web-tests.tap`. Eight launch assertions still
tapped legacy controls hidden by the requested touchscreen default; the
updated cabinet suite subsequently passed all 37 checks using the offered
strip, pad or Rally dial. Two mobile checks found Start in the landscape
home-indicator area; the corrected mobile/cabinet rerun passed all 39 checks.
A subsequent visual review found the kiosk-preview problem described above;
the mobile/cabinet/kiosk rerun passed all 42 checks (`mobile-cabinet-final.tap`).
The additional browser-toolbar cases and stricter clipping-edge checks are
recorded separately in `final-engine-room.tap`, which passed all 39 checks
(room/aisle performance, expanded mobile circuits and kiosk). Both mobile
reports match source fingerprint
`0264c45432feea8ad4b07dd3dc47b8aa65f23094df07d348fbd0d9fbd0fe143a`:
202 WebKit and 206 Chromium passing-screen regions across nine configurations
per engine, plus changing centered pictures for every cabinet in each circuit.
Pass-by captures use held pointer drags; Chromium CDP touch input is exercised
separately by the performance replay.
The full run is not an all-green result.

The three BASIC failures were rerun verbatim three times each against both
`f891e01` and the working sources, serialized on the same browser/host:

| BASIC case | Starting revision | Working revision |
| --- | --- | --- |
| SERPENT load, steering and three rounds | 3/3 pass | 3/3 pass |
| RAIDER boss progression | 0/3 pass | 1/3 pass |
| DEFENDER tapered bomb collision | 3/3 pass | 3/3 pass |

RAIDER fails before this work and varies between runs. The original SERPENT
load timeout and DEFENDER empty marker did not reproduce in isolation. The
BASIC interpreter, runtime WASM and these three tapes are byte-identical to
`f891e01`. Logs and the copied diagnostic runner are retained as
`basic-isolation.tap` and `basic-isolation.mjs` in the artifact directory;
run the latter with `node --test` from the repository root. No BASIC assertion
or gameplay rule was relaxed to obtain these results.

`npm test` passed all 20 checks. `git diff --check` passed. Native parity
tests regenerated audio with binary differences; those unrelated generated
outputs were restored from HEAD after the full web run.

The final serialized `test/layout.mjs`, `test/arcade-room.mjs` and
`test/aisle-perf.mjs` run passed all 139 checks (`layout-room-final.tap`),
including the complete 105-case layout matrix across 21 device/viewports.
The expanded attract suite passed all 18 checks (`attract-final.tap`),
including all nine PNG fallbacks in portrait and landscape in both engines.

The final normal/lite gallery contains 16 images per mode and matches the
expanded mobile reports and all six current performance traces. Representative
captures were inspected against R1–R4 at phone and full size, including rest,
half-walk and both credit states. This review caught the blank kiosk control
deck before the fullscreen-preview correction; its diagnostic screenshot is
preserved under `artifacts/arcade-room/diagnostics/`.

The room deliberately uses generic painted cabinets and an original mosaic,
omits museum/character branding and the radiator, and keeps #76's compact
phone/landscape composition. The repo owner approved the final R1–R4 gallery
in this session on 2026-10-25 at 19:04 UTC ("i approve"). This approval applies
to the 32-image gallery with source fingerprint `0264c454…`.

After the initial approval, the owner requested a branch push and pull request;
PR #78 contains that implementation. The subsequent follow-up is described
above. Production deployment and issue closure remain separate actions.
Physical iPhone 14 Safari confirmation is still unavailable here; the supplied
WebKit results are explicitly emulated.
