// The HOST / JOIN / SCAN ANSWER dance, as one reusable routine.
//
// test/multiplayer.mjs has driven this inline since it was the only
// caller. It now has two more — test/multiplayer-webkit.mjs (two
// Playwright engines, including cross-engine WebKit<->Chromium) and
// test/multiplayer-ios.mjs (a real iPhone over the WebKit inspector) —
// and the steps are identical for all of them, because every one of
// them reaches the page through the same `evaluate()` contract.
//
// Signaling transport: `window.BlipQR.lastRenderedText` is what the
// rendering side just encoded into its QR canvas, and assigning
// `window.BlipQR.testInject` is the scan-side hook that resolves the
// pending scan with that exact string. So the bytes crossing between
// the two devices are precisely the bytes a camera would have decoded —
// the camera optics are what's skipped, not the payload, the SDP
// validation, or any of the pairing UI.

import { evaluate, waitFor, sleep } from './cdp.mjs';
import { QR_READY, clickHsBtn, openModal, getScanErrText, isAtChoiceScreen } from './multiplayer-harness.mjs';

export async function readRenderedText(cdp) {
  const text = await evaluate(cdp, 'window.BlipQR.lastRenderedText');
  if (typeof text !== 'string' || !text) throw new Error('QR renderer did not expose its test payload');
  return text;
}

export async function injectNextScan(cdp, text) {
  if (typeof text !== 'string' || !text) throw new Error('cannot inject an empty scan payload');
  await evaluate(cdp, `window.BlipQR.testInject = ${JSON.stringify(text)}`);
}

/** HOST: open the modal, take the host role, wait for the offer QR, and
 * return the payload it encodes. */
export async function hostShowsOffer(cdp, timeoutMs = 20000) {
  await openModal(cdp);
  await clickHsBtn(cdp, 'HOST');
  await waitFor(cdp, QR_READY, timeoutMs);
  return readRenderedText(cdp);
}

/** JOIN: open the modal, take the guest role (which auto-starts a
 * scan), feed it `offerText`, and return the answer payload it renders
 * back. Surfaces the modal's own error line rather than a bare timeout —
 * "SDP rejected" and "never decoded" are different failures. */
export async function guestAnswersOffer(cdp, offerText, timeoutMs = 25000) {
  await openModal(cdp);
  // Injected *before* JOIN, not after: blip_qr.js's scan() reads
  // window.BlipQR.testInject once, synchronously, at the top of the
  // call, and JOIN starts that scan immediately. Setting it afterwards
  // leaves the scan waiting on a camera that will never see a code.
  await injectNextScan(cdp, offerText);
  await clickHsBtn(cdp, 'JOIN');
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const err = await getScanErrText(cdp);
    if (err) throw new Error(`guest rejected the offer: ${err}`);
    if (await isAtChoiceScreen(cdp)) throw new Error('guest fell back to the choice screen');
    if (await evaluate(cdp, QR_READY)) return readRenderedText(cdp);
    if (Date.now() > deadline) throw new Error('guest never rendered an answer QR');
    await sleep(200);
  }
}

/** SCAN ANSWER on the host, completing signaling. */
export async function hostTakesAnswer(cdp, answerText) {
  await injectNextScan(cdp, answerText);
  await clickHsBtn(cdp, 'SCAN');
}

/** The whole exchange. Resolves once both sides report a net role,
 * which is what "the DataChannel opened" looks like from the page. */
export async function pairOverQr(host, guest, { connectTimeoutMs = 30000 } = {}) {
  const offer = await hostShowsOffer(host);
  const answer = await guestAnswersOffer(guest, offer);
  await hostTakesAnswer(host, answer);
  const hostRole = await waitFor(host, 'window.blipNetRole()', connectTimeoutMs);
  const guestRole = await waitFor(guest, 'window.blipNetRole()', connectTimeoutMs);
  return { offer, answer, hostRole, guestRole };
}
