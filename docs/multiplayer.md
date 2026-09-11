# Two-device multiplayer

Status: **shipped** (Rally only) — WebRTC + Supabase Realtime signaling,
QR-code signaling as the offline alternative. See [Phasing](#phasing) for
how it was built up, and [Testing](#testing) for how it's covered.

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
isn't the DataChannel we're trying to set up. Two options, both shipped:

Both connect with host (local) ICE candidates only, per the same-room
[scope decision](#goal) — a STUN server can still be added cheaply later
for the odd double-NAT home network, but **no TURN relay**, ever, in
scope: that would mean game traffic transiting a server, which is
exactly what "same room only" is choosing not to pay for.

1. **Supabase Realtime as a signaling relay** (`web/blip_net.js`'s
   `host()`/`join()`). BLIP already has a Supabase project wired up for
   high scores ([`docs/highscores.md`](highscores.md)) — its
   [Realtime](https://supabase.com/docs/guides/realtime) channels carry
   the handful of small JSON messages (offer, answer) a WebRTC handshake
   needs. Host creates a short numeric room code, publishes its offer to
   a channel named for that code, guest types the code in and subscribes
   to the same channel. Needs a moment of real internet on *both* phones
   to complete pairing; once the DataChannel opens, gameplay no longer
   touches Supabase (or the internet) at all.
2. **QR-code signaling** (`hostQR()`/`joinQR()`, the "NO WIFI? PAIR VIA QR
   CODE" option) — for when reaching Supabase isn't an option. Host
   renders its SDP offer as a QR code; guest scans it with the camera,
   generates an answer, shows *that* as a QR code; host scans it back.
   Zero network needed for pairing at all. Both sides wait for their own
   ICE gathering to finish before rendering the code ("vanilla ICE" — no
   separate candidate-exchange round to race against), which keeps the
   payload to one QR code per side; verified this fits and decodes
   correctly at realistic SDP sizes and well beyond (see
   [Testing](#testing)).

Either way, once `RTCPeerConnection` reports `connected` and the
DataChannel's `open` event fires, signaling is done and out of the
picture for the rest of the match.

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
6. **QR-code signaling** as the offline alternative to Supabase Realtime.

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
   byte-for-byte; the room-code generator/validator (format, zero-padding,
   normalization). Follows the same pattern
   [`test/fill-canvas.test.mjs`](../test/fill-canvas.test.mjs) already set:
   give it inputs, assert the output. Fast, zero flakiness, run on every
   `npm test`.
2. **Rust unit tests — `cargo test`, no wasm/browser.**
   `crates/rally/src/net.rs`'s own `#[cfg(test)]` module — the
   `NetState` struct's byte layout round-trips (`pack` then `unpack`
   equals the original), including NaN/infinity and wrong-length inputs,
   compiled for the host target — no wasm32 target, no browser,
   sub-second.
3. **Headless two-browser integration tests — a committed CDP harness.**
   [`test/lib/cdp.mjs`](../test/lib/cdp.mjs) (a hand-rolled Chrome
   DevTools Protocol client — raw WebSocket, `Runtime.evaluate` +
   `Input.dispatchTouchEvent`, no Puppeteer/Playwright dependency) plus
   [`test/multiplayer.mjs`](../test/multiplayer.mjs), which launches two
   real headless Chromium instances, has one host and the other join over
   the real Supabase Realtime signaling, and drives real input across the
   real DataChannel to confirm both sides agree on the resulting paddle
   position. `npm run test:multiplayer`.

   Two real bugs surfaced by this harness during development: the pairing
   modal's own "close" path was tearing down the connection it had just
   made (fixed in `blip_net_ui.js`), and Chrome's background-tab
   throttling was killing an already-open DataChannel within about a
   second in headless mode (fixed with
   `--disable-backgrounding-occluded-windows` and friends in
   `test/lib/cdp.mjs`'s launch flags).

   Watch for one specific headless gotcha before trusting a red result:
   Chrome hides local ICE candidates behind a `.local` mDNS hostname by
   default (a privacy feature), which can fail to resolve inside a
   locked-down CI network namespace even though the two processes are on
   the same host. Launch with `--disable-features=WebRtcHideLocalIpsWithMdns`
   for these tests specifically if connections mysteriously never leave
   `checking` state.

   The QR-signaling path has its own real-camera-equivalent check: Chrome
   can feed a specific video file into `getUserMedia` in headless mode
   (`--use-fake-device-for-media-stream
   --use-file-for-fake-video-capture=<file>.y4m`), so a rendered QR code
   from one instance's canvas can be piped through ffmpeg into a video
   file the other instance's fake camera reads — exercising the real
   `getUserMedia` → `<video>` → canvas-sampling → jsQR decode path on both
   sides, not just the plain encode/decode round-trip.

**CI.** None of this repo's tests run in CI today — `.github/workflows/`
only has the Pages deploy. Add a `.github/workflows/test.yml` (`cargo
test`, `npm test`, then the CDP harness with a chromium install step —
e.g. `browser-actions/setup-chrome`, or `apt-get install chromium`,
either works on `ubuntu-latest`) that runs on every PR. Worth doing
regardless of this feature, but this is the point where it stops being
optional: a flaky two-peer connection test that only a human remembers to
run by hand won't get run.

**What automated tests can't fully cover:** real WiFi conditions (signal
quality, and — the actual cause of at least one real-world pairing
failure seen so far — routers that block device-to-device traffic between
their own clients, a common "AP/client isolation" setting with no
software workaround from BLIP's side), and a QR code held up to an actual
phone camera rather than fed through Chrome's fake-camera device (focus
distance, screen glare, real permission prompts). The three tiers above
prove the connection/signaling/game-sync logic itself is correct; those
two remain manual, real-hardware checks.

## Open questions

- **Cheating.** A guest's browser console can just send fabricated
  `{t:'input'}` messages. Fine for a casual arcade cabinet game between
  two people in the same room; would need host-side input sanity checks
  (e.g. reject an input rate above what's physically possible) if this
  ever mattered more.

## Related

[High scores via Supabase](highscores.md) — the Realtime channel this
plan proposes reusing for signaling lives on the same project.
