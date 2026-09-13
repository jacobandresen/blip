# Plan: automated two-emulator Rally pairing test

Status: **implemented, not yet run end-to-end**. The primitives below
were hand-verified in a live session (see "Validation" below), and
[`test/multiplayer-android.mjs`](../test/multiplayer-android.mjs) now
implements the full flow against `scripts/lib/android-emulator.sh`. A full
boot-two-fresh-emulators integration run hasn't been done against the
finished test yet — see the note at the end of "Work items".

## Goal

Extend the existing automated coverage —
[`test/multiplayer-proto.test.mjs`](../test/multiplayer-proto.test.mjs)
(wire protocol unit tests) and
[`test/multiplayer.mjs`](../test/multiplayer.mjs) (real-camera QR pairing
between two desktop Chromium instances) — with a third test that drives
the actual JACK IN → HOST/JOIN → QR-scan → DataChannel → input-sync flow
between two **real Chrome-for-Android** processes on the two emulators
[`scripts/setup-android-multiplayer-test.sh`](../scripts/setup-android-multiplayer-test.sh)
already provisions — fully unattended, no human tapping, and **no kernel
modules, no root, no physical hardware**.

## Why the desktop approach doesn't just port over

`test/multiplayer.mjs` fakes a camera via Chromium's
`--use-fake-device-for-media-stream
--use-file-for-fake-video-capture=<file>.y4m`. The Android emulator has no
equivalent flag for Chrome-for-Android. An earlier version of this plan
proposed `v4l2loopback` (a Linux kernel module) to fake a V4L2 camera
device the emulator's `-camera-back webcamN` could point at — **ruled
out**: installing kernel modules is not an option in this environment (or
many CI/dev machines).

## The key primitive: `-camera-back imagefile:<path>` / `videofile:<path>`

The Android emulator's `-camera-back`/`-camera-front` flags accept an
`imagefile:<filename>` mode (also `videofile:<filename>`,
`image360:<filename>`) that renders that file directly as the camera
feed — discovered via `emulator -help-all`. This needs no kernel module,
no root, and no physical webcam.

An earlier attempt used `-camera-back virtualscene` +
`-virtualscene-poster <wall|table>=<file>` (a 3D show-apartment model
with swappable posters) — **abandoned**: the poster surfaces aren't
reliably in the default camera's field of view, and multiple attempts to
load different images produced no visible change on the object we could
see. `imagefile:`/`videofile:` avoids all of that; the camera sees
exactly the file, full-frame, with no 3D scene in the way.

## Validation performed this session

1. Booted an AVD with `-camera-back imagefile:/tmp/live_cam.png` (a solid
   red test PNG). Opened a minimal `getUserMedia()` + jsQR test page in
   Chrome on the emulator, granted the camera permission prompts via
   `adb shell input tap` (bounds read from `uiautomator dump`, not hardcoded
   coordinates), and pulled a real decoded video frame back to the host
   via `canvas.toDataURL()` over Chrome DevTools Protocol (CDP) — it was
   the red image, confirming `imagefile:` feeds real frames into
   `getUserMedia()`.
2. Overwrote `/tmp/live_cam.png` with a blue PNG **while the stream was
   already open** — the live feed did **not** change. The file is read
   once when the camera is opened, not hot-reloaded frame-by-frame.
3. From the same page (still connected via CDP), called
   `track.stop()` then a fresh `getUserMedia()` **without restarting the
   emulator** — the feed immediately picked up the new (blue) file
   content. Confirms: **each fresh camera open re-reads the file from
   disk**.
4. Confirmed Chrome's remote-debugging socket is reachable from the host
   via plain `adb forward tcp:<port> localabstract:chrome_devtools_remote`
   — ordinary CDP calls (`Runtime.evaluate`, `uiautomator`-located taps,
   canvas capture) work against Chrome-for-Android the same way
   `test/lib/cdp.mjs` already drives desktop Chromium.

## Why this is sufficient for Rally's actual pairing flow

In the real flow (`docs/multiplayer.md`), each device opens its camera
**exactly once**: the guest opens it to scan the host's offer QR; the
host separately opens it to scan the guest's answer QR. Neither device
needs to hot-swap its own camera mid-stream. So the test only needs to
**write the correct QR PNG to each device's fixed `imagefile:` path
*before* the button tap that triggers `getUserMedia()` on that device** —
no live-swap trick needed for the main flow. The stop/reopen trick from
validation step 3 is kept as a documented fallback only if the UI turns
out to pre-open the camera for a live preview before the actual scan
action.

## Test design (mirrors `test/multiplayer.mjs`, swapped transport)

1. **Boot.** Extend `android_emulator_boot()` in
   `scripts/lib/android-emulator.sh` with an optional camera-source
   argument, so the test can request
   `-camera-back imagefile:/tmp/blip-host-cam.png` for the host AVD and a
   separate fixed path for the guest AVD. Manual/default use keeps
   `-camera-back none`, unaffected — this stays purely additive to the
   generic, project-agnostic library.
2. **Connect.** `adb forward tcp:<port> localabstract:chrome_devtools_remote`
   per emulator; reuse `test/lib/cdp.mjs`'s `connect`/`evaluate`/`waitFor`
   unchanged, pointed at those forwarded ports instead of a locally
   spawned Chromium process.
3. **Drive the real flow**, step for step:
   - Host: tap JACK IN → HOST, wait for its QR canvas, grab the PNG via
     `canvas.toDataURL()` over CDP.
   - Write that PNG to the **guest's** fixed camera-source path.
   - Guest: tap JACK IN → JOIN (this is the point its camera opens — the
     file is already in place, so the fresh open reads the right
     content), poll for a decode exactly like the desktop test does, grab
     its answer QR PNG.
   - Write that PNG to the **host's** fixed camera-source path.
   - Host: tap SCAN, assert `window.blipNetRole()` is 1 on host / 2 on
     guest.
   - Assert the match starts (`dial-hand-p2` rotation leaves center) on
     both.
   - Dispatch a `KeyboardEvent` on the guest, assert the paddle fraction
     converges on both sides within tolerance.
   - Call `window.BlipNet.cancel()`, assert both report role 0.
4. **New test file**: `test/multiplayer-android.mjs`, sharing the
   QR-PNG-generation and CDP-driving helpers factored out of
   `test/multiplayer.mjs` into `test/lib/` rather than duplicated.
5. **npm script**: `test:multiplayer:android`, gated behind a preflight
   check (adb/emulator on PATH, `blip_host`/`blip_guest` AVDs already
   created) that **skips with a clear message** rather than failing hard
   if the environment can't support it. No privileged/root requirement
   this time (unlike the abandoned `v4l2loopback` plan), but still likely
   dev-machine-only given emulator boot time and the KVM requirement for
   reasonable performance in CI.

## Work items

1. ✅ Added the camera-source parameter to `android_emulator_boot` in
   `scripts/lib/android-emulator.sh` (stays project-agnostic — no
   blip-specific code in the generic lib). Also added
   `android_emulator_forward_devtools` (CDP-over-adb) and
   `android_emulator_grant_camera_permission` (taps Chrome's + Android's
   camera prompts) alongside it.
2. ✅ Added `connectAndroid` to `test/lib/cdp.mjs`, reusing the existing
   `connect()` core (generalized to filter page targets by URL substring,
   since a real Chrome instance always has other tabs open).
3. ✅ Wrote `test/multiplayer-android.mjs` (mirrors `test/multiplayer.mjs`'s
   structure) plus `test/lib/android-emulator.mjs`, a thin Node wrapper
   around the bash lib's CLI so the test can drive AVD lifecycle without
   duplicating any of its logic.
4. ✅ Wired `npm run test:multiplayer:android` and documented this tier in
   `docs/multiplayer.md`'s Testing section, including the skip-not-fail
   behavior on a machine without `/dev/kvm`.
5. ⬜ Validate end-to-end. The primitives above were validated
   individually against a live emulator; the finished test hasn't yet
   been run against two freshly-booted AVDs start to finish. Run
   `npm run test:multiplayer:android` (expect a first run to take a
   while — SDK/AVD provisioning if not already cached from
   `scripts/setup-android-multiplayer-test.sh`) and debug whatever the
   first real run surfaces; touch-driven UI timing and permission-prompt
   ordering on real Chrome-for-Android are exactly the kind of thing that
   tends to need a round or two of adjustment the first time.
