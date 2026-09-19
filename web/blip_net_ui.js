/* Rally two-device pairing UI.
 *
 * The flow is deliberately small:
 *   HOST -> show the offer -> scan the answer
 *   JOIN -> scan the offer -> show the answer
 *
 * Signaling and QR/camera work live in blip_net.js and blip_qr.js. This file
 * only connects those pieces to the pairing modal.
 */
(function () {
  'use strict';

  if (!window.BlipNet || !window.BlipNetProto || !window.BlipQR) return;

  var modalEl = null;
  var stopActiveScan = null;

  function el(tag, className, parent) {
    var node = document.createElement(tag);
    if (className) node.className = className;
    if (parent) parent.appendChild(node);
    return node;
  }

  function clear(node) {
    while (node.firstChild) node.removeChild(node.firstChild);
  }

  /** Clear a container *and* give up the panel's grown-for-a-code size.
   *
   * Only for the handful of places that replace one screen of the modal
   * with another. This used to live in clear() itself, on the reasoning
   * that clearing a body is exactly when its screen changes — but the
   * ICE list re-renders through clear() too, every 700ms, and it is not
   * a screen change. Its first tick therefore snapped the panel back to
   * 340px while a 671px code was still on it, and the code was cropped
   * to the panel from that moment on: the player saw a QR with its right
   * edge and one finder pattern sliced off, which no camera can read.
   *
   * So the shrink is named and called deliberately, and clear() went
   * back to being what it says it is. */
  function clearScreen(node) {
    clear(node);
    var panel = node && node.closest ? node.closest('.blip-hs-panel') : null;
    if (panel) panel.classList.remove('showing-code');
  }

  function signalKind(state) {
    if (state === 'connected') return 'ok';
    if (state === 'failed' || state === 'timeout' || state === 'disconnected') return 'err';
    // 'checking-slow' stays a 'wait': nothing has failed yet, and the
    // connection may still complete. It is a hint, not a verdict.
    return 'wait';
  }

  /** A blocked network is the likeliest reason two devices that scanned
   * each other's codes never connect, and it is invisible from in here:
   * guest WiFi with client isolation, or a VPN routing LAN traffic
   * elsewhere. Retrying cannot fix either, so the message says what to
   * change instead of inviting another identical attempt. */
  function blockedText() {
    return 'Blocked by this network. Try the same WiFi, without guest mode or a VPN.';
  }

  function isBlocked(detail) {
    return !!(detail && detail.reason && detail.reason.indexOf('cannot reach each other') !== -1);
  }

  function signalText(state, detail) {
    if (state === 'connected') return 'Connected';
    if (state === 'network-mismatch') {
      return 'These devices look like they are on different networks.';
    }
    if (state === 'checking-slow') {
      return 'Still connecting… if this network blocks device-to-device traffic, it won\'t.';
    }
    if (state === 'timeout') return isBlocked(detail) ? blockedText() : 'Timed out. Try again.';
    if (state === 'disconnected') return 'Disconnected';
    if (state === 'failed') {
      if (detail && detail.reason && detail.reason.indexOf('scanned') !== -1) {
        return 'Invalid code. Try again.';
      }
      // WebKit can withhold every ICE candidate from a page that was
      // refused camera access — see warmUpIceMedia() in blip_net.js.
      // Without this line that failure is indistinguishable from a dead
      // network, and the one thing the player could actually do about it
      // goes unsaid.
      if (detail && detail.reason && detail.reason.indexOf('camera access') !== -1) {
        return 'Allow camera access to connect.';
      }
      if (isBlocked(detail)) return blockedText();
      // A local fault, found by the loopback probe before any code was
      // scanned — nothing about the other device or the network can fix
      // it, so the message must not suggest retrying.
      if (detail && detail.reason && detail.reason.indexOf('WebRTC could not open') !== -1) {
        return 'WebRTC is blocked on this device. Check browser or policy settings.';
      }
      return 'Could not connect. Try again.';
    }
    return state === 'answering' ? 'Connecting…' : 'Waiting…';
  }

  /** A persistent note about the *network*, separate from the status
   * line. The status line is overwritten by the next thing that happens
   * — and for the guest, the very next thing is "✓ Host code scanned",
   * about a second later — so a warning left there is gone before it can
   * be read. This one stays up until the attempt ends, which is what a
   * player needs when the message is "these two devices cannot reach
   * each other". Styled amber like the QR size warning: not a failure
   * yet, but the reason it is about to be one. */
  function netWarning(body, text) {
    var warn = body.blipNetWarnEl;
    if (!warn) {
      // Its own class, not the QR size warning's. Both can be on screen
      // at once — a guest on a different network whose panel is also too
      // small for a scannable code — and sharing a class means
      // querySelector returns whichever happens to come first, which is
      // a trap for anything (tests included) trying to read one of them.
      warn = el('div', 'blip-net-warn', body);
      body.blipNetWarnEl = warn;
    }
    warn.textContent = text || '';
    warn.style.display = text ? 'block' : 'none';
  }

  /* ---- Live ICE candidate list ----------------------------------------
   *
   * While the two devices try to reach each other, every candidate pair
   * they are testing is shown, colour-coded: green for a path being
   * attempted or already working, red for one that has been ruled out,
   * dim for one still queued. Until now this phase was a single line of
   * text for up to a minute, which gave the player nothing to act on and
   * no way to tell a slow network from a dead one.
   *
   * It doubles as the fastest diagnosis available for the common
   * network faults: every pair red means something is dropping
   * device-to-device traffic (client isolation, a VPN), while no pairs
   * at all means the two sides never exchanged usable candidates.
   */
  var iceWatch = null;

  function iceRowText(pair) {
    // The remote end is the informative half — the address this device is
    // trying to reach. The local one is always us.
    return pair.remote || '?';
  }

  /** green attempted//succeeded, red ruled out, dim still queued. */
  function iceRowKind(pair) {
    if (pair.state === 'succeeded') return 'ok';
    if (pair.state === 'failed') return 'err';
    if (pair.state === 'in-progress') {
      // In progress but nothing has ever answered is what a blocked path
      // looks like right up until ICE admits it, so show the doubt.
      return (pair.requestsSent > 2 && pair.responsesReceived === 0) ? 'err' : 'ok';
    }
    return 'idle'; // waiting / frozen — queued, not yet tried
  }

  /** The list before ICE has anything to report.
   *
   * Not an empty box, and above all not `display: none`: the list sits
   * under the code, and a box that appears when the first candidate
   * arrives is 60px of new content pushing the panel taller a second
   * after the code was sized to it. The code then either loses its
   * bottom or has to be redrawn while a camera is pointed at it. So the
   * space is held from the start, and held by something worth reading —
   * this is also the only screen that says the connection is being
   * attempted at all while ICE is still gathering. */
  function iceListPlaceholder(container) {
    clear(container);
    // Named for the network, not for "looking" — the scan screen's own
    // status line already says "Looking for a code…", and on the host
    // both are on screen at the same time. Two lines a second apart
    // saying the same word about different things is a puzzle the player
    // does not need.
    el('div', 'blip-ice-empty', container).textContent = 'Checking network routes…';
  }

  function renderIceList(container, stats) {
    if (!stats || !stats.pairs || !stats.pairs.length) {
      iceListPlaceholder(container);
      return;
    }
    clear(container);
    // One row per remote address, not per pair. A device with two local
    // interfaces produces a pair per interface against the same remote,
    // which renders as the same address listed twice — indistinguishable
    // from a bug. The player's question is "can this device be reached",
    // so collapse to the most advanced state any path to it has managed.
    var rank = { succeeded: 3, 'in-progress': 2, failed: 1 };
    var best = {};
    stats.pairs.forEach(function (p) {
      var key = p.remote || '?';
      var prev = best[key];
      if (!prev || (rank[p.state] || 0) > (rank[prev.state] || 0)) best[key] = p;
    });
    Object.keys(best).map(function (k) { return best[k]; }).forEach(function (pair) {
      var row = el('div', 'blip-ice-row', container);
      el('span', 'blip-ice-dot ' + iceRowKind(pair), row);
      var label = el('span', 'blip-ice-addr', row);
      label.textContent = iceRowText(pair);
      var st = el('span', 'blip-ice-state', row);
      st.textContent = pair.state === 'in-progress' ? 'trying' :
        pair.state === 'succeeded' ? 'connected' :
        pair.state === 'failed' ? 'blocked' : pair.state;
    });
  }

  function startIceWatch(body) {
    stopIceWatch();
    var container = el('div', 'blip-ice-list', body);
    iceListPlaceholder(container);
    iceWatch = setInterval(function () {
      if (!window.BlipNet || typeof window.BlipNet.iceStats !== 'function') return;
      window.BlipNet.iceStats().then(function (stats) {
        if (!iceWatch || !container.isConnected) return;
        renderIceList(container, stats);
      }, function () { /* stats are best-effort diagnostics */ });
    }, 700);
  }

  function stopIceWatch() {
    if (iceWatch) {
      clearInterval(iceWatch);
      iceWatch = null;
    }
  }

  function showConnectionResult(status, body, state, detail) {
    // A mid-attempt hint: update the line, then stand back. It must not
    // dismiss the modal or bounce the player to the choice screen, both
    // of which the terminal states below do.
    if (state === 'network-mismatch') {
      // Both: the status line for whoever is looking right now, and a
      // persistent line that survives the next status update.
      status.set(signalText(state, detail), signalKind(state));
      netWarning(body, 'These devices look like they are on different networks — ' +
        'put both on the same WiFi, without guest mode, a hotspot or a VPN.');
      return;
    }
    if (state === 'checking-slow') {
      status.set(signalText(state, detail), signalKind(state));
      return;
    }
    if (state !== 'connected' && state !== 'failed' &&
        state !== 'timeout' && state !== 'disconnected') return;

    status.set(signalText(state, detail), signalKind(state));
    if (state === 'connected') {
      stopIceWatch();
      setTimeout(dismissModal, 500);
      return;
    }
    if (state === 'failed' || state === 'timeout' || state === 'disconnected') {
      // Left on screen deliberately: the final state of every pair is
      // the explanation for the failure the player is being shown.
      stopIceWatch();
    }
    if (state === 'failed' || state === 'timeout') {
      // Returning to the choice screen wipes this message, so a message
      // worth reading needs time to be read. 1.2s is fine for "Invalid
      // code" — short, and the player already knows what they did. The
      // blocked-network line is a sentence asking them to go change
      // something about their WiFi, and it is the only place that
      // explanation appears anywhere.
      setTimeout(function () {
        if (modalEl) showChoice(body);
      }, isBlocked(detail) ? 6000 : 1200);
    }
  }

  function dismissModal() {
    stopIceWatch();
    if (stopActiveScan) {
      stopActiveScan();
      stopActiveScan = null;
    }
    if (modalEl) {
      modalEl.remove();
      modalEl = null;
    }
  }

  function closeModal() {
    dismissModal();
    window.BlipNet.cancel();
  }

  function openModal() {
    if (modalEl) return;

    modalEl = el('div', 'blip-hs-modal blip-net-modal', document.body);
    var panel = el('div', 'blip-hs-panel', modalEl);
    el('div', 'blip-hs-title', panel).textContent = 'PLAY NEARBY';

    var cameraWarning = el('div', 'blip-hs-err', panel);
    var thisModal = modalEl;
    checkForCamera(function (hasCamera) {
      if (modalEl !== thisModal || hasCamera) return;
      cameraWarning.textContent = 'Camera required for pairing.';
    });

    var body = el('div', '', panel);
    showChoice(body);

    var row = el('div', 'blip-hs-row', panel);
    var close = el('button', 'blip-hs-btn ghost', row);
    close.type = 'button';
    close.textContent = 'CLOSE';
    close.addEventListener('click', closeModal);
  }

  function showChoice(body) {
    clearScreen(body);
    var row = el('div', 'blip-hs-row', body);

    var host = el('button', 'blip-hs-btn', row);
    host.type = 'button';
    host.textContent = 'HOST';
    host.addEventListener('click', function () { showHostQR(body); });

    var join = el('button', 'blip-hs-btn', row);
    join.type = 'button';
    join.textContent = 'JOIN';
    join.addEventListener('click', function () { showJoinQR(body); });
  }

  function codeAreaSize(preferred) {
    var byHeight = window.innerHeight * 0.40;
    var byWidth = window.innerWidth * 0.78;
    return Math.max(140, Math.round(Math.min(preferred, byHeight, byWidth)));
  }

  /** The box a QR code gets to fill — the first guess at it.
   *
   * Measured from the panel rather than assumed, because the code can
   * only grow in whole modules: a 57-module code in a 280px box has to
   * choose between 228px and 285px, and that is the difference between
   * hunting for the angle that works and reading it at a glance.
   *
   * Only the width here is exact. The height is a budget — the panel's
   * other contents do not exist yet when this is called — and a budget
   * is a guess, so shrinkToFit() measures the real layout afterwards and
   * takes back whatever this gave away too generously. Being a little
   * too large here is therefore cheap; being too small is not, because
   * nothing grows the code again.
   *
   * Falls back to a viewport fraction when the element has not been laid
   * out yet (clientWidth 0), which is the safe direction to be wrong in. */
  function codeBoxWidth(canvas) {
    var parent = canvas.parentNode;
    var avail = parent && parent.clientWidth ? parent.clientWidth : 0;
    if (!avail) return codeAreaSize(280);
    // Room for the title, the status line, the route list and the
    // buttons. Where what is left is too small to scan, the code is
    // still drawn and the player is told — see drawCode() — which is
    // the honest outcome on a screen that genuinely cannot show one.
    var byHeight = window.innerHeight - 150;
    // The cap is generous rather than tidy: a code is scanned from a
    // hand's distance by another phone's camera, so its size on glass is
    // the single biggest thing deciding whether that camera reads it.
    // Beyond ~900px the returns really do stop.
    return Math.max(MIN_CODE_PX, Math.min(Math.floor(avail), Math.floor(byHeight), 900));
  }

  function qrCanvas(parent) {
    var canvas = el('canvas', 'blip-qr-canvas', parent);
    // No width/height here on purpose: BlipQR.render sets both, sized so
    // one module is a whole number of device pixels. Fixing a CSS size
    // up front is what forced the browser to resample the code by a
    // fractional factor, which is the single most effective way to make
    // a QR undecodable (see render()'s comment in web/blip_qr.js).
    canvas.style.cssText =
      'display:block;position:static;top:auto;left:auto;transform:none;clip-path:none;' +
      'touch-action:auto;margin:10px auto;' +
      'image-rendering:pixelated;border-radius:4px;';
    return canvas;
  }

  /** Persistent warning line under a code, distinct from the status bar
   * (which the caller overwrites immediately after rendering) and from
   * `.blip-hs-err` (which means "this attempt failed"). A code that is
   * too small is not a failed attempt — it is drawn, and it may even
   * scan — but the player must be able to see that it is the likely
   * reason nothing is happening, instead of blaming the camera. */
  function codeWarning(canvas, text) {
    var warn = canvas.blipWarnEl;
    if (!warn) {
      warn = el('div', 'blip-qr-warn', null);
      canvas.parentNode.insertBefore(warn, canvas.nextSibling);
      canvas.blipWarnEl = warn;
    }
    warn.textContent = text || '';
    warn.style.display = text ? 'block' : 'none';
  }

  function statusBar(parent, initialText, initialKind) {
    var wrap = el('div', 'blip-net-status', parent);
    el('span', 'blip-net-status-dot', wrap);
    var text = el('span', 'blip-net-status-text', wrap);

    function set(value, kind) {
      text.textContent = value;
      wrap.className = 'blip-net-status' + (kind ? ' ' + kind : '');
    }

    set(initialText, initialKind);
    return { set: set };
  }

  function scanErrorMessage(err) {
    var name = err && err.name;
    if (name === 'NotAllowedError' || name === 'PermissionDeniedError') {
      return 'Allow camera access and try again.';
    }
    if (name === 'NotFoundError' || name === 'DevicesNotFoundError') {
      return 'No camera found.';
    }
    if (name === 'NotReadableError') return 'Camera is busy.';
    return 'Camera unavailable.';
  }

  function checkForCamera(callback) {
    if (!navigator.mediaDevices ||
        typeof navigator.mediaDevices.enumerateDevices !== 'function') {
      callback(true);
      return;
    }
    navigator.mediaDevices.enumerateDevices()
      .then(function (devices) {
        callback(devices.some(function (device) {
          return device.kind === 'videoinput';
        }));
      })
      .catch(function () {
        callback(true);
      });
  }

  var TOO_SMALL_TEXT = 'This code is too small to scan reliably — make the window taller, ' +
    'or show it on a larger screen.';

  /** The smallest box a code is ever asked to fit.
   *
   * Well below scannable, and deliberately: this is not a size anything
   * aims for, it is the point at which shrinking is called off. A code
   * this small is already being labelled unscannable, and making it
   * smaller still would only take away the one thing left — something
   * for the player to point a camera at while they make the window
   * bigger. */
  var MIN_CODE_PX = 120;

  /** Re-render smaller until the panel stops scrolling.
   *
   * The height a code may take was a guess — viewport minus a constant
   * for the title, status line and buttons. Guesses about other people's
   * content are wrong by exactly the amount that matters: the panel
   * overflowed by 55-115px at every desktop size, so it scrolled, and
   * what fell below the fold was the SCAN ANSWER button the host has to
   * press to finish pairing. Scroll the panel at all and half the code
   * goes out of view with it.
   *
   * Measuring is exact where arithmetic was not: render, ask the panel
   * whether it now overflows, and if it does give the code that much
   * less and render again. Two passes are enough in practice — the code
   * snaps to whole modules, so each pass removes at least one module row
   * — and the loop is bounded regardless.
   */
  function shrinkToFit(canvas, text, info) {
    var panel = canvas.closest ? canvas.closest('.blip-hs-panel') : null;
    if (!panel || !info) return info;
    for (var pass = 0; pass < 3; pass++) {
      var overflow = panel.scrollHeight - panel.clientHeight;
      if (overflow <= 0) break;
      // Shrinking stops at the point it stops buying anything. Below the
      // scannable threshold the code cannot be read at any size, so
      // trading more of it away does not make the screen work — it just
      // makes the useless code smaller, and on a window short enough to
      // reach here the panel still overflows afterwards. Leave it as
      // large as it was, keep the "too small to scan" line up, and let
      // the panel scroll to its CLOSE button.
      if (!info.scannable) break;
      // At least one module smaller, whatever the overflow. The code
      // snaps to a whole number of pixels per module, so asking for
      // "3px less" usually lands on the same cell size and renders the
      // identical code — the loop then spends its passes discovering
      // that nothing changed, and the panel is still 3px too tall. One
      // module is the smallest step that is guaranteed to be a step.
      var perModule = Math.max(1, info.cssPxPerModule);
      var next = Math.max(MIN_CODE_PX, Math.floor(Math.min(info.cssPx - overflow, info.cssPx - perModule)));
      if (next >= info.cssPx) break; // already at the floor
      info = window.BlipQR.render(canvas, text, { fitCssPx: next });
    }
    return info;
  }

  /** Grow the panel around a code. Giving the size back again is
   * clearScreen()'s job, at the one moment it can be right: when this
   * screen is replaced by another. */
  function growPanelForCode(canvas) {
    var panel = canvas && canvas.closest ? canvas.closest('.blip-hs-panel') : null;
    if (panel) panel.classList.add('showing-code');
  }

  /** Draw `text` into `canvas`, sized to the panel it sits in.
   *
   * The one place a code is drawn. renderCode() wraps it for the first
   * draw of a new payload (where a failure means the pairing cannot go
   * on) and redrawCode() for putting a code back that was already on
   * screen (where it can). */
  function drawCode(canvas, text) {
    // Which payload this canvas is currently showing. The deferred refit
    // below reads it back and stands down if a newer code has since been
    // drawn — otherwise it could re-render the canvas with the *previous*
    // payload, replacing the new code with the old one at whatever moment
    // the panel happened to settle.
    canvas.blipCodeText = text;
    growPanelForCode(canvas);
    var info = window.BlipQR.render(canvas, text, { fitCssPx: canvas.blipFitCssPx || codeBoxWidth(canvas) });
    info = shrinkToFit(canvas, text, info);
    // Said out loud rather than left to the player to deduce from a scan
    // that never lands. At this size the other phone's camera cannot
    // resolve the modules, and every other part of the UI would look
    // like it was working.
    codeWarning(canvas, info.scannable ? '' : TOO_SMALL_TEXT);

    // One correction, once, after the caller has finished building the
    // screen — the code is drawn before the status line and the SCAN
    // button exist, so the panel cannot yet know how tall it will be.
    // Two frames is after layout has settled for the whole screen. It
    // earns its keep: without it three of the seven viewports in
    // test/qr-scannability.mjs's fit test overflow again.
    //
    // Deliberately not a ResizeObserver. Watching the panel and
    // re-rendering whenever it changes puts an open-ended asynchronous
    // redraw into the UI: the code could be replaced at any moment,
    // including while a player already has a camera pointed at it. The
    // late-arriving content that made an observer look necessary was the
    // ICE list, and that is fixed where it belongs — the list holds its
    // height from the start, so rows appearing no longer move anything.
    if (canvas.blipFitCssPx) return; // a test pinned the size; leave it pinned
    requestAnimationFrame(function () {
      requestAnimationFrame(function () {
        if (!canvas.isConnected || canvas.blipCodeText !== text) return;
        var refit = shrinkToFit(canvas, text, info);
        codeWarning(canvas, refit.scannable ? '' : TOO_SMALL_TEXT);
      });
    });
  }

  function renderCode(canvas, text, status) {
    try {
      drawCode(canvas, text);
      return true;
    } catch (err) {
      status.set('Code unavailable. Try again.', 'err');
      window.BlipNet.cancel();
      return false;
    }
  }

  /** Put a code back on screen after something else borrowed the space —
   * the camera, which hides the code while it runs.
   *
   * It has to be re-drawn rather than merely un-hidden: the panel shrank
   * to camera size while the code was away, and a canvas restored at its
   * old width is a code wider than the panel, which is a code with its
   * edge cropped off. Deliberately does not cancel the pairing if this
   * throws — this exact payload has already rendered once, and the
   * pairing it belongs to is still perfectly good. */
  function redrawCode(canvas) {
    if (!canvas || !canvas.blipCodeText) return;
    try { drawCode(canvas, canvas.blipCodeText); } catch (err) { /* it drew once */ }
  }

  // Test-only, same convention as window.__blipNetDebug /
  // window.BlipQR.testInject: render a code through the *real* modal
  // path, warning included. Tests that need a payload of a specific size
  // cannot get one out of a live offer, and calling BlipQR.render()
  // directly would skip exactly the part worth testing — whether the
  // player is told when the code is too small to scan. Inert for
  // players; nothing in the UI calls it.
  window.__blipRenderCodeForTest = function (canvas, text, fitCssPx) {
    if (fitCssPx) canvas.blipFitCssPx = fitCssPx;
    var seen = null;
    var ok = renderCode(canvas, text, { set: function (t, k) { seen = { text: t, kind: k }; } });
    var warn = canvas.blipWarnEl;
    return {
      ok: ok,
      render: window.BlipQR.lastRender,
      status: seen,
      warnShown: !!(warn && warn.offsetHeight > 0),
      warnText: (warn && warn.textContent) || ''
    };
  };

  /** What to say while the camera is actually running.
   *
   * blip_qr.js reports 'scanning' on every animation frame, with how
   * long it has been looking and whether anything QR-shaped is in view.
   * None of that reached the screen: the line said "Point at the code."
   * and then never changed again, so a scan that was working perfectly
   * looked identical to one that had died. The commonest reaction to
   * that is to assume the code is not being picked up and give up —
   * usually while holding the phone too far away for the modules to
   * resolve, which is the one thing the player could have fixed.
   *
   * The advice escalates with time rather than dumping it all at once,
   * because the first seconds of a normal scan need no advice at all. */
  function scanningText(detail) {
    if (detail && detail.candidates > 0) return 'Code in view — hold steady…';
    var seconds = Math.floor(((detail && detail.elapsedMs) || 0) / 1000);
    if (seconds >= 15) return 'Still looking — move closer, or add light.';
    if (seconds >= 6) return 'Looking — fill the frame with the code.';
    return 'Looking for a code…';
  }

  function handleScanStatus(status, state, detail) {
    if (state === 'opening') status.set('Opening camera…', 'wait');
    if (state === 'buffering') status.set('Starting camera…', 'wait');
    if (state === 'streaming') status.set('Point at the code.', 'wait');
    if (state === 'scanning') {
      // 'active' is the pulsing dot the host already uses while it works
      // — the point is that the player can see this is running.
      //
      // Guarded against rewriting the same string 60 times a second:
      // 'scanning' arrives every frame, and only the text is news.
      var text = scanningText(detail);
      if (status.lastScanText !== text) {
        status.lastScanText = text;
        status.set(text, 'active');
      }
    }
    if (state === 'found') status.set('Code found…', 'ok');
    if (state === 'play-error' || state === 'unsupported') {
      status.set('Camera unavailable.', 'err');
    }
  }

  function showHostQR(body) {
    clearScreen(body);
    var canvas = qrCanvas(body);
    var status = statusBar(body, 'Creating code…', 'wait');

    window.BlipNet.host(function (offerSdp) {
      if (!renderCode(canvas, offerSdp, status)) return;
      status.set('Show this code to the other phone.', 'wait');
      startIceWatch(body);
      showScanButton(body, 'SCAN ANSWER', function (answerSdp) {
        // Both players tapping HOST is the commonest way to get this
        // wrong, and it used to end in "Could not connect. Try again."
        // a second later — a verdict on the network for what is
        // actually a two-word instruction. The code says which half of
        // the exchange it is, so say it.
        if (codeRole(answerSdp) === 'host') {
          status.set('That is a host code. The other phone should tap JOIN.', 'err');
          return 'retry';
        }
        status.set('✓ Answer scanned. Contacting client…', 'active');
        window.BlipNet.submitAnswer(answerSdp);
      }, false, [canvas], function (err) {
        status.set(scanErrorMessage(err), 'err');
      }, function (state, detail) {
        handleScanStatus(status, state, detail);
      });
    }, function (state, detail) {
      showConnectionResult(status, body, state, detail);
    });
  }

  function showJoinQR(body) {
    clearScreen(body);
    var scanStatus = statusBar(body, 'Scan the host code.', 'wait');

    showScanButton(body, 'SCAN HOST CODE', function (offerSdp) {
      // The mirror image: two players who both tapped JOIN, or one who
      // scanned the answer their own device produced a moment ago.
      if (codeRole(offerSdp) === 'answer') {
        scanStatus.set('That is an answer code. The other phone should tap HOST.', 'err');
        return 'retry';
      }
      clearScreen(body);
      var canvas = qrCanvas(body);
      var status = statusBar(body, 'Creating answer…', 'wait');

      window.BlipNet.join(offerSdp, function (answerSdp) {
        if (!renderCode(canvas, answerSdp, status)) return;
        status.set('✓ Host code scanned. Show this code to the host.', 'ok');
        startIceWatch(body);
      }, function (state, detail) {
        showConnectionResult(status, body, state, detail);
      });
    }, true, [], function (err) {
      scanStatus.set(scanErrorMessage(err), 'err');
    }, function (state, detail) {
      handleScanStatus(scanStatus, state, detail);
    });
  }

  /** What was actually decoded, in one line.
   *
   * "✓ scanned" alone leaves the player trusting a claim. Naming what
   * came off the code — which half of the exchange it was, how many
   * routes it carries, how big it was — is the difference between a
   * reassurance and evidence, and it is the only place the scanned
   * content is ever visible to a human. It also makes a wrong scan
   * (a stale code, the device's own code) obvious instead of silent. */
  /** Which half of the exchange a scanned payload is, or null if it is
   * not a pairing code at all.
   *
   * Reads the compact form first, because that is what every code
   * carries now: `B1|ufrag|pwd|fp|setup|host:port,...` (packForQr in
   * web/blip_sdp_slim.js). The SDP shapes below are the fallback path's,
   * kept because packForQr passes an SDP through whole when it cannot
   * represent it. */
  function codeRole(text) {
    if (typeof text !== 'string' || !text) return null;
    if (text.slice(0, 3) === 'B1|') {
      var parts = text.split('|');
      if (parts.length !== 6 || !parts[5]) return null;
      return parts[4] === 'a' ? 'host' : (parts[4] === 'c' ? 'answer' : null);
    }
    if (text.indexOf('a=setup:actpass') !== -1) return 'host';
    if (text.indexOf('a=setup:active') !== -1) return 'answer';
    return null;
  }

  var ROLE_NAMES = { host: 'host code', answer: 'answer code' };

  function codeRoutes(text) {
    if (text.slice(0, 3) === 'B1|') {
      return (text.split('|')[5] || '').split(',').filter(Boolean).length;
    }
    return (text.match(/(?:^|\n)a=candidate:/g) || []).length;
  }

  /** The one-line verdict on a scanned code: what to say, and whether it
   * is good news.
   *
   * Both come out of the same call on purpose. They were two decisions —
   * the wording here, the colour at the call site — each asking "is this
   * a pairing code?" in its own way, which is two chances to disagree
   * and print a green tick over a red failure. */
  function describeScan(text) {
    if (typeof text !== 'string' || !text) return { line: '\u2717 empty code', ok: false };
    var role = codeRole(text);
    // No tick for something that is not a pairing code. A player who
    // photographs a poster, a URL, a boarding pass — anything with a QR
    // on it — used to get a green "\u2713 code \u00b7 0 routes" above a
    // status line saying the code was invalid, which is two answers to
    // the same question.
    if (!role) return { line: '\u2717 not a pairing code \u00b7 ' + text.length + ' bytes', ok: false };
    var routes = codeRoutes(text);
    return {
      line: '\u2713 ' + ROLE_NAMES[role] + ' \u00b7 ' + routes + ' route' +
        (routes === 1 ? '' : 's') + ' \u00b7 ' + text.length + ' bytes',
      ok: true
    };
  }

  function showScanButton(body, label, onScanned, autoStart, hideEls, onScanError, onScanStatus) {
    var holder = el('div', '', body);

    function startScanning() {
      if (hideEls) {
        hideEls.forEach(function (node) {
          if (node) node.style.display = 'none';
        });
      }
      clearScreen(holder);

      var size = codeAreaSize(260);
      var wrap = el('div', '', holder);
      wrap.style.cssText =
        'position:relative;width:' + size + 'px;aspect-ratio:1/1;margin:10px auto;';

      var video = el('video', '', wrap);
      video.setAttribute('playsinline', '');
      video.setAttribute('muted', '');
      video.muted = true;
      video.style.cssText =
        'display:block;position:absolute;inset:0;width:100%;height:100%;' +
        'object-fit:cover;border-radius:4px;background:#000;';

      var overlay = el('canvas', 'blip-scan-overlay', wrap);
      overlay.style.cssText =
        'display:block;position:absolute;inset:0;width:100%;height:100%;' +
        'pointer-events:none;';
      var rect = wrap.getBoundingClientRect();
      overlay.width = Math.max(1, Math.round(rect.width));
      overlay.height = Math.max(1, Math.round(rect.height));

      var error = el('div', 'blip-hs-err', holder);

      // A scan that never decodes otherwise runs forever, looking
      // identical to one about to succeed. After this long the honest
      // thing is to say it is not working and offer the action that
      // might fix it, rather than let the player keep holding a phone up
      // to a code that is not being read.
      var GIVE_UP_MS = 25000;
      var stuckTimer = setTimeout(function () {
        if (!stopActiveScan) return; // already decoded or errored
        error.textContent = 'Not reading this code. Fill the frame with it, add light, or retry.';
        var retry = el('button', 'blip-hs-btn', holder);
        retry.type = 'button';
        retry.textContent = 'RETRY SCAN';
        retry.addEventListener('click', function () {
          if (stopActiveScan) { stopActiveScan(); stopActiveScan = null; }
          startScanning();
        }, { once: true });
      }, GIVE_UP_MS);

      stopActiveScan = window.BlipQR.scan(video, overlay, function (text, scanErr) {
        stopActiveScan = null;
        clearTimeout(stuckTimer);
        if (scanErr) {
          error.textContent = scanErrorMessage(scanErr);
          if (onScanError) onScanError(scanErr);
          return;
        }
        // Shown on `body`, not `holder` — holder is cleared on the next
        // line, and this has to outlive the screen that produced it.
        var result = body.blipScanResultEl;
        if (!result) {
          result = el('div', 'blip-scan-result', body);
          body.blipScanResultEl = result;
        }
        // Green is for a code that is what it should be; anything else
        // gets the red treatment, so this line cannot read as success
        // while the status line underneath reports a failure.
        var verdict = describeScan(text);
        result.textContent = verdict.line;
        result.className = 'blip-scan-result' + (verdict.ok ? '' : ' bad');
        result.style.display = 'block';
        clear(holder);
        // A caller returning 'retry' means "I read that code and it is
        // the wrong one" — a mistake the player can fix on the spot, so
        // the screen goes back to how it was with the button armed
        // again, rather than tearing the pairing down.
        if (onScanned(text) === 'retry') offerButton();
      }, function (state, detail) {
        if (onScanStatus) onScanStatus(state, detail || {});
      });
    }

    /** The idle state of this control: whatever the scan hid is visible
     * again, and one button offering to (re)start the camera. */
    function offerButton() {
      if (hideEls) {
        hideEls.forEach(function (node) {
          if (!node) return;
          node.style.display = '';
          // Re-drawn, not just un-hidden — see redrawCode().
          redrawCode(node);
        });
      }
      clear(holder);
      var button = el('button', 'blip-hs-btn', holder);
      button.type = 'button';
      button.textContent = label;
      button.addEventListener('click', startScanning, { once: true });
    }

    if (autoStart) {
      startScanning();
      return;
    }
    offerButton();
  }

  window.addEventListener('DOMContentLoaded', function () {
    var button = document.getElementById('net-play-btn');
    if (!button) return;
    button.textContent = 'PLAY NEARBY';
    button.addEventListener('click', openModal);
  });
}());
