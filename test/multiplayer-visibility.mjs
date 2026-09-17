// Can the player see that something is happening?
//
// Two places where this feature did its work in complete silence, and
// both read as "broken" rather than "working":
//
//   1. Scanning. blip_qr.js reports progress on every animation frame --
//      how long it has been looking, whether anything QR-shaped is in
//      view -- and none of it reached the screen. The line said "Point at
//      the code." and then never changed, so a scan that was running
//      perfectly looked exactly like one that had died. The natural
//      reaction is to conclude the code is not being picked up.
//
//   2. Connecting. Up to a minute of one unchanging line while ICE tried
//      every candidate pair, with no way to tell a slow network from a
//      dead one.
//
// Both now show their working. These tests pin that, because "it looks
// like nothing is happening" is invisible to every other test in the
// suite -- they all read state the player cannot see.

import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { launchEngine } from './lib/engine.mjs';
import { hostShowsOffer, guestAnswersOffer, hostTakesAnswer } from './lib/pairing.mjs';
import {
  createFileServer, HTTP_PORT, loadRally, openModal, clickHsBtn, getStatusText,
  launchWithCamera, writeBlankVideo, blockAllCandidates, pollUntil, evaluate, sleep,
} from './lib/multiplayer-harness.mjs';

const ENGINE = process.env.BLIP_HOST_ENGINE || 'chromium';

test('scanning shows that it is running, and says what to try', async (t) => {
  const dir = mkdtempSync(path.join(tmpdir(), 'blip-vis-'));
  const cam = path.join(dir, 'blank.y4m');
  // A camera pointed at nothing: the scan runs and never decodes, which
  // is exactly the state that used to look identical to a dead scan.
  await writeBlankVideo(cam);

  const server = createFileServer();
  await new Promise((r) => server.listen(HTTP_PORT, r));
  const { proc, cdp } = await launchWithCamera(9781, cam);
  t.after(async () => {
    proc.kill();
    await new Promise((r) => server.close(r));
  });

  await loadRally(cdp);
  await openModal(cdp);
  await clickHsBtn(cdp, 'JOIN');

  const seen = [];
  await t.test('the status line moves past "Point at the code."', async () => {
    const text = await pollUntil(async () => {
      const s = await getStatusText(cdp);
      if (s && !seen.includes(s)) seen.push(s);
      return s && /Looking/.test(s) ? s : null;
    }, 20000, 200);
    t.diagnostic(`status progression: ${JSON.stringify(seen)}`);
    assert.match(text, /Looking/);
  });

  await t.test('the indicator shows the scan as active, not merely waiting', async () => {
    // The pulsing dot the host already uses while it works — this is the
    // part a player registers without reading anything.
    // The kind lands on the status wrapper, which is what drives the
    // dot's colour and pulse in CSS (.blip-net-status.active …).
    const kind = await evaluate(cdp, `(function () {
      var w = document.querySelector('.blip-net-status');
      return w ? w.className : null;
    })()`);
    t.diagnostic(`status dot class: ${kind}`);
    assert.match(kind || '', /active/, 'the scan indicator never became active');
  });

  await t.test('after a while it offers something to actually do', async () => {
    // Escalating advice: silence first (a normal scan needs none), then
    // the two things that fix most failures — fill the frame, add light.
    const text = await pollUntil(async () => {
      const s = await getStatusText(cdp);
      return s && /fill the frame|move closer|add light/i.test(s) ? s : null;
    }, 25000, 300);
    t.diagnostic(`advice shown: ${JSON.stringify(text)}`);
    assert.match(text, /fill the frame|move closer|add light/i);
  });
});

test('the candidate pairs are shown on screen while connecting', async (t) => {
  const server = createFileServer();
  await new Promise((r) => server.listen(HTTP_PORT, r));
  const hostBrowser = await launchEngine(ENGINE);
  const guestBrowser = await launchEngine(ENGINE);
  const host = hostBrowser.cdp;
  const guest = guestBrowser.cdp;
  t.after(async () => {
    await hostBrowser.browser.close().catch(() => {});
    await guestBrowser.browser.close().catch(() => {});
    await new Promise((r) => server.close(r));
  });
  await Promise.all([loadRally(host), loadRally(guest)]);

  const readRows = () => evaluate(host, `(function () {
    return Array.prototype.map.call(document.querySelectorAll('.blip-ice-row'), function (r) {
      var dot = r.querySelector('.blip-ice-dot');
      return { kind: dot ? dot.className.replace('blip-ice-dot', '').trim() : '',
               text: r.textContent.replace(/\\s+/g, ' ').trim() };
    });
  })()`);

  // Blocked on purpose: every candidate pair is doomed, which keeps the
  // modal in its connecting state long enough to watch the list evolve.
  // A healthy pairing connects in well under a second and dismisses the
  // modal, leaving nothing to observe.
  const offer = await hostShowsOffer(host);
  const answer = await guestAnswersOffer(guest, blockAllCandidates(offer));
  await hostTakesAnswer(host, blockAllCandidates(answer));

  await t.test('a row appears for each pair being tried', async () => {
    const rows = await pollUntil(async () => {
      const r = await readRows();
      return r.length ? r : null;
    }, 20000, 250);
    t.diagnostic(`rows: ${JSON.stringify(rows)}`);
    assert.ok(rows.length > 0, 'no candidate pairs were shown');
  });

  await t.test('a path that cannot get through ends red', async () => {
    const rows = await pollUntil(async () => {
      const r = await readRows();
      return r.some((x) => x.kind === 'err') ? r : null;
    }, 60000, 500);
    const red = rows.filter((x) => x.kind === 'err');
    t.diagnostic(`red rows: ${JSON.stringify(red)}`);
    assert.ok(red.length > 0, 'a blocked path never turned red');
  });

  // The colours themselves are pinned separately, and deliberately not
  // by watching a live connection. On a blocked network ICE moves a pair
  // from waiting to in-progress to failed faster than the list's render
  // tick, so "green" is a state the UI may legitimately never paint —
  // asserting it live is a race the test loses at random, which is how
  // this started out and why it is written this way now.
  //
  // window.BlipNet.iceStats is the public seam the list reads, so
  // substituting it drives the real render path with known input.
  await t.test('each pair state maps to the colour it should', async () => {
    await evaluate(host, `(function () {
      window.BlipNet.iceStats = function () {
        return Promise.resolve({ pairs: [
          { state: 'succeeded',   remote: 'host udp 192.168.0.9:1111', requestsSent: 9, responsesReceived: 9 },
          { state: 'in-progress', remote: 'host udp 192.168.0.9:2222', requestsSent: 1, responsesReceived: 0 },
          { state: 'failed',      remote: 'host udp 10.255.255.1:3333', requestsSent: 9, responsesReceived: 0 },
          { state: 'waiting',     remote: 'host udp 192.168.0.9:4444', requestsSent: 0, responsesReceived: 0 },
          { state: 'in-progress', remote: 'host udp 10.255.255.1:5555', requestsSent: 9, responsesReceived: 0 }
        ] });
      };
      return true;
    })()`);

    const rows = await pollUntil(async () => {
      const r = await readRows();
      return r.length === 5 ? r : null;
    }, 8000, 200);
    t.diagnostic(`mapped rows: ${JSON.stringify(rows)}`);

    const kindFor = (needle) => (rows.find((r) => r.text.indexOf(needle) !== -1) || {}).kind;
    assert.equal(kindFor('1111'), 'ok',   'a connected path should read green');
    assert.equal(kindFor('2222'), 'ok',   'a path being attempted should read green');
    assert.equal(kindFor('3333'), 'err',  'a failed path should read red');
    assert.equal(kindFor('4444'), 'idle', 'a queued path should read dim');
    // Tried repeatedly with nothing ever answering is what a blocked path
    // looks like before ICE admits it — shown as blocked, not as progress.
    assert.equal(kindFor('5555'), 'err',
      'a path with many requests and no replies should read red, not green');
  });
});
