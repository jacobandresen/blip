// What a player sees when the network will not carry the traffic.
//
// This is the failure that looks most like success. Both QR codes scan,
// both devices report progress, the SDP exchange completes perfectly --
// because it happened through a camera, not the network -- and then
// nothing. The usual causes are not bugs at all: guest or corporate WiFi
// with client isolation, where the access point refuses to pass traffic
// between its own clients, or a VPN routing LAN traffic somewhere else.
// Neither is visible from inside the page, and neither is fixed by
// trying again, which is exactly what "Could not connect. Try again."
// invites the player to do.
//
// Before this test existed the whole path was unexercised: the harness
// carried a `blockAllCandidates()` helper that nothing called, the UI had
// no notion of a blocked network, and a player got sixty seconds of
// silence followed by a generic timeout.
//
// The block is produced the way docs/multiplayer.md describes: every UDP
// host candidate is rewritten to a reserved, unroutable address. The
// candidates stay real enough for ICE to genuinely try them, and every
// check fails -- which is precisely what client isolation looks like from
// in here, without needing two networks or real firewall rules.

import test from 'node:test';
import assert from 'node:assert/strict';
import { hostShowsOffer, guestAnswersOffer, hostTakesAnswer, injectNextScan } from './lib/pairing.mjs';
import {
  openPair, HTTP_PORT, blockAllCandidates, rewriteRoutes,
  getStatusText, clickHsBtn, pollUntil, evaluate, waitFor,
} from './lib/multiplayer-harness.mjs';

const HOST_ENGINE = process.env.BLIP_HOST_ENGINE || 'chromium';
const GUEST_ENGINE = process.env.BLIP_GUEST_ENGINE || 'chromium';

test('a blocked network is reported, not left to time out in silence', async (t) => {
  const { host, guest } = await openPair(t, HOST_ENGINE, GUEST_ENGINE);

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
