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
      return 'Could not connect. Try again.';
    }
    return state === 'answering' ? 'Connecting…' : 'Waiting…';
  }

  function showConnectionResult(status, body, state, detail) {
    // A mid-attempt hint: update the line, then stand back. It must not
    // dismiss the modal or bounce the player to the choice screen, both
    // of which the terminal states below do.
    if (state === 'checking-slow') {
      status.set(signalText(state, detail), signalKind(state));
      return;
    }
    if (state !== 'connected' && state !== 'failed' &&
        state !== 'timeout' && state !== 'disconnected') return;

    status.set(signalText(state, detail), signalKind(state));
    if (state === 'connected') {
      setTimeout(dismissModal, 500);
      return;
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
    clear(body);
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

  /** The width a QR code actually has to work with, as a hard limit.
   *
   * Distinct from codeAreaSize()'s preferred size because the QR can
   * only grow in whole modules: a 145-module code in a 280px box has to
   * choose between 145px and 290px, and the difference between those two
   * is the difference between unscannable and comfortable. Measuring the
   * panel the code actually sits in, rather than assuming 280, is what
   * lets it take the second option whenever the room is really there.
   *
   * Falls back to the preferred size when the element has not been laid
   * out yet (clientWidth 0), which is the safe direction to be wrong in. */
  function codeBoxWidth(canvas) {
    var parent = canvas.parentNode;
    var avail = parent && parent.clientWidth ? parent.clientWidth : 0;
    if (!avail) return codeAreaSize(280);
    // Width only. Viewport *height* deliberately does not constrain this:
    // the panel scrolls vertically (.blip-hs-panel has overflow-y: auto),
    // so a code taller than a short landscape window costs the player a
    // scroll, while a code shrunk to fit that window costs them the
    // ability to scan it at all. Horizontal space is the real limit,
    // because nothing scrolls sideways.
    // Capped so a very wide panel does not produce a needlessly huge
    // code; beyond this, extra size buys no scanning reliability.
    return Math.max(120, Math.min(Math.floor(avail), 420));
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

  function renderCode(canvas, text, status) {
    try {
      var info = window.BlipQR.render(canvas, text, { fitCssPx: canvas.blipFitCssPx || codeBoxWidth(canvas) });
      // Said out loud rather than left to the player to deduce from a
      // scan that never lands. At this size the other phone's camera
      // cannot resolve the modules, and every other part of the UI would
      // look like it was working.
      codeWarning(canvas, info.scannable ? '' :
        'This code is too small to scan reliably — make the window taller, ' +
        'or show it on a larger screen.');
      return true;
    } catch (err) {
      status.set('Code unavailable. Try again.', 'err');
      window.BlipNet.cancel();
      return false;
    }
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

  function handleScanStatus(status, state) {
    if (state === 'opening') status.set('Opening camera…', 'wait');
    if (state === 'streaming') status.set('Point at the code.', 'wait');
    if (state === 'found') status.set('Code found…', 'wait');
    if (state === 'play-error' || state === 'unsupported') {
      status.set('Camera unavailable.', 'err');
    }
  }

  function showHostQR(body) {
    clear(body);
    var canvas = qrCanvas(body);
    var status = statusBar(body, 'Creating code…', 'wait');

    window.BlipNet.host(function (offerSdp) {
      if (!renderCode(canvas, offerSdp, status)) return;
      status.set('Show this code to the other phone.', 'wait');
      showScanButton(body, 'SCAN ANSWER', function (answerSdp) {
        status.set('✓ Answer scanned. Contacting client…', 'active');
        window.BlipNet.submitAnswer(answerSdp);
      }, false, [canvas], function (err) {
        status.set(scanErrorMessage(err), 'err');
      }, function (state) {
        handleScanStatus(status, state);
      });
    }, function (state, detail) {
      showConnectionResult(status, body, state, detail);
    });
  }

  function showJoinQR(body) {
    clear(body);
    var scanStatus = statusBar(body, 'Scan the host code.', 'wait');

    showScanButton(body, 'SCAN HOST CODE', function (offerSdp) {
      clear(body);
      var canvas = qrCanvas(body);
      var status = statusBar(body, 'Creating answer…', 'wait');

      window.BlipNet.join(offerSdp, function (answerSdp) {
        if (!renderCode(canvas, answerSdp, status)) return;
        status.set('✓ Host code scanned. Show this code to the host.', 'ok');
      }, function (state, detail) {
        showConnectionResult(status, body, state, detail);
      });
    }, true, [], function (err) {
      scanStatus.set(scanErrorMessage(err), 'err');
    }, function (state) {
      handleScanStatus(scanStatus, state);
    });
  }

  function showScanButton(body, label, onScanned, autoStart, hideEls, onScanError, onScanStatus) {
    var holder = el('div', '', body);

    function startScanning() {
      if (hideEls) {
        hideEls.forEach(function (node) {
          if (node) node.style.display = 'none';
        });
      }
      clear(holder);

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
      stopActiveScan = window.BlipQR.scan(video, overlay, function (text, scanErr) {
        stopActiveScan = null;
        if (scanErr) {
          error.textContent = scanErrorMessage(scanErr);
          if (onScanError) onScanError(scanErr);
          return;
        }
        clear(holder);
        onScanned(text);
      }, function (state, detail) {
        if (onScanStatus) onScanStatus(state, detail || {});
      });
    }

    if (autoStart) {
      startScanning();
      return;
    }

    var button = el('button', 'blip-hs-btn', holder);
    button.type = 'button';
    button.textContent = label;
    button.addEventListener('click', startScanning, { once: true });
  }

  window.addEventListener('DOMContentLoaded', function () {
    var button = document.getElementById('net-play-btn');
    if (!button) return;
    button.textContent = 'PLAY NEARBY';
    button.addEventListener('click', openModal);
  });
}());
