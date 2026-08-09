var MAX_COINS = 5;

if ('serviceWorker' in navigator) {
  var _manifest = document.querySelector('link[rel=manifest]');
  if (_manifest) {
    var _swUrl = new URL('sw.js', new URL(_manifest.href, location.href)).href;
    navigator.serviceWorker.register(_swUrl);
  }
}

(function () {
  function updateFixedOverlayVisibility() {
    var atBottom = window.scrollY + window.innerHeight >= document.documentElement.scrollHeight - 60;
    var badges = document.querySelector('.left-badges');
    if (badges) badges.classList.toggle('near-bottom', atBottom);
    var lang = document.querySelector('.lang-switcher');
    if (lang) lang.classList.toggle('near-bottom', atBottom);
  }
  window.addEventListener('scroll',  updateFixedOverlayVisibility, { passive: true });
  window.addEventListener('resize',  updateFixedOverlayVisibility, { passive: true });
  document.addEventListener('DOMContentLoaded', updateFixedOverlayVisibility);
}());

function getCoins() {
  try {
    var n = parseInt(sessionStorage.getItem('blip-coins') || '0', 10);
    return isNaN(n) ? 0 : Math.min(Math.max(n, 0), MAX_COINS);
  } catch (e) { return 0; }
}

function saveCoins(n) {
  try { sessionStorage.setItem('blip-coins', n); } catch (e) {}
}

function updateCoinsHud() {
  var n = getCoins(), icons = '';
  for (var i = 0; i < MAX_COINS; i++) icons += i < n ? '●' : '○';
  var text = 'COINS ' + icons;
  document.querySelectorAll('[data-coins-hud]').forEach(function (el) {
    el.textContent = text;
  });
}

/* ---- Audio ---- */
var _kioskAudioCtx = null;

function getKioskAudio() {
  if (typeof Howler !== 'undefined' && Howler.ctx) {
    if (Howler.ctx.state === 'suspended') Howler.ctx.resume();
    return Howler.ctx;
  }
  if (!_kioskAudioCtx) _kioskAudioCtx = new (window.AudioContext || window.webkitAudioContext)();
  if (_kioskAudioCtx.state === 'suspended') _kioskAudioCtx.resume();
  return _kioskAudioCtx;
}

function playCoinInsert() {
  var ctx = getKioskAudio();
  var t   = ctx.currentTime;
  [{ freq: 1047, start: 0 }, { freq: 1319, start: 0.055 }].forEach(function(note) {
    var osc  = ctx.createOscillator();
    var gain = ctx.createGain();
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.type = 'square';
    osc.frequency.value = note.freq;
    gain.gain.setValueAtTime(0.22, t + note.start);
    gain.gain.exponentialRampToValueAtTime(0.001, t + note.start + 0.11);
    osc.start(t + note.start);
    osc.stop(t + note.start + 0.12);
  });
}

function playNoRoom() {
  var ctx  = getKioskAudio();
  var t    = ctx.currentTime;
  var osc  = ctx.createOscillator();
  var gain = ctx.createGain();
  osc.connect(gain);
  gain.connect(ctx.destination);
  osc.type = 'sawtooth';
  osc.frequency.setValueAtTime(200, t);
  osc.frequency.exponentialRampToValueAtTime(65, t + 0.38);
  gain.gain.setValueAtTime(0.32, t);
  gain.gain.exponentialRampToValueAtTime(0.001, t + 0.38);
  osc.start(t);
  osc.stop(t + 0.39);
}

function flashCoins() {
  ['insert-coin', 'kiosk-coins-hud', 'kiosk-insert-btn'].forEach(function(id) {
    var el = document.getElementById(id);
    if (!el) return;
    el.classList.remove('coin-flash');
    void el.offsetWidth;
    el.classList.add('coin-flash');
    el.addEventListener('animationend', function() { el.classList.remove('coin-flash'); }, { once: true });
  });
}

function insertCoin() {
  var n = getCoins();
  if (n < MAX_COINS) {
    saveCoins(n + 1);
    playCoinInsert();
    updateCoinsHud();
    flashCoins();
    if (typeof window.onCoinInserted === 'function') window.onCoinInserted();
  } else {
    playNoRoom();
    ['insert-coin', 'kiosk-insert-btn'].forEach(function(id) {
      var el = document.getElementById(id);
      if (!el) return;
      el.classList.remove('shake');
      void el.offsetWidth;
      el.classList.add('shake');
      el.addEventListener('animationend', function() { el.classList.remove('shake'); }, { once: true });
    });
  }
}

/* ---- Shared gamepad polling ----
 * Polls the first connected gamepad every frame and reports logical button
 * state changes ('ArrowUp' | 'ArrowDown' | 'ArrowLeft' | 'ArrowRight' |
 * 'Space' | 'KeyZ') to onDown/onUp. Used both by game pages (shell.js, which
 * turns these into synthetic keyboard events on the game canvas) and the
 * kiosk landing page (which uses them to move the card selection).
 */
function pollGamepad(onDown, onUp) {
  var DEADZONE = 0.25;
  var held = {};
  var polling = false;

  var BTN_MAP = [
    { idx: 0,  code: 'Space'      },  // A / Cross   — Button 1
    { idx: 1,  code: 'KeyZ'       },  // B / Circle  — Button 2
    { idx: 2,  code: 'Space'      },
    { idx: 3,  code: 'Space'      },
    { idx: 9,  code: 'Space'      },
    { idx: 12, code: 'ArrowUp'    },
    { idx: 13, code: 'ArrowDown'  },
    { idx: 14, code: 'ArrowLeft'  },
    { idx: 15, code: 'ArrowRight' },
  ];

  function tick() {
    var pads = navigator.getGamepads ? navigator.getGamepads() : [];
    var pad = null;
    for (var i = 0; i < pads.length; i++) {
      if (pads[i] && pads[i].connected) { pad = pads[i]; break; }
    }
    if (!pad) { polling = false; return; }

    var want = {};
    for (var j = 0; j < BTN_MAP.length; j++) {
      var m = BTN_MAP[j];
      var b = pad.buttons[m.idx];
      if (b && (b.pressed || b.value > 0.5)) want[m.code] = true;
    }
    var ax = pad.axes[0] || 0, ay = pad.axes[1] || 0;
    if (ax < -DEADZONE) want['ArrowLeft']  = true;
    if (ax >  DEADZONE) want['ArrowRight'] = true;
    if (ay < -DEADZONE) want['ArrowUp']    = true;
    if (ay >  DEADZONE) want['ArrowDown']  = true;

    var code;
    for (code in want) {
      if (!held[code]) { held[code] = true; onDown(code); }
    }
    for (code in held) {
      if (held[code] && !want[code]) { held[code] = false; onUp(code); }
    }

    requestAnimationFrame(tick);
  }

  window.addEventListener('gamepadconnected', function () {
    if (!polling) { polling = true; requestAnimationFrame(tick); }
  });
}

// iOS suspends AudioContext on load and re-suspends after backgrounding.
// Howler's autoUnlock is disabled (it calls unload() on non-44100 Hz devices, destroying
// all WASM sounds), so we resume Howler.ctx manually on any gesture and on tab refocus.
(function () {
  function unlockAudio() {
    if (typeof Howler === 'undefined') return;
    if (!Howler.ctx) {
      // Force Howler to create its AudioContext now, while inside a user gesture.
      // On iOS, a context created outside a gesture starts suspended; inside one it starts running.
      // Howler.volume() triggers _setupAudioContext() internally via `ctx || _()`.
      Howler.volume();
    }
    if (Howler.ctx && Howler.ctx.state !== 'running') {
      Howler.ctx.resume();
    }
  }
  document.addEventListener('touchstart', unlockAudio, { passive: true, capture: true });
  document.addEventListener('click',      unlockAudio, { capture: true });
  document.addEventListener('visibilitychange', function () {
    if (!document.hidden) unlockAudio();
  });
}());
