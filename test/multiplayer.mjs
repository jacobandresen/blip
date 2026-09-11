// End-to-end test for two-device Rally (docs/multiplayer.md): two
// independent headless Chromium instances, host and guest, pair over the
// real Supabase Realtime signaling channel and open a real WebRTC
// DataChannel — nothing here is mocked. Per the plan's scope decision
// ("same room only"), two processes on one machine's loopback interface
// *are* a same-room pair as far as WebRTC's host ICE candidates are
// concerned, which is exactly what makes this automatable at all.
//
// Requires: a `chromium` (or `$BLIP_CHROMIUM`) binary, network access to
// the Supabase project configured in web/blip_config.js (git-ignored —
// copy web/blip_config.example.js and fill in your own project to run
// this locally), and `./build_web.sh` already having produced
// web/rally/index.wasm.
//
// Run: npm run test:multiplayer

import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { launch, evaluate, waitFor, sleep, killAll } from './lib/cdp.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const WEB_DIR = path.join(__dirname, '..', 'web');
const PORT = 8098; // distinct from the 8080 devs run by hand, so both can coexist
const HOST_DEBUG_PORT = 9331;
const GUEST_DEBUG_PORT = 9332;

const MIME = {
  '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css',
  '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png',
  '.svg': 'image/svg+xml', '.ico': 'image/x-icon', '.woff2': 'font/woff2',
};

let server;
let procs = [];

before(async () => {
  server = createServer(async (req, res) => {
    try {
      let p = decodeURIComponent(req.url.split('?')[0]);
      if (p.endsWith('/')) p += 'index.html';
      const full = path.join(WEB_DIR, p);
      if (!full.startsWith(WEB_DIR)) { res.writeHead(403); res.end(); return; }
      const data = await readFile(full);
      res.writeHead(200, { 'Content-Type': MIME[path.extname(full)] || 'application/octet-stream' });
      res.end(data);
    } catch (e) {
      res.writeHead(404);
      res.end('not found');
    }
  });
  await new Promise((resolve) => server.listen(PORT, '127.0.0.1', resolve));
});

after(async () => {
  killAll(procs);
  await new Promise((resolve) => server.close(resolve));
});

/** The dial hand's rotation encodes the paddle fraction — see
 * shell.js's `window.blipPaddles`. Reading it back from the live DOM
 * (rather than trying to intercept the callback, which races against
 * shell.js's own unconditional `window.blipPaddles = ...` assignment)
 * gives an injection-free way to observe *both* sides' idea of the right
 * paddle's position — host (simulated) and guest (mirrored from the
 * network) — without any test-only hook in the game or bridge code. */
const READ_RIGHT_FRACTION = `(function () {
  var el = document.getElementById('dial-hand-p2');
  if (!el) return null;
  var m = /rotate\\(([-\\d.eE]+)rad\\)/.exec(el.style.transform || '');
  if (!m) return null;
  var SWEEP = 3 * Math.PI;
  return parseFloat(m[1]) / SWEEP + 0.5;
})()`;

async function loadRally(cdp) {
  await evaluate(cdp, 'true'); // ensure Runtime is ready before navigating
  await cdp.send('Page.navigate', { url: `http://127.0.0.1:${PORT}/rally/index.html` });
  await waitFor(cdp, "document.readyState === 'complete'", 15000);
  await waitFor(cdp, "typeof window.BlipNet === 'object' && typeof window.blipNetRole === 'function'", 15000);
  // The coin wall: shell.js swallows *every* keydown at the window
  // capture phase while it's up (`overlay.classList.contains('visible')`
  // -> stopImmediatePropagation — see web/shell.js), which would
  // otherwise silently eat both this test's synthetic KeyI/KeyK events
  // and blip_net.js's own guest-input listener. Dismiss it the same way
  // a real player would: a click/tap on the overlay inserts a coin.
  if (await evaluate(cdp, "document.getElementById('need-coin-overlay').classList.contains('visible')")) {
    await evaluate(cdp, "document.getElementById('need-coin-overlay').click()");
  }
}

test('two-device Rally: pairs, opens a DataChannel, and syncs input+state end to end', async (t) => {
  const hostChrome = await launch(HOST_DEBUG_PORT);
  const guestChrome = await launch(GUEST_DEBUG_PORT);
  procs = [hostChrome.proc, guestChrome.proc];
  const host = hostChrome.cdp;
  const guest = guestChrome.cdp;

  await Promise.all([loadRally(host), loadRally(guest)]);

  await t.test('host generates a room code', async () => {
    await evaluate(host, "document.getElementById('net-play-btn').click()");
    await evaluate(host, `
      Array.from(document.querySelectorAll('.blip-hs-btn'))
        .find(function (b) { return b.textContent === 'HOST'; }).click();
    `);
    const code = await waitFor(host, "(document.querySelector('.blip-hs-code') || {}).textContent", 5000);
    assert.match(code, /^[0-9]{4}$/, `expected a 4-digit room code, got ${JSON.stringify(code)}`);
    t.diagnostic(`room code: ${code}`);
    host._roomCode = code;
  });

  await t.test('guest joins with that code and the DataChannel opens on both sides', async () => {
    const code = host._roomCode;
    await evaluate(guest, "document.getElementById('net-play-btn').click()");
    await evaluate(guest, `
      Array.from(document.querySelectorAll('.blip-hs-btn'))
        .find(function (b) { return b.textContent === 'JOIN'; }).click();
    `);
    await evaluate(guest, `document.querySelector('.blip-hs-input').value = ${JSON.stringify(code)}`);
    // Fire a real 'input' event — the UI normalizes on that event, not on .value alone.
    await evaluate(guest, `
      document.querySelector('.blip-hs-input').dispatchEvent(new Event('input', { bubbles: true }));
    `);
    await evaluate(guest, `
      Array.from(document.querySelectorAll('.blip-hs-btn'))
        .find(function (b) { return b.textContent === 'CONNECT'; }).click();
    `);

    const hostRole = await waitFor(host, 'window.blipNetRole()', 15000);
    const guestRole = await waitFor(guest, 'window.blipNetRole()', 15000);
    assert.equal(hostRole, 1, 'host should report role 1 once its DataChannel opens');
    assert.equal(guestRole, 2, 'guest should report role 2 once its DataChannel opens');
  });

  await t.test('the match actually starts on both devices (Rust picks up the net role)', async () => {
    // Title screen reports a flat ~0 fraction (lpad_y/rpad_y start at 0,
    // clamped); reset_for_serve() (start_game(), on seeing the net role)
    // centers both paddles to ~0.5 — the transition off Title, observed
    // with zero cooperation from the game/bridge code.
    const hostFrac = await waitFor(host, `(${READ_RIGHT_FRACTION}) > 0.3`, 10000);
    const guestFrac = await waitFor(guest, `(${READ_RIGHT_FRACTION}) > 0.3`, 10000);
    assert.ok(hostFrac, 'host paddle never left the Title-screen position');
    assert.ok(guestFrac, 'guest paddle never left the Title-screen position');
  });

  await t.test('guest input reaches the host and the resulting state syncs back', async () => {
    const before = await evaluate(guest, READ_RIGHT_FRACTION);
    assert.ok(before > 0.3 && before < 0.7, `expected a centered paddle to start, got ${before}`);

    // Simulate the guest's on-screen P2 dial being spun "up" — exactly
    // the KeyboardEvent bindDial() would dispatch on a real touch, which
    // is what blip_net.js's guest input capture is listening for.
    await evaluate(guest, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keydown', {
        bubbles: true, cancelable: true, key: 'i', code: 'KeyI',
      }));
    `);

    // Wait for the guest's own mirrored view to show real movement first
    // (proves guest -> host -> guest made at least one full round trip),
    // then RELEASE immediately before comparing host vs guest — both
    // paddles are in continuous motion the whole time the key is held,
    // so a snapshot taken mid-motion catches the two sides at slightly
    // different points along that motion (one DataChannel round trip
    // apart) and is a flaky comparison by construction. Once the key is
    // up, motion actually stops on both sides and a settled snapshot is a
    // fair, non-racy comparison.
    //
    // Kept deliberately fast, for a second reason beyond flakiness: any
    // key press (including this one) also satisfies Rally's own
    // any_key_pressed() launch condition on the *real* Serve state it's
    // in — the ball is now live, and if it goes on to actually score, the
    // resulting reset_for_serve() recenters both paddles regardless of
    // what this test is doing. Rally's SCORE_WIN-line ball can't cross
    // the ~450px field in under ~0.8s even at the sharpest launch angle
    // (see bounce_paddle's angle clamp), so finishing this whole check
    // well inside that window is what keeps it a non-issue rather than a
    // rare, unrelated flake.
    await waitFor(guest, `(${READ_RIGHT_FRACTION}) < ${before} - 0.03`, 8000);
    await evaluate(guest, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keyup', {
        bubbles: true, cancelable: true, key: 'i', code: 'KeyI',
      }));
    `);
    await sleep(150); // let the last couple of state packets in flight land on both sides

    const guestAfter = await evaluate(guest, READ_RIGHT_FRACTION);
    const hostAfter = await evaluate(host, READ_RIGHT_FRACTION);
    assert.ok(guestAfter < before,
      `guest's own mirrored paddle should show it moved up (was ${before}, now ${guestAfter})`);
    assert.ok(hostAfter < before,
      `host should have actually simulated the remote input (was ${before}, now ${hostAfter})`);
    assert.ok(Math.abs(guestAfter - hostAfter) < 0.05,
      `host and guest should agree on the settled paddle position (host ${hostAfter}, guest ${guestAfter})`);

    // And it should have actually stopped — not still drifting from a
    // keyup that never latched (an edge-triggered send bug would either
    // never have moved it at all, caught above, or never have released
    // it, caught here). A jump to ~0.5 here would be reset_for_serve()
    // recentering both paddles because the point actually finished (see
    // the timing note above) — vanishingly unlikely inside this ~150ms
    // window, but if the game itself changed underneath the check, that's
    // not a networking failure, so it's called out rather than asserted
    // through blind.
    const settled1 = await evaluate(guest, READ_RIGHT_FRACTION);
    await sleep(150);
    const settled2 = await evaluate(guest, READ_RIGHT_FRACTION);
    const looksLikeAPointReset = Math.abs(settled2 - 0.5) < 0.02 && Math.abs(settled1 - settled2) > 0.05;
    if (looksLikeAPointReset) {
      t.diagnostic(`paddle snapped to ~0.5 between settle reads (${settled1} -> ${settled2}) — ` +
        'the ball most likely scored and reset_for_serve() ran; not treating as a sync failure');
    } else {
      assert.ok(Math.abs(settled1 - settled2) < 0.01,
        `paddle should have stopped moving after keyup (${settled1} -> ${settled2})`);
    }
  });

  await t.test('a clean disconnect is observed on both sides', async () => {
    await evaluate(host, 'window.BlipNet.cancel()');
    const hostRole = await waitFor(host, 'window.blipNetRole() === 0', 5000);
    assert.equal(hostRole, true);
    // The guest's channel closing is a consequence of the host's peer
    // connection tearing down — allow a little longer for that to land.
    const guestRole = await waitFor(guest, 'window.blipNetRole() === 0', 8000);
    assert.equal(guestRole, true);
  });
});
