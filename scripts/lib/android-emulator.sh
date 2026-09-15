#!/usr/bin/env bash
#
# 2026, Jacob Andresen <jacob.andresen@gmail.com>
#
# Generic, project-agnostic helpers for provisioning and driving headless
# Android emulators (AVDs) from a script — install the SDK if missing,
# create/boot AVDs, skip the Android + Chrome first-run wizards, and open
# a URL in Chrome. Nothing in this file mentions blip; copy it wholesale
# into another project.
#
# Can be used two ways:
#   1. Sourced as a function library:
#        source scripts/lib/android-emulator.sh
#        android_emulator_ensure_sdk
#        android_emulator_create_avd_if_missing my_avd
#        android_emulator_boot my_avd 5554 /tmp/my_avd.log
#        android_emulator_wait_for_boot emulator-5554
#        android_emulator_skip_setup_wizard emulator-5554
#        android_emulator_open_url_dismiss_onboarding emulator-5554 http://127.0.0.1:8080
#
#   2. Run directly as a CLI for ad hoc use:
#        ./scripts/lib/android-emulator.sh ensure-sdk
#        ./scripts/lib/android-emulator.sh create-avd my_avd [api-level]
#        ./scripts/lib/android-emulator.sh boot my_avd 5554
#        ./scripts/lib/android-emulator.sh wait-for-boot emulator-5554
#        ./scripts/lib/android-emulator.sh open emulator-5554 https://example.com
#        ./scripts/lib/android-emulator.sh kill emulator-5554
#
# Env overrides:
#   ANDROID_SDK_ROOT   where the SDK lives (default: $HOME/Android/Sdk)
#   API_LEVEL          system image API level (default: 34)
#   AVD_DEVICE_PROFILE hardware profile for new AVDs (default: pixel_6)

set -uo pipefail

ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}"
API_LEVEL="${API_LEVEL:-34}"
AVD_DEVICE_PROFILE="${AVD_DEVICE_PROFILE:-pixel_6}"
_CMDLINE_TOOLS_URL="https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip"

export ANDROID_SDK_ROOT
export ANDROID_HOME="$ANDROID_SDK_ROOT"

_ae_log() { printf '\033[1;36m==>\033[0m %s\n' "$1"; }
_ae_warn() { printf '\033[1;33m!!\033[0m %s\n' "$1"; }

android_emulator_sdkmanager() { echo "${ANDROID_SDK_ROOT}/cmdline-tools/latest/bin/sdkmanager"; }
android_emulator_avdmanager() { echo "${ANDROID_SDK_ROOT}/cmdline-tools/latest/bin/avdmanager"; }
android_emulator_bin() { echo "${ANDROID_SDK_ROOT}/emulator/emulator"; }
android_emulator_adb() { echo "${ANDROID_SDK_ROOT}/platform-tools/adb"; }
android_emulator_start_adb() {
  local adb
  adb="$(android_emulator_adb)"
  "$adb" start-server >/dev/null
}

# Installs the Android command-line tools (if `sdkmanager` isn't already
# present), then platform-tools, the emulator package, and a system image.
# Safe to call repeatedly — sdkmanager --install skips anything already
# installed.
android_emulator_ensure_sdk() {
  local system_image="system-images;android-${API_LEVEL};google_apis_playstore;x86_64"
  local sdkmanager
  sdkmanager="$(android_emulator_sdkmanager)"

  if [[ ! -x "$sdkmanager" ]]; then
    _ae_log "sdkmanager not found under \$ANDROID_SDK_ROOT ($ANDROID_SDK_ROOT) — fetching command-line tools"
    command -v curl >/dev/null || {
      echo "curl is required" >&2
      return 1
    }
    command -v unzip >/dev/null || {
      echo "unzip is required" >&2
      return 1
    }
    command -v java >/dev/null || {
      echo "a JDK (java) is required for sdkmanager" >&2
      return 1
    }

    mkdir -p "$ANDROID_SDK_ROOT/cmdline-tools"
    local tmpzip
    tmpzip="$(mktemp -d)/cmdline-tools.zip"
    curl -fSL "$_CMDLINE_TOOLS_URL" -o "$tmpzip" || return 1
    rm -rf "$ANDROID_SDK_ROOT/cmdline-tools/latest"
    unzip -q "$tmpzip" -d "$ANDROID_SDK_ROOT/cmdline-tools"
    # the zip extracts to cmdline-tools/cmdline-tools — sdkmanager expects .../cmdline-tools/latest
    mv "$ANDROID_SDK_ROOT/cmdline-tools/cmdline-tools" "$ANDROID_SDK_ROOT/cmdline-tools/latest"
    rm -rf "$(dirname "$tmpzip")"
    _ae_log "Installed command-line tools to $ANDROID_SDK_ROOT/cmdline-tools/latest"
  fi

  _ae_log "Accepting SDK licenses"
  yes | "$sdkmanager" --licenses >/dev/null 2>&1 || true

  _ae_log "Installing platform-tools, emulator, and $system_image (skips anything already installed)"
  "$sdkmanager" --install "platform-tools" "emulator" "$system_image" >/dev/null

  if [[ ! -e /dev/kvm ]]; then
    _ae_warn "/dev/kvm not found — the emulator will fall back to slow software rendering. Enable virtualization in BIOS/host settings if this is unexpected."
  elif ! [[ -r /dev/kvm && -w /dev/kvm ]]; then
    _ae_warn "/dev/kvm exists but isn't read/write for $(whoami). Run: sudo usermod -aG kvm $(whoami)  (then log out/in) for hardware acceleration."
  fi
}

# android_emulator_create_avd_if_missing <name>
android_emulator_create_avd_if_missing() {
  local name="$1"
  local system_image="system-images;android-${API_LEVEL};google_apis_playstore;x86_64"
  local avdmanager
  avdmanager="$(android_emulator_avdmanager)"

  if "$avdmanager" list avd | grep -q "Name: ${name}$"; then
    _ae_log "AVD '$name' already exists"
  else
    _ae_log "Creating AVD '$name'"
    echo "no" | "$avdmanager" create avd \
      --name "$name" \
      --package "$system_image" \
      --device "$AVD_DEVICE_PROFILE" \
      --force
  fi
}

# android_emulator_boot <name> <port> [logfile] -- backgrounds the
# emulator process and prints its PID.
# android_emulator_boot <name> <port> [logfile] [camera-back mode]
#
# camera-back mode defaults to "none". Pass "imagefile:<path>" to have the
# emulator's back camera stream exactly that image file's contents on every
# fresh getUserMedia() call — no v4l2loopback/kernel module, no root, no
# host webcam required. See docs/android-multiplayer-test-plan.md.
#
# Per-instance resource caps: an AVD's own config.ini defaults to 2 vCPUs
# / 1536M RAM — fine for one instance, but two side by side on a 4-core
# host ask for every core with zero headroom left for the host OS, adb,
# this test rig's own scripts, or Chrome's own worker threads. That
# oversubscription measurably contributes to CDP calls hanging and the
# WebGL canvas-paint-freeze rig limitation under load. -cores/-memory
# below cap each instance at 1 vCPU / 1024M by default (so two together
# ask for 2 cores / 2GB, not 4 cores / 3GB+) — override per-run on a
# beefier host via BLIP_EMU_CORES/BLIP_EMU_MEMORY if the default is too
# conservative. Measured on a 4-core host: load average roughly halved
# (~15 -> ~8) with the cap in place. Not a full fix — QEMU/SwiftShader's
# own worker threads still scale off the host's core count regardless of
# the guest vCPU cap, so occasional CDP hangs remain possible; retry
# logic around a CDP connection attempt is the real safety net for that.
android_emulator_boot() {
  local name="$1" port="$2" logfile="${3:-/tmp/${name}-emulator.log}"
  local camera="${4:-none}"
  local emulator
  emulator="$(android_emulator_bin)"
  local cores="${BLIP_EMU_CORES:-1}"
  local memory="${BLIP_EMU_MEMORY:-1024}"
  _ae_log "Booting $name on emulator-$port (log: $logfile, camera-back: $camera, ${cores} core / ${memory}M)"
  "$emulator" -avd "$name" \
    -port "$port" \
    -no-window -no-audio -no-boot-anim -no-snapshot \
    -gpu swiftshader_indirect \
    -cores "$cores" -memory "$memory" \
    -camera-back "$camera" -camera-front none \
    >"$logfile" 2>&1 &
  echo $!
}

# android_emulator_wait_for_boot <serial> [max-tries (x2s)]
android_emulator_wait_for_boot() {
  local serial="$1" max_tries="${2:-180}" tries=0
  local adb
  adb="$(android_emulator_adb)"
  _ae_log "Waiting for $serial to finish booting (this can take a couple of minutes on first boot)"
  "$adb" -s "$serial" wait-for-device
  while true; do
    local booted
    booted=$("$adb" -s "$serial" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')
    [[ "$booted" == "1" ]] && break
    tries=$((tries + 1))
    if ((tries > max_tries)); then
      echo "Timed out waiting for $serial to boot" >&2
      return 1
    fi
    sleep 2
  done
  _ae_log "$serial is booted"
}

# android_emulator_skip_setup_wizard <serial> -- marks the device as
# already provisioned so it skips Android's own first-boot setup wizard
# (language/wifi/account walkthrough), which otherwise blocks every app
# including Chrome from being usable.
android_emulator_skip_setup_wizard() {
  local serial="$1"
  local adb
  adb="$(android_emulator_adb)"
  "$adb" -s "$serial" shell settings put global device_provisioned 1
  "$adb" -s "$serial" shell settings put secure user_setup_complete 1
}

# _android_emulator_tap_if_present <serial> <full-resource-id>
# Looks up a view by its fully-qualified resource-id (e.g.
# "com.android.chrome:id/negative_button" or
# "com.android.permissioncontroller:id/permission_allow_foreground_only_button")
# via uiautomator and taps its center if found. Returns 1 (no error
# message) if not present.
_android_emulator_tap_if_present() {
  local serial="$1" id="$2"
  local adb
  adb="$(android_emulator_adb)"
  "$adb" -s "$serial" shell uiautomator dump /sdcard/_ae_ui.xml >/dev/null 2>&1 || true
  local bounds
  bounds=$("$adb" -s "$serial" shell cat /sdcard/_ae_ui.xml 2>/dev/null |
    grep -o "resource-id=\"${id}\"[^>]*bounds=\"\[[0-9]*,[0-9]*\]\[[0-9]*,[0-9]*\]\"" |
    grep -o '\[[0-9]*,[0-9]*\]\[[0-9]*,[0-9]*\]' | head -1) || true
  [[ -z "$bounds" ]] && return 1
  local x1 y1 x2 y2
  IFS='[],' read -r _ x1 y1 _ x2 y2 _ <<<"$bounds"
  "$adb" -s "$serial" shell input tap $(((x1 + x2) / 2)) $(((y1 + y2) / 2))
  return 0
}

# android_emulator_open_url_dismiss_onboarding <serial> <url>
# Opens <url> in Chrome and clears Chrome's own one-time "sign in" and
# "turn on notifications" onboarding screens (unrelated to the Android
# setup wizard above), so subsequent automation lands on the actual page.
# Keyed off Chrome's resource-ids rather than screen coordinates, so it
# works across differently sized/DPI AVDs.
android_emulator_open_url_dismiss_onboarding() {
  local serial="$1" url="$2"
  local adb
  adb="$(android_emulator_adb)"
  "$adb" -s "$serial" shell am start -a android.intent.action.VIEW -d "$url" com.android.chrome >/dev/null
  sleep 3
  local dismissed_any=0
  for _ in $(seq 1 8); do
    local hit=0
    _android_emulator_tap_if_present "$serial" "com.android.chrome:id/signin_fre_dismiss_button" && hit=1
    _android_emulator_tap_if_present "$serial" "com.android.chrome:id/negative_button" && hit=1
    ((hit == 1)) && dismissed_any=1
    ((hit == 0)) && break
    sleep 2
  done
  # Re-navigate only if a dialog was actually dismissed — it may have
  # left Chrome on a blank tab. Skipping this in the common (no dialog)
  # case avoids piling up duplicate tabs on repeat calls, which matters
  # because CDP has to pick one of them (see test/lib/cdp.mjs's connect).
  if ((dismissed_any == 1)); then
    "$adb" -s "$serial" shell am start -a android.intent.action.VIEW -d "$url" com.android.chrome >/dev/null
    # Chrome needs a short transition after first-run activity dismissal
    # before the requested tab becomes debuggable.
    sleep 3
  fi
}

# android_emulator_reset_chrome <serial> -- stop Chrome before a fresh
# browser-driven test run. Keep Chrome's first-run state: clearing the app
# data here makes every run race the onboarding activity and DevTools startup.
android_emulator_reset_chrome() {
  local serial="$1"
  local adb
  adb="$(android_emulator_adb)"
  "$adb" -s "$serial" shell am force-stop com.android.chrome >/dev/null 2>&1 || true
  sleep 1
}

# android_emulator_grant_camera_permission <serial>
# Taps through Chrome's own "wants to use your camera" JS-visible prompt
# (positive_button = Allow) and, if it appears, the OS-level runtime
# permission dialog underneath it (permission_allow_foreground_only_button
# = "While using the app"). Call this right after triggering a
# getUserMedia() call for the first time on a given origin/device; a no-op
# (returns without tapping anything) if the permission was already granted
# or no prompt shows up within a couple of seconds.
android_emulator_grant_camera_permission() {
  local serial="$1"
  for _ in $(seq 1 8); do
    local hit=0
    _android_emulator_tap_if_present "$serial" "com.android.chrome:id/positive_button" && hit=1
    _android_emulator_tap_if_present "$serial" "com.android.permissioncontroller:id/permission_allow_foreground_only_button" && hit=1
    ((hit == 0)) && break
    sleep 1
  done
}

# android_emulator_reverse_port <serial> <port> -- forwards a TCP port
# from the host machine's loopback into the emulator's loopback, so pages
# served on the host (e.g. a local dev server) are reachable from inside
# the guest at the same 127.0.0.1:<port>.
android_emulator_reverse_port() {
  local serial="$1" port="$2"
  local adb
  adb="$(android_emulator_adb)"
  "$adb" -s "$serial" reverse "tcp:${port}" "tcp:${port}"
}

# android_emulator_forward_devtools <serial> <local-port>
#
# Exposes Chrome-for-Android's existing remote-debugging socket on
# 127.0.0.1:<local-port> on the host, with no extra Chrome flags needed —
# `chrome://inspect`-style debugging is on by default. Once forwarded,
# `http://127.0.0.1:<local-port>/json/list` behaves like desktop Chromium's
# CDP HTTP endpoint (same shape test/lib/cdp.mjs already speaks).
android_emulator_forward_devtools() {
  local serial="$1" local_port="$2"
  local adb
  adb="$(android_emulator_adb)"
  "$adb" -s "$serial" forward "tcp:${local_port}" localabstract:chrome_devtools_remote
}

# android_emulator_kill <serial>
android_emulator_kill() {
  local serial="$1"
  local adb
  adb="$(android_emulator_adb)"
  "$adb" -s "$serial" emu kill 2>/dev/null || true
}

# ---------------------------------------------------------------------------
# CLI entry point — only runs when this file is executed directly, not
# when it's sourced by another script.
# ---------------------------------------------------------------------------
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  set -e
  cmd="${1:-}"
  shift || true
  case "$cmd" in
  start-adb)
    android_emulator_start_adb
    ;;
  ensure-sdk)
    android_emulator_ensure_sdk
    ;;
  create-avd)
    android_emulator_create_avd_if_missing "$1"
    ;;
  boot)
    android_emulator_boot "$1" "$2" "$3" "$4"
    ;;
  wait-for-boot)
    android_emulator_wait_for_boot "$1"
    ;;
  skip-setup-wizard)
    android_emulator_skip_setup_wizard "$1"
    ;;
  open)
    android_emulator_open_url_dismiss_onboarding "$1" "$2"
    ;;
  reset-chrome)
    android_emulator_reset_chrome "$1"
    ;;
  reverse-port)
    android_emulator_reverse_port "$1" "$2"
    ;;
  forward-devtools)
    android_emulator_forward_devtools "$1" "$2"
    ;;
  grant-camera-permission)
    android_emulator_grant_camera_permission "$1"
    ;;
  kill)
    android_emulator_kill "$1"
    ;;
  *)
    cat <<EOF
Usage: $0 <command> [args]

Commands:
  start-adb                           start the shared ADB daemon
  ensure-sdk                         install SDK + emulator + system image
  create-avd <name>                  create an AVD if it doesn't exist
  boot <name> <port> [log] [cam]      boot an AVD headless, print its PID
                                       (cam: "none" or "imagefile:<path>")
  wait-for-boot <serial>             block until emulator-<port> is booted
  skip-setup-wizard <serial>         skip Android's first-boot wizard
  open <serial> <url>                open <url> in Chrome, dismiss onboarding
  reset-chrome <serial>              clear Chrome tabs and app state
  reverse-port <serial> <port>       adb reverse tcp:<port> for a serial
  forward-devtools <serial> <port>   adb forward tcp:<port> to Chrome's CDP socket
  grant-camera-permission <serial>   tap through Chrome + OS camera prompts, if shown
  kill <serial>                      kill a running emulator

Env: ANDROID_SDK_ROOT (default ~/Android/Sdk), API_LEVEL (default 34),
     AVD_DEVICE_PROFILE (default pixel_6)
EOF
    exit 1
    ;;
  esac
fi
