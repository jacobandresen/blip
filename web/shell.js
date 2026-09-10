(function () {
'use strict';

// ---- Zoom lock ----------------------------------------------------------
// Every touch on a game page is game input, never a browser gesture.
// `body { touch-action: none }` (shell.css) already kills single-finger
// pan and double-tap zoom everywhere on the page — including a rapid mash
// on the fire button. The one thing that leaves open is iOS Safari's
// pinch: it ignores user-scalable=no and can start a page zoom from a
// two-finger 'gesture' (a thumb held on fire + a thumb on the d-pad
// drifting apart reads as one). Swallow those events outright.
['gesturestart', 'gesturechange', 'gestureend'].forEach(function (t) {
  document.addEventListener(t, function (e) { e.preventDefault(); }, { passive: false });
});

var TOPBAR_H   = 56;
var MARQUEE_H  = 28; // reserved header height once #marquee-bar exists
var PAD        = 16; // padding around the canvas on all sides (= bezel width)

var loader    = document.getElementById('loader');
var barInner  = document.getElementById('bar-inner');
var statusEl  = document.getElementById('status');
var canvas    = document.getElementById('glcanvas');
var overlay   = document.getElementById('need-coin-overlay');

updateCoinsHud();

// ---- Per-game marquee + cabinet accent ----
// Builds the backlit name sign in JS (rather than requiring it in every
// game's HTML) so rally's hand-maintained page picks it up for free, same as
// the shell-generated ones.
(function () {
  var game = (typeof blipGameFromPath === 'function') ? blipGameFromPath(window.location.pathname) : null;
  if (!game) return;
  document.documentElement.style.setProperty('--cab', game.accent);
  var bar = document.createElement('div');
  bar.id = 'marquee-bar';
  bar.innerHTML =
    '<span class="marquee-bulbs"></span>' +
    '<span id="marquee-name">' + game.name + '</span>' +
    '<span class="marquee-bulbs"></span>';
  document.body.insertBefore(bar, document.body.firstChild);

  var logo = document.querySelector('.blip-logo');
  if (logo) {
    logo.classList.add('boot');
    logo.addEventListener('animationend', function () {
      logo.classList.remove('boot');
    }, { once: true });

    // After ~12s of no input, a "> MORE GAMES" nudge fades in under the
    // logo (.logo-hint, styled in shell.css) — the logo is the only way
    // back to the cabinet's game grid on touch, so the reminder shouldn't
    // be a slow one. Any input hides it and restarts the clock:
    // pointer/touch for taps, and keydown in the capture phase so it also
    // catches the on-screen controls and gamepad (injectKey() dispatches
    // bubbling keydowns on the canvas).
    var hint = document.createElement('span');
    hint.className = 'logo-hint';
    hint.setAttribute('aria-hidden', 'true');
    hint.textContent = '> MORE GAMES';
    logo.appendChild(hint);
    var hintTimer = null;
    function hintIdle() {
      logo.classList.remove('show-hint');
      clearTimeout(hintTimer);
      hintTimer = setTimeout(function () { logo.classList.add('show-hint'); }, 12000);
    }
    ['pointerdown', 'touchstart'].forEach(function (ev) {
      window.addEventListener(ev, hintIdle, { passive: true });
    });
    document.addEventListener('keydown', hintIdle, true);
    hintIdle();
    // a finished game is the moment you're most likely to want out — surface
    // the way back right away instead of waiting the idle-out.
    window.addEventListener('blip-game-over', function () {
      clearTimeout(hintTimer);
      hintTimer = setTimeout(function () { logo.classList.add('show-hint'); }, 2500);
    });
  }
}());

// Stop all audio when navigating away (pagehide is reliable on iOS Safari / PWA)
window.addEventListener('pagehide', function () {
  if (typeof Howler !== 'undefined') Howler.stop();
});

// ---- UI audio (coin insert / no room) ----

var uiAudio = null;
function getUiAudio() {
  if (typeof Howler !== 'undefined' && Howler.ctx) {
    if (Howler.ctx.state === 'suspended') Howler.ctx.resume();
    return Howler.ctx;
  }
  if (!uiAudio) uiAudio = new (window.AudioContext || window.webkitAudioContext)();
  if (uiAudio.state === 'suspended') uiAudio.resume();
  return uiAudio;
}

// ---- Touch control feedback ----
// A felt "detent" the instant a control catches, so you know it registered
// without looking down from the game. Android gets a real Vibration-API
// buzz. iOS Safari has no Vibration API (and never has) and no web access
// to the Taptic Engine, so there it falls back to a ~40 ms sub-bass burst
// through the speaker — more felt-in-the-hand than heard. Only ever fires
// on a touch device; a no-op under a mouse or on the coin overlay.
var HAS_TOUCH = ('ontouchstart' in window) || navigator.maxTouchPoints > 0;
function feedbackTick() {
  if (!HAS_TOUCH || overlay.classList.contains('visible')) return;
  if (navigator.vibrate) {
    try { if (navigator.vibrate(7)) return; } catch (e) {}
  }
  try {
    var ctx = getUiAudio();
    if (!ctx) return;
    var t = ctx.currentTime;
    var osc = ctx.createOscillator(), gain = ctx.createGain();
    osc.type = 'sine';
    osc.frequency.setValueAtTime(150, t);
    osc.frequency.exponentialRampToValueAtTime(55, t + 0.026);
    gain.gain.setValueAtTime(0.0001, t);
    gain.gain.exponentialRampToValueAtTime(0.2, t + 0.004);
    gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.05);
    osc.connect(gain); gain.connect(ctx.destination);
    osc.start(t); osc.stop(t + 0.055);
  } catch (e) {}
}
// A real coin makes two distinct sounds in sequence: the metallic clink
// of it dropping through the chute (bright, inharmonic, near-instant —
// real metal doesn't ring in tidy octaves the way a synth voice does),
// then the register's own electronic "credit accepted" chime a beat
// later. Modelling both, rather than just the chime alone, is what reads
// as an actual coin rather than a UI beep.
function playCoinInsert() {
  var ctx = getUiAudio(), t = ctx.currentTime;

  // The clink: filtered noise for the transient "tik" of metal on metal,
  // plus a few short inharmonic tones for the coin's own brief ring.
  var noiseBuf = ctx.createBuffer(1, Math.ceil(ctx.sampleRate * 0.045), ctx.sampleRate);
  var noiseData = noiseBuf.getChannelData(0);
  for (var i = 0; i < noiseData.length; i++) noiseData[i] = Math.random() * 2 - 1;
  var noise = ctx.createBufferSource();
  noise.buffer = noiseBuf;
  var noiseFilter = ctx.createBiquadFilter();
  noiseFilter.type = 'bandpass';
  noiseFilter.frequency.value = 4200;
  noiseFilter.Q.value = 1.1;
  var noiseGain = ctx.createGain();
  noiseGain.gain.setValueAtTime(0.4, t);
  noiseGain.gain.exponentialRampToValueAtTime(0.001, t + 0.045);
  noise.connect(noiseFilter); noiseFilter.connect(noiseGain); noiseGain.connect(ctx.destination);
  noise.start(t); noise.stop(t + 0.045);

  [3000, 4550, 6100].forEach(function (freq, i) {
    var osc = ctx.createOscillator(), gain = ctx.createGain();
    osc.type = 'triangle'; osc.frequency.value = freq;
    osc.connect(gain); gain.connect(ctx.destination);
    var start = t + i * 0.006;
    gain.gain.setValueAtTime(0.16 / (i + 1), start);
    gain.gain.exponentialRampToValueAtTime(0.0008, start + 0.1);
    osc.start(start); osc.stop(start + 0.11);
  });

  // The credit chime, arriving just after the coin lands.
  [{ freq: 1047, start: 0.1 }, { freq: 1319, start: 0.155 }].forEach(function (note) {
    var osc = ctx.createOscillator(), gain = ctx.createGain();
    osc.connect(gain); gain.connect(ctx.destination);
    osc.type = 'square'; osc.frequency.value = note.freq;
    gain.gain.setValueAtTime(0.22, t + note.start);
    gain.gain.exponentialRampToValueAtTime(0.001, t + note.start + 0.11);
    osc.start(t + note.start); osc.stop(t + note.start + 0.12);
  });
}

// The coin that visually drops into #insert-coin-btn's slot on a
// successful insert — a real element (not a pseudo-element) so a class
// toggle can animate it on demand, injected here rather than duplicated
// across shell.html and all six per-game index.html files.
var coinDropAnim = null;
(function () {
  var btn = document.getElementById('insert-coin-btn');
  if (!btn) return;
  coinDropAnim = document.createElement('span');
  coinDropAnim.id = 'coin-drop-anim';
  coinDropAnim.setAttribute('aria-hidden', 'true');
  btn.appendChild(coinDropAnim);
}());
function dropCoinAnimation() {
  if (!coinDropAnim) return;
  // The slot (#insert-coin-btn::after) sits flush against the button's
  // content-box edge, so its centre is padding-right + half its own 6px
  // width in from the button's outer edge — read padding back from the
  // computed style (it changes across the responsive breakpoints) rather
  // than guessing a constant, so the coin actually lands on the slot
  // instead of just somewhere near it.
  var btn = coinDropAnim.parentElement;
  var padRight = parseFloat(getComputedStyle(btn).paddingRight) || 8;
  coinDropAnim.style.setProperty('--slot-x', (padRight + 3) + 'px');
  coinDropAnim.classList.remove('dropping');
  void coinDropAnim.offsetWidth;
  coinDropAnim.classList.add('dropping');
  coinDropAnim.addEventListener('animationend', function () {
    coinDropAnim.classList.remove('dropping');
  }, { once: true });
}
function playNoRoom() {
  var ctx = getUiAudio(), t = ctx.currentTime;
  var osc = ctx.createOscillator(), gain = ctx.createGain();
  osc.connect(gain); gain.connect(ctx.destination);
  osc.type = 'sawtooth';
  osc.frequency.setValueAtTime(200, t);
  osc.frequency.exponentialRampToValueAtTime(65, t + 0.38);
  gain.gain.setValueAtTime(0.32, t);
  gain.gain.exponentialRampToValueAtTime(0.001, t + 0.38);
  osc.start(t); osc.stop(t + 0.39);
}
function flashCoinBar() {
  ['insert-coin-btn'].forEach(function (id) {
    var el = document.getElementById(id);
    if (!el) return;
    el.classList.remove('coin-flash');
    void el.offsetWidth;
    el.classList.add('coin-flash');
    el.addEventListener('animationend', function () { el.classList.remove('coin-flash'); }, { once: true });
  });
}

// Called from WASM on game-over restart — spend a coin or block if empty.
window.blipSpendCoin = function () {
  var n = getCoins();
  if (n <= 0) {
    overlay.classList.add('visible');
    return 0;
  }
  saveCoins(n - 1);
  updateCoinsHud();
  return 1;
};

// Called from WASM the moment a game ends (game-over or won). Hands the
// final score to the shared high-score board; a no-op beyond updating the
// local best if no backend is configured (web/blip_config.js).
window.blipGameOver = function (score) {
  window.dispatchEvent(new Event('blip-game-over'));   // nudges the "MORE GAMES" hint
  if (!window.blipScores) return;
  var game = (typeof blipGameFromPath === 'function')
    ? blipGameFromPath(window.location.pathname) : null;
  window.blipScores.onGameOver(game ? game.slug : null, score);
};

// Called from WASM (title / game-over screens) to show the leading score
// and who holds it. Prefers the leaderboard #1 that blip_scores.js caches
// in 'blip-top-<slug>'; falls back to this browser's own best
// ('blip-best-<slug>') tagged with the claimed handle ('blip-handle').
function blipTopEntry() {
  var game = (typeof blipGameFromPath === 'function')
    ? blipGameFromPath(window.location.pathname) : null;
  if (!game) return null;
  try {
    var raw = localStorage.getItem('blip-top-' + game.slug);
    if (raw) {
      var o = JSON.parse(raw);
      if (o && (o.score | 0) > 0) return { score: o.score | 0, name: o.handle || '' };
    }
  } catch (e) {}
  try {
    var best = parseInt(localStorage.getItem('blip-best-' + game.slug) || '0', 10) || 0;
    if (best > 0) return { score: best, name: localStorage.getItem('blip-handle') || '' };
  } catch (e) {}
  return null;
}
window.blipHighScore = function () {
  var t = blipTopEntry();
  return t ? t.score : 0;
};
window.blipHighName = function () {
  var t = blipTopEntry();
  return t && t.name ? String(t.name).toUpperCase().slice(0, 14) : '';
};

// Called from WASM when game mode is chosen on the title screen.
// mode 0 = 1-player (CPU controls right paddle), 1 = 2-player.
window.blipSetMode = function (mode) {
  var d = document.getElementById('paddle-dial-p2');
  if (!d) return;
  var isCpu = (mode === 0);
  d.classList.toggle('cpu-mode', isCpu);
  var label = document.getElementById('dial-label-p2');
  if (label) label.textContent = isCpu ? 'CPU' : '2P';
};

// Called from WASM (rally) every frame with the two paddle positions
// (0 = top of travel … 1 = bottom). Spins the on-screen dials to match —
// so they turn under keyboard play on the desktop, and the P2 dial turns
// on its own while the CPU plays it.
(function () {
  var h1 = document.getElementById('dial-hand');
  var h2 = document.getElementById('dial-hand-p2');
  // Full paddle travel ≈ 1½ turns of the knob, geared like a real spinner.
  var SWEEP = 3 * Math.PI;
  window.blipPaddles = function (left, right) {
    if (h1) h1.style.transform = 'rotate(' + ((left  - 0.5) * SWEEP) + 'rad)';
    if (h2) h2.style.transform = 'rotate(' + ((right - 0.5) * SWEEP) + 'rad)';
  };
}());

overlay.addEventListener('click', function () {
  var n = getCoins();
  if (n >= MAX_COINS) return;
  saveCoins(n + 1);
  updateCoinsHud();
  playCoinInsert();
  dropCoinAnimation();
  flashCoinBar();
  overlay.classList.remove('visible');
});

document.getElementById('insert-coin-btn').addEventListener('click', function () {
  var n = getCoins();
  if (n >= MAX_COINS) {
    playNoRoom();
    var btn = this;
    btn.classList.remove('shake');
    void btn.offsetWidth;
    btn.classList.add('shake');
    btn.addEventListener('animationend', function () { btn.classList.remove('shake'); }, { once: true });
    return;
  }
  saveCoins(n + 1);
  updateCoinsHud();
  playCoinInsert();
  dropCoinAnimation();
  flashCoinBar();
  overlay.classList.remove('visible');
});

// Mirror the overlay's visibility onto <body> so CSS can key off it
// (body.need-coin flashes the top-right coin button, kiosk.css/shell.css)
// without every show/hide site having to know. Also: the moment the
// overlay closes (a coin just went in), hand keyboard focus back to the
// game canvas so the title screen's "PRESS ANY KEY" responds to a real
// keypress — the coin click/tap left focus on the button or the body.
var overlayWasVisible = overlay.classList.contains('visible');
new MutationObserver(function () {
  var vis = overlay.classList.contains('visible');
  document.body.classList.toggle('need-coin', vis);
  if (overlayWasVisible && !vis && canvas) {
    try { canvas.focus(); } catch (e) {}
  }
  overlayWasVisible = vis;
}).observe(overlay, { attributes: true, attributeFilter: ['class'] });

// Landing on a game page with no credits — a fresh session, a shared deep
// link — is walking up to a cold cabinet: you put a coin in before you
// play. Same overlay the in-game "continue?" prompt uses; it blocks
// injectKey() until a coin goes in, so the title screen can't be started
// for free.
if (getCoins() <= 0) overlay.classList.add('visible');

// (No ambient cabinet hum on a game page — while a game is running its
// own audio is the whole soundscape. The transformer drone lives only on
// the landing page, index.html, where the machine is idling in attract.)

// ---- Canvas sizing ----
// The kiosk bar is position:fixed;bottom:0. We leave PAD px on each side
// plus full clearance for the bar so it never overlaps the canvas.

function fillCanvas() {
  var tb = document.getElementById('topbar');
  // clamp to 56 so a mis-read of 0 before layout doesn't eat the whole screen
  TOPBAR_H = tb ? Math.max(tb.offsetHeight, 56) : 56;
  var mb = document.getElementById('marquee-bar');
  MARQUEE_H = mb ? Math.max(mb.offsetHeight, 28) : 0;
  var w = window.innerWidth  - PAD * 2;
  var h = window.innerHeight - TOPBAR_H - MARQUEE_H - PAD * 2;
  canvas.style.setProperty('width',  w + 'px', 'important');
  canvas.style.setProperty('height', h + 'px', 'important');
  canvas.style.setProperty('top',    (MARQUEE_H + PAD) + 'px', 'important');
  canvas.style.setProperty('left',   PAD + 'px', 'important');
  canvas.style.setProperty('transform', 'none', 'important');
  // The touch controls (dpad, fire buttons, paddle dials — all plain CSS,
  // positioned "bottom: calc(--topbar-h + ...)") key off this custom
  // property instead of a hardcoded 56px, so they stay clear of the bar
  // even when it renders taller than 56px (e.g. its content wrapping or
  // growing on a narrow phone) instead of overlapping it.
  document.documentElement.style.setProperty('--topbar-h', TOPBAR_H + 'px');
}
window.addEventListener('resize', fillCanvas);
fillCanvas();

function hideLoader() {
  if (loader && loader.style.display !== 'none') {
    loader.style.display = 'none';
    fillCanvas();
    canvas.focus();
  }
}

(function waitForCanvas() {
  if (canvas.width > 0 && canvas.height > 0) { hideLoader(); return; }
  setTimeout(waitForCanvas, 50);
})();
setTimeout(hideLoader, 3000);

canvas.addEventListener('webglcontextlost', function (e) {
  e.preventDefault();
  alert('WebGL context lost. Please reload the page.');
}, false);

// ---- On-screen controls ----
// The deck shows one of two controllers — the Super Nintendo pad (the
// default) or the classic arcade joystick + fire buttons — and BOTH,
// plus a physical keyboard and a physical gamepad, funnel through
// BlipController (blip_controller.js). It turns every logical press into
// the same synthetic KeyboardEvent on #glcanvas the old touch code used,
// so the WASM games are untouched. The rally paddle dials go through the
// library's bindDial().

var isRally = window.location.pathname.indexOf('/rally/') !== -1;

// Block the real keyboard from reaching the game while the coin wall is up.
window.addEventListener('keydown', function (e) {
  if (overlay.classList.contains('visible')) e.stopImmediatePropagation();
}, true);

(function () {
  var game = (typeof blipGameFromPath === 'function')
    ? blipGameFromPath(window.location.pathname) : null;
  var buttonSpecs = (game && game.buttons) || [{ key: ' ', code: 'Space' }];
  var primary = buttonSpecs[0] || { key: ' ', code: 'Space' };

  function coinGated() { return overlay.classList.contains('visible'); }
  function dispatch(spec, type) {
    canvas.dispatchEvent(new KeyboardEvent(type, {
      bubbles: true, cancelable: true, key: spec.key, code: spec.code
    }));
  }
  // START taps the primary action — the title screen's "PRESS FIRE", the
  // serve, the launch. SELECT leaves for the arcade's game grid (the trip
  // the BLIP logo makes) and is allowed even off the coin wall.
  function tapPrimary() {
    if (coinGated()) return;
    dispatch(primary, 'keydown');
    dispatch(primary, 'keyup');
  }
  function goToKiosk() { window.location.href = '../index.html'; }

  // Live input state -> the joystick ball's lean. The SNES pad lights its
  // own buttons (inside the library); this is the stick half — the ball
  // leans to whatever's held, whether that's the stick's own drag, the
  // keyboard, or the gamepad.
  var stick = document.getElementById('stick-base');
  var held = { up: false, down: false, left: false, right: false };
  function leanStick() {
    if (!stick) return;
    var x = (held.right ? 1 : 0) - (held.left ? 1 : 0);
    var y = (held.down  ? 1 : 0) - (held.up   ? 1 : 0);
    if (x && y) { x *= 0.7071; y *= 0.7071; }
    stick.style.setProperty('--dx', x);
    stick.style.setProperty('--dy', y);
  }
  function reflectInput(name, down) {
    if (Object.prototype.hasOwnProperty.call(held, name)) { held[name] = down; leanStick(); }
  }

  BlipController.init({
    canvas: canvas,
    buttons: buttonSpecs,
    gate: coinGated,
    feedback: feedbackTick,
    onInput: reflectInput,
    onStart: tapPrimary,
    onSelect: goToKiosk
  });
  BlipController.bindKeyboard();
  BlipController.bindGamepad(pollGamepad);

  // ---- Rally: hide both deck controllers, run the paddle dials ----
  if (isRally) {
    ['stick-base', 'fire-buttons', 'snes-pad'].forEach(function (id) {
      var el = document.getElementById(id);
      if (el) el.style.display = 'none';
    });
    var ct = document.getElementById('control-toggle');
    if (ct) ct.style.display = 'none';

    var dialP1 = document.getElementById('paddle-dial');
    var dialP2 = document.getElementById('paddle-dial-p2');
    if (dialP1) dialP1.style.display = 'block';
    if (dialP2) dialP2.style.display = 'block';

    if ('ontouchstart' in window || navigator.maxTouchPoints > 0) {
      var rallyMode = null;               // null = title, 0 = 1P, 1 = 2P
      var applyMode = function (m) { rallyMode = m; window.blipSetMode(m); };

      // A tap on the canvas (not on a dial) = Space — start 1P / launch /
      // any-key during a rally.
      canvas.addEventListener('touchstart', function (e) {
        if (e.target && e.target.closest && e.target.closest('#paddle-dial, #paddle-dial-p2')) return;
        e.preventDefault();
        if (!coinGated()) { dispatch({ key: ' ', code: 'Space' }, 'keydown'); dispatch({ key: ' ', code: 'Space' }, 'keyup'); }
        if (rallyMode === null) applyMode(0);
      }, { passive: false });

      BlipController.bindDial(dialP1, {
        up:   { key: 'ArrowUp',   code: 'ArrowUp' },
        down: { key: 'ArrowDown', code: 'ArrowDown' },
        tap:  { key: ' ', code: 'Space' },
        onTap:      function () { if (rallyMode === null) applyMode(0); },
        onInteract: function () { if (rallyMode === null) applyMode(0); }
      });
      BlipController.bindDial(dialP2, {
        up:   { key: 'i', code: 'KeyI' },
        down: { key: 'k', code: 'KeyK' },
        tap:  { key: '2', code: 'Digit2' },
        onTap:      function () { if (rallyMode === null) applyMode(1); },
        onInteract: function () { if (rallyMode === null) applyMode(1); }
      });
    }
    return;
  }

  // ---- The game pad ----
  var pad = document.getElementById('snes-pad');
  if (pad) {
    // A and B are both Button 1 (fire) — but a two-action game (Meteors'
    // hyperspace) has no shoulder buttons to put the second action on, so
    // A becomes Button 2 there. B stays fire.
    if (buttonSpecs[1] && buttonSpecs[1].code !== buttonSpecs[0].code) {
      var aBtn = pad.querySelector('.snes-a');
      if (aBtn) aBtn.setAttribute('data-blip', 'button2');
    }
    BlipController.bindButtons(pad);                        // A / B, SELECT / START
    BlipController.bindDpad(pad.querySelector('.snes-dpad')); // the d-pad cross
    BlipController.registerVisual(pad);                     // keyboard lights it too
  }

  // ---- The subtle toggle back to the joystick (and back again) ----
  var toggle = document.getElementById('control-toggle');
  if (toggle) {
    toggle.addEventListener('click', function () {
      blipSetControls(blipControls() === 'stick' ? 'pad' : 'stick');
    });
  }
  window.onBlipControlsChange = function () {
    BlipController.releaseAll();
    held.up = held.down = held.left = held.right = false;
    leanStick();
  };

  // ---- Fire buttons (joystick mode): one per game.buttons entry, built
  // here and wired through the library exactly like the SNES face buttons.
  (function () {
    var host = document.getElementById('fire-buttons');
    if (!host) return;
    var names = ['button1', buttonSpecs[1] ? 'button2' : 'button1'];
    names.forEach(function (n) {
      var btn = document.createElement('div');
      btn.className = 'arcade-btn';
      btn.setAttribute('data-blip', n);
      btn.innerHTML = '<span class="arcade-btn-cap"></span>';
      host.appendChild(btn);
    });
    BlipController.bindButtons(host);
    BlipController.registerVisual(host);
  }());

  // ---- The 8-way restrictor-gate joystick ----
  // Its drag is bound to #topbar (which carries no transform), not
  // #stick-base (inside .deck-panel's 3D rotateX) — so hit-testing is the
  // plain 2D geometry it looks like. It only turns a locked gate direction
  // into up/down/left/right key state via BlipController.set(); the ball's
  // lean is reflectInput()'s job, the same path a keypress drives.
  (function () {
    var base = document.getElementById('stick-base');
    var fire = document.getElementById('fire-buttons');
    var bar  = document.getElementById('topbar');
    if (!base || !bar) return;

    var _st = (game && game.stick) || {};
    var ENGAGE  = _st.engage  != null ? _st.engage  : 16;
    var RELEASE = _st.release != null ? _st.release : 9;
    var MAX_R   = _st.maxR    != null ? _st.maxR    : 46;
    var HYST    = _st.hyst    != null ? _st.hyst    : 8;
    var activeId = null;
    var engaged = false;
    var lockDeg = null;
    var pendingMove = null;
    var moveRaf = 0;
    var nextMoveOk = 0;
    var wantDir = { up: false, down: false, left: false, right: false };
    var GATE_KEYS = {
      0:   ['up'],            45:  ['up', 'right'],
      90:  ['right'],         135: ['down', 'right'],
      180: ['down'],          225: ['down', 'left'],
      270: ['left'],          315: ['up', 'left']
    };

    function setDir(dir, want) {
      if (wantDir[dir] === want) return;
      wantDir[dir] = want;
      BlipController.set(dir, want);
    }
    function setLock(deg) {
      if (deg === lockDeg) return;
      lockDeg = deg;
      var keys = deg === null ? [] : GATE_KEYS[deg];
      ['up', 'down', 'left', 'right'].forEach(function (dir) {
        setDir(dir, keys.indexOf(dir) !== -1);
      });
    }

    var pivot = null;
    function apply(e) {
      var dx = e.clientX - pivot.x;
      var dy = e.clientY - pivot.y;
      var dist = Math.sqrt(dx * dx + dy * dy);
      if (dist > MAX_R) {
        var k = 1 - MAX_R / dist;
        pivot.x += dx * k; pivot.y += dy * k;
        dx = e.clientX - pivot.x; dy = e.clientY - pivot.y;
        dist = MAX_R;
      }
      engaged = engaged ? dist > RELEASE : dist > ENGAGE;
      if (!engaged) { setLock(null); return; }
      var ang = Math.atan2(dx, -dy) * 180 / Math.PI;
      if (ang < 0) ang += 360;
      var next;
      if (lockDeg === null) {
        next = (Math.round(ang / 45) % 8) * 45;
      } else {
        var off = ((ang - lockDeg + 540) % 360) - 180;
        next = Math.abs(off) > 22.5 + HYST ? (Math.round(ang / 45) % 8) * 45 : lockDeg;
      }
      setLock(next);
    }
    function release() {
      activeId = null; pivot = null; engaged = false;
      if (moveRaf) { cancelAnimationFrame(moveRaf); moveRaf = 0; }
      pendingMove = null; nextMoveOk = 0;
      setLock(null);
      window.removeEventListener('pointermove', onMove, true);
      window.removeEventListener('pointerup', onEnd, true);
      window.removeEventListener('pointercancel', onEnd, true);
    }
    function isStickTouch(e) {
      if (fire && e.target && e.target.closest && e.target.closest('#fire-buttons')) return false;
      if (fire) {
        var fr = fire.getBoundingClientRect();
        if (fr.width && e.clientX >= fr.left - 6) return false;
      }
      return true;
    }
    function flushMove() {
      moveRaf = 0;
      var e = pendingMove; pendingMove = null;
      if (e && pivot && e.pointerId === activeId) { nextMoveOk = performance.now() + 8; apply(e); }
    }
    function onMove(e) {
      if (e.pointerId !== activeId || !pivot) return;
      e.preventDefault();
      pendingMove = e;
      if (performance.now() >= nextMoveOk) flushMove();
      else if (!moveRaf) moveRaf = requestAnimationFrame(flushMove);
    }
    function onEnd(e) {
      if (e.type === 'pointercancel' || e.pointerId === activeId) release();
    }
    bar.addEventListener('pointerdown', function (e) {
      if (blipControls() !== 'stick') return;   // pad mode — deck is the SNES pad
      if (activeId !== null) release();
      if (!isStickTouch(e)) return;
      e.preventDefault();
      activeId = e.pointerId;
      pivot = { x: e.clientX, y: e.clientY };
      window.addEventListener('pointermove', onMove, true);
      window.addEventListener('pointerup', onEnd, true);
      window.addEventListener('pointercancel', onEnd, true);
    });
    window.addEventListener('blur', release);
    document.addEventListener('visibilitychange', function () {
      if (document.hidden) release();
    });
  }());
}());


})();
