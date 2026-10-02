(function () {
'use strict';

// ---- Zoom lock ----------------------------------------------------------
// Every touch on a game page is game input. `body { touch-action: none }`
// (shell.css) stops pan and double-tap zoom; iOS Safari still starts a pinch
// zoom from a two-finger gesture (a thumb on fire and one on the cross
// drifting apart), so swallow those.
['gesturestart', 'gesturechange', 'gestureend'].forEach(function (t) {
  document.addEventListener(t, function (e) { e.preventDefault(); }, { passive: false });
});

var TOPBAR_H   = 56;
var MARQUEE_H  = 28; // reserved header height once #marquee-bar exists
var FRAME_PAD  = 16; // padding around the canvas on all sides (= bezel width)

var loader    = document.getElementById('loader');
var barInner  = document.getElementById('bar-inner');
var statusEl  = document.getElementById('status');
var canvas    = document.getElementById('glcanvas');
var overlay   = document.getElementById('need-coin-overlay');

updateCoinsHud();

// ---- Per-game marquee + cabinet accent ----
// Built here rather than in each game's HTML.
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

    // After ~12s without input a "> MORE GAMES" nudge fades in under the
    // logo, the only way back to the game grid on touch. Any input hides it
    // and restarts the clock (keydown in the capture phase catches the
    // on-screen controls and gamepad too).
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

// A kiosk nobody is playing goes back to the cabinet: in fullscreen, two
// minutes without a key, a touch or a pad press (the deck and a gamepad both
// arrive as key events on the canvas).
(function () {
  var IDLE_MS = 120000, last = Date.now();
  function poke() { last = Date.now(); }
  ['keydown', 'pointerdown', 'pointermove', 'touchstart'].forEach(function (ev) {
    window.addEventListener(ev, poke, { capture: true, passive: true });
  });
  setInterval(function () {
    if (!document.documentElement.hasAttribute('data-fullscreen') || document.hidden) return;
    // Somebody typing their name is not idle.
    if (document.querySelector('.blip-hs-modal')) { poke(); return; }
    if (Date.now() - last >= IDLE_MS) window.location.href = '../index.html';
  }, 5000);
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
// A felt detent the instant a control catches: a Vibration-API buzz on
// Android; iOS has neither that nor Taptic access, so a ~40 ms sub-bass burst
// instead. Touch devices only, and never on the coin overlay.
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
// A coin is two sounds: the inharmonic metallic clink through the chute, then
// the register's "credit accepted" chime a beat later.
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

// The coin that drops into #insert-coin-btn's slot: a real element so a class
// toggle can animate it, injected here for every game page.
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
  // The slot is the centre of the coin plate at the button's content edge:
  // padding-right plus half the plate. Both change across breakpoints.
  var btn = coinDropAnim.parentElement;
  var padRight = parseFloat(getComputedStyle(btn).paddingRight) || 8;
  var plateW = parseFloat(getComputedStyle(btn, '::after').width) || 18;
  coinDropAnim.style.setProperty('--slot-x', (padRight + plateW / 2) + 'px');
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

// Called from WASM when play starts (from the title or a game-over
// restart), once more per second player. Out of coins, the debt is
// kept and the coins that go in next pay it before the game goes on.
var coinsOwed = 0;
window.blipSpendCoin = function () {
  var n = getCoins();
  if (n <= 0) {
    coinsOwed++;
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

// Called from WASM (title / game-over screens): the leading score and its
// holder. Prefers the leaderboard #1 blip_scores.js caches in
// 'blip-top-<slug>', else this browser's best ('blip-best-<slug>') with the
// claimed handle ('blip-handle').
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
  // Two at the table: sideways the touch trackpad becomes one per gutter.
  document.documentElement.toggleAttribute('data-versus', mode === 1);
  if (typeof fillCanvas === 'function') fillCanvas();
  d.classList.toggle('cpu-mode', isCpu);
  document.documentElement.toggleAttribute('data-cpu', isCpu);
  var label = document.getElementById('dial-label-p2');
  if (label) label.textContent = isCpu ? 'CPU' : '2P';
};

// Called from WASM (rally) every frame with both paddle positions (0 top .. 1
// bottom), so the dials turn under keyboard play and the CPU's.
(function () {
  var h1 = document.getElementById('dial-hand');
  var h2 = document.getElementById('dial-hand-p2');
  // Full paddle travel ≈ 1½ turns of the knob, geared like a real spinner.
  var SWEEP = 3 * Math.PI;
  // Called every frame; only a change restyles the knob.
  var l0 = null, r0 = null;
  window.blipPaddles = function (left, right) {
    if (h1 && left !== l0)  { l0 = left;  h1.style.transform = 'rotate(' + ((left  - 0.5) * SWEEP) + 'rad)'; }
    if (h2 && right !== r0) { r0 = right; h2.style.transform = 'rotate(' + ((right - 0.5) * SWEEP) + 'rad)'; }
  };
}());

// A coin in: it pays off anything owed first, and the wall only lifts
// once nothing is.
function coinIn() {
  var n = getCoins() + 1;
  if (coinsOwed > 0) { coinsOwed--; n--; }
  saveCoins(n);
  updateCoinsHud();
  playCoinInsert();
  dropCoinAnimation();
  flashCoinBar();
  if (coinsOwed <= 0) overlay.classList.remove('visible');
}

overlay.addEventListener('click', function () {
  if (getCoins() >= MAX_COINS) return;
  coinIn();
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
  coinIn();
});

// Mirror the overlay onto <body> (body.need-coin lights the coin button).
// When it closes, a coin just went in: give focus back to the canvas so the
// title screen answers the keyboard.
var overlayWasVisible = overlay.classList.contains('visible');
new MutationObserver(function () {
  var vis = overlay.classList.contains('visible');
  document.body.classList.toggle('need-coin', vis);
  if (overlayWasVisible && !vis && canvas) {
    try { canvas.focus(); } catch (e) {}
  }
  overlayWasVisible = vis;
}).observe(overlay, { attributes: true, attributeFilter: ['class'] });

// No credits on arrival (fresh session, deep link): the coin wall is up, and
// it blocks injectKey() until a coin goes in.
if (getCoins() <= 0) overlay.classList.add('visible');

// (No ambient cabinet hum on a game page — while a game is running its
// own audio is the whole soundscape. The transformer drone lives only on
// the landing page, index.html, where the machine is idling in attract.)

// ---- Canvas sizing ----
// The kiosk bar is position:fixed;bottom:0. We leave PAD px on each side
// plus full clearance for the bar so it never overlaps the canvas.

// Sideways the deck moves into the picture's side gutters (one layout rule
// for every page, in kiosk.js).
function landscape() { return blipLandscape(); }
function applyLayout() { blipApplyLayout(); }
applyLayout();

/** How much of the screen's bottom the deck covers: its bar, plus
 * anything drawn outside it. */
function deckCover(bar) {
  // Start from the bottom of the screen, not from the bar's own top:
  // every drawn control is INSIDE the bar, so seeding this with the bar
  // meant nothing could ever lower it and the whole bar got reserved.
  var top = window.innerHeight;
  // What the deck draws, not its boxes: the stick base is three caps tall for
  // the floating ball and only its bottom third is inked.
  var parts = document.querySelectorAll(
    '#topbar .stick-boot, #topbar .fire-buttons, ' +
    '#topbar .snes-dpad, #topbar .snes-face, ' +
    '#topbar #paddle-dial, #topbar #paddle-dial-p2');
  for (var i = 0; i < parts.length; i++) {
    var r = parts[i].getBoundingClientRect();
    if (r.height > 0 && r.top < top) top = r.top;
  }
  // Upright the whole bar is drawn: the cabinet body (#topbar::before,
  // 5px above the bar) and the plate. None of it may lap over the picture.
  if (!landscape() && parts.length) top = Math.min(top, bar.getBoundingClientRect().top - 5);
  // A few pixels of margin. The ball is left out: it floats over the game.
  // Before the deck is built, fall back to the bar.
  if (top >= window.innerHeight) return Math.ceil(bar.offsetHeight);
  return Math.ceil(window.innerHeight - top) + 4;
}

/** Fullscreen on a PC: no deck and no cabinet frame, so the picture runs to
 * the edges under the top bar (see #fullscreen-btn in shell.css). */
function bareScreen() {
  return document.documentElement.hasAttribute('data-fullscreen') && !blipHasTouch();
}

var lastFit = '';
function fillCanvas() {
  applyLayout();
  var tb = document.getElementById('topbar');
  var bare = bareScreen();
  // The frame is FRAME_PAD wide; without it the picture needs no margin.
  var PAD = bare ? 0 : FRAME_PAD;
  // Sideways the deck sits in the letterbox and takes no height. Upright,
  // reserve what the deck covers (caps sit proud of the bar), not the bar's
  // height.
  TOPBAR_H = bare || landscape() ? 0 : (tb ? Math.max(deckCover(tb), 56) : 56);
  var mb = document.getElementById('marquee-bar');
  MARQUEE_H = mb ? Math.max(mb.offsetHeight, 28) : 0;
  // Sideways the controls stand in the two gutters; keep the picture out
  // of them (a wide game like Brawler would otherwise run under the cross).
  var gl = PAD, gr = PAD;
  if (landscape() && tb) {
    var mid = window.innerWidth / 2;
    var ctl = tb.querySelectorAll('.snes-dpad, .snes-face, .stick-base, .stick-ball, .fire-buttons, #paddle-dial, #paddle-dial-p2, #touch-strip .ts-half');
    for (var i = 0; i < ctl.length; i++) {
      var r = ctl[i].getBoundingClientRect();
      if (!r.width || !r.height) continue;
      if (r.left + r.width / 2 < mid) gl = Math.max(gl, Math.ceil(r.right) + 6);
      else gr = Math.max(gr, Math.ceil(window.innerWidth - r.left) + 6);
    }
  }
  var w = window.innerWidth - gl - gr;
  var h = window.innerHeight - TOPBAR_H - MARQUEE_H - PAD * 2;
  canvas.style.setProperty('width',  w + 'px', 'important');
  canvas.style.setProperty('height', h + 'px', 'important');
  canvas.style.setProperty('top',    (MARQUEE_H + PAD) + 'px', 'important');
  canvas.style.setProperty('left',   gl + 'px', 'important');
  canvas.style.setProperty('transform', 'none', 'important');
  // The controls position themselves off --topbar-h, the bar's real height.
  document.documentElement.style.setProperty('--topbar-h', TOPBAR_H + 'px');
  syncBuffer();
}

/** The macroquad runtime sizes the canvas's drawing buffer on a window
 * resize only (its window.onresize). A layout change that moves the canvas
 * without one (fullscreen on or off, another controller) would leave the
 * game drawing for the old size, stretched into the new box. */
function syncBuffer() {
  if (canvas.width === canvas.clientWidth && canvas.height === canvas.clientHeight) return;
  if (typeof window.onresize !== 'function') return;
  try { window.onresize(); } catch (e) { /* the game has not loaded yet */ }
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

// Re-fit whenever the deck changes shape (marquee injected, caps added,
// controller switched) instead of at guessed moments.
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

// Fullscreen went on or off (kiosk.js owns the button): the deck comes or
// goes, so the picture is fitted again.
window.onBlipFullscreenChange = function () {
  fillCanvas();
  try { canvas.focus({ preventScroll: true }); } catch (e) {}
};

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
// The pad (default) or the arcade stick, plus keyboard and gamepad, all go
// through BlipController (blip_controller.js), which turns each press into a
// synthetic KeyboardEvent on #glcanvas, so the games are untouched. Rally's
// dials use its bindDial().

var isRally = window.location.pathname.indexOf('/rally/') !== -1;

// While the game is being played, <html data-live> pauses the decorative
// main-thread animations (logo glow, marquee bulbs): each one restyles and
// repaints every frame on the thread the game and the touch deck share.
(function () {
  var root = document.documentElement, t = null;
  window.addEventListener('keydown', function () {
    if (!t) root.setAttribute('data-live', '');
    else clearTimeout(t);
    t = setTimeout(function () { t = null; root.removeAttribute('data-live'); }, 4000);
  }, true);
}());

// Block the real keyboard from reaching the game while the coin wall is up.
// Not the high-score prompt's: its Enter and Escape are for the name field.
window.addEventListener('keydown', function (e) {
  if (e.target && e.target.closest && e.target.closest('.blip-hs-modal')) return;
  if (overlay.classList.contains('visible')) e.stopImmediatePropagation();
}, true);

(function () {
  var game = (typeof blipGameFromPath === 'function')
    ? blipGameFromPath(window.location.pathname) : null;
  var buttonSpecs = (game && game.buttons) || [{ key: ' ', code: 'Space' }];
  var primary = buttonSpecs[0] || { key: ' ', code: 'Space' };
  // deck.js has built the deck (two stations, stick and pad each, caps from
  // BLIP_GAMES) and set data-players / data-caps / data-seats. Rally has
  // dials instead of the stick and pad, and hides them below.
  var twoUp = !isRally;
  var root = document.documentElement;

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

  // Live input -> the stick ball's lean (the pad lights its own buttons in
  // the library), whether from a drag, the keyboard or a gamepad. One entry
  // per station: the stick, its four logical names, and their state.
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

  // ---- The controller selector ----
  // Oldest to newest: JOYSTICK · PAD (· TOUCH on a game with a strip, on a
  // touch screen); Rally has DIAL · TOUCH there and nothing to pick elsewhere.
  var picker = document.getElementById('control-toggle');
  var touchSpec = game && game.touch;
  function syncPicker() {
    if (!picker) return;
    var now = blipControls();
    Array.prototype.forEach.call(picker.children, function (b) {
      var on = b.getAttribute('data-mode') === now ||
               (b.getAttribute('data-mode') === 'dial' && now !== 'touch');
      b.setAttribute('aria-checked', on ? 'true' : 'false');
    });
  }
  if (picker) {
    var modes = isRally ? ['dial'] : ['stick', 'pad'];
    var LABEL = { stick: 'joystick' };
    if (touchSpec && blipHasTouch()) modes.push('touch');
    if (modes.length < 2) modes = [];
    modes.forEach(function (m) {
      var b = document.createElement('button');
      b.type = 'button';
      b.setAttribute('role', 'radio');
      b.setAttribute('data-mode', m);
      b.textContent = LABEL[m] || m;
      b.addEventListener('click', function () {
        blipSetControls(m === 'dial' ? blipPhysicalControls() : m);
        // Back to the game, so the keyboard and gamepad drive it at once.
        try { canvas.focus({ preventScroll: true }); } catch (e) {}
      });
      picker.appendChild(b);
    });
    syncPicker();
  }
  var touchPlay = bindTouchPlay();
  window.onBlipControlsChange = function () {
    BlipController.releaseAll();
    touchPlay.release();
    stations.forEach(function (st) { st.held = {}; leanStick(st); });
    syncPicker();
    // The controllers are different heights, so the picture has to be
    // re-fitted or it keeps the other one's reservation.
    if (typeof fillCanvas === 'function') fillCanvas();
  };

  // ---- Touch play ----
  // With TOUCH chosen, a finger anywhere but a button (the strip, the picture,
  // the bezel) plays. 'drag' and 'paddles' hand finger positions to the game
  // (blip_touch_* in blip_bridge.js); 'swipe' taps arrow keys. Serpent's
  // picture takes finger swipes whatever controller is chosen. Touch screens
  // only; a touchscreen laptop's mouse can hover-play the trackpad too.
  function bindTouchPlay() {
    var none = { release: function () {}, resync: function () {} };
    if (!touchSpec || !blipHasTouch()) return none;
    var kind = touchSpec.kind;
    var SWIPE_PX = 22;                 // a flick this long turns the snake
    var slots = [null, null];          // { id, x, y, pressed, mouse } in canvas fractions
    var swipe = null;                  // { id, x, y, moved }
    var fireHeld = false;
    var rect = null;

    var strip = document.createElement('div');
    strip.id = 'touch-strip';
    strip.setAttribute('aria-hidden', 'true');
    var halves = kind === 'paddles' || kind === 'platform' ? ['P1', '2P'] : [''];
    strip.innerHTML = '<div class="ts-glass">' +
      halves.map(function (h, i) {
        return '<div class="ts-half" data-slot="' + i + '">' +
          (h ? '<span class="ts-tag">' + h + '</span>' : '') + '</div>';
      }).join('') +
      '<span class="ts-demo"></span><span class="ts-thumb"></span><span class="ts-arrow"></span>' +
      '<span class="ts-hint for-touch">' + (kind === 'drag' ? 'Drag to move &middot; ' : '') +
      touchSpec.hint + '</span>' +
      '<span class="ts-hint for-mouse">' + touchSpec.mouseHint + '</span></div>';
    var bar = document.getElementById('topbar');
    if (bar) bar.appendChild(strip);
    var glass = strip.querySelector('.ts-glass');

    // Bubbler: while a second seat is open, a button seats player two (their
    // own bubble key). Upright it sits in the trackpad, sideways in the
    // bottom-right corner of the screen.
    var join = null;
    if (kind === 'platform') {
      join = document.createElement('button');
      join.type = 'button';
      join.className = 'ts-join';
      join.innerHTML = '<span class="tj-plus">+</span>2P JOIN';
      join.addEventListener('pointerdown', function (e) {
        e.preventDefault();
        if (coinGated()) return;
        BlipController.set('p2button1', true);
        setTimeout(function () { BlipController.set('p2button1', false); }, 60);
      });
    }
    function placeJoin() {
      if (!join) return;
      var home = blipLandscape() ? document.body : glass;
      if (join.parentNode !== home) home.appendChild(join);
    }

    // The trackpad is never wider than the game's picture: the canvas
    // letterboxed to the size the game reported (blip_picture).
    function fitStrip() {
      placeJoin();
      if (!bar) return;
      var cr = canvas.getBoundingClientRect(), br = bar.getBoundingClientRect();
      var pic = window.blipPicture, w = cr.width;
      if (pic && pic.w && pic.h) w = Math.min(cr.width, Math.round(cr.height * pic.w / pic.h));
      glass.style.left = Math.round(cr.left + (cr.width - w) / 2 - br.left) + 'px';
      glass.style.width = w + 'px';
    }
    window.addEventListener('blip-picture', fitStrip);
    window.addEventListener('resize', fitStrip);
    if (typeof ResizeObserver === 'function') new ResizeObserver(fitStrip).observe(canvas);
    fitStrip();
    var thumb = strip.querySelector('.ts-thumb');
    var arrow = strip.querySelector('.ts-arrow');
    var p2tag = strip.querySelector('.ts-half[data-slot="1"] .ts-tag');

    // 0 = nothing, 1 = a PC's pointer hovering, 2 = pressed.
    window.blipTouchDown = function (slot) {
      var t = slots[slot];
      return t ? (t.pressed ? 2 : 1) : 0;
    };
    window.blipTouchPos = function (slot, axis) {
      var t = slots[slot];
      return t ? (axis ? t.y : t.x) : 0;
    };
    window.blipHaptic = function () {
      if (blipControls() === 'touch' && navigator.vibrate) {
        try { navigator.vibrate(12); } catch (e) {}
      }
    };

    function frac(e) {
      return { x: (e.clientX - rect.left) / rect.width, y: (e.clientY - rect.top) / rect.height };
    }
    // Rally: each player drags on their own half, except against the CPU,
    // where the whole screen is player one's.
    function slotFor(e) {
      if (kind !== 'paddles' || root.hasAttribute('data-cpu')) return 0;
      return e.clientX < window.innerWidth / 2 ? 0 : 1;
    }
    function setFire(down) {
      if (down === fireHeld) return;
      fireHeld = down;
      if (!down || !coinGated()) dispatch(primary, down ? 'keydown' : 'keyup');
    }
    function tapKey(spec) {
      if (coinGated()) return;
      dispatch(spec, 'keydown');
      dispatch(spec, 'keyup');
    }
    function showThumb(t) {
      if (kind === 'drag' && t) {
        var sr = glass.getBoundingClientRect();
        thumb.style.transform = 'translateX(' + (rect.left + t.x * rect.width - sr.left) + 'px)';
      }
      var held = !!(slots[0] || slots[1] || swipe);
      strip.classList.toggle('held', held);
      // The demo finger has made its point once the player has touched.
      if (held) strip.classList.add('used');
      Array.prototype.forEach.call(strip.querySelectorAll('.ts-half'), function (h, i) {
        h.classList.toggle('held', !!slots[i]);
      });
    }
    function flashArrow(dir) {
      arrow.setAttribute('data-dir', dir);
      arrow.classList.remove('flash');
      void arrow.offsetWidth;
      arrow.classList.add('flash');
    }
    function ignored(e) {
      return e.target && e.target.closest &&
        e.target.closest('a, button, #need-coin-overlay, #rotate-hint, #control-toggle, .blip-hs-modal');
    }

    // A PC plays the trackpad like a laptop's: the pointer moves the paddle
    // just by hovering over it (or the picture), a click is the tap.
    function mouseSlot() {
      for (var i = 0; i < slots.length; i++) if (slots[i] && slots[i].mouse) return i;
      return -1;
    }
    function onSurface(e) {
      return e.target === canvas || !!(e.target && e.target.closest && e.target.closest('.ts-glass'));
    }
    function placeMouse(e, pressed) {
      var slot = slotFor(e), was = mouseSlot();
      // Held, the click stays with the bat it started on.
      if (was !== -1 && slots[was].pressed) slot = was;
      else if (was !== -1 && was !== slot) slots[was] = null;
      var f = frac(e);
      slots[slot] = { id: e.pointerId, x: f.x, y: f.y, pressed: pressed, mouse: true };
      return slot;
    }
    function dropMouse() {
      var m = mouseSlot();
      if (m === -1) return;
      slots[m] = null;
      if (kind === 'drag') setFire(false);
      showThumb(null);
    }

    // The hint follows whatever was used last: a touchscreen laptop has both.
    function noteInput(e) {
      var type = e.pointerType === 'mouse' ? 'mouse' : 'touch';
      if (strip.getAttribute('data-input') !== type) strip.setAttribute('data-input', type);
    }
    window.addEventListener('pointerdown', noteInput, true);
    window.addEventListener('pointermove', noteInput, true);

    // ---- Platform (Bubbler): one surface. Every touch blows a bubble the
    // moment it lands (a tap judged on release missed whenever the thumb
    // slid or lingered), a slide runs (the anchor trails the thumb), a swipe
    // up jumps. Presses go through BlipController, onto the player's keys.
    var RUN_PX = 10, FLICK_PX = 20, LEASH_PX = 28;
    var plat = {};                     // pointerId -> { who, x0, y0, dir }
    // Once player two is in, the right half (of the trackpad, or of the
    // screen sideways) is theirs, with the same gestures on their keys.
    var PLAT_NAMES = [
      { left: 'left', right: 'right', bubble: 'button1', jump: 'button2' },
      { left: 'p2left', right: 'p2right', bubble: 'p2button1', jump: 'p2button2' }];
    function whoOf(e) {
      if (!root.hasAttribute('data-versus')) return 0;
      var g = glass.getBoundingClientRect();
      var f = g.width ? (e.clientX - g.left) / g.width : e.clientX / window.innerWidth;
      return f < 0.5 ? 0 : 1;
    }
    function lightZones() {
      var lit = {};
      for (var id in plat) lit[plat[id].who] = true;
      Array.prototype.forEach.call(strip.querySelectorAll('.ts-half'), function (h, i) {
        h.classList.toggle('held', !!lit[i]);
      });
      var any = Object.keys(plat).length > 0;
      strip.classList.toggle('held', any);
      if (any) strip.classList.add('used');
    }
    function runTo(p, dir) {
      var n = PLAT_NAMES[p.who];
      if (dir === p.dir) return;
      if (p.dir) BlipController.set(n[p.dir], false, { silentClick: true });
      if (dir) BlipController.set(n[dir], true, { silentClick: true });
      p.dir = dir;
    }
    function tapButton(name) {
      BlipController.set(name, true);
      setTimeout(function () { BlipController.set(name, false); }, 60);
      if (navigator.vibrate) { try { navigator.vibrate(8); } catch (e) {} }
    }
    function platDown(e) {
      var p = plat[e.pointerId] = { who: whoOf(e), x0: e.clientX, y0: e.clientY, dir: null };
      tapButton(PLAT_NAMES[p.who].bubble);
      lightZones();
    }
    function platMove(e) {
      var p = plat[e.pointerId];
      if (!p) return;
      var dx = e.clientX - p.x0;
      if (dx > LEASH_PX) { p.x0 = e.clientX - LEASH_PX; dx = LEASH_PX; }
      if (dx < -LEASH_PX) { p.x0 = e.clientX + LEASH_PX; dx = -LEASH_PX; }
      runTo(p, dx > RUN_PX ? 'right' : dx < -RUN_PX ? 'left' : null);
      if (p.y0 - e.clientY > FLICK_PX) { tapButton(PLAT_NAMES[p.who].jump); p.y0 = e.clientY; }
      else if (e.clientY > p.y0) p.y0 = e.clientY;
    }
    function platUp(e) {
      var p = plat[e.pointerId];
      if (!p) return;
      runTo(p, null);
      delete plat[e.pointerId];
      lightZones();
    }
    function platRelease() {
      for (var id in plat) platUp({ pointerId: id });
    }
    // The game changed mode and the shell let go of every key: a thumb still
    // running picks up again on its next move.
    function resync() {
      for (var id in plat) plat[id].dir = null;
    }

    window.addEventListener('pointerdown', function (e) {
      if (kind !== 'platform') return;
      if (blipControls() !== 'touch' || ignored(e) || coinGated()) return;
      e.preventDefault();
      platDown(e);
    }, { passive: false });
    window.addEventListener('pointermove', function (e) {
      if (kind === 'platform' && plat[e.pointerId]) { e.preventDefault(); platMove(e); }
    }, { passive: false });
    window.addEventListener('pointerup', function (e) { if (kind === 'platform') platUp(e); });

    window.addEventListener('pointerdown', function (e) {
      if (kind === 'platform') return;
      var on = blipControls() === 'touch';
      if (!on && !(kind === 'swipe' && e.target === canvas && e.pointerType !== 'mouse')) return;
      if (ignored(e) || coinGated()) return;
      e.preventDefault();
      rect = canvas.getBoundingClientRect();
      if (kind === 'swipe') {
        swipe = { id: e.pointerId, x: e.clientX, y: e.clientY, moved: false };
      } else if (e.pointerType === 'mouse') {
        placeMouse(e, true);
        if (kind === 'drag') setFire(true);
      } else {
        var slot = slotFor(e);
        // A second finger on a relative bat would jump it; the first keeps it.
        if (kind === 'paddles' && slots[slot] && !slots[slot].mouse) return;
        var f = frac(e);
        slots[slot] = { id: e.pointerId, x: f.x, y: f.y, pressed: true, mouse: false };
        if (kind === 'drag') setFire(true);
      }
      showThumb(slots[0]);
    }, { passive: false });

    window.addEventListener('pointermove', function (e) {
      if (swipe && e.pointerId === swipe.id) {
        e.preventDefault();
        var dx = e.clientX - swipe.x, dy = e.clientY - swipe.y;
        if (Math.max(Math.abs(dx), Math.abs(dy)) < SWIPE_PX) return;
        var dir = Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? 'right' : 'left') : (dy > 0 ? 'down' : 'up');
        // Re-anchor, so one drag can chain turns round a corner; a straight
        // drag turns once.
        swipe.x = e.clientX; swipe.y = e.clientY; swipe.moved = true;
        if (dir === swipe.dir) return;
        swipe.dir = dir;
        var code = 'Arrow' + dir.charAt(0).toUpperCase() + dir.slice(1);
        tapKey({ key: code, code: code });
        flashArrow(dir);
        return;
      }
      if (e.pointerType === 'mouse' && (kind === 'drag' || kind === 'paddles')) {
        var m = mouseSlot();
        var held = m !== -1 && slots[m].pressed;
        if (blipControls() !== 'touch' || coinGated() || (!held && !onSurface(e))) { dropMouse(); return; }
        if (!held) rect = canvas.getBoundingClientRect();
        showThumb(slots[placeMouse(e, held)]);
        return;
      }
      for (var i = 0; i < slots.length; i++) {
        var t = slots[i];
        if (!t || t.id !== e.pointerId) continue;
        e.preventDefault();
        var f = frac(e);
        t.x = f.x; t.y = f.y;
        if (i === 0) showThumb(t);
      }
    }, { passive: false });

    function end(e) {
      if (swipe && e.pointerId === swipe.id) {
        if (!swipe.moved && e.type === 'pointerup') tapKey(primary);
        swipe = null;
      }
      for (var i = 0; i < slots.length; i++) {
        var t = slots[i];
        if (!t || t.id !== e.pointerId) continue;
        // The mouse goes back to hovering if it is still over the trackpad.
        if (t.mouse && e.type === 'pointerup' && onSurface(e)) t.pressed = false;
        else slots[i] = null;
      }
      if (kind === 'drag' && !(slots[0] && slots[0].pressed)) setFire(false);
      showThumb(slots[0]);
    }
    window.addEventListener('pointerup', end);
    // iOS cancels fingers that are still down when a second one moves
    // (blip_controller.js), so a finger's pointercancel is not a lift; the
    // last finger's touchend releases everything instead.
    window.addEventListener('pointercancel', function (e) { if (e.pointerType === 'mouse') end(e); });
    ['touchend', 'touchcancel'].forEach(function (t) {
      document.addEventListener(t, function (e) { if (!e.touches.length) release(); }, { passive: true });
    });
    document.documentElement.addEventListener('mouseleave', function () {
      var m = mouseSlot();
      if (m !== -1 && !slots[m].pressed) dropMouse();
    });

    function release() {
      platRelease();
      slots[0] = slots[1] = null;
      swipe = null;
      setFire(false);
      showThumb(null);
    }
    window.addEventListener('blur', release);
    document.addEventListener('visibilitychange', function () {
      if (document.hidden) release();
    });
    // Rally's right half is the CPU's until a second player sits down.
    if (p2tag) new MutationObserver(function () {
      p2tag.textContent = root.hasAttribute('data-cpu') ? 'CPU' : '2P';
    }).observe(root, { attributes: true, attributeFilter: ['data-cpu'] });
    return { release: release, resync: resync };
  }

  // ---- Rally: hide both deck controllers, run the paddle dials ----
  if (isRally) {
    // Same panel as every other game (data-dials swaps its controls for
    // the dials in kiosk-sized slots; see shell.css).
    root.setAttribute('data-dials', '');

    var dialP1 = document.getElementById('paddle-dial');
    var dialP2 = document.getElementById('paddle-dial-p2');

    if ('ontouchstart' in window || navigator.maxTouchPoints > 0) {
      var rallyMode = null;               // null = title, 0 = 1P, 1 = 2P
      var applyMode = function (m) { rallyMode = m; window.blipSetMode(m); };

      // A tap on the canvas (not on a dial) = Space — start 1P / launch /
      // any-key during a rally.
      canvas.addEventListener('touchstart', function (e) {
        if (blipControls() === 'touch') return;
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
  // One pad per station (built by deck.js), wired to that station's names.
  stations.forEach(function (st, who) {
    var pad = document.getElementById(who ? 'snes-pad-p2' : 'snes-pad');
    if (!pad) return;
    BlipController.bindButtons(pad);
    BlipController.bindDpad(pad.querySelector('.snes-dpad'), { names: st.dirs });
    BlipController.registerVisual(pad);
  });

  // Called from WASM by the two-seat games: 0 = one player, 1 = two, 2 =
  // title screen with station two lit and waiting (a one-player game keeps it
  // dead to the touch).
  if (twoUp) window.blipSetMode = function (code) {
    open = code === 2;
    versus = code === 1;
    applyPlayers();
    if (open) root.setAttribute('data-open', '');
    else root.removeAttribute('data-open');
    if (versus) root.setAttribute('data-versus', '');
    else root.removeAttribute('data-versus');
    // The second station is on the panel either way; what changes is
    // whether anybody is sitting at it.
    Array.prototype.forEach.call(document.querySelectorAll('.deck-tag[data-second]'),
      function (el) { el.textContent = versus ? '2P' : (open ? 'JOIN' : 'CPU'); });
    // This runs inside the game's frame: a key-up injected now re-enters
    // miniquad's event handler and panics, so let go once the frame returns.
    queueMicrotask(function () {
      BlipController.releaseAll();
      touchPlay.resync();
      stations.forEach(function (st) { st.held = {}; leanStick(st); });
    });
    if (typeof fillCanvas === 'function') fillCanvas();
    rotateHint();
  };

  // Two players on a touch phone are asked to turn sideways (orientation
  // locked where allowed); upright still works. Sideways, a cabinet waiting
  // for a second player lays out both stations so there is somewhere to press
  // to join.
  var open = false;
  function applyPlayers() {
    var both = versus || (open && landscape() && matchMedia('(pointer: coarse)').matches);
    root.setAttribute('data-players', both ? '2' : '1');
  }
  window.addEventListener('resize', applyPlayers);

  var hint = null, hintDismissed = false;
  function rotateHint() {
    // The touch trackpad splits for two upright, so it needs no turning.
    var want = versus && !hintDismissed && !landscape() && blipControls() !== 'touch' &&
               matchMedia('(pointer: coarse)').matches;
    if (want && !hint) {
      hint = document.createElement('div');
      hint.id = 'rotate-hint';
      hint.innerHTML = '<div class="rh-phone"></div><div class="rh-text">TURN SIDEWAYS<br>FOR 2 PLAYERS</div>' +
        '<button type="button" class="rh-stay">PLAY UPRIGHT</button>';
      hint.querySelector('.rh-stay').addEventListener('click', function () {
        hintDismissed = true;
        rotateHint();
      });
      document.body.appendChild(hint);
      try { screen.orientation.lock('landscape').catch(function () {}); } catch (e) {}
    }
    if (hint) hint.classList.toggle('show', want);
    if (!versus) hintDismissed = false;
  }
  window.addEventListener('resize', rotateHint);

  // ---- Fire buttons (joystick mode) ----
  // The caps were built by deck.js; a cap the game does not declare has no
  // logical name and does nothing.
  stations.forEach(function (st) {
    if (!st.fire) return;
    BlipController.bindButtons(st.fire);
    BlipController.registerVisual(st.fire);
  });

  // The deck is built: fit the picture to what it covers, and once more next
  // frame for the separately injected marquee.
  if (typeof fillCanvas === 'function') {
    fillCanvas();
    requestAnimationFrame(fillCanvas);
  }

  // ---- The 8-way restrictor-gate joystick ----
  // The drag is bound to #topbar (no transform), not the stick base inside
  // the 3D-rotated .deck-panel, so hit-testing is plain 2D. It sets key state
  // via BlipController.set(); reflectInput() draws the lean. One per station,
  // each with its own pointer and lock.
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
    // Which station's half the touch is in, then whether it is left of that
    // station's caps (both stations are stick left, caps right).
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
