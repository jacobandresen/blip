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
 * just the word. Every raw status/state value is printed alongside a
 * plain-language line, not instead of it (signalText(), pairSummary()),
 * and describeConnectivity() answers "do we know anything yet?"/"does
 * this look like the network is blocking us?" continuously while a
 * connection is stuck (the heartbeat, every 5s) and on demand (the
 * `stats` command) — not just once it's too late to matter, at the very
 * end (reportFailureDiagnosis()). See termPrint() below.
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
          // The same plain-language read the heartbeat prints every 5s
          // while waiting — on demand here too, so `stats` answers "are
          // we missing info?" / "is the network blocking us?" directly
          // instead of leaving the reader to interpret req/resp numbers.
          var info = describeConnectivity(pairs);
          termPrint(info.text, info.kind, true);
          pairs.forEach(function (p) {
            termPrint('candidate-pair ' + pairSummary(p), p.state === 'succeeded' ? 'ok' : (p.state === 'failed' ? 'err' : undefined), true);
          });
        });
      },
    },
    check: {
      desc: 'check the connection right now (pings the peer if connected)',
      run: function () {
        checkConnection(function (text, kind) { termPrint(text, kind, true); });
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

  /** A live, one-line, plain-language read of "what do we actually know
   * right now" from whatever candidate-pair stats exist — the same two
   * questions worth asking the whole time a connection is stuck, not
   * just once it's given up: do we have anything to go on yet, and does
   * what we have look like the network is blocking this. Shared between
   * the heartbeat below (prints it every 5s while waiting) and
   * reportFailureDiagnosis() (the fuller version once an attempt
   * actually gives up), so the two can never disagree with each other —
   * one honest classification, read at two different times. */
  function describeConnectivity(pairs) {
    if (!pairs || !pairs.length) {
      return { text: 'no connectivity info yet (still exchanging candidates)', kind: undefined };
    }
    var succeeded = pairs.some(function (p) { return p.state === 'succeeded' || p.nominated; });
    if (succeeded) return { text: 'a route was found, connecting…', kind: 'ok' };
    var checked = pairs.filter(function (p) { return p.requestsSent > 0; });
    var neverResponded = checked.filter(function (p) { return p.responsesReceived === 0; });
    if (checked.length && neverResponded.length === checked.length) {
      return {
        text: neverResponded.length + '/' + pairs.length + ' route(s) tried, 0 responses back — ' +
          'looks like something is blocking device-to-device traffic',
        kind: 'err',
      };
    }
    return { text: pairs.length + ' route(s) still being checked…', kind: undefined };
  }

  // Just so a long, quiet "checking" doesn't read as a frozen dialog — a
  // periodic "still here" line counting down to blip_net.js's own
  // CONNECT_TIMEOUT_MS, alongside describeConnectivity()'s live read of
  // what's actually known so far (updates as ICE learns more; reads "no
  // connectivity info yet" until the first candidate pair shows up).
  var heartbeatTimer = null;
  var connectDeadline = 0;
  function startHeartbeat() {
    var timeoutMs = (window.BlipNet && window.BlipNet.CONNECT_TIMEOUT_MS) || 60000;
    connectDeadline = Date.now() + timeoutMs;
    heartbeatTimer = setInterval(function () {
      var remaining = Math.max(0, Math.round((connectDeadline - Date.now()) / 1000));
      var info = describeConnectivity(lastKnownPairs);
      termPrint(remaining + 's until timeout — ' + info.text, info.kind);
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
   * itself, built on the exact same classification describeConnectivity()
   * has been printing every 5s of the wait (so this is never a surprise —
   * "no info yet"/"looks blocked" should already have shown up in the
   * heartbeat above whatever this line ends up saying). Grounded in
   * getStats() numbers, not a guess: a pair that sent connectivity-check
   * requests and got zero responses back means something between the two
   * devices is dropping that traffic — several real causes look
   * identical from here (a router's "client/AP isolation" setting, a
   * VPN active on either device routing "local" traffic through a
   * remote tunnel instead, an OS-level firewall blocking inbound UDP),
   * so all of them get named rather than guessing at just one. */
  function reportFailureDiagnosis() {
    if (typeof window.__blipNetStats !== 'function') { termPrint('diagnosis: no getStats() available to explain this', 'err'); return; }
    window.__blipNetStats().then(function (pairs) {
      if (!pairs || !pairs.length) pairs = lastKnownPairs; // see lastKnownPairs' own comment above
      if (!pairs || !pairs.length) {
        termPrint('diagnosis: no ICE candidate pairs ever formed — missing info, not a network block. ' +
          'The two devices never received usable candidates from each other in the first place (a signaling problem).', 'err');
        return;
      }
      var info = describeConnectivity(pairs);
      if (info.kind === 'ok') {
        termPrint('diagnosis: a candidate pair actually connected — whatever failed came after that (the DataChannel itself, most likely). Worth just trying again.', 'err');
        return;
      }
      if (info.kind === 'err') {
        termPrint('diagnosis: ' + info.text + '.', 'err');
        termPrint('likely cause: the WiFi’s "client/AP isolation" setting (blocks devices from reaching each other directly — common on guest/public networks), a VPN active on either device (routes "local" traffic through a remote tunnel instead — easy to overlook, worth turning off and retrying), or an OS-level firewall on either device blocking inbound UDP.', 'err');
        return;
      }
      termPrint('diagnosis: ' + pairs.length + ' candidate pair(s) tried, none succeeded (' +
        pairs.map(function (p) { return p.state; }).join(', ') + ') — the devices could not reach each other on this network.', 'err');
    });
  }

  /** The CHECK button (in the info bar under the QR/scan area) and the
   * `check` console command both call this — an on-demand answer to "can
   * I actually reach the other side right now", not just a status label.
   * Once the DataChannel is open it's a real ping/pong round trip
   * (window.BlipNet.ping(), blip_net.js) — proof the other page is still
   * there and responding, not just that WebRTC's own state says
   * 'connected'. Before that (still exchanging/checking ICE candidates)
   * there's no channel to ping yet, so it falls back to the same
   * describeConnectivity() read of getStats() the heartbeat already
   * prints — "do we know anything yet" / "does this look blocked" — just
   * fetched fresh instead of waiting for the next 5s tick.
   * `report(text, kind)` fires exactly once with the result. */
  function checkConnection(report) {
    report('checking…', undefined);
    var debug = typeof window.__blipNetDebug === 'function' ? window.__blipNetDebug() : null;
    if (debug && debug.dcState === 'open' && window.BlipNet && typeof window.BlipNet.ping === 'function') {
      window.BlipNet.ping(function (rtt, err) {
        if (rtt != null) report('peer reached — ' + Math.round(rtt) + 'ms round trip', 'ok');
        else report('no response from peer (' + err + ')', 'err');
      });
      return;
    }
    if (typeof window.__blipNetStats !== 'function') { report('nothing to check yet', undefined); return; }
    window.__blipNetStats().then(function (pairs) {
      var info = describeConnectivity(pairs);
      report(info.text, info.kind);
    });
  }

  function signalKind(s) {
    if (s === 'connected') return 'ok';
    if (s === 'failed' || s === 'timeout') return 'err';
    return undefined;
  }

  /** blip_net.js's raw status strings ('waiting', 'answering', …) are
   * exactly what a real WebRTC signaling flow calls these states, but
   * they don't say what to actually *do* about one — printed alongside
   * the raw value (never replacing it) so both the plain meaning and
   * the exact wire term are always on screen together. */
  function signalText(s) {
    switch (s) {
      case 'waiting': return 'waiting for the other device to scan this code';
      case 'answering': return 'code scanned — now connecting to the host';
      case 'connected': return 'connected!';
      case 'disconnected': return 'disconnected';
      case 'failed': return 'connection failed';
      case 'timeout': return 'gave up — nobody connected in time';
      default: return s;
    }
  }

  /** Print each `a=candidate:` line in `sdp` (an offer or answer, either
   * just generated or just scanned) as one readable line — real
   * connection info (type, protocol, address:port), not just the byte
   * count the surrounding "offer ready"/"offer scanned" line already
   * gives. Lets a real ICE candidate's address get eyeballed directly —
   * e.g. an mDNS-hidden `*.local` name, an unexpected IPv6 literal, or a
   * candidate on the wrong network entirely — right at the point it was
   * exchanged, without waiting for a `stats` candidate-pair line that
   * only exists once ICE has actually started checking it. */
  function printCandidates(sdp) {
    // Same parser slimSdpForQr() uses to decide what to keep — one
    // tested place for `a=candidate:` line syntax instead of two
    // untested, independently-drifting copies of it.
    var parseCandidateLine = window.BlipSdpSlim && window.BlipSdpSlim.parseCandidateLine;
    if (typeof parseCandidateLine !== 'function') return;
    var any = false;
    sdp.split(/\r\n|\n/).forEach(function (line) {
      var candidate = parseCandidateLine(line);
      if (!candidate) return;
      any = true;
      termPrint('  candidate: ' + (candidate.type || '?') + ' ' + (candidate.protocol || '?') + ' ' +
        (candidate.address || '?') + ':' + (candidate.port || '?'), undefined, true);
    });
    if (!any) termPrint('  (no candidates in this SDP)', 'err', true);
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

    // Outside `body` (not cleared by showChoice()/showHostQR()/
    // showJoinQR()) so it stays up across every step, not just the one
    // it happened to render during.
    var cameraWarn = el('div', 'blip-hs-err', panel);
    var thisModal = modalEl;
    checkForCamera(function (hasCamera) {
      if (modalEl !== thisModal) return; // closed (or reopened) while this was in flight
      if (hasCamera) return;
      cameraWarn.textContent = 'no camera detected on this device — HOST and JOIN both need one ' +
        '(each side scans the other’s code back)';
      if (termLogEl) termPrint('no camera detected on this device (enumerateDevices found no videoinput)', 'err', true);
    });

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

  /** The QR/scan square is the single biggest thing in the panel — sizing
   * it off width alone (the old `max-width:100%` safety net) is fine on a
   * narrow *portrait* phone, but does nothing on a *landscape* or just
   * short window, where the real constraint is height: a fixed 280px
   * square plus the status bar/CLOSE row underneath it can overflow the
   * panel's own max-height (calc(100vh - 32px)) with nothing on screen to
   * suggest there's more below (the panel deliberately hides its own
   * scrollbar — see .blip-hs-panel in shell.css — so an overflow here
   * isn't just ugly, it's undiscoverable). Considering *both* budgets
   * directly in JS (rather than reaching for CSS `aspect-ratio` +
   * `max-height` tricks on a fixed-width box) keeps this simple and
   * exactly predictable. Recomputed fresh each time a QR/scan screen is
   * shown, not on resize — nobody rotates their phone mid-scan. Floor of
   * 140 — small, but a code held close to the camera is still real cargo
   * space for an SDP; see docs/multiplayer.md on why bigger is generally
   * better for an *old* camera specifically, when there's room for it. */
  function codeAreaSize(preferred) {
    var byHeight = window.innerHeight * 0.40;
    var byWidth = window.innerWidth * 0.78;
    return Math.max(140, Math.round(Math.min(preferred, byHeight, byWidth)));
  }

  function qrCanvas(parent) {
    // Classed (not just typed) so test/multiplayer.mjs and the scan
    // preview's own overlay canvas below can never be confused for one
    // another — both are `<canvas>` elements inside the same panel.
    var canvas = el('canvas', 'blip-qr-canvas', parent);
    // Neutralize shell.css's bare `canvas { position:fixed; clip-path:...
    // }` rule — meant only for the game's own #glcanvas, but a bare tag
    // selector catches every canvas on the page, including this one.
    var size = codeAreaSize(280);
    canvas.style.cssText =
      'display:block;position:static;top:auto;left:auto;transform:none;clip-path:none;' +
      'touch-action:auto;margin:10px auto;width:' + size + 'px;height:' + size + 'px;' +
      'image-rendering:pixelated;border-radius:4px;';
    return canvas;
  }

  /** A plain-language, glanceable status line under the QR/scan area —
   * classed apart from the terminal below it (.blip-net-term) so it's the
   * one thing you don't have to read tiny scrolling monospace to get:
   * did the code I just showed get scanned, did the one I just scanned
   * parse, are we connected. `set(text, kind)` ('ok'/'err'/'wait'/
   * undefined) updates it; a CHECK button on the right runs
   * checkConnection() on demand (a real ping/pong once connected, an ICE
   * stats read before that) and prints the result both here and to the
   * terminal, so tapping it never requires typing `check` at the prompt
   * to get the same answer. */
  function statusBar(parent, initialText, initialKind) {
    var wrap = el('div', 'blip-net-status', parent);
    var dot = el('span', 'blip-net-status-dot', wrap);
    var text = el('span', 'blip-net-status-text', wrap);
    var checkBtn = el('button', 'blip-net-status-check', wrap);
    checkBtn.type = 'button';
    checkBtn.textContent = 'CHECK';
    checkBtn.addEventListener('click', function () {
      checkBtn.disabled = true;
      checkConnection(function (t, kind) {
        checkBtn.disabled = false;
        set(t, kind);
        termPrint('check: ' + t, kind, true);
      });
    });
    function set(t, kind) {
      text.textContent = t;
      wrap.className = 'blip-net-status' + (kind ? ' ' + kind : '');
    }
    set(initialText, initialKind);
    return { set: set, el: wrap };
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

  /** Both HOST and JOIN eventually need a working camera — QR-only
   * signaling means the SDP travels both directions, so even the host
   * (after showing its own offer code) still has to scan the guest's
   * answer code back. There's no camera-free path through this feature
   * at all. Without this check, a device with no camera (a desktop with
   * no webcam, say) only finds that out several steps in, after already
   * tapping HOST or JOIN and a SCAN button — this answers it upfront,
   * before the user wastes a step on a pairing that can never complete.
   * `cb(true)` (assume a camera exists) if the check itself can't run —
   * a missed warning is a minor inconvenience; a *wrong* one would block
   * a real camera that just couldn't be enumerated this way. */
  function checkForCamera(cb) {
    if (!navigator.mediaDevices || typeof navigator.mediaDevices.enumerateDevices !== 'function') {
      cb(true);
      return;
    }
    navigator.mediaDevices.enumerateDevices()
      .then(function (devices) { cb(devices.some(function (d) { return d.kind === 'videoinput'; })); })
      .catch(function () { cb(true); });
  }

  function showHostQR(body, panel) {
    clear(body);
    var canvas = qrCanvas(body);
    var status = statusBar(body, 'generating your code…', 'wait');
    termSetUser('host@blip');
    termPrint('blip-pair --host --signal=qr', 'cmd');
    termPrint('generating local session description...');
    startTelemetry();

    window.BlipNet.host(function (offerSdp) {
      window.BlipQR.render(canvas, offerSdp);
      var candidates = (offerSdp.match(/a=candidate:/g) || []).length;
      termPrint('offer ready: ' + offerSdp.length + ' bytes, ' + candidates + ' ice candidate(s)', 'ok');
      printCandidates(offerSdp);
      termPrint('qr rendered — waiting for peer to scan it');
      status.set('showing code — waiting for the other device to scan it', 'wait');
      showScanButton(body, panel, 'SCAN THEIR ANSWER', function (text) {
        var candidates2 = (text.match(/a=candidate:/g) || []).length;
        termPrint('answer scanned: ' + text.length + ' bytes', 'ok');
        printCandidates(text);
        termPrint('applying remote description...');
        status.el.style.display = ''; // hidden by hideEls below, while the camera view had the screen
        status.set('✓ answer scanned (' + text.length + ' bytes, ' + candidates2 + ' route(s)) — connecting…', 'ok');
        window.BlipNet.submitAnswer(text);
      }, false, [canvas, status.el]);
    }, function (s) {
      termPrint(signalText(s) + ' (signal: ' + s + ')', signalKind(s));
      // 'waiting'/'answering' are just process states already covered by
      // the richer messages above (qr rendered / answer scanned) — only
      // the terminal outcomes are worth overwriting that with.
      if (s === 'connected' || s === 'failed' || s === 'timeout' || s === 'disconnected') {
        status.set(signalText(s), signalKind(s));
      }
      if (s === 'connected') {
        // Only ever printed once the DataChannel has actually opened —
        // never as a startup banner (that read as success before the
        // connection was even attempted, which is exactly backwards).
        termPrint('jacking in.. success', 'ok', true);
        stopTelemetry();
        setTimeout(dismissModal, 600);
        return;
      }
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
    var scanStatus = statusBar(body, 'point your camera at the host’s code', 'wait');
    showScanButton(body, panel, 'SCAN HOST’S CODE', function (offerSdp) {
      var candidates = (offerSdp.match(/a=candidate:/g) || []).length;
      termPrint('offer scanned: ' + offerSdp.length + ' bytes, ' + candidates + ' ice candidate(s)', 'ok');
      printCandidates(offerSdp);
      clear(body);
      var canvas = qrCanvas(body);
      var status = statusBar(body,
        '✓ host code scanned (' + offerSdp.length + ' bytes, ' + candidates + ' route(s)) — generating your code…', 'ok');
      termPrint('generating answer...');
      window.BlipNet.join(offerSdp, function (answerSdp) {
        window.BlipQR.render(canvas, answerSdp);
        termPrint('answer ready: ' + answerSdp.length + ' bytes — show it to your host', 'ok');
        printCandidates(answerSdp);
        status.set('showing your code — waiting for the host to scan it', 'wait');
      }, function (s) {
        termPrint(signalText(s) + ' (signal: ' + s + ')', signalKind(s));
        if (s === 'connected' || s === 'failed' || s === 'timeout' || s === 'disconnected') {
          status.set(signalText(s), signalKind(s));
        }
        if (s === 'connected') {
          termPrint('jacking in.. success', 'ok', true);
          stopTelemetry();
          setTimeout(dismissModal, 600);
          return;
        }
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
   * (the guest's very first step has nothing else to tap first).
   * `hideEls` (optional): elements to hide the moment scanning actually
   * starts — the host's "SCAN THEIR ANSWER" step reuses the same `body`
   * its own offer QR + status bar are still sitting in (useful right up
   * until this point, dead weight once the camera view takes over), and
   * on a short/landscape screen that's real vertical space worth taking
   * back rather than pushing the CLOSE button below the fold. */
  function showScanButton(body, panel, label, onScanned, autoStart, hideEls) {
    var holder = el('div', '', body);
    function startScanning() {
      if (hideEls) hideEls.forEach(function (e) { if (e) e.style.display = 'none'; });
      clear(holder);
      // codeAreaSize() (same helper qrCanvas() uses) accounts for a short/
      // landscape viewport, not just a narrow one — a fixed 260px square
      // here was the other half of the overflow this fixes.
      var wrap = el('div', '', holder);
      wrap.style.cssText = 'position:relative;width:' + codeAreaSize(260) + 'px;aspect-ratio:1/1;margin:10px auto;';
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
      // wrap's size can be less than 260px (codeAreaSize() above) — size
      // the overlay's own bitmap to match whatever it actually rendered
      // at, not the 260px we asked for, or blip_qr.js's marker math
      // (which maps a point onto `overlay.width`/`overlay.height`) would
      // place every marker off by the difference.
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
        // What was actually parsed, not just "OK" — the same byte/route
        // summary the caller (showHostQR/showJoinQR) prints again once it
        // knows whether this was an offer or an answer, but available
        // here immediately, right at the moment a candidate pattern
        // turned into real decoded data.
        var candidates = (text.match(/a=candidate:/g) || []).length;
        termPrint('decode OK — parsed ' + text.length + ' bytes, ' + candidates + ' ice candidate(s)', 'ok');
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
          case 'found':
            // The candidate pattern actually decoded, right here — the
            // 'decode OK' line above fires ~150ms later (blip_qr.js holds
            // the confirmation box on screen for a beat first), so this
            // is the frame it really happened on.
            termPrint('frame ' + detail.frames + ': candidate pattern decoded successfully', 'ok');
            break;
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
