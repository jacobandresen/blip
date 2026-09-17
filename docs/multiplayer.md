# Two-device multiplayer

Status: **shipped** (Rally only). Multiplayer uses QR-code signaling and a
direct WebRTC DataChannel. It needs no account, signaling service, or
ongoing internet connection once the match starts.

## Scope

Two players use devices on the same WiFi network. One device is the host and
authoritatively simulates Rally; the other is the guest and renders the
host's state while sending its paddle input back. There is no TURN relay,
reconnect, spectator mode, or support for more than two players.

## Pairing flow

The title screen's **PLAY NEARBY** button opens a small pairing modal:

1. Choose **HOST** or **JOIN**.
2. **HOST** shows an offer code. The other device scans it.
3. **JOIN** creates an answer code. The host scans it with **SCAN ANSWER**.
4. Both devices connect and start Rally.

JOIN opens the camera automatically. HOST only needs the extra scan action
because it must first leave its offer visible for the other device. The
modal shows one short status line for the current action or result:
`Creating code…`, `Point at the code.`, `Host code scanned`, `Contacting
client…`, `Connected`, or a short retry message. A green check confirms a
code was scanned; the host's amber status dot pulses while it contacts the
client. Camera permission, missing-camera, and invalid-code errors are stated
directly. **CLOSE** cancels the current attempt.

## Architecture

### Transport

`web/blip_net.js` creates a host-authoritative `RTCPeerConnection` with no
ICE servers. The host runs Rally's normal simulation and sends the current
game state every rendered frame. The guest sends input changes back and
renders the latest state; it does not run its own simulation.

The guest input path dispatches the same synthetic `keydown`/`keyup` events
used by the existing game controls, so the Rust game code does not need a
second input mechanism. The host and guest roles are exposed to Rally
through the small networking API used by the game FFI.

### Signaling

WebRTC requires an SDP offer, an SDP answer, and ICE candidates before its
DataChannel can open. QR codes carry those values directly:

1. `host()` gathers local host candidates and renders the offer.
2. `join()` validates the scanned offer, gathers the answer, and renders it.
3. `submitAnswer()` validates the scanned answer and completes the host setup.

`web/blip_sdp_slim.js` removes unnecessary candidate lines before encoding,
keeping the code readable by older phone cameras. Scanned SDP is normalized
and rejected when it is empty, malformed, or has no ICE candidates.

### Browser engines, mDNS, and the camera permission

Measured on an iPhone 13 (iOS 26.6) and on Playwright's headless WebKit:

| | candidates with `iceServers: []`, no media permission |
| --- | --- |
| Chromium | host candidates, raw LAN IPs |
| **real iOS Safari** | one host candidate in ~2s, as an mDNS `<uuid>.local` name |
| **headless WebKit** | **none at all; gathering never completes** |

WebKit hides the LAN address rather than handing it out. Where it can do
that privately it emits an mDNS `.local` candidate; where it cannot — a
build with no mDNS responder, such as headless WebKit — it emits nothing
instead of exposing a raw IP, and hosting then fails after the gather
timeout with no candidates. There is no STUN fallback, because blip
configures no ICE servers by design, and an unreachable STUN entry does
not substitute: it is the media grant WebKit gates on, not the presence
of a server.

So `host()` and `join()` take the camera permission before gathering
(`warmUpIceMedia()` in `web/blip_net.js`), stopping the stream the
instant it arrives. This costs the player nothing — both pairing roles
open the camera moments later anyway to scan a code, so it only moves
the prompt earlier — and it is a no-op wherever `mediaDevices` is
absent. A refusal is not fatal on Chromium or on real iOS Safari; on
headless WebKit it often stops being fatal too, once the request has
been made at all, which is odd but consistently observed. Where it does
remain fatal the modal says `Allow camera access to connect.` rather
than a generic failure — a defensive path, not a common one. What the
test suite pins is the property that failed originally: hosting must
reach a legible terminal state rather than sit on `Creating code…` until
the connect timeout.

Because the peers can exchange `.local` candidates, both devices must be
able to resolve mDNS on the shared network. This is the same class of
requirement as the client/AP isolation note below.

### iOS needs HTTPS for the camera

iOS Safari exposes `navigator.mediaDevices` **only in a secure context**.
Served over plain HTTP at a LAN address, `navigator.mediaDevices` is
`undefined` on an iPhone — not "permission denied", absent — so the QR
camera scan cannot run at all, and PLAY NEARBY cannot be completed by
hand. Pairing with a real iPhone therefore requires the site to be
served over HTTPS. (`localhost` also counts as secure, but a phone
cannot reach the development machine's localhost, and iOS has no reverse
port-forwarding equivalent to `adb reverse`.)

The automated iOS test is unaffected, because it injects the scanned
payload rather than photographing it — which is worth keeping in mind:
a green `test:multiplayer:ios` over plain HTTP does **not** demonstrate
that a human could scan the code on that same server. The same
limitation is why the run cannot hold a Screen Wake Lock on the phone;
`navigator.wakeLock` is secure-context-only too.

The QR/camera implementation is in `web/blip_qr.js`; it adds a quiet zone
around rendered codes, uses a centered square camera crop, and reports only
the current scan state to the pairing UI. The camera preview includes a
lightweight framing overlay but does not expose candidate dumps or debug
telemetry to players.

### Wire format

The browser protocol in `web/blip_net_proto.js` uses a small tagged payload:

- Guest to host: `{t: 'input', up: bool, down: bool}`.
- Host to guest: a fixed-size binary `NetState` packet containing phase,
  ball position/velocity, paddle positions, and scores.
- Ping/pong remains available as a developer diagnostic, but is not part of
  the player flow.

## Code locations

| Concern | Location |
| --- | --- |
| Pairing modal and status messages | `web/blip_net_ui.js` |
| WebRTC lifecycle and input transport | `web/blip_net.js` |
| QR rendering and camera scanning | `web/blip_qr.js` |
| Scan-region heuristic | `web/blip_qr_heuristic.js` |
| Candidate trimming and parsing | `web/blip_sdp_slim.js` |
| Browser wire protocol | `web/blip_net_proto.js` |
| Rust game-state packet | `crates/rally/src/net.rs` |
| Pairing styles | `web/shell.css` |
| QR exchange, as one reusable routine | `test/lib/pairing.mjs` |
| CDP-shaped adapters (Playwright engines, safaridriver) | `test/lib/engine.mjs`, `test/lib/safari-webdriver.mjs` |
| Real-iPhone bridge and wake lock | `test/lib/ios-device.mjs`, `test/lib/ios-wakelock.mjs` |
| Untrusted-peer limits (sizes, ids) | `web/blip_net_proto.js` |
| Direction rules, send guard, pong budget | `web/blip_net.js` |
| Host-state validation and clamping | `crates/rally/src/net.rs` (`sanitize_state`) |
| Blocked-network detection | `web/blip_net.js` (`neverConnected`), `web/blip_net_ui.js` |

## Testing

Run the browser-independent tests with:

```sh
npm test
```

They cover the wire protocol, QR scan-region heuristic, SDP candidate
trimming, and canvas geometry. Rust networking tests are run with:

```sh
cargo test -p rally
```

The pairing harness is:

```sh
npm run test:multiplayer
```

By default, `test/multiplayer-pairing.mjs` launches two Chromium instances and
injects the exact payload rendered into each QR code into the other side's
test-only scan hook. This avoids requiring a physical or fake camera while still exercising the
pairing UI, SDP validation, WebRTC connection, and game protocol. It requires
Chromium and a built `web/rally/index.wasm`.

To run the additional real fake-camera/jsQR path, use:

```sh
BLIP_MULTIPLAYER_REAL_CAMERA=1 npm run test:multiplayer
```

That mode renders each QR into a video file and feeds it through
`getUserMedia`; it requires `ffmpeg`. A camera failure in this optional mode
does not invalidate the camera-free signaling test.

### Engine coverage

`test/multiplayer-pairing.mjs` runs Chromium on both sides. The second device in
this feature is usually a phone, and on iOS every browser is WebKit, so
that half is covered separately by Playwright's WebKit:

```sh
npm run test:multiplayer:webkit   # WebKit <-> WebKit
npm run test:multiplayer:cross    # Chromium <-> WebKit, both directions
```

`BLIP_HOST_ENGINE` / `BLIP_GUEST_ENGINE` (`chromium`, `webkit`,
`firefox`) select the pairing. These need no machine-level setup:
Playwright's browsers are a devDependency, and the Chromium-based tests
fall back to Playwright's Chromium when none is on `PATH`, so the suite
runs on macOS as well as on a Linux CI box.

Headless Chrome is launched with SwiftShader rather than `--disable-gpu`.
Rally is a WebGL game; with no GL context the wasm never starts, and
every state-sync assertion fails against a game that was never running.

### Hardening, QR and blocked-network suites

```sh
npm run test:multiplayer:hardening   # a hostile peer on a live connection
npm run test:multiplayer:blocked     # what the player sees when traffic is blocked
npm run test:qr                      # can the code on screen actually be scanned
npm run test:multiplayer:preflight   # the loopback probe and the subnet check
npm run test:multiplayer:visibility  # scanning feedback and the live ICE list
```

`test/multiplayer-proto.test.mjs` covers the wire format in isolation (no
browser). `test/multiplayer-hardening.mjs` covers the rules that need a
live connection to mean anything — direction, size caps, the pong budget
— and every check ends by confirming the connection still carries a real
match, because hardening that quietly broke the channel would be worse
than the hole it closed.

`test/qr-scannability.mjs` screenshots the canvas **through the real
compositor** and decodes those pixels, across four viewports and three
payload sizes. It also keeps the defect itself as an executable
demonstration: one test asserts that the old fixed-grid-plus-CSS-scale
approach still loses codes, and reports the percentage it loses.

Both take `BLIP_HOST_ENGINE`/`BLIP_GUEST_ENGINE` (and `BLIP_QR_ENGINE`),
so every combination of Chromium and WebKit is covered in both roles.

### On a real iPhone

```sh
npm run test:multiplayer:ios      # Mac hosts in WebKit, the phone joins over WiFi
```

The one test where the guest is real hardware — real iOS Safari, real
WebRTC, real WiFi between the peers. It skips (rather than fails) when
no phone is attached. Setup:

- iPhone attached over USB and trusted by this Mac.
- Settings > Apps > Safari > Advanced > **Web Inspector** and **Remote
  Automation**, both on.
- `uv tool install pymobiledevice3` — the bridge is started automatically.

iOS has no adb and no CDP; the equivalent is the WebKit remote
inspector, which since iOS 17 sits behind Apple's RemoteXPC tunnel.
`pymobiledevice3 webinspector cdp` re-exposes it as an ordinary CDP
endpoint on localhost, so `test/lib/cdp.mjs` drives a phone with the
same `connect()`/`evaluate()` it uses for desktop Chrome.
(`ios_webkit_debug_proxy` no longer works for this — it fails during the
TLS handshake and reports no targets.)

A sleeping phone keeps listing its inspector target long after it stops
answering on it, so the harness takes a Screen Wake Lock on the page as
its first step (`test/lib/ios-wakelock.mjs`). That keeps a managed
device awake without needing Auto-Lock set to Never, which an MDM
profile may forbid.

`BLIP_IOS_HOST=safari` drives the real Safari.app as host via
`safaridriver` instead of Playwright's WebKit; that additionally needs
`sudo safaridriver --enable` and Develop > Allow Remote Automation.

The Android harness provides an additional real Chrome-for-Android path when
Linux, `/dev/kvm`, the Android SDK, and an emulator are available:

```sh
npm run test:multiplayer:android
```

Automated tests cannot reproduce every real-phone camera condition (focus,
glare, permissions, or QR moiré), nor WiFi access-point isolation. Those
remain hardware/network checks outside the browser harness.

## Hardening

The peer on the other end of the DataChannel is not trusted. Pairing is a
QR code anyone in the room can photograph, and nothing authenticates the
far side once the channel is open, so every inbound message is untrusted
input. Three layers enforce that.

### Wire format (`web/blip_net_proto.js`)

| Limit | Value | Why |
| --- | --- | --- |
| `MAX_PACKET_CHARS` | 4096 | Checked **before** `JSON.parse`, whose cost scales with input size and which blocks the thread running the game loop. Real packets are ~50 characters. |
| `MAX_ID_CHARS` | 64 | A ping obliges a pong that echoes the id back. Unbounded, that is an amplifier. |

Decoders also reject non-strings, arrays, and unknown protocol versions,
and never throw whatever the payload. `__proto__` in a packet cannot
reach `Object.prototype` — `JSON.parse` makes it an ordinary own
property and nothing here spreads or merges the parsed object. That is
pinned by a test, because the day someone rewrites a decoder as an object
spread is the day it stops being true.

### Direction is part of the protocol

Input is guest to host; state is host to guest. Both are enforced on
receipt, not merely by convention:

- A **host ignores inbound state.** It simulates the match; storing a
  peer's idea of it would also be an unbounded per-message allocation on
  a side that never reads it.
- A **guest ignores inbound input.** This one matters more than it looks:
  input is turned into real synthetic key events, and the guest's own
  input capture forwards those back to the host — so honouring it would
  let a malicious host puppet the guest into playing against itself.

Binary frames are type-checked (`instanceof ArrayBuffer`), not just
size-checked. `binaryType = 'arraybuffer'` is a request about how *we*
decode frames, not a guarantee about what arrives; a `Blob` has no
numeric `byteLength`, so a size check alone passes it through into a
`Uint8Array` that silently comes out empty.

Pongs are rate limited (`MAX_PONGS_PER_SEC`, 10/s) and dropped rather
than queued once the budget is spent — a late pong is useless to a real
pinger, and a backlog is what a flooder wants.

### Sending: `MAX_SEND_BYTES`

A guard against ourselves rather than a peer. **Measured on WebKit:
sending a 1MB frame closes the sender's own DataChannel, silently.**
`send()` returns normally, nothing throws, and the channel is gone a
moment later; 256KB is fine. No catch block can recover from that,
because there is no exception — the only defence is not to make the call,
so every send goes through one `safeSend()` that refuses anything over
64KB. Nothing blip sends comes near it (a state packet is 36 bytes); the
cap exists so a future field or a wrong buffer degrades into a dropped
message instead of a connection that dies invisibly.

### Believing the host (`crates/rally/src/net.rs`)

`unpack_state` decodes any 36 bytes, bit patterns included — a torn
packet must not panic, and the decoder is not the place for policy.
`sanitize_state` is that policy, and the guest applies it before copying
anything into its `Game`:

- **Rejected** (keep the last good frame): any non-finite float, a phase
  that is not one of the four sent on the wire, a score outside the
  possible range. `NaN` is the dangerous one — every `<`/`>` comparison
  against it is false, so collision and scoring logic silently stop
  firing rather than failing loudly.
- **Clamped**: positions and velocities slightly outside the field. A
  real host reports a ball past the edge on the frame it scores, so
  rejecting those would drop good packets and stutter the view.

A 20,000-iteration fuzz over random bytes asserts the end-to-end
property: whatever arrives, the guest gets a usable state or nothing —
never a panic, never a `NaN` it will go on to compare against.

## QR codes: why they failed to scan

The single worst defect found in this feature, and the least visible.

The code was drawn on a fixed ~400px grid and then given a CSS size the
browser had to resample it to. **Resampling a QR by a fractional factor
merges and drops whole module rows.** Swept across the plausible
on-screen size range, **51–78% of display sizes failed to decode** —
including the 280px default the pairing modal shipped with.

The failure has no gradient and no symptom. The code looks perfectly
fine to a human; it decodes or it does not, depending on how the ratio
happens to land for that particular payload length. A developer testing
one payload on one screen can see it work every time and ship it, while
users with a different-length SDP find the camera "broken".

It had gone unnoticed because every existing test read the payload from
`BlipQR.lastRenderedText` or decoded the canvas via `toDataURL()` — both
bypass the browser's rendering entirely. **No test had ever looked at the
pixels a player's phone would actually photograph.**

The fix: `render()` takes the box it has to fit, picks a cell size in
whole **device** pixels, and sets an explicit CSS size of exactly
bitmap ÷ devicePixelRatio. One module is a whole number of device pixels
and the browser resamples nothing. With exact rendering a code decodes
from a pristine bitmap down to one pixel per module, so the grid — not
the size — was always the problem.

The box is measured from the container rather than assumed, because the
code can only grow in whole modules: a 145-module code in a 280px box
must choose between 145px and 290px, and that is the difference between
unscannable and comfortable. Only width constrains it — the panel scrolls
vertically, so a code taller than a short landscape window costs a
scroll, while one shrunk to fit costs the ability to scan at all.

### When it cannot be made scannable

Below `MIN_CSS_PX_PER_MODULE` (2 CSS px/module) the code is drawn but
reported as unscannable, and the modal says so beneath it:

> This code is too small to scan reliably — make the window taller, or
> show it on a larger screen.

Amber, not red: the attempt has not failed, and the code may still scan.
But it is almost certainly why nothing is happening, and without the line
the player has no way to tell that from a bad camera. The warning clears
again once there is room — a stale warning is its own bug.

## Telling the player what is happening

Three of this feature's failures used to be indistinguishable from it
working, which is the worst property a failure can have. Each now shows
its working.

### Before anything is scanned: the loopback probe

`BlipNet.preflight()` connects two `RTCPeerConnection`s inside the page
and sends one DataChannel message between them. The traffic never leaves
the machine, so it is expected to succeed anywhere WebRTC functions at
all; a failure means the problem is local and total — disabled by policy
or an extension, a build without SCTP, or an engine that yields no ICE
candidates. It runs alongside the real attempt from the moment HOST or
JOIN is pressed, and reports in about 70ms, so a device that cannot do
this at all says so before the player points a camera at anything:

> WebRTC is blocked on this device. Check browser or policy settings.

Cached per page load — the answer cannot change, and the probe is not
free. It also collects this device's own addresses, which is what makes
the next check possible.

### On scanning: are these devices even on the same network?

The guest is the only side holding both sets of candidates, so it is the
only one that can notice the two devices are nowhere near each other. If
no candidate in the scanned offer shares a /24 with any local address,
it says so **immediately** — measured at ~300ms, against a 60-second
connect timeout:

> These devices look like they are on different networks — put both on
> the same WiFi, without guest mode, a hotspot or a VPN.

Deliberately a warning, not a failure: two devices on different /24s can
still route to each other, so a mismatch is suggestive rather than
conclusive. It also stays quiet unless *both* sides published plain IPv4
host candidates — mDNS `.local` candidates carry no address to compare,
and guessing would be worse than silence. It is rendered as a persistent
line rather than a status update, because the status line is overwritten
about a second later by "✓ Host code scanned".

### While scanning: the camera is working, and here is what to try

`blip_qr.js` reports progress on every animation frame — how long it has
been looking, and whether anything QR-shaped is in view. None of it used
to reach the screen: the line read `Point at the code.` and then never
changed, so a scan running perfectly looked exactly like one that had
died. The natural conclusion is that the code is not being picked up,
usually while holding the phone too far away for the modules to resolve
— the one thing the player could have fixed.

Now the indicator pulses (the same `active` state the host uses) and the
advice escalates, because the first seconds of a normal scan need none:

| | |
| --- | --- |
| a code is in view | `Code in view — hold steady…` |
| under 6s | `Looking for a code…` |
| 6s | `Looking — fill the frame with the code.` |
| 15s | `Still looking — move closer, or add light.` |

### While connecting: the candidate pairs, in colour

Every candidate pair the two devices are testing is listed live, one row
per remote address, colour-coded:

- **green** — being attempted, or already carrying traffic
- **red** — ruled out; nothing got through this path
- **dim** — queued, not yet tried

Rows collapse by remote address rather than showing one per local
interface, since a machine with two interfaces otherwise lists the same
address twice, which reads as a bug. The most advanced state any path to
that address reached is the one shown.

This is also the fastest diagnosis available for the two common network
faults: every row red means something is dropping device-to-device
traffic, and no rows at all means the two sides never exchanged usable
candidates.

## When the network blocks the traffic

The failure that looks most like success. Both codes scan, both sides
report progress, and the SDP exchange completes perfectly — because it
happened through a camera, not the network. Then nothing. The usual
causes are not bugs: guest or corporate WiFi with client isolation (the
access point refuses to pass traffic between its own clients), or a VPN
routing LAN traffic elsewhere.

Previously this produced sixty seconds of `Contacting client…` followed
by `Could not connect. Try again.` — which invites the one action that
cannot possibly help.

Now:

1. After 8s in ICE `checking`, while the player is still watching:
   `Still connecting… if this network blocks device-to-device traffic, it
   won't.` A hint, not a verdict — the connection may still complete.
2. On a terminal failure: `Blocked by this network. Try the same WiFi,
   without guest mode or a VPN.`

Detecting it needs care, because engines disagree about how a dead path
is reported. **Chromium never uses ICE's own `failed` state here** — it
takes `iceConnectionState` to `disconnected` and `connectionState` to
`failed`. Keying off ICE `failed` alone misses the common case entirely.
What is unambiguous is the combination: the DataChannel never opened
(`role` is still 0) and ICE never reached a connected state.

The message also dwells for 6s instead of the usual 1.2s before the modal
returns to the choice screen. It is a sentence asking the player to go
change something about their WiFi, and it is the only place that
explanation appears.

## Reliability notes

- Each new `host()` or `join()` cancels the previous attempt.
- Promise and DataChannel callbacks ignore stale attempts.
- Stopping a scan prevents a late decode or camera error from changing a
  closed modal.
- QR rendering and camera errors are surfaced as short UI messages rather
  than silent failures.
- The service worker cache is versioned in `web/sw.js`; bump it when changing
  the shipped pairing assets.

## Open questions

- A rematch could reuse an open DataChannel instead of requiring a new QR
  exchange.
- A guest can fabricate input from its browser console; input validation would
  only matter for competitive play.

## Related

[High scores via Supabase](highscores.md) is separate. Multiplayer no longer
depends on Supabase.
