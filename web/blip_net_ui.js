/* Rally-only "JACK IN" pairing UI — the modal on top of web/blip_net.js.
 * QR-code pairing only (see docs/multiplayer.md) — no network involved in
 * pairing at all, just two camera scans. Reuses the .blip-hs-* modal
 * classes (shell.css) that blip_scores.js's name/recovery prompts already
 * use elsewhere. Kept separate from blip_net.js: that file is pure
 * networking (and is what test/multiplayer.mjs drives directly, headless,
 * with no DOM UI in the way); this one is just the on-screen wiring
 * around it.
 *
 * No hand-holding prose in the dialog itself — instead a terminal-style
 * console (.blip-net-term, styled in shell.css) stacked below the panel
 * prints what's actually happening: SDP sizes, ICE candidate counts,
 * RTCPeerConnection/DataChannel state, camera + scan progress, and —
 * straight off RTCPeerConnection.getStats() — which ICE candidate pairs
 * were actually tried and whether their connectivity checks got a
 * response, so a "timeout" comes with an actual diagnosis instead of
 * just the word. See termPrint()/reportFailureDiagnosis() below.
 *
 * It also takes real input — type 'help' at its prompt for the command
 * list. `status`/`stats` are on-demand copies of the same real data the
 * auto-telemetry above prints; `ls`/`ps`/`top` are simulated (there's no
 * real filesystem/process table behind a browser tab to show). See
 * COMMANDS/runCommand() below.
 */
(function () {
  'use strict';
  // Any of these missing (script blocked, load order broken, one of the
  // two vendored QR libraries failed) — drop the whole feature rather
  // than show a HOST/JOIN that can't actually pair.
  if (!window.BlipNet || !window.BlipNetProto || !window.BlipQR) return;

  var modalEl = null;
  // Set once by openModal(), read by the `host`/`join` commands below —
  // the same `body`/`panel` showChoice()'s HOST/JOIN buttons already
  // close over, so typing the command runs the exact same code as
  // tapping the button, not a re-implementation of it.
  var modalBody = null;
  var modalPanel = null;

  function el(tag, className, parent) {
    var e = document.createElement(tag);
    if (className) e.className = className;
    if (parent) parent.appendChild(e);
    return e;
  }

  function clear(node) { while (node.firstChild) node.removeChild(node.firstChild); }

  // ---- the terminal --------------------------------------------------------

  var termLogEl = null;
  var termPromptUserEl = null;
  var termLineCount = 0;
  var TERM_MAX_LINES = 80;
  var termStartedAt = 0;

  function buildTerminal(parent) {
    var term = el('div', 'blip-net-term', parent);
    el('div', 'blip-net-term-bar', term).textContent = 'blip-net — signaling console';
    termLogEl = el('div', 'blip-net-term-log', term);
    var prompt = el('div', 'blip-net-term-prompt', term);
    termPromptUserEl = el('span', '', prompt);
    termPromptUserEl.textContent = 'blip';
    prompt.appendChild(document.createTextNode(':~$ '));
    var typed = el('span', 'blip-net-term-typed', prompt);
    el('span', 'blip-net-term-cursor', prompt);
    // A real <input> is what actually captures keystrokes (and, on a
    // phone, is what opens the keyboard when tapped) — but it's never
    // shown itself. It's parked off-screen; `typed` above mirrors its
    // value so what's on screen is plain terminal text with a blinking
    // block cursor, not a form field with a border/focus ring/native
    // caret. Tapping anywhere on the prompt line focuses it, same as
    // clicking into a real terminal.
    var input = el('input', 'blip-net-term-input', prompt);
    input.type = 'text';
    input.autocomplete = 'off';
    input.autocapitalize = 'off';
    input.spellcheck = false;
    input.setAttribute('aria-label', 'debug console command');
    input.addEventListener('input', function () { typed.textContent = input.value; });
    input.addEventListener('keydown', function (e) {
      if (e.key !== 'Enter') return;
      var raw = input.value;
      input.value = '';
      typed.textContent = '';
      runCommand(raw);
    });
    prompt.addEventListener('click', function () { input.focus(); });
    termLogEl.addEventListener('click', function () { input.focus(); });
    return term;
  }

  function termReset() {
    if (termLogEl) clear(termLogEl);
    termLineCount = 0;
    termStartedAt = Date.now();
    if (termPromptUserEl) termPromptUserEl.textContent = 'blip';
  }

  function termSetUser(name) {
    if (termPromptUserEl) termPromptUserEl.textContent = name;
  }

  /** Print one line to the terminal. `kind`: 'cmd' (a typed command, no
   * timestamp), 'ok' (success/milestone), 'err' (failure), or omitted for
   * plain info output. `bare` drops the `[t.ts]` prefix an info/ok/err
   * line would otherwise get — for a typed command's own output, which
   * should read like real stdout (no per-line timestamp), not like the
   * auto-generated connection telemetry above it. */
  function termPrint(text, kind, bare) {
    if (!termLogEl) return;
    var line = el('div', 'blip-net-term-line' + (kind ? ' ' + kind : ''), termLogEl);
    if (kind === 'cmd') {
      line.textContent = '$ ' + text;
    } else if (bare) {
      line.textContent = text;
    } else {
      line.textContent = '[' + ((Date.now() - termStartedAt) / 1000).toFixed(2) + 's] ' + text;
    }
    termLineCount++;
    if (termLineCount > TERM_MAX_LINES) {
      var first = termLogEl.firstChild;
      if (first) termLogEl.removeChild(first);
    }
    termLogEl.scrollTop = termLogEl.scrollHeight;
  }

  function padRight(s, n) {
    s = String(s);
    while (s.length < n) s += ' ';
    return s;
  }

  // ---- typed commands -------------------------------------------------------
  // A small, honest debug console bolted onto the pairing terminal: `help`
  // lists what's here, `status`/`stats` are the SAME real data the
  // auto-telemetry above already prints (just on demand), and `ls`/`ps`/
  // `top` are simulated — a bit of decoration consistent with the rest of
  // this thing looking like a shell, not a genuine filesystem/process
  // table (there isn't one to show).

  function printCommandList() {
    termPrint('available commands:', undefined, true);
    Object.keys(COMMANDS).sort().forEach(function (name) {
      termPrint('  ' + padRight(name, 8) + COMMANDS[name].desc, undefined, true);
    });
  }

  function simulateLs() {
    var files = ['offer.sdp', 'answer.sdp', 'ice-candidates.log', 'session.pid', 'blip-net.sock'];
    termPrint(files.join('  '), undefined, true);
  }

  function simulatePs() {
    var role = 'blip';
    if (typeof window.__blipNetDebug === 'function') {
      var d = window.__blipNetDebug();
      if (d.role === 1) role = 'host';
      else if (d.role === 2) role = 'guest';
    }
    var rows = [
      ['PID', 'USER', 'COMMAND'],
      ['1', role, 'blip-net-agent'],
      ['42', role, 'ice-checker'],
      ['57', role, 'qr-scanner'],
      ['88', role, 'datachannel-mux'],
    ];
    rows.forEach(function (r) {
      termPrint(padRight(r[0], 6) + padRight(r[1], 8) + r[2], undefined, true);
    });
  }

  function zeroPad(n) { return n < 10 ? '0' + n : String(n); }

  function simulateTop() {
    var now = new Date();
    var clock = zeroPad(now.getHours()) + ':' + zeroPad(now.getMinutes()) + ':' + zeroPad(now.getSeconds());
    termPrint('top - ' + clock + ' up 0 min,  1 user,  load average: 0.31, 0.24, 0.19', undefined, true);
    termPrint('Tasks:   4 total,   1 running,   3 sleeping', undefined, true);
    termPrint('%CPU:  2.8 us,  0.6 sy,  0.0 ni, 96.6 id', undefined, true);
    termPrint('', undefined, true);
    termPrint(padRight('PID', 6) + padRight('USER', 8) + padRight('%CPU', 7) + padRight('%MEM', 7) + 'COMMAND', undefined, true);
    [['1', 'blip', '2.1', '1.2', 'blip-net-agent'],
     ['42', 'blip', '0.9', '0.3', 'ice-checker'],
     ['57', 'blip', '0.2', '0.1', 'qr-scanner']].forEach(function (r) {
      termPrint(padRight(r[0], 6) + padRight(r[1], 8) + padRight(r[2], 7) + padRight(r[3], 7) + r[4], undefined, true);
    });
  }

  var COMMANDS = {
    help: { desc: 'list available commands', run: function () { printCommandList(); } },
    man: {
      desc: 'man <cmd> for details, or this list',
      run: function (args) {
        if (!args.length) { printCommandList(); return; }
        var name = args[0].toLowerCase();
        var cmd = COMMANDS[name];
        if (!cmd) { termPrint('no manual entry for ' + name, 'err', true); return; }
        termPrint(name.toUpperCase() + '(1)', undefined, true);
        termPrint('    ' + cmd.desc, undefined, true);
      },
    },
    clear: {
      desc: 'clear this console',
      run: function () { if (termLogEl) clear(termLogEl); termLineCount = 0; },
    },
    status: {
      desc: 'real connection state: role, RTCPeerConnection, DataChannel',
      run: function () {
        if (typeof window.__blipNetDebug !== 'function') { termPrint('__blipNetDebug unavailable', 'err', true); return; }
        var d = window.__blipNetDebug();
        var roleName = d.role === 1 ? 'host' : (d.role === 2 ? 'guest' : 'none');
        termPrint('role: ' + roleName, undefined, true);
        termPrint('pc.connectionState: ' + d.pcConnectionState, undefined, true);
        termPrint('pc.iceConnectionState: ' + d.pcIceConnectionState, undefined, true);
        termPrint('datachannel.readyState: ' + d.dcState, undefined, true);
      },
    },
    stats: {
      desc: 'real ICE candidate-pair stats, right now',
      run: function () {
        if (typeof window.__blipNetStats !== 'function') { termPrint('__blipNetStats unavailable', 'err', true); return; }
        window.__blipNetStats().then(function (pairs) {
          if (!pairs || !pairs.length) { termPrint('no candidate pairs yet', undefined, true); return; }
          pairs.forEach(function (p) {
            termPrint('candidate-pair ' + pairSummary(p), p.state === 'succeeded' ? 'ok' : (p.state === 'failed' ? 'err' : undefined), true);
          });
        });
      },
    },
    ls: { desc: 'list files in the current directory', run: simulateLs },
    ps: { desc: 'list running processes', run: simulatePs },
    top: { desc: 'show live resource usage', run: simulateTop },
    host: {
      desc: 'start hosting — same as tapping HOST',
      run: function () {
        if (!modalBody || !modalPanel) { termPrint('no active session', 'err', true); return; }
        if (stopActiveScan) { stopActiveScan(); stopActiveScan = null; }
        showHostQR(modalBody, modalPanel);
      },
    },
    join: {
      desc: 'scan a host’s code — same as tapping JOIN',
      run: function () {
        if (!modalBody || !modalPanel) { termPrint('no active session', 'err', true); return; }
        if (stopActiveScan) { stopActiveScan(); stopActiveScan = null; }
        showJoinQR(modalBody, modalPanel);
      },
    },
  };

  function runCommand(raw) {
    var trimmed = (raw || '').trim();
    if (!trimmed) return;
    termPrint(trimmed, 'cmd');
    var parts = trimmed.split(/\s+/);
    var name = parts[0].toLowerCase();
    var cmd = COMMANDS[name];
    if (!cmd) {
      termPrint('command not found: ' + name + ' — type \'help\' for a list', 'err', true);
      return;
    }
    cmd.run(parts.slice(1));
  }

  // A low-level heartbeat straight off window.__blipNetDebug() (blip_net.js's
  // own debug/test introspection hook) — real RTCPeerConnection/DataChannel
  // state, not anything synthesized here, printed only when it actually
  // changes.
  var debugPollTimer = null;
  var lastDebugSnapshot = null;
  function startDebugPoll() {
    lastDebugSnapshot = null;
    debugPollTimer = setInterval(function () {
      if (typeof window.__blipNetDebug !== 'function') return;
      var d = window.__blipNetDebug();
      if (lastDebugSnapshot) {
        if (d.pcConnectionState !== lastDebugSnapshot.pcConnectionState) {
          termPrint('pc.connectionState -> ' + d.pcConnectionState);
        }
        if (d.pcIceConnectionState !== lastDebugSnapshot.pcIceConnectionState) {
          termPrint('pc.iceConnectionState -> ' + d.pcIceConnectionState);
        }
        if (d.dcState !== lastDebugSnapshot.dcState) {
          termPrint('datachannel.readyState -> ' + d.dcState, d.dcState === 'open' ? 'ok' : undefined);
        }
      }
      lastDebugSnapshot = d;
    }, 350);
  }
  function stopDebugPoll() {
    if (debugPollTimer) { clearInterval(debugPollTimer); debugPollTimer = null; }
  }

  // The actual ICE connectivity-check attempts, straight off
  // RTCPeerConnection.getStats() (window.__blipNetStats(), blip_net.js) —
  // pc.iceConnectionState alone says checks are happening, not which
  // candidate pairs, what kind (host/srflx/relay), or whether a check's
  // request ever got a response. Slower than the debug poll above
  // (getStats() is heavier) and only prints a pair when its summary
  // string actually changes.
  var statsPollTimer = null;
  var lastPairsKey = null;
  // The most recent *non-empty* pairs snapshot — kept because Chrome can
  // (and, observed firsthand while building this, does) hand back an
  // empty getStats() report for a pair that has already moved to
  // 'failed', right as pc.connectionState itself flips to 'failed'. A
  // live re-query made right at that moment can come back with nothing
  // to show, even though the poller below saw the real pair a second
  // earlier — reportFailureDiagnosis() falls back to this rather than
  // wrongly concluding no candidates were ever exchanged at all.
  var lastKnownPairs = null;
  function pairSummary(p) {
    return p.local + ' <-> ' + p.remote + ': ' + p.state +
      (p.nominated ? ' nominated' : '') + ' (req ' + p.requestsSent + '/resp ' + p.responsesReceived + ')';
  }
  function startStatsPoll() {
    lastPairsKey = null;
    lastKnownPairs = null;
    statsPollTimer = setInterval(function () {
      if (typeof window.__blipNetStats !== 'function') return;
      window.__blipNetStats().then(function (pairs) {
        if (!pairs || !pairs.length) return;
        lastKnownPairs = pairs;
        var key = pairs.map(pairSummary).join('|');
        if (key === lastPairsKey) return;
        lastPairsKey = key;
        pairs.forEach(function (p) {
          termPrint('candidate-pair ' + pairSummary(p), p.state === 'succeeded' ? 'ok' : (p.state === 'failed' ? 'err' : undefined));
        });
      });
    }, 1500);
  }
  function stopStatsPoll() {
    if (statsPollTimer) { clearInterval(statsPollTimer); statsPollTimer = null; }
    lastPairsKey = null;
  }

  // Just so a long, quiet "checking" doesn't read as a frozen dialog — a
  // periodic "still here" line counting down to blip_net.js's own
  // CONNECT_TIMEOUT_MS.
  var heartbeatTimer = null;
  var connectDeadline = 0;
  function startHeartbeat() {
    var timeoutMs = (window.BlipNet && window.BlipNet.CONNECT_TIMEOUT_MS) || 60000;
    connectDeadline = Date.now() + timeoutMs;
    heartbeatTimer = setInterval(function () {
      var remaining = Math.max(0, Math.round((connectDeadline - Date.now()) / 1000));
      termPrint('still waiting for a connection — ' + remaining + 's until timeout');
    }, 5000);
  }
  function stopHeartbeat() {
    if (heartbeatTimer) { clearInterval(heartbeatTimer); heartbeatTimer = null; }
  }

  function startTelemetry() {
    stopTelemetry();
    startDebugPoll();
    startStatsPoll();
    startHeartbeat();
  }
  function stopTelemetry() {
    stopDebugPoll();
    stopStatsPoll();
    stopHeartbeat();
  }

  /** Printed once, right when a connection attempt gives up ('failed' or
   * 'timeout') — turns whatever candidate-pair stats are left into an
   * actual explanation instead of leaving "signal: timeout" to speak for
   * itself. Grounded in getStats() numbers, not a guess: a pair that sent
   * connectivity-check requests and got zero responses back means
   * something between the two devices is dropping that traffic — the two
   * indistinguishable-from-JS causes are a router's "client/AP isolation"
   * setting (blocks devices from reaching each other directly; common on
   * guest/public WiFi) and an OS-level firewall on either device blocking
   * inbound UDP, so both get named rather than picking one. */
  function reportFailureDiagnosis() {
    if (typeof window.__blipNetStats !== 'function') { termPrint('no getStats() available for a diagnosis', 'err'); return; }
    window.__blipNetStats().then(function (pairs) {
      if (!pairs || !pairs.length) pairs = lastKnownPairs; // see lastKnownPairs' own comment above
      if (!pairs || !pairs.length) {
        termPrint('diagnosis: no ICE candidate pairs ever formed — the two devices never received usable candidates from each other (a signaling problem, not a network one).', 'err');
        return;
      }
      var succeeded = pairs.some(function (p) { return p.state === 'succeeded' || p.nominated; });
      if (succeeded) {
        termPrint('diagnosis: a candidate pair actually connected — whatever failed came after that (the DataChannel itself, most likely). Worth just trying again.', 'err');
        return;
      }
      var checked = pairs.filter(function (p) { return p.requestsSent > 0; });
      var neverResponded = checked.filter(function (p) { return p.responsesReceived === 0; });
      if (checked.length && neverResponded.length === checked.length) {
        termPrint('diagnosis: ' + neverResponded.length + '/' + pairs.length +
          ' candidate pair(s) sent connectivity checks and got zero responses back — ' +
          'something between the two devices is dropping that traffic.', 'err');
        termPrint('likely cause: the WiFi’s "client/AP isolation" setting (blocks devices from reaching each other directly — common on guest/public networks), or an OS-level firewall on either phone blocking inbound UDP.', 'err');
        return;
      }
      termPrint('diagnosis: ' + pairs.length + ' candidate pair(s) tried, none succeeded (' +
        pairs.map(function (p) { return p.state; }).join(', ') + ') — the devices could not reach each other on this network.', 'err');
    });
  }

  function signalKind(s) {
    if (s === 'connected') return 'ok';
    if (s === 'failed' || s === 'timeout') return 'err';
    return undefined;
  }

  // ---- modal shell -----------------------------------------------------------

  // Dismiss the modal WITHOUT touching the connection — the success path
  // (a match just connected and is about to start) needs the DataChannel
  // to survive the modal closing, not get torn down by it.
  function dismissModal() {
    stopTelemetry();
    if (modalEl) { modalEl.remove(); modalEl = null; }
    if (stopActiveScan) { stopActiveScan(); stopActiveScan = null; }
  }
  // The CLOSE button / an abort: dismiss AND actually cancel — there's no
  // live connection worth keeping in either case (canceling is a no-op if
  // nothing is connected yet).
  function closeModal() {
    dismissModal();
    window.BlipNet.cancel();
  }

  function openModal() {
    if (modalEl) return;
    modalEl = el('div', 'blip-hs-modal blip-net-modal', document.body);
    var panel = el('div', 'blip-hs-panel', modalEl);
    el('div', 'blip-hs-title', panel).textContent = 'JACK IN';

    var body = el('div', '', panel);
    modalBody = body;
    modalPanel = panel;
    showChoice(body, panel);

    var row = el('div', 'blip-hs-row', panel);
    var close = el('button', 'blip-hs-btn ghost', row);
    close.type = 'button';
    close.textContent = 'CLOSE';
    close.addEventListener('click', closeModal);

    buildTerminal(modalEl);
    termReset();
    termPrint('jacking in..');
  }

  function showChoice(body, panel) {
    clear(body);
    var row = el('div', 'blip-hs-row', body);
    var hostBtn = el('button', 'blip-hs-btn', row);
    hostBtn.type = 'button';
    hostBtn.textContent = 'HOST';
    hostBtn.addEventListener('click', function () { showHostQR(body, panel); });

    var joinBtn = el('button', 'blip-hs-btn', row);
    joinBtn.type = 'button';
    joinBtn.textContent = 'JOIN';
    joinBtn.addEventListener('click', function () { showJoinQR(body, panel); });
  }

  function qrCanvas(parent) {
    // Classed (not just typed) so test/multiplayer.mjs and the scan
    // preview's own overlay canvas below can never be confused for one
    // another — both are `<canvas>` elements inside the same panel.
    var canvas = el('canvas', 'blip-qr-canvas', parent);
    // Neutralize shell.css's bare `canvas { position:fixed; clip-path:...
    // }` rule — meant only for the game's own #glcanvas, but a bare tag
    // selector catches every canvas on the page, including this one.
    // `max-width:100%` + `height:auto` (the canvas's own width/height
    // *attributes*, set by render() below, are always square, so this
    // keeps it square) caps growth back down on a phone too narrow to fit
    // 280px inside the panel's own padding, rather than overflowing it.
    canvas.style.cssText =
      'display:block;position:static;top:auto;left:auto;transform:none;clip-path:none;' +
      'touch-action:auto;margin:10px auto;width:280px;max-width:100%;height:auto;' +
      'image-rendering:pixelated;border-radius:4px;';
    return canvas;
  }

  /** Turn a getUserMedia() rejection into a specific, actionable message
   * instead of one generic "Camera unavailable". */
  function scanErrorMessage(err) {
    var name = err && err.name;
    if (name === 'NotAllowedError' || name === 'PermissionDeniedError') {
      return 'Camera permission denied — allow camera access for this site (check the address bar / browser settings) and try again.';
    }
    if (name === 'NotFoundError' || name === 'DevicesNotFoundError') return 'No camera found on this device.';
    if (name === 'NotReadableError') return 'Camera is busy or unavailable — another app may be using it.';
    if (name === 'OverconstrainedError') return 'No camera on this device matched what was requested.';
    return 'Camera unavailable' + (err && err.message ? ': ' + err.message : '') + '.';
  }

  function showHostQR(body, panel) {
    clear(body);
    var canvas = qrCanvas(body);
    termSetUser('host@blip');
    termPrint('blip-pair --host --signal=qr', 'cmd');
    termPrint('generating local session description...');
    startTelemetry();

    window.BlipNet.host(function (offerSdp) {
      window.BlipQR.render(canvas, offerSdp);
      var candidates = (offerSdp.match(/a=candidate:/g) || []).length;
      termPrint('offer ready: ' + offerSdp.length + ' bytes, ' + candidates + ' ice candidate(s)', 'ok');
      termPrint('qr rendered — waiting for peer to scan it');
      showScanButton(body, panel, 'SCAN THEIR ANSWER', function (text) {
        termPrint('answer scanned: ' + text.length + ' bytes', 'ok');
        termPrint('applying remote description...');
        window.BlipNet.submitAnswer(text);
      });
    }, function (s) {
      termPrint('signal: ' + s, signalKind(s));
      if (s === 'connected') { stopTelemetry(); setTimeout(dismissModal, 600); return; }
      if (s === 'failed' || s === 'timeout') {
        reportFailureDiagnosis();
        stopTelemetry();
        setTimeout(function () { if (modalEl) showChoice(body, panel); }, 1500);
      }
    });
  }

  function showJoinQR(body, panel) {
    clear(body);
    termSetUser('guest@blip');
    termPrint('blip-pair --join --signal=qr', 'cmd');
    startTelemetry();
    showScanButton(body, panel, 'SCAN HOST’S CODE', function (offerSdp) {
      var candidates = (offerSdp.match(/a=candidate:/g) || []).length;
      termPrint('offer scanned: ' + offerSdp.length + ' bytes, ' + candidates + ' ice candidate(s)', 'ok');
      clear(body);
      var canvas = qrCanvas(body);
      termPrint('generating answer...');
      window.BlipNet.join(offerSdp, function (answerSdp) {
        window.BlipQR.render(canvas, answerSdp);
        termPrint('answer ready: ' + answerSdp.length + ' bytes — show it to your host', 'ok');
      }, function (s) {
        termPrint('signal: ' + s, signalKind(s));
        if (s === 'connected') { stopTelemetry(); setTimeout(dismissModal, 600); return; }
        if (s === 'failed' || s === 'timeout') {
          reportFailureDiagnosis();
          stopTelemetry();
          setTimeout(function () { if (modalEl) showChoice(body, panel); }, 1500);
        }
      });
    }, true);
  }

  var stopActiveScan = null;

  /** A button that, once tapped, opens the camera and scans for one QR
   * code — `onScanned(text)` fires once, after which the camera stops
   * itself. `autoStart` skips the button and opens the camera immediately
   * (the guest's very first step has nothing else to tap first). */
  function showScanButton(body, panel, label, onScanned, autoStart) {
    var holder = el('div', '', body);
    function startScanning() {
      clear(holder);
      // max-width:100% safety net — the box shrinks on a narrow phone
      // rather than overflowing the panel.
      var wrap = el('div', '', holder);
      wrap.style.cssText = 'position:relative;width:260px;max-width:100%;aspect-ratio:1/1;margin:10px auto;';
      var video = el('video', '', wrap);
      video.setAttribute('playsinline', '');
      video.setAttribute('muted', '');
      video.muted = true;
      video.style.cssText = 'display:block;position:absolute;inset:0;width:100%;height:100%;object-fit:cover;border-radius:4px;background:#000;';
      // Drawn on top of the video by blip_qr.js's scan() — a green box on
      // a confirmed decode, yellow circles on whatever its finder-pattern
      // heuristic currently thinks might be a code. Classed apart from
      // .blip-qr-canvas (the rendered-code canvas) so nothing querying
      // for one can pick up the other.
      var overlay = el('canvas', 'blip-scan-overlay', wrap);
      overlay.style.cssText = 'display:block;position:absolute;inset:0;width:100%;height:100%;pointer-events:none;';
      // wrap's size can shrink below 260px on a narrow phone (max-width:
      // 100% above) — size the overlay's own bitmap to match whatever it
      // actually rendered at, not the 260px we asked for, or
      // blip_qr.js's marker math (which maps a point onto `overlay.width`
      // / `overlay.height`) would place every marker off by the
      // difference.
      var wrapRect = wrap.getBoundingClientRect();
      overlay.width = Math.max(1, Math.round(wrapRect.width));
      overlay.height = Math.max(1, Math.round(wrapRect.height));
      // Still a real error surface (not hand-holding prose) — kept for
      // test/multiplayer.mjs, which reads it to detect a failed scan.
      var err = el('div', 'blip-hs-err', holder);

      termPrint('camera: opening (facingMode=environment, 1920x1920 ideal)');
      var lastCandidates = -1;
      stopActiveScan = window.BlipQR.scan(video, overlay, function (text, scanErr) {
        stopActiveScan = null;
        if (scanErr) {
          var msg = scanErrorMessage(scanErr);
          err.textContent = msg;
          termPrint((scanErr && scanErr.name ? scanErr.name + ': ' : '') + msg, 'err');
          return;
        }
        termPrint('decode OK', 'ok');
        clear(holder);
        onScanned(text);
      }, function (state, detail) {
        switch (state) {
          case 'streaming':
            termPrint('camera stream acquired' + (detail.width ? ' (' + detail.width + 'x' + detail.height + ')' : ''), 'ok');
            break;
          case 'scanning': {
            var changed = detail.candidates !== lastCandidates;
            var heartbeat = detail.frames % 40 === 0;
            if (changed || heartbeat) {
              lastCandidates = detail.candidates;
              termPrint('frame ' + detail.frames + ': ' +
                (detail.candidates > 0 ? detail.candidates + ' candidate pattern(s) in view' : 'no pattern detected'),
                detail.candidates > 0 ? 'ok' : undefined);
            }
            if (detail.decodeError) termPrint('decode exception: ' + detail.decodeError, 'err');
            break;
          }
          case 'play-error':
            termPrint('camera stream would not play' + (detail.message ? ': ' + detail.message : ''), 'err');
            break;
          case 'unsupported':
            termPrint('getUserMedia unavailable in this browser', 'err');
            break;
          default:
            break;
        }
      });
    }
    if (autoStart) {
      startScanning();
    } else {
      var btn = el('button', 'blip-hs-btn', holder);
      btn.type = 'button';
      btn.textContent = label;
      btn.addEventListener('click', function () { startScanning(); }, { once: true });
    }
  }

  window.addEventListener('DOMContentLoaded', function () {
    var btn = document.getElementById('net-play-btn');
    if (btn) btn.addEventListener('click', openModal);
  });
}());
