#!/usr/bin/env bash
# Provisions two Android emulators for manually exercising the two-device
# Rally multiplayer flow (docs/multiplayer.md) on real Chrome for Android,
# side by side, on one machine. This is the blip-specific glue; all the
# generic "install SDK / boot an AVD / dismiss Chrome onboarding" logic
# lives in scripts/lib/android-emulator.sh, which has no blip-specific
# code and can be reused as-is by other projects.
#
# What this script does:
#   1. Installs the Android SDK/emulator/system-image if missing.
#   2. Creates two AVDs — "blip_host" and "blip_guest" — if missing.
#   3. Boots both emulators headless and waits for both to finish booting.
#   4. Skips the Android + Chrome first-run wizards on both, then opens
#      Rally in Chrome (requires a local web server already serving web/
#      on BLIP_WEB_PORT — see the printed next steps if not).
#   5. Prints the adb device IDs and next manual steps.
#
# Why the actual QR pairing isn't scripted here: the desktop e2e test
# (test/multiplayer.mjs) fakes a camera via Chromium's
# --use-fake-device-for-media-stream/--use-file-for-fake-video-capture.
# The Android emulator's camera pipeline (-camera-back/-camera-front)
# only supports a real webcam, a synthetic 3D "virtual scene", or "none" —
# there's no equivalent "play this video file into getUserMedia" hook, so
# QR-code scanning between two emulator instances can't be scripted the
# same way. This script gets both devices up, booted, and running Rally
# in Chrome so a human (or a future adb-input-driven script) can drive
# the actual JACK IN / HOST / JOIN / QR-scan flow, e.g. by pointing one
# emulator's camera at the other's screen, or via -camera-back webcam0
# if you have physical webcams to hand.
#
# Usage:
#   ./scripts/setup-android-multiplayer-test.sh             # set up + boot
#   ./scripts/setup-android-multiplayer-test.sh --teardown  # kill both AVDs
#
# Env overrides:
#   ANDROID_SDK_ROOT   where the SDK lives (default: $HOME/Android/Sdk)
#   API_LEVEL          system image API level (default: 34)
#   BLIP_WEB_PORT      port the local web server is reachable on from the
#                      host machine (default: 8080)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/android-emulator.sh
source "${SCRIPT_DIR}/lib/android-emulator.sh"

BLIP_WEB_PORT="${BLIP_WEB_PORT:-8080}"
HOST_AVD="blip_host"
GUEST_AVD="blip_guest"
HOST_PORT=5554   # -> adb serial emulator-5554
GUEST_PORT=5556  # -> adb serial emulator-5556

log() { printf '\033[1;36m==>\033[0m %s\n' "$1"; }

if [[ "${1:-}" == "--teardown" ]]; then
  log "Killing blip_host/blip_guest emulators"
  android_emulator_kill "emulator-${HOST_PORT}"
  android_emulator_kill "emulator-${GUEST_PORT}"
  exit 0
fi

android_emulator_ensure_sdk

android_emulator_create_avd_if_missing "$HOST_AVD"
android_emulator_create_avd_if_missing "$GUEST_AVD"

mkdir -p /tmp/blip-avd-logs
android_emulator_boot "$HOST_AVD" "$HOST_PORT" /tmp/blip-avd-logs/host.log >/dev/null
android_emulator_boot "$GUEST_AVD" "$GUEST_PORT" /tmp/blip-avd-logs/guest.log >/dev/null

android_emulator_wait_for_boot "emulator-${HOST_PORT}"
android_emulator_wait_for_boot "emulator-${GUEST_PORT}"

android_emulator_skip_setup_wizard "emulator-${HOST_PORT}"
android_emulator_skip_setup_wizard "emulator-${GUEST_PORT}"

# Rally's pairing itself never needs the network layer (QR-only
# signaling, see docs/multiplayer.md) — this port-forward only exists so
# each emulator's Chrome can load the game from the host machine's dev
# web server.
android_emulator_reverse_port "emulator-${HOST_PORT}" "$BLIP_WEB_PORT"
android_emulator_reverse_port "emulator-${GUEST_PORT}" "$BLIP_WEB_PORT"

RALLY_URL="http://127.0.0.1:${BLIP_WEB_PORT}/rally/index.html"

log "Both emulators are up:"
"$(android_emulator_adb)" devices

if curl -fsS "$RALLY_URL" -o /dev/null 2>/dev/null; then
  log "Loading Rally in Chrome and dismissing onboarding on both emulators"
  android_emulator_open_url_dismiss_onboarding "emulator-${HOST_PORT}" "$RALLY_URL"
  android_emulator_open_url_dismiss_onboarding "emulator-${GUEST_PORT}" "$RALLY_URL"

  cat <<EOF

Next steps:
  Rally is loaded in Chrome on both emulators. Tap JACK IN -> HOST on
  emulator-${HOST_PORT}, JACK IN -> JOIN on emulator-${GUEST_PORT}, then
  hold the guest's screen mirror (scrcpy, or a real webcam pointed
  -camera-back webcam0) up to the host's QR code, and vice versa for the
  answer — see the note at the top of this script for why the QR-scan
  step itself isn't scripted.
EOF
else
  cat <<EOF

Next steps:
  1. Serve the web build, then re-run this script to load + dismiss
     Chrome's onboarding on both devices:
       npx http-server web -p ${BLIP_WEB_PORT}
     (or: python3 -m http.server ${BLIP_WEB_PORT} --directory web)
  2. Re-run: ./scripts/setup-android-multiplayer-test.sh
EOF
fi

cat <<EOF

Tear down both emulators with:
  ./scripts/setup-android-multiplayer-test.sh --teardown
EOF
