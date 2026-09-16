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
    return 'wait';
  }

  function signalText(state, detail) {
    if (state === 'connected') return 'Connected';
    if (state === 'timeout') return 'Timed out. Try again.';
    if (state === 'disconnected') return 'Disconnected';
    if (state === 'failed') {
      if (detail && detail.reason && detail.reason.indexOf('scanned') !== -1) {
        return 'Invalid code. Try again.';
      }
      return 'Could not connect. Try again.';
    }
    return state === 'answering' ? 'Connecting…' : 'Waiting…';
  }

  function showConnectionResult(status, body, state, detail) {
    if (state !== 'connected' && state !== 'failed' &&
        state !== 'timeout' && state !== 'disconnected') return;

    status.set(signalText(state, detail), signalKind(state));
    if (state === 'connected') {
      setTimeout(dismissModal, 500);
      return;
    }
    if (state === 'failed' || state === 'timeout') {
      setTimeout(function () {
        if (modalEl) showChoice(body);
      }, 1200);
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

  function qrCanvas(parent) {
    var canvas = el('canvas', 'blip-qr-canvas', parent);
    var size = codeAreaSize(280);
    canvas.style.cssText =
      'display:block;position:static;top:auto;left:auto;transform:none;clip-path:none;' +
      'touch-action:auto;margin:10px auto;width:' + size + 'px;height:' + size + 'px;' +
      'image-rendering:pixelated;border-radius:4px;';
    return canvas;
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
      window.BlipQR.render(canvas, text);
      return true;
    } catch (err) {
      status.set('Code unavailable. Try again.', 'err');
      window.BlipNet.cancel();
      return false;
    }
  }

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
        status.set('Connecting…', 'wait');
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
        status.set('Show this code to the host.', 'wait');
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
