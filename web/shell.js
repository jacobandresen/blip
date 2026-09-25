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

// Sideways the deck would take a third of the screen's height. A 5:4
// picture in a 2:1 screen leaves a wide gutter down each side doing
// nothing, so the deck moves into the gutters instead.
function landscape() {
  return window.innerHeight <= 520 && window.innerWidth > window.innerHeight * 1.25;
}
function applyLayout() {
  document.documentElement.setAttribute('data-layout', landscape() ? 'landscape' : 'upright');
}
applyLayout();

/** How much of the screen's bottom the deck covers: its bar, plus
 * anything drawn outside it. */
function deckCover(bar) {
  // Start from the bottom of the screen, not from the bar's own top:
  // every drawn control is INSIDE the bar, so seeding this with the bar
  // meant nothing could ever lower it and the whole bar got reserved.
  var top = window.innerHeight;
  // The parts that are DRAWN, not the boxes holding them: the stick's
  // base is three caps tall to give the ball room to float and only its
  // bottom third is ever inked, so reserving the box gave the deck 46px
  // of empty screen and the pad 78.
  var parts = document.querySelectorAll(
    '#topbar .stick-ball, #topbar .stick-boot, #topbar .fire-buttons, ' +
    '#topbar .snes-dpad, #topbar .snes-face, ' +
    '#topbar #paddle-dial, #topbar #paddle-dial-p2');
  for (var i = 0; i < parts.length; i++) {
    var r = parts[i].getBoundingClientRect();
    if (r.height > 0 && r.top < top) top = r.top;
  }
  // A few pixels of margin: the ball rises as the stick leans. If the
  // deck has not been built yet there is nothing to measure, so fall
  // back to the bar.
  if (top >= window.innerHeight) return Math.ceil(bar.offsetHeight);
  return Math.ceil(window.innerHeight - top) + 10;
}

var lastFit = '';
function fillCanvas() {
  applyLayout();
  var tb = document.getElementById('topbar');
  // Sideways the deck overlays the picture's own letterbox, so it takes
  // no height at all.
  //
  // Upright, reserve what the deck actually COVERS, not the height of
  // its bar: the stick's ball floats above the bar by design and the
  // caps sit proud of it, so a bar-height reservation let the deck lap
  // over the bottom of the picture — far enough on a taller deck to
  // hide Bouncer's bat.
  TOPBAR_H = landscape() ? 0 : (tb ? Math.max(deckCover(tb), 56) : 56);
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

/** Re-fit only when something actually changed. The deck is watched by
 * a ResizeObserver and fillCanvas writes --topbar-h, which the deck's
 * own children read — so an unguarded re-fit can feed itself and the
 * picture flickers between two sizes. */
function refit() {
  var key = [window.innerWidth, window.innerHeight, TOPBAR_H, MARQUEE_H,
             canvas.style.width, canvas.style.height].join('|');
  fillCanvas();
  var now = [window.innerWidth, window.innerHeight, TOPBAR_H, MARQUEE_H,
             canvas.style.width, canvas.style.height].join('|');
  if (now === key) return;
  lastFit = now;
}
window.addEventListener('resize', fillCanvas);
window.addEventListener('orientationchange', function () { setTimeout(fillCanvas, 60); });
fillCanvas();

// Re-fit whenever the deck changes shape rather than at a handful of
// guessed moments. The bar is built in pieces — the marquee arrives
// separately, the caps are added by the block below, a controller
// switch changes the height — and every one of those used to need its
// own fillCanvas() call, with a race left over wherever one was missed.
if (typeof ResizeObserver === 'function') {
  var deckWatch = new ResizeObserver(function () { refit(); });
  // The bar's own height is fixed in CSS, so watching it alone never
  // fires for the thing that actually moves the deck's drawn edge: the
  // caps, which go from nothing to full size when the deck is built.
  ['#topbar', '#marquee-bar', '#fire-buttons', '#fire-buttons-p2',
   '#snes-pad .snes-face', '#snes-pad .snes-dpad'].forEach(function (sel) {
    var el = document.querySelector(sel);
    if (el) deckWatch.observe(el);
  });
}

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
  // A two-player cabinet seats two stations. The deck is built for both
  // up front and the second one stays hidden until the game says a
  // second player has joined (blip_set_mode -> window.blipSetMode).
  // One cabinet, one control panel: every page gets two stations of a
  // stick and four caps, and a game that reads fewer leaves the spares
  // dead. Rally is the exception — spinners, see its branch below.
  var twoUp = !isRally;
  var root = document.documentElement;
  if (twoUp) {
    // '1' is the resting state: one player, station two present and
    // dead. Only a game with a second player moves off it.
    root.setAttribute('data-players', '1');
    root.setAttribute('data-caps', '4');
  }
  // Declared here because the pad's face is built before the joystick
  // deck's caps and both read it.
  var CAPS = 4;
  var specs = buttonSpecs.length ? buttonSpecs : [primary];
  // A one-button game still gets two live caps: the pad's A and B have
  // always both been fire.
  if (specs.length < 2) specs = [specs[0], specs[0]];
  // Only a game that seats two players wires station two; elsewhere it
  // is panel furniture. Wiring it to names the game has never heard of
  // also does nothing, but by accident rather than by construction.
  function specsFor(who) { return who === 1 && !(game && game.players === 2) ? [] : specs; }

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
  // One entry per station: the stick element, the four logical names its
  // gate drives, and what each of them is currently doing.
  var stations = [
    { stick: document.getElementById('stick-base'),
      fire:  document.getElementById('fire-buttons'),
      dirs:  { up: 'up', down: 'down', left: 'left', right: 'right' },
      caps:  function (i) { return 'button' + (i + 1); },
      held:  {} },
    { stick: document.getElementById('stick-base-p2'),
      fire:  document.getElementById('fire-buttons-p2'),
      dirs:  { up: 'p2up', down: 'p2down', left: 'p2left', right: 'p2right' },
      caps:  function (i) { return 'p2button' + (i + 1); },
      held:  {} }
  ];
  if (!twoUp) stations.length = 1;
  function leanStick(st) {
    if (!st.stick) return;
    var x = (st.held[st.dirs.right] ? 1 : 0) - (st.held[st.dirs.left] ? 1 : 0);
    var y = (st.held[st.dirs.down]  ? 1 : 0) - (st.held[st.dirs.up]   ? 1 : 0);
    if (x && y) { x *= 0.7071; y *= 0.7071; }
    st.stick.style.setProperty('--dx', x);
    st.stick.style.setProperty('--dy', y);
  }
  // In a one-player game the arrows are player ONE's own (P1_ALONE in
  // the game), so an arrow press has to lean player one's stick — by
  // name alone it leaned the dead second one.
  var versus = false;
  function reflectInput(name, down) {
    stations.forEach(function (st, who) {
      if (who === 1 && !versus) return;
      for (var d in st.dirs) {
        if (st.dirs[d] === name) { st.held[name] = down; leanStick(st); return; }
      }
      if (who === 0 && !versus && stations[1]) {
        for (var e in stations[1].dirs) {
          if (stations[1].dirs[e] === name) {
            st.held[st.dirs[e]] = down; leanStick(st); return;
          }
        }
      }
    });
  }

  BlipController.init({
    canvas: canvas,
    buttons: buttonSpecs,
    keys: (game && game.keys) || null,
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
  // One pad per station, wired to that station's logical names. A pad's
  // face carries A / B for most games; a game that declares four buttons
  // (a fighter) gets four caps in a square instead, laid out and
  // lettered the same way as the arcade deck's.
  stations.forEach(function (st, who) {
    var pad = document.getElementById(who ? 'snes-pad-p2' : 'snes-pad');
    if (!pad) return;
    var face = pad.querySelector('.snes-face');
    if (face) {
      face.classList.add('four');
      face.innerHTML = '';
      var mine = specsFor(who);
      for (var i = 0; i < CAPS; i++) {
        var spec = mine[i];
        var cap = document.createElement('button');
        cap.type = 'button';
        cap.className = 'snes-btn cap' + (i + 1) + (spec ? '' : ' spare');
        if (spec) {
          cap.setAttribute('data-blip', st.caps(i));
          if (spec.label) cap.setAttribute('aria-label', spec.label);
        }
        cap.innerHTML = '<span></span>'
          + (spec && spec.label ? '<i class="snes-legend">' + spec.label + '</i>' : '');
        face.appendChild(cap);
      }
    }
    BlipController.bindButtons(pad);
    BlipController.bindDpad(pad.querySelector('.snes-dpad'), { names: st.dirs });
    BlipController.registerVisual(pad);
  });

  // ---- The subtle toggle back to the joystick (and back again) ----
  var toggle = document.getElementById('control-toggle');
  if (toggle) {
    toggle.addEventListener('click', function () {
      blipSetControls(blipControls() === 'stick' ? 'pad' : 'stick');
    });
  }
  window.onBlipControlsChange = function () {
    BlipController.releaseAll();
    stations.forEach(function (st) { st.held = {}; leanStick(st); });
    // The two controllers are different heights, so the picture has to
    // be re-fitted or it keeps the other one's reservation.
    if (typeof fillCanvas === 'function') fillCanvas();
  };

  // Called from WASM (brawler) when the mode is chosen: 1 = two players,
  // so the second station appears on the deck.
  // 0 = one player, 1 = two, 2 = title screen with the second station
  // lit and waiting. The third state is what makes "touch it to join"
  // possible: in a one-player game it has to be dead to the touch.
  if (twoUp) window.blipSetMode = function (code) {
    var open = code === 2;
    versus = code === 1;
    root.setAttribute('data-players', versus ? '2' : '1');
    if (open) root.setAttribute('data-open', '');
    else root.removeAttribute('data-open');
    // The second station is on the panel either way; what changes is
    // whether anybody is sitting at it.
    Array.prototype.forEach.call(document.querySelectorAll('.deck-tag[data-second]'),
      function (el) { el.textContent = versus ? '2P' : (open ? 'JOIN' : 'CPU'); });
    BlipController.releaseAll();
    stations.forEach(function (st) { st.held = {}; leanStick(st); });
    if (typeof fillCanvas === 'function') fillCanvas();
  };

  // ---- Fire buttons (joystick mode) ----
  // Four caps in a square per station. A cap past what the game
  // declares gets no logical name, so it does nothing. Legends go ON
  // the cap; the square leaves no room under the top row.
  stations.forEach(function (st, who) {
    if (!st.fire) return;
    st.fire.classList.add('four');
    var mine = specsFor(who);
    for (var i = 0; i < CAPS; i++) {
      var spec = mine[i];
      var btn = document.createElement('div');
      btn.className = 'arcade-btn lettered' + (spec ? '' : ' spare');
      if (spec) {
        btn.setAttribute('data-blip', st.caps(i));
        if (spec.label) btn.setAttribute('aria-label', spec.label);
      }
      btn.innerHTML = '<span class="arcade-btn-cap"></span>'
        + (spec && spec.label ? '<span class="arcade-btn-label">' + spec.label + '</span>' : '');
      st.fire.appendChild(btn);
    }
    BlipController.bindButtons(st.fire);
    BlipController.registerVisual(st.fire);
  });

  // The deck is built now, so the picture can be fitted to what it
  // actually covers. fillCanvas() ran at load, before any of these caps
  // existed, and fell back to reserving the whole bar. Once more after
  // the frame settles, because the marquee bar is injected separately
  // and changes the height above the picture as well.
  if (typeof fillCanvas === 'function') {
    fillCanvas();
    requestAnimationFrame(fillCanvas);
  }

  // ---- The 8-way restrictor-gate joystick ----
  // Its drag is bound to #topbar (which carries no transform), not the
  // stick base (inside .deck-panel's 3D rotateX) — so hit-testing is the
  // plain 2D geometry it looks like. It only turns a locked gate
  // direction into key state via BlipController.set(); the ball's lean is
  // reflectInput()'s job, the same path a keypress drives. One of these
  // per station, each with its own pointer and its own lock, so two
  // players can work their sticks at the same time.
  stations.forEach(function (st, who) {
    var base = st.stick;
    var fire = st.fire;
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
      BlipController.set(st.dirs[dir], want);
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
    // Whose half of the deck the touch landed in, and then: is it the
    // stick's side of that half rather than the caps'. Both stations are
    // laid out the same way — stick left, caps right — so "left of this
    // station's caps" is the whole test on either of them.
    function isStickTouch(e) {
      if (base.getBoundingClientRect().width === 0) return false;
      if (stations.length > 1) {
        var br = bar.getBoundingClientRect();
        if ((e.clientX < br.left + br.width / 2) !== (who === 0)) return false;
      }
      if (fire && e.target && e.target.closest && e.target.closest('.fire-buttons')) return false;
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
  });
}());


})();
