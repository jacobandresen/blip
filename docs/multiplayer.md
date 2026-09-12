# Two-device multiplayer

Status: **shipped** (Rally only) — WebRTC, QR-code signaling only. A
Supabase Realtime relay existed briefly as a second signaling path but
was dropped in favor of settling on one well-tested path rather than
maintaining two (see [Signaling](#signaling-how-the-two-phones-find-each-other-before-the-datachannel-exists)).
See [Phasing](#phasing) for how it was built up, and
[Testing](#testing) for how it's covered.

## Goal

Let two players on two separate phones play a head-to-head round (Rally
first) against each other, without either of them needing a shared
account, a login, or — ideally — a live internet connection once the
match has started.

**Scope decision: same room only.** Two devices on the same WiFi network
— not two players in different locations. This is a deliberate v1 limit,
not a fallback: it means every connection uses direct WebRTC host
candidates and **no TURN relay is ever needed**, which keeps the "no
server, no ongoing cost, no third party touching gameplay traffic" pitch
intact. Revisit only if remote play is explicitly wanted later (see the
old NAT/TURN trade-off this ruled out, folded into
[Signaling](#signaling-how-the-two-phones-find-each-other-before-the-datachannel-exists)
below).

Non-goals for v1: more than two players, spectators, reconnect-after-drop
mid-rally (a dropped connection ends the match), any game but Rally, and
— per the scope decision above — play between devices that aren't on the
same network. Rally is the only game already built with a "two-player"
concept ([`Mode::TwoPlayer`](../crates/rally/src/main.rs)) — everything
else here generalizes once Rally proves it out.

## Architecture

### Transport: WebRTC DataChannel, host-authoritative

Two peers, not a shared simulation. **One phone is the host**: it runs
the real Rally simulation exactly like local two-player does today (P1 =
local input, P2 = input arriving over the DataChannel), and streams the
authoritative ball/paddle/score state to the guest every frame. **The
guest phone doesn't simulate anything** — it renders whatever state
arrives and sends its own paddle input back. This sidesteps two hard
problems a "both sides simulate" (lockstep) design would hit:

- Floating-point results for the same math can differ across devices/
  browsers, so two independent simulations fed the same inputs can
  silently diverge (a desynced game where the ball is in two different
  places on two screens).
- Rally's frame timing already runs on real elapsed time
  (`blip.delta_time`, not a fixed tick), which is exactly what lockstep
  needs to *not* have.

WebRTC's DataChannel is the natural fit for the transport itself: a
direct peer-to-peer link once the two phones are on the same WiFi, no
server relaying gameplay traffic, and no new browser APIs beyond what's
already used for the leaderboard's networking. The cost is input latency
for the guest — their paddle move takes one network round-trip to show
up. On a same-room WiFi link that's typically single-digit-to-
low-double-digit milliseconds, fine for Rally's paddle speed. Revisit if
it ever feels laggy; client-side prediction on the guest (move the local
paddle optimistically, reconcile against the next host state) is the
standard fix and can be layered in later without changing the wire
format.

### Signaling: how the two phones find each other before the DataChannel exists

WebRTC needs a brief handshake (SDP offer/answer + ICE candidates)
*before* any peer-to-peer traffic can flow, and that handshake has to
travel over some channel that already exists — which, by definition,
isn't the DataChannel we're trying to set up.

**QR codes, exclusively** (`web/blip_net.js`'s `host()`/`join()`, wired up
by `web/blip_net_ui.js`'s "PLAY NEARBY" modal). Host renders its SDP
offer as a QR code; guest scans it with the camera, generates an answer,
shows *that* as a QR code; host scans it back. Zero network needed for
pairing at all — no server, no account, nothing to reach before the two
phones can even see each other. Both sides wait for their own ICE
gathering to finish before rendering the code ("vanilla ICE" — no
separate candidate-exchange round to race against), which keeps the
payload to one QR code per side; verified this fits and decodes
correctly at realistic SDP sizes and well beyond, and — the part worth
actually proving rather than assuming — that a real `getUserMedia` camera
capture can decode one (see [Testing](#testing)).

A Supabase Realtime relay (a short numeric room code typed on the other
phone, publishing the offer/answer as broadcast messages) was built and
shipped first, then **dropped**: maintaining two signaling paths for one
feature wasn't worth it once QR alone proved reliable, and QR has a real
edge the room-code path never had — it needs no internet on either
phone, not even for a moment. If Realtime signaling is ever wanted back
(e.g. a lower-friction option for players comfortable typing a code), the
migration that granted it access is reverted
(`supabase/migrations/20260911140000_drop_multiplayer_signaling.sql`) —
re-adding the original grant is a one-migration change, not a redesign.

Direct host (local) ICE candidates only, per the same-room
[scope decision](#goal) — a STUN server can still be added cheaply later
for the odd double-NAT home network, but **no TURN relay**, ever, in
scope: that would mean game traffic transiting a server, which is
exactly what "same room only" is choosing not to pay for.

Once `RTCPeerConnection` reports `connected` and the DataChannel's `open`
event fires, signaling is done and out of the picture for the rest of the
match.

### Wire format

Small and unversioned-but-tagged from day one (`{ v: 1, ... }`) since
this will need to evolve:

- **Guest → host, every input change** (not every frame — only on
  transition, same idea as the existing `key_pressed`/`key_held` split in
  [`crates/blip/src/input.rs`](../crates/blip/src/input.rs)):
  `{ t: 'input', up: bool, down: bool }`. Tiny and cheap enough to also
  just send on an interval (e.g. 30Hz) instead of on-change if that proves
  simpler — DataChannel in unreliable/unordered mode (like a game socket)
  is the right config for this either way (`ordered: false,
  maxRetransmits: 0` — a dropped/late input sample should be superseded
  by the next one, not retried).
- **Host → guest, every rendered frame**: a fixed 36-byte binary packet
  (`crates/rally/src/net.rs`'s `NetState`) — phase, ball xy/velocity,
  both paddle y's, both scores. Same unreliable/unordered channel — a
  skipped state frame is invisible once the next one arrives.

### Where this plugs into the existing code

- **Guest input in → the game, unchanged.** `blip_controller.js`'s
  synthetic-KeyboardEvent pipeline is already how the on-screen dial and
  physical keyboard both reach the WASM side (`is_key_down`/
  `is_key_pressed` in `crates/blip/src/input.rs` never know or care where
  the KeyboardEvent came from). `blip_net.js` dispatches the *same*
  synthetic `keydown`/`keyup` for `I`/`K` (rally's P2 keys) whenever a
  `{t:'input'}` message arrives — no Rust changes needed on the
  guest-acts-as-host-input side, or on a host that's just running normal
  local `Mode::TwoPlayer`.
- **Host state out / guest state in → three small FFI imports**, mirroring
  the existing `web.rs` pattern (`blip_paddles`, `blip_high_name`) rather
  than adding new wasm exports: `blip_net_role()`, `blip_net_send(ptr,
  len)`, `blip_net_poll(ptr, cap)`. A `NetRole` (`None`/`Host`/`Guest`)
  alongside `rally/main.rs`'s existing `Mode`; a guest never runs
  `update_play`'s simulation at all, just copies the latest polled packet
  into `Game`'s fields — `draw_play` doesn't change, it already just
  reads those fields.
- **Title screen.** A "PLAY NEARBY" entry point opens the pairing modal
  (`blip_net_ui.js`) before `start_game()` ever runs; once paired,
  `update_title` picks up the role via `blip_net_role()` and starts a
  normal `Mode::TwoPlayer` match.

## Phasing

Built in this order:

1. **Signaling + connection only, no gameplay.** Two browser tabs
   exchange a room code via Supabase Realtime, establish an
   `RTCPeerConnection` + DataChannel, and confirm it opens. This was the
   riskiest, least-code-reused part — isolated first.
2. **One-way state stream.** The `blip_net_send`/`blip_net_poll` FFI on a
   host running normal local `Mode::TwoPlayer`, confirming the wire
   format and frame rate before any rendering was on the line.
3. **Guest rendering.** `Mode::NetGuest`, pointing `draw_play` at the
   received state.
4. **Guest input back to the host.** `blip_net.js`'s synthetic
   KeyboardEvents from the guest's real touch/keyboard input — a full
   two-device match playing end-to-end.
5. **UX pass.** Title-screen entry point, room-code pairing screen,
   "opponent disconnected" handling.
6. **QR-code signaling** added as a second, offline path alongside the
   room-code one.
7. **Settled on QR-only.** Dropped the Supabase Realtime relay (host/join
   room codes, the RLS grant, the room-code proto helpers and their
   tests) once QR alone proved reliable end to end — one signaling path
   to maintain instead of two, and QR's "no internet at all" property is
   strictly better for this feature's actual pitch. `test/multiplayer.mjs`
   was rewritten around QR as the sole, real path (see
   [Testing](#testing)) rather than kept as a second suite.

Still open: a rematch button that re-uses the existing connection instead
of re-pairing.

## Testing

The same-room decision above is what makes this automatable at all: two
headless Chromium instances on the same CI runner *are* "two devices in
the same room" as far as WebRTC's host ICE candidates are concerned — no
real network, no phones needed to test the connection logic. Three tiers,
cheapest and most deterministic first:

1. **Pure-logic unit tests — `node:test`, no browser.**
   [`test/multiplayer-proto.test.mjs`](../test/multiplayer-proto.test.mjs) —
   the wire-format encode/decode (`{t:'input',...}`) round-trips
   byte-for-byte. Follows the same pattern
   [`test/fill-canvas.test.mjs`](../test/fill-canvas.test.mjs) already set:
   give it inputs, assert the output. Fast, zero flakiness, run on every
   `npm test`.
2. **Rust unit tests — `cargo test`, no wasm/browser.**
   `crates/rally/src/net.rs`'s own `#[cfg(test)]` module — the
   `NetState` struct's byte layout round-trips (`pack` then `unpack`
   equals the original), including NaN/infinity and wrong-length inputs,
   compiled for the host target — no wasm32 target, no browser,
   sub-second.
3. **Headless two-browser integration test — a committed CDP harness,
   real camera decode both directions, requires `ffmpeg` too.**
   [`test/lib/cdp.mjs`](../test/lib/cdp.mjs) (a hand-rolled Chrome
   DevTools Protocol client — raw WebSocket, `Runtime.evaluate`, no
   Puppeteer/Playwright dependency) plus
   [`test/multiplayer.mjs`](../test/multiplayer.mjs), which launches two
   real headless Chromium instances and pairs them the same way two real
   phones would — not a shortcut through the SDP strings. Chrome can feed
   a specific video file into `getUserMedia` in headless mode
   (`--use-fake-device-for-media-stream
   --use-file-for-fake-video-capture=<file>.y4m`): the test renders the
   host's real offer QR, pipes it through `ffmpeg` into a video file, and
   points the guest's fake camera at it, so the guest's JOIN button
   actually opens a (fake) camera, samples (fake) video frames, and
   decodes a real QR code with jsQR — then does the same in reverse for
   the host scanning the guest's answer. Confirmed passing on 6
   consecutive runs. `npm run test:multiplayer`.

   Real bugs this harness (and its ad hoc predecessor, before it was
   committed) surfaced during development: the pairing modal's own
   "close" path was tearing down the connection it had just made (fixed
   in `blip_net_ui.js`); Chrome's background-tab throttling was killing
   an already-open DataChannel within about a second in headless mode
   (fixed with `--disable-backgrounding-occluded-windows` and friends in
   `test/lib/cdp.mjs`'s launch flags); a stray bare `canvas {...}` rule in
   shell.css (meant only for the game's own `#glcanvas`) was hijacking the
   QR `<canvas>` via `position:fixed`, pushing it off the modal entirely
   (fixed by overriding those properties inline on the QR canvas
   specifically); and a test-readiness check that treated HTML's own
   default 300×150 canvas size as "the QR is ready" caught the placeholder
   canvas instead of the rendered code (fixed by checking for a *square*
   canvas above a minimum size instead).

   Watch for one specific headless gotcha before trusting a red result on
   the connection itself: Chrome hides local ICE candidates behind a
   `.local` mDNS hostname by default (a privacy feature), which can fail
   to resolve inside a locked-down CI network namespace even though the
   two processes are on the same host. Launch with
   `--disable-features=WebRtcHideLocalIpsWithMdns` for these tests
   specifically if connections mysteriously never leave `checking` state.

**CI.** None of this repo's tests run in CI today — `.github/workflows/`
only has the Pages deploy. Add a `.github/workflows/test.yml` (`cargo
test`, `npm test`, then the CDP harness with a chromium install step —
e.g. `browser-actions/setup-chrome`, or `apt-get install chromium`,
either works on `ubuntu-latest` — plus `ffmpeg`, already preinstalled on
GitHub-hosted runners) that runs on every PR. Worth doing regardless of
this feature, but this is the point where it stops being optional: a
flaky two-peer connection test that only a human remembers to run by hand
won't get run.

**What automated tests can't fully cover:** real WiFi conditions (signal
quality, and — the actual cause of at least one real-world pairing
failure seen so far — routers that block device-to-device traffic between
their own clients, a common "AP/client isolation" setting with no
software workaround from BLIP's side), and a QR code held up to an actual
phone camera rather than fed through Chrome's fake-camera device (focus
distance, screen glare, real permission prompts). The three tiers above
prove the connection/signaling/game-sync logic itself is correct; those
two remain manual, real-hardware checks.

This gap wasn't hypothetical: the very first real-hardware attempt hit
"nothing happens when I point the camera at the QR code" — `blip_qr.js`'s
`render()` drew the modules edge-to-edge with no light margin around
them, so the code sat directly against the pairing modal's own dark
background with no [quiet
zone](https://en.wikipedia.org/wiki/QR_code#Quiet_zone) at all. jsQR (like
most real scanners) can fail to even *locate* a code with no quiet zone,
rather than just decode it less reliably — invisible to the fake-camera
test above, since that test pads every frame with its own margin before
handing it to the fake device, which papered over the exact bug a real
camera hit immediately. Fixed by drawing a real 4-module white margin
into the canvas itself (so it survives a screenshot, a video frame,
anything); the test now pads with the modal's own dark background color
instead of white, so it no longer accidentally supplies the quiet zone
the app should be responsible for.

The quiet-zone fix alone did not resolve a second real-hardware report of
"still nothing" — the underlying cause of that one is still unconfirmed
(candidates: a screen-to-camera moiré/module-size problem from the QR
encoding a full SDP with embedded ICE candidates at a fairly high QR
version, an `NotAllowedError`/permission issue on that device, or
something else entirely). Since the fake-camera test only proves the
happy path (a clean, well-lit, correctly-framed code decodes), it can't
diagnose a *specific* failed attempt after the fact — there's no log to
go back and read. So `blip_qr.js`'s `scan()` now reports live status
(`onStatus(state, detail)`: opening → streaming → a continuously updating
frame count and "no code found yet"/"N possible codes in view", or a
specific camera-permission/hardware error) instead of staying silent
until it either works or times out, and draws on-screen markers: a green
box around a confirmed decode (from jsQR's own `location`), or a yellow
circle around anything a lightweight reimplementation of jsQR's own
finder-pattern ratio check (1:1:3:1:1 dark:light:dark:light:dark run
lengths, scanned per-row against a single global threshold — coarser than
jsQR's real per-region adaptive one, so it flags more false positives,
but it's cheap and only ever drives a cosmetic marker) thinks might be a
QR pattern *before* a full decode succeeds. The next real-hardware report
should come with what the status line actually said, which narrows the
remaining possibilities a lot faster than "still nothing" did.

Before that report came back, "it just failed on my old iPad" pointed
straight at the QR being too dense/small to resolve rather than a
diagnosable error — the one real lever that matters most for an *old*
camera specifically. Three changes address that directly:

- **`web/blip_sdp_slim.js`** trims the SDP's `a=candidate:` lines to
  `typ host` only (no STUN/TURN configured, so nothing else should
  appear anyway), drops literal IPv6 addresses (kept: plain IPv4 and
  Chrome/Safari's `*.local` mDNS-hidden hostnames — both privacy-hiding
  and a real address, unlike an IPv6 literal, are perfectly usable),
  and caps the result at 4. A clean test VM's single network interface
  never shows the difference, but a real laptop with a VPN adapter, a
  virtualization bridge, or an IPv6 address alongside the IPv4 one can
  advertise several times that many — every one of them is another
  `a=candidate:` line in the QR code, pushing it to a denser version
  with smaller modules. Unit-tested directly (`test/sdp-slim.test.mjs`)
  since the multi-candidate case isn't reproducible in the single-NIC
  e2e environment.
- The code itself renders bigger on screen (240px → 280px for the
  offer/answer QR, 220px → 260px for the camera-scan preview), with a
  `max-width:100%` / measured-overlay-bitmap safety net so it still
  fits a narrow phone rather than overflowing the pairing modal.
- `blip_qr.js` now asks `getUserMedia` for `{width:{ideal:1920},
  height:{ideal:1920}}` instead of leaving resolution unspecified —
  several browsers otherwise default to a fairly low capture resolution
  (routinely 640×480) unless asked for more, handing jsQR far fewer
  real pixels than the camera is actually capable of. `ideal` (not
  `exact`) means a camera below this just gives its best; the request
  never fails over it.

## Open questions

- **Cheating.** A guest's browser console can just send fabricated
  `{t:'input'}` messages. Fine for a casual arcade cabinet game between
  two people in the same room; would need host-side input sanity checks
  (e.g. reject an input rate above what's physically possible) if this
  ever mattered more.

## Related

[High scores via Supabase](highscores.md) — a separate feature on the
same Supabase project. Multiplayer no longer depends on Supabase at all
(see [Signaling](#signaling-how-the-two-phones-find-each-other-before-the-datachannel-exists));
this project is the only remaining reason Rally's page ever touched it.
