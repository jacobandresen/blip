# Two-device multiplayer ("via Bluetooth")

Status: **planning** — nothing built yet. This is the design doc to work
from; see [Phasing](#phasing) for where to start.

## Goal

Let two players on two separate phones play a head-to-head round (Rally
first) against each other, without either of them needing a shared
account, a login, or — ideally — a live internet connection once the
match has started. "Via Bluetooth" is the ask; [the constraint
below](#the-bluetooth-constraint-read-this-first) is why the plan doesn't
literally use the Web Bluetooth API for the game traffic, and what it
does instead to still deliver that.

Non-goals for v1: more than two players, spectators, reconnect-after-drop
mid-rally (a dropped connection ends the match), any game but Rally.
Rally is the only game already built with a "two-player" concept
([`Mode::TwoPlayer`](../crates/rally/src/main.rs)) — everything else here
generalizes once Rally proves it out.

## The Bluetooth constraint — read this first

The Web Bluetooth API (`navigator.bluetooth`, what a web page can
actually call) only lets a browser act as a GATT **central** — the side
that scans for and connects to a peripheral. There is no standard,
shipped API for a web page to **advertise as a peripheral** so a *second*
web page could find and connect to *it*. Two browser tabs cannot pair
with each other over Web Bluetooth; one side would have to be a real BLE
peripheral device (a fitness band, a keyboard, a microcontroller — not a
phone running Chrome). This isn't a permissions or flag issue, it's the
shape of the spec. Confirm this hasn't changed before starting
([caniuse: Web Bluetooth](https://caniuse.com/web-bluetooth),
[spec](https://webbluetoothcg.github.io/web-bluetooth/)) — but as of
today, phone-to-phone Web Bluetooth between two plain web pages isn't
possible.

So "multiplayer via Bluetooth" between two phones running BLIP as a
website has to mean one of two things:

**A. Bluetooth as the network link, WebRTC as the transport.** Most
phones can turn on Bluetooth tethering (a Bluetooth PAN) between two
devices, which is an ordinary IP link once it's up — the browser doesn't
know or care that Bluetooth is underneath it. [WebRTC
DataChannel](https://developer.mozilla.org/en-US/docs/Web/API/RTCDataChannel)
works over any IP path, so it works here too. This needs **zero new
browser APIs** — it's the same WebRTC code whether the two phones share
WiFi, a Bluetooth PAN, or a USB-tethered link. The Bluetooth part is a
manual OS-settings step for the players, not something BLIP's code
drives (there's no web API to switch on Bluetooth tethering for them).

**B. A native BLE peripheral, for real.** BLIP would need a native or
hybrid shell (Capacitor, say) using the platform's BLE peripheral APIs
directly, bypassing the browser entirely for the networking. This is a
materially bigger project — a second build target, app-store-adjacent
distribution concerns, and it stops being "just a website." Worth
knowing about, not worth building for a first pass.

**Recommendation: build A.** It delivers the actual thing a player
wants — *play a friend standing next to you, without needing a shared
WiFi network or a server relaying every move* — while staying inside the
web platform BLIP already lives on. Bluetooth tethering is the literal
answer to "via Bluetooth" for the player who has no WiFi to share; WebRTC
also works unmodified over plain WiFi for everyone else, which is the
common case in practice (two phones on the same home/venue network).

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

The cost is input latency for the guest — their paddle move takes one
network round-trip to show up. On a same-room WebRTC link (WiFi or
Bluetooth PAN) that's typically single-digit-to-low-double-digit
milliseconds, fine for Rally's paddle speed. Revisit if it ever feels
laggy; client-side prediction on the guest (move the local paddle
optimistically, reconcile against the next host state) is the standard
fix and can be layered in later without changing the wire format.

### Signaling: how the two phones find each other before the DataChannel exists

WebRTC needs a brief handshake (SDP offer/answer + ICE candidates)
*before* any peer-to-peer traffic can flow, and that handshake has to
travel over some channel that already exists — which, by definition,
isn't the DataChannel we're trying to set up. Two options, worth building
in this order:

1. **Supabase Realtime as a signaling relay (build first).** BLIP already
   has a Supabase project wired up for high scores
   ([`docs/highscores.md`](highscores.md)) — its
   [Realtime](https://supabase.com/docs/guides/realtime) channels can
   carry the handful of small JSON messages (offer, answer, ICE
   candidates) a WebRTC handshake needs. Host creates a short numeric
   room code, publishes its offer to a channel named for that code, guest
   types the code in and subscribes to the same channel. Needs a moment
   of real internet on *both* phones to complete pairing; once the
   DataChannel opens, gameplay no longer touches Supabase (or the
   internet) at all — so a Bluetooth-tethered pair with no other
   connectivity would need to pair *before* switching off shared WiFi, or
   pair over the Bluetooth link's own quick handshake with option 2
   below.
2. **QR-code signaling (fully offline, build second if wanted).** Host
   renders its SDP offer as a QR code; guest scans it with the camera,
   generates an answer, shows *that* as a QR code; host scans it back.
   Zero network needed for pairing at all — genuinely works with
   Bluetooth (or nothing) as the only link between the two phones. Keep
   the offer ICE-candidate-free host-only (no STUN/TURN gathering,
   `iceTransportPolicy` limited to local candidates) so the payload fits
   comfortably in one QR code; STUN/TURN aren't needed anyway for two
   devices on the same local link. Clunkier UX (two camera scans) —
   why it's the second build, not the first.

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
- **Host → guest, every rendered frame (or throttled, e.g. 30Hz)**:
  `{ t: 'state', ballX, ballY, ballVx, ballVy, lpadY, rpadY, scoreL,
  scoreR }`. Same unreliable/unordered channel — a skipped state frame is
  invisible once the next one arrives.
- **Either direction, once**: `{ t: 'bye' }` on an intentional quit, so
  the other side can show "opponent left" instead of just timing out.

### Where this plugs into the existing code

This is the part worth doing early, because it's the part that proves
the architecture doesn't need a rewrite of Rally itself:

- **Guest input in → the game, unchanged.** `blip_controller.js`'s
  synthetic-KeyboardEvent pipeline is already how the on-screen dial and
  physical keyboard both reach the WASM side (`is_key_down`/
  `is_key_pressed` in `crates/blip/src/input.rs` never know or care where
  the KeyboardEvent came from). A new tiny module (`blip_net.js`, say)
  that dispatches the *same* synthetic
  `keydown`/`keyup` for `I`/`K` (rally's P2 keys) whenever a `{t:'input'}`
  message arrives needs **no Rust changes at all** on the guest-acts-as-
  host-input side, or on a host that's just running normal local
  `Mode::TwoPlayer`.
- **Host state out → new FFI, unavoidable.** Nothing today exports
  per-frame game state out of the WASM sandbox — `web.rs`'s existing
  exports (`blip_high_score`, `blip_game_over`, …) are all one-shot
  events, not a per-frame stream. This needs a new export, e.g.
  `blip_net_state(ptr, cap) -> len` that Rally's own `update_play`/
  `draw_play` writes a small fixed struct into (ball/paddle/scores as
  `f32`/`i32`), read from JS once a frame and forwarded over the
  DataChannel when this device is hosting. Scoped to `rally` only for
  v1 — don't generalize the struct shape until a second game needs it.
- **Guest render-only mode → small new `Game` branch in `rally/main.rs`.**
  A `Mode::NetGuest` (alongside the existing `OnePlayer`/`TwoPlayer`) that
  skips `update_play`'s simulation entirely and instead copies the latest
  net state (received via a new `blip_net_apply_state(ptr, len)` import,
  the mirror of the export above) straight into `Game`'s fields before
  drawing. `draw_play` doesn't change at all — it already just reads
  `Game`'s fields.
- **Title screen → a third option.** Today `update_title` offers 1P
  (press your own dial) or 2P (press "2"/spin P2's dial). Add a "PLAY
  ONLINE" (or "PLAY NEARBY") entry that hands off to the JS pairing UI
  (room code or QR) before `start_game()` ever runs, then calls into a
  `blip_set_net_role(host: bool)`-style import so `Game` knows which of
  the three modes it's in for the rest of the match.

## Phasing

1. **Signaling + connection only, no gameplay.** Two browser tabs (or
   phones) exchange a room code via Supabase Realtime, establish an
   `RTCPeerConnection` + DataChannel, and ping-pong a counter over it.
   Prove this works over WiFi, then over a Bluetooth-tethered pair, before
   touching Rally at all. This is the riskiest, least-code-reused part —
   isolate it first.
2. **One-way state stream.** Wire the `blip_net_state` export on a host
   running normal local `Mode::TwoPlayer`, stream it to a guest tab that
   just logs it — confirms the FFI shape and frame rate before any
   rendering is on the line.
3. **Guest rendering.** Add `Mode::NetGuest`, point `draw_play` at the
   received state, confirm a spectator-mode guest tracks a host's local
   match smoothly.
4. **Guest input back to the host.** Wire `blip_net.js`'s synthetic
   KeyboardEvents from the guest's real touch/keyboard input, confirm a
   full remote match plays end-to-end.
5. **UX pass.** Title-screen entry point, room-code / QR pairing screens,
   "opponent disconnected" handling, a rematch button that re-uses the
   existing connection instead of re-pairing.
6. **(Optional) QR-code signaling** as the offline alternative to
   Supabase Realtime, once the above is solid.

## Open questions

- **NAT/TURN.** Two phones on the same WiFi or Bluetooth PAN connect
  directly (host candidates, no relay needed). Two phones on *different*
  networks (one's on cellular, playing "remotely") need STUN at minimum,
  likely TURN behind carrier-grade NAT — a TURN relay is a real
  (small-dollar) hosting cost and pulls game traffic through a server
  again, which cuts against the "no server needed" pitch. Decide whether
  same-room-only is an acceptable v1 scope limit (recommended) before
  building for the general case.
- **Cheating.** A guest's browser console can just send fabricated
  `{t:'input'}` messages. Fine for a casual arcade cabinet game between
  two people in the same room; would need host-side input sanity checks
  (e.g. reject an input rate above what's physically possible) if this
  ever mattered more.
- **iOS Safari.** WebRTC is supported; Bluetooth tethering as a
  *peripheral* (the phone being tethered *to*) has historically been
  spottier on iOS than Android. Test the actual PAN link on real
  hardware before promising it works everywhere — the WiFi path is the
  fallback either way.

## Related

[High scores via Supabase](highscores.md) — the Realtime channel this
plan proposes reusing for signaling lives on the same project.
