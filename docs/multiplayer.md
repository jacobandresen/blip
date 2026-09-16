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

By default, `test/multiplayer.mjs` launches two Chromium instances and
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

`test/multiplayer.mjs` runs Chromium on both sides. The second device in
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
