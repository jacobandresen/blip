#!/bin/bash
# Runs every BLIP BASIC tape natively, mashing keys, and fails on any BASIC error.
# test/basic/*.BLIP are checks that END when they pass. One tape at a time.
set -u
cd "$(dirname "$0")/.."
cargo build --release -q -p blip_basic || exit 1
secs="${BLIP_TAPE_SECS:-40}"
failed=0
for tape in web/TAPE/*.BLIP test/basic/*.BLIP; do
  for seed in ${BLIP_TAPE_SEEDS:-1 2}; do
    out=$(BLIP_BOT=1 BLIP_BOT_FUZZ="$seed" BLIP_BOT_MAXT="$secs" BLIP_BASIC_TAPE="$tape" \
      timeout $((secs * 4 + 30)) xvfb-run -a -s "-screen 0 720x810x24" target/release/blip_basic 2>&1)
    code=$?
    case "$tape" in test/*) want="outcome=ended" ;; *) want="RESULT" ;; esac
    if [ $code -ne 0 ] || ! grep -q "$want" <<<"$out"; then
      echo "FAIL $tape seed=$seed exit=$code: $(grep -m1 -E 'BASIC ERROR|panicked|RESULT' <<<"$out")"
      failed=1
    else
      echo "ok   $tape seed=$seed $(grep -m1 RESULT <<<"$out")"
    fi
    case "$tape" in test/*) break ;; esac
  done
done
exit $failed
