// Find out the connection cannot work *before* asking the player to do
// anything with it.
//
// The worst version of this feature's failure is silent and slow: you
// scan a code, both screens look busy, and a minute later you get a
// generic error. Nothing in that minute told you the cause, and for two
// of the causes -- WebRTC disabled on the device, or the two devices not
// being on the same network -- no amount of retrying will help.
//
// So two checks run up front:
//
//   - A loopback probe. Two RTCPeerConnections inside the page, one
//     DataChannel message actually sent and received. The traffic never
//     leaves the machine, so it is expected to succeed on any working
//     setup; a failure means WebRTC itself is unavailable here (policy,
//     an extension, a build without SCTP, an engine that yields no ICE
//     candidates at all).
//   - A subnet comparison on the guest, which is the only side holding
//     both sets of candidates and can therefore notice that the two
//     devices are nowhere near each other.

import test from 'node:test';
import assert from 'node:assert/strict';
import { hostShowsOffer, guestAnswersOffer, hostTakesAnswer, injectNextScan } from './lib/pairing.mjs';
import {
  openPage, openPair, HTTP_PORT, loadRally, rewriteRoutes, blockAllCandidates,
  openModal, clickHsBtn, getStatusText, pollUntil, evaluate, waitFor, sleep, QR_READY,
} from './lib/multiplayer-harness.mjs';

const ENGINE = process.env.BLIP_HOST_ENGINE || 'chromium';

test(`the WebRTC loopback probe (${ENGINE})`, async (t) => {
  const { browser, cdp } = await openPage(t, ENGINE);
  await loadRally(cdp);

  await t.test('succeeds on a working browser, and actually moves a message', async () => {
    const started = Date.now();
    const r = await evaluate(cdp, 'window.BlipNet.preflight()');
    t.diagnostic(`preflight: ${JSON.stringify(r)} in ${Date.now() - started}ms`);
    assert.equal(r.ok, true, `the probe failed on a working browser: ${r.detail || r.reason}`);
  });

  await t.test('is cached — it cannot change within a page load, and is not free', async () => {
    const started = Date.now();
    const r = await evaluate(cdp, 'window.BlipNet.preflight()');
    assert.equal(r.ok, true);
    assert.ok(Date.now() - started < 500, 'a second preflight re-ran the probe instead of reusing it');
  });
});

test(`a device without WebRTC is told immediately, not after the connect timeout (${ENGINE})`, async (t) => {
  const { browser, cdp } = await openPage(t, ENGINE);
  await loadRally(cdp);

  // Taken away before anything has run, so nothing is cached — the same
  // position a player is in whose browser or policy disallows WebRTC.
  await evaluate(cdp, `(function () {
    window.RTCPeerConnection = undefined;
    return typeof window.RTCPeerConnection;
  })()`);

  const r = await evaluate(cdp, 'window.BlipNet.preflight()');
  assert.equal(r.ok, false, 'the probe passed on a browser with no RTCPeerConnection');
  assert.match(r.reason, /WebRTC could not open/);
  t.diagnostic(`detail: ${r.detail}`);
});

test(`the guest reports a different-network pairing early (${ENGINE})`, async (t) => {
  const { host, guest } = await openPair(t, ENGINE, ENGINE);

  // The guest must have gathered its own addresses for the comparison to
  // mean anything; without them the check stays quiet by design.
  const pre = await evaluate(guest, 'window.BlipNet.preflight()');
  if (!pre.addresses || !pre.addresses.length) {
    t.skip(`this engine published no IPv4 host candidates (${JSON.stringify(pre.addresses)}), ` +
      'so there is nothing to compare subnets against — see looksLikeDifferentNetwork()');
    return;
  }
  t.diagnostic(`guest addresses: ${pre.addresses.join(', ')}`);

  const offer = await hostShowsOffer(host);
  // Every candidate moved to a reserved address in a subnet the guest is
  // certainly not on — what a hotspot, guest VLAN or VPN looks like.
  //
  // A subnet the guest is certainly not on — what a hotspot, guest VLAN
  // or VPN looks like. rewriteRoutes understands both payload shapes, so
  // this keeps working whichever form the QR happens to carry.
  const foreign = rewriteRoutes(offer, '10.255.255.7');
  assert.notEqual(foreign, offer, 'the rewrite changed nothing — wrong payload shape?');

  await injectNextScan(guest, foreign);
  await openModal(guest);
  await clickHsBtn(guest, 'JOIN');

  // Read the persistent warning, not the status line: the status line is
  // overwritten about a second later by "✓ Host code scanned", so
  // polling it is a race the test would sometimes lose and sometimes win.
  const started = Date.now();
  const text = await pollUntil(async () => evaluate(guest, `(function () {
    var w = document.querySelector('.blip-net-warn');
    return (w && w.offsetHeight > 0 && /different networks/i.test(w.textContent)) ? w.textContent : null;
  })()`), 20000, 250);
  const elapsed = Date.now() - started;

  t.diagnostic(`"${text}" after ${elapsed}ms`);
  assert.match(text, /different networks/i);
  // The whole point is that it beats the connect timeout by a wide margin.
  assert.ok(elapsed < 15000, `took ${elapsed}ms — too slow to count as early`);

  // Both warnings can be on screen at once — a guest on a different
  // network whose panel is also too small to show a scannable code — and
  // they must stay tellable apart. They shared a class at first, which
  // made querySelector return whichever came first in the DOM: a trap
  // for anything reading one of them, this test very much included.
  // The different-network warning lands within ~300ms, well before the
  // guest has finished producing its own answer code — so wait for that
  // code to exist before asking for a version of it too small to scan.
  await waitFor(guest, QR_READY, 20000);
  const both = await evaluate(guest, `(function () {
    var c = document.querySelector('.blip-hs-panel canvas.blip-qr-canvas');
    window.__blipRenderCodeForTest(c, window.BlipQR.lastRenderedText, 120);
    var q = document.querySelector('.blip-qr-warn');
    var n = document.querySelector('.blip-net-warn');
    return { qr: q ? q.textContent : null, net: n ? n.textContent : null,
             qrShown: !!(q && q.offsetHeight > 0), netShown: !!(n && n.offsetHeight > 0),
             count: document.querySelectorAll('.blip-qr-warn, .blip-net-warn').length };
  })()`);

  t.diagnostic(`both warnings: ${JSON.stringify(both)}`);
  assert.ok(both.qrShown, 'the too-small warning is not visible');
  assert.ok(both.netShown, 'the different-network warning is not visible');
  assert.match(both.qr, /too small to scan/i);
  assert.match(both.net, /different networks/i);
  assert.equal(both.count, 2, 'expected exactly one warning of each kind');
});

// ---- the network refusing to carry the traffic ---------------------------
//
// Folded in from what used to be a separate file: it shares this suite's
// subject (what the player is told when the network, not the code, is the
// problem), its setup, and its helpers. Two files meant two copies of the
// same pairing dance to reach the interesting moment.
//
// This is the failure that looks most like success. Both codes scan, both
// sides report progress, and the SDP exchange completes perfectly --
// because it happened through a camera, not the network. Then nothing. The
// usual causes are not bugs: guest or corporate WiFi with client isolation,
// or a VPN routing LAN traffic elsewhere.

test('a blocked network is reported, not left to time out in silence', async (t) => {
  const { host, guest } = await openPair(t, ENGINE, ENGINE);

  await t.test('pairing still completes — the codes exchange fine, only the traffic is blocked', async () => {
    const offer = await hostShowsOffer(host);
    // Shape-agnostic: the payload may be the compact form or plain SDP,
    // and what matters is only that it names routes this test can move
    // somewhere unreachable. Matching the literal word "candidate" tied
    // this to the SDP shape and broke silently when the payload changed.
    assert.notEqual(rewriteRoutes(offer, '10.255.255.1'), offer,
      `the offer names no routes to block: ${offer.slice(0, 80)}`);

    // Both directions: the offer carries the host's candidates and the
    // answer carries the guest's, so blocking one still leaves a usable
    // pair. This is the whole point — no pair may work.
    const answer = await guestAnswersOffer(guest, blockAllCandidates(offer));
    await hostTakesAnswer(host, blockAllCandidates(answer));

    // Neither side should ever report a role: a role is set on
    // DataChannel open, and nothing can open here.
    await new Promise((r) => setTimeout(r, 1500));
    assert.equal(await evaluate(host, 'window.blipNetRole()'), 0);
    assert.equal(await evaluate(guest, 'window.blipNetRole()'), 0);
  });

  await t.test('the player is told something is wrong while still watching, not after a minute', async () => {
    // The hint fires after ICE_SLOW_HINT_MS (8s) in 'checking'. Without
    // it, this screen says "Contacting client…" for the full 60-second
    // connect timeout with no indication anything is amiss.
    const text = await pollUntil(async () => {
      const s = await getStatusText(host);
      return s && /Still connecting|Blocked by this network/.test(s) ? s : null;
    }, 25000, 500);
    t.diagnostic(`host status after the block: ${JSON.stringify(text)}`);
    assert.match(text, /Still connecting|Blocked by this network/);
  });

  await t.test('the final message names the network, and does not just say "try again"', async () => {
    // Either route gets here: ICE reaching 'failed', or the connect
    // timeout expiring with ICE still unfinished. Both are the same
    // situation and both must produce the same actionable message.
    // Sampled quickly: the modal returns to the choice screen a few
    // seconds after a terminal status, taking the status element with it.
    const text = await pollUntil(async () => {
      const s = await getStatusText(host);
      return s && /Blocked by this network/.test(s) ? s : null;
    }, 60000, 250);
    assert.match(text, /Blocked by this network/);
    assert.match(text, /WiFi|VPN/, 'the message should say what to change');
    assert.doesNotMatch(text, /^Could not connect\. Try again\.$/,
      'a blocked network must not be reported as a generic failure — retrying cannot fix it');
  });
});
