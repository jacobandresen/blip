# Android multiplayer test

Runs Rally PLAY NEARBY between two independent Android 34 emulators running
Chrome-for-Android. QR scanning is replaced by `window.BlipQR.testInject`;
WebRTC, ICE, DataChannel, and game state synchronization are real.

## Run

From the repository root:

```sh
npm run test:multiplayer:android
```

Requirements: Linux, `/dev/kvm`, Android SDK/emulator, and enough disk space
for the Android 34 image. The test provisions or reuses `blip_host` and
`blip_guest`, boots `emulator-5554` and `emulator-5556`, serves Rally on port
8099, forwards Chrome DevTools on 9541/9542, and stops both emulators.
Without `/dev/kvm`, it skips.

## Expected result

Seven passing tests:

1. Host offer QR.
2. Guest answer QR.
3. DataChannel roles.
4. Match start.
5. Guest input/state synchronization.
6. Clean disconnect.

The outer test plus six subtests produce seven total passing tests. Rally
enters `Serve` after pairing, so the test sends the host launch action before
checking movement.

## Failure triage

Use the first failing subtest:

| Failure | Meaning |
|---|---|
| Offer or answer QR | Page startup or signaling |
| DataChannel roles | ICE/WebRTC or SDP exchange |
| Match start | Android input or game startup |
| Input/state sync | DataChannel application protocol |
| Disconnect | Cleanup or connection teardown |
