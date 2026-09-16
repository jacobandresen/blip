// Keeping a *corporate* iPhone awake for the length of a test run.
//
// The iOS guest side of the multiplayer harness talks to the phone over
// the WebKit inspector (see test/lib/ios-cdp.mjs). The moment the screen
// sleeps, that target stops answering — every evaluate() hangs until the
// CDP server gives up with "device did not report an inspection target
// in time". The obvious fix, Settings > Display & Brightness > Auto-Lock
// > Never, is not available on an MDM-managed device where a profile
// pins the maximum auto-lock.
//
// So the page holds itself awake instead: the Screen Wake Lock API,
// which WebKit has shipped since iOS 16.4. Two wrinkles this handles:
//
//   - Safari gates `navigator.wakeLock.request()` behind transient user
//     activation, so the request has to be evaluated with CDP's
//     `userGesture: true` rather than through the plain evaluate()
//     helper.
//   - A wake lock is released automatically whenever the document stops
//     being visible, and is *not* restored on its own. The page installs
//     its own `visibilitychange` handler to re-request one, so a stray
//     app switch or a lock that slips through mid-run recovers without
//     the harness noticing.

import { evaluate } from './cdp.mjs';

const INSTALL = `(function () {
  if (window.__blipWakeKeeper) return 'already-installed';
  window.__blipWakeKeeper = { lock: null, acquires: 0, lastError: null };
  window.__blipWakeAcquire = function () {
    if (!navigator.wakeLock) {
      window.__blipWakeKeeper.lastError = 'unsupported';
      return Promise.resolve('unsupported');
    }
    return navigator.wakeLock.request('screen').then(function (s) {
      window.__blipWakeKeeper.lock = s;
      window.__blipWakeKeeper.acquires++;
      return 'acquired';
    }, function (e) {
      window.__blipWakeKeeper.lastError = e.name + ': ' + e.message;
      return 'failed:' + e.name;
    });
  };
  document.addEventListener('visibilitychange', function () {
    if (document.visibilityState === 'visible') window.__blipWakeAcquire();
  });
  return 'installed';
})()`;

/** Install the keeper and take the first lock. Returns 'acquired',
 * 'unsupported', or 'failed:<name>'. */
export async function acquireWakeLock(cdp) {
  await evaluate(cdp, INSTALL);
  // userGesture:true is the whole reason this bypasses evaluate().
  const r = await cdp.send('Runtime.evaluate', {
    expression: 'window.__blipWakeAcquire()',
    returnByValue: true, awaitPromise: true, userGesture: true,
  });
  if (r.exceptionDetails) {
    throw new Error(r.exceptionDetails.exception?.description || 'wake lock request threw');
  }
  // The returned value is deliberately ignored: pymobiledevice3's CDP
  // bridge does not honour awaitPromise, so a promise-valued expression
  // comes back as an empty object rather than what it resolved to. The
  // keeper records its own outcome on the page, so read that instead —
  // which works the same over a bridge that does await promises.
  for (let i = 0; i < 20; i++) {
    const s = await wakeLockStatus(cdp);
    if (s.held) return 'acquired';
    if (s.lastError === 'unsupported') return 'unsupported';
    if (s.lastError) return `failed:${s.lastError}`;
    await new Promise((res) => setTimeout(res, 150));
  }
  return 'failed:timeout';
}

export async function wakeLockStatus(cdp) {
  return evaluate(cdp, `(function () {
    var k = window.__blipWakeKeeper;
    if (!k) return { installed: false };
    return { installed: true, held: !!(k.lock && !k.lock.released), acquires: k.acquires, lastError: k.lastError };
  })()`);
}
