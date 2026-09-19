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
import { hostShowsOffer, guestAnswersOffer, hostTakesAnswer, injectNextScan } from './lib/pairing.mjs';
import {
  openPage, openPair, HTTP_PORT, loadRally, openModal, clickHsBtn, getStatusText, waitFor,
  writeBlankVideo, blockAllCandidates, pollUntil, evaluate, sleep,
} from './lib/multiplayer-harness.mjs';

const ENGINE = process.env.BLIP_HOST_ENGINE || 'chromium';

test('scanning shows that it is running, and says what to try', async (t) => {
  const dir = mkdtempSync(path.join(tmpdir(), 'blip-vis-'));
  const cam = path.join(dir, 'blank.y4m');
  // A camera pointed at nothing: the scan runs and never decodes, which
  // is exactly the state that used to look identical to a dead scan.
  await writeBlankVideo(cam);

  // openPage takes a camFile and applies the fake-camera flags itself,
  // so the page, its server and their teardown are handled in one place
  // rather than assembled here.
  const { cdp } = await openPage(t, 'chromium', { camFile: cam });

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
  const { host, guest } = await openPair(t, ENGINE, ENGINE);

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

// What a scan actually read, in one line.
//
// The scan-result line is the only place in the UI that shows the
// *content* of a code rather than an assurance about it — which half of
// the exchange it was, how many routes it carries, how big it was. That
// is the difference between "✓ scanned" (a claim) and evidence, and it
// is what makes a wrong scan — a stale code, the device's own code —
// visible instead of silent.
//
// It is also the kind of line that rots without anyone noticing, and it
// did: it parsed raw SDP, the payload became the compact `B1|…` form,
// and from then on every real scan in the product reported "code · 0
// routes". Nothing failed. The evidence just quietly stopped being
// evidence, and read like a failure to anyone who looked at it.
test('the scan result says what was actually read', async (t) => {
  const { host, guest } = await openPair(t, ENGINE, ENGINE);

  const offer = await hostShowsOffer(host);
  const answer = await guestAnswersOffer(guest, offer);
  await hostTakesAnswer(host, answer);

  const line = await pollUntil(async () => evaluate(host, `(function () {
    var el = document.querySelector('.blip-scan-result');
    return el && el.offsetHeight > 0 ? el.textContent : null;
  })()`), 10000, 150);

  t.diagnostic(`host scan result: ${JSON.stringify(line)} for a ${answer.length}-byte payload`);

  // The half of the exchange it was. A host that has just scanned
  // something calling itself a host code is scanning the wrong screen.
  assert.match(line, /answer code/,
    `the host scanned an answer and the line called it something else: ${line}`);
  // Routes, and not zero — the compact payload always carries at least
  // one, and zero here is the exact symptom of a summary that no longer
  // understands the format it is being handed.
  const routes = Number((line.match(/(\d+) routes?/) || [])[1]);
  assert.ok(routes > 0, `the line reports ${routes} routes for a payload carrying real ones: ${line}`);
  assert.equal(routes, answer.split('|')[5].split(',').filter(Boolean).length,
    'the route count does not match what the payload actually carries');
  assert.match(line, new RegExp(`${answer.length} bytes`),
    `the byte count does not match the ${answer.length}-byte payload: ${line}`);
});

// Scanning the wrong half of the exchange.
//
// Two players both tapping HOST is the commonest way to get this wrong,
// and the least self-explanatory: each one has a code, each one has a
// SCAN button, and the screens look exactly like a pairing that is going
// well. What used to happen is that the scan succeeded, the offer was
// submitted where an answer belonged, WebRTC refused it, and a second
// later the player was told "Could not connect. Try again." — a verdict
// on their network for what is really a two-word instruction.
//
// The payload says which half it is. These pin that the UI reads it,
// says so, and leaves the pairing standing so the mistake can be fixed
// on the spot rather than started over.
test('a code scanned into the wrong half says so, and can be retried', async (t) => {
  const { host, guest } = await openPair(t, ENGINE, ENGINE);

  const hostCode = await hostShowsOffer(host);
  const otherHostCode = await hostShowsOffer(guest); // both players tapped HOST

  await injectNextScan(host, otherHostCode);
  await clickHsBtn(host, 'SCAN ANSWER');

  const screen = await pollUntil(async () => evaluate(host, `(function () {
    var p = document.querySelector('.blip-hs-panel');
    var s = document.querySelector('.blip-net-status-text');
    var c = p && p.querySelector('canvas.blip-qr-canvas');
    var status = s && s.offsetHeight > 0 ? s.textContent : '';
    if (!/host code/i.test(status)) return null;
    var fits = null;
    if (c && c.offsetHeight) {
      var cb = c.getBoundingClientRect(), pb = p.getBoundingClientRect();
      fits = cb.right <= pb.left + p.clientLeft + p.clientWidth + 0.5 &&
             cb.bottom <= pb.top + p.clientTop + p.clientHeight + 0.5;
    }
    return {
      status: status,
      codeShown: !!(c && c.offsetHeight),
      codeFits: fits,
      buttons: Array.prototype.map.call(p.querySelectorAll('.blip-hs-btn'), function (b) { return b.textContent; })
    };
  })()`), 10000, 200);

  t.diagnostic(`host after scanning another host code: ${JSON.stringify(screen)}`);

  // Names what was scanned and what to do about it — not "try again".
  assert.match(screen.status, /host code/i);
  assert.match(screen.status, /JOIN/,
    'the message should say which button the other phone needs, since that is the whole fix');
  // The pairing is untouched: the host's own code is still up, still
  // whole, and the scan can simply be repeated.
  assert.ok(screen.codeShown, 'the host code vanished — it is still valid and still needed');
  assert.equal(screen.codeFits, true,
    'the restored code does not fit its panel, so it came back at the size it had before the ' +
    'camera shrank the panel — a cropped code, which no camera can read');
  assert.ok(screen.buttons.some((b) => /SCAN ANSWER/.test(b)),
    'no way to scan again after a wrong code — the only way out is to start the pairing over');

  // Scanning the right code afterwards still works: the retry is real,
  // not just a button that looks like one.
  //
  // The other player has to back out of HOST first, which is exactly
  // what the message just told them to do.
  await clickHsBtn(guest, 'CLOSE');
  await sleep(300);
  await openModal(guest);
  const answer = await guestAnswersOffer(guest, hostCode);
  await injectNextScan(host, answer);
  await clickHsBtn(host, 'SCAN ANSWER');
  const opened = await waitFor(host, `(function () {
    var d = window.__blipNetDebug && window.__blipNetDebug();
    return !!(d && d.dcState === 'open');
  })()`, 20000);
  assert.ok(opened, 'the pairing did not complete after recovering from a wrong scan');
});

test('a code that is not a pairing code is not reported as one', async (t) => {
  const { host } = await openPair(t, ENGINE, ENGINE);
  await hostShowsOffer(host);

  await injectNextScan(host, 'https://example.com/some-other-qr-code');
  await clickHsBtn(host, 'SCAN ANSWER');

  const seen = await pollUntil(async () => evaluate(host, `(function () {
    var r = document.querySelector('.blip-scan-result');
    return r && r.offsetHeight > 0 ? { text: r.textContent, cls: r.className } : null;
  })()`), 10000, 200);

  t.diagnostic(`scan result for a non-pairing QR: ${JSON.stringify(seen)}`);
  // The line reports what was read. A tick and a route count for a
  // random QR off a poster is the UI agreeing with itself that a failure
  // succeeded, one line above a status saying the code was invalid.
  assert.doesNotMatch(seen.text, /✓/, 'a non-pairing code was reported with a tick');
  assert.match(seen.text, /not a pairing code/i);
  assert.match(seen.cls, /bad/, 'the line is still styled as a success');
});
