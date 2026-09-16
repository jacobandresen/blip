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
`Creating code…`, `Point at the code.`, `Connecting…`, `Connected`, or a
short retry message. Camera permission, missing-camera, and invalid-code
errors are stated directly. **CLOSE** cancels the current attempt.

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
