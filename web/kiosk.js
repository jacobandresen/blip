var MAX_COINS = 5;

/* ---- On-screen controller choice ----
 * The deck shows the Super Nintendo pad by default; a subtle
 * #control-toggle flips it back to the classic arcade joystick. The
 * choice is one word in localStorage ('pad' | 'stick') and is applied as
 * <html data-controls> as early as possible so neither control flashes
 * before its CSS decides which is visible. Shared by every page's deck. */
(function () {
  var m = 'pad';
  try { m = localStorage.getItem('blip-controls') || 'pad'; } catch (e) {}
  document.documentElement.setAttribute('data-controls', m === 'stick' ? 'stick' : 'pad');
}());

function blipControls() {
  return document.documentElement.getAttribute('data-controls') === 'stick' ? 'stick' : 'pad';
}

function blipSetControls(mode) {
  mode = (mode === 'stick') ? 'stick' : 'pad';
  try { localStorage.setItem('blip-controls', mode); } catch (e) {}
  document.documentElement.setAttribute('data-controls', mode);
  if (typeof window.onBlipControlsChange === 'function') {
    try { window.onBlipControlsChange(mode); } catch (e) {}
  }
}

/* ---- Per-game cabinet identity ----
 * Shared by the kiosk landing page (card colours) and each game's shell page
 * (marquee sign + bezel glow), so every cabinet reads as its own machine
 * instead of five copies of the same green box. `accent` is an "r, g, b"
 * triple so CSS can build both solid and translucent colours from one value.
 */
// `buttons` lists the on-screen arcade buttons for the shell's touch/visual
// controls, one entry per button (key/code = what injectKey()/the keyboard
// listener match against). Omitted = a single default fire button (Space).
//
// `stick` overrides the touch 8-way-stick geometry for that game (shell.js
// defaults if absent). Fields, all optional: engage/release (px from the
// floating pivot to catch a gate detent / fall back to neutral), maxR (how
// far the pivot is allowed to trail the thumb — smaller = a shorter unwind
// to stop or reverse), and hyst (deg past the 22.5deg midline the thumb
// must rotate before the lock jumps to the next detent — the notch).
var BLIP_GAMES = {
  serpent:            { name: 'SERPENT',  accent: '50, 200, 50'   },
  bouncer:            { name: 'BOUNCER',  accent: '0, 200, 200'   },
  galactic_defender:  { name: 'DEFENDER', accent: '200, 50, 200'  },
  rally:              { name: 'RALLY',    accent: '220, 50, 50'   },
  meteors:            { name: 'METEORS',  accent: '180, 180, 180',
                         buttons: [{ key: ' ', code: 'Space' }, { key: 'z', code: 'KeyZ' }] },
  // Raider is a bullet-weaving shooter — fine adjustments, not broad
  // sweeps. Smaller dead zone so a light nudge already catches a detent; a
  // long pivot leash so the map stays put instead of creeping (re-centring
  // then reliably neutralises); a firmer notch so a held dodge doesn't
  // slip a detent mid-weave.
  sky_raider:         { name: 'RAIDER', accent: '50, 100, 220',
                         stick: { engage: 11, release: 6, maxR: 58, hyst: 12 } }
};

// Pick out the game slug from a shell-page URL, e.g. "/blip/serpent/index.html"
// or "/serpent/" both resolve to "serpent". Returns null off the game pages.
function blipGameFromPath(pathname) {
  var m = /\/([a-z_]+)\/(?:index\.html)?$/i.exec(pathname || '');
  var g = m && BLIP_GAMES[m[1]];
  return g ? { slug: m[1], name: g.name, accent: g.accent, buttons: g.buttons, stick: g.stick } : null;
}

if ('serviceWorker' in navigator) {
  var _manifest = document.querySelector('link[rel=manifest]');
  if (_manifest) {
    var _swUrl = new URL('sw.js', new URL(_manifest.href, location.href)).href;
    navigator.serviceWorker.register(_swUrl);
  }
}

(function () {
  // The about / history / controls pages (they carry a .page wrapper) keep
  // the corner badges + flags visible the whole time — don't fade them
  // near the bottom there. This only trims them on the landing page.
  if (document.querySelector('.page')) return;
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

// A real coin makes two distinct sounds in sequence: the metallic clink
// of it dropping through the chute (bright, inharmonic, near-instant —
// real metal doesn't ring in tidy octaves the way a synth voice does),
// then the register's own electronic "credit accepted" chime a beat
// later. Modelling both, rather than just the chime alone, is what reads
// as an actual coin rather than a UI beep. (Kept in sync with shell.js's
// copy, used on the game pages.)
function playCoinInsert() {
  var ctx = getKioskAudio();
  var t   = ctx.currentTime;

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

  [{ freq: 1047, start: 0.1 }, { freq: 1319, start: 0.155 }].forEach(function(note) {
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

// The coin that visually drops into the insert-coin button's slot on a
// successful insert (see kiosk.css's #coin-drop-anim / @keyframes
// coin-drop). A real element rather than a pseudo-element so a class
// toggle can animate it on demand; injected here rather than duplicated
// across index.html/history.html/about.html.
var coinDropAnim = null;
(function () {
  var btn = document.getElementById('kiosk-insert-btn');
  if (!btn) return;
  coinDropAnim = document.createElement('span');
  coinDropAnim.id = 'coin-drop-anim';
  coinDropAnim.setAttribute('aria-hidden', 'true');
  btn.appendChild(coinDropAnim);
}());
function dropCoinAnimation() {
  if (!coinDropAnim) return;
  // See shell.js's copy of this function for why this is measured rather
  // than a fixed guess: the slot's centre is padding-right + half its own
  // width in from the button's edge, and that padding changes across the
  // responsive breakpoints.
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
  ['insert-coin', 'kiosk-insert-btn'].forEach(function(id) {
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
    dropCoinAnimation();
    updateCoinsHud();
    updateCoinBeckon();
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

// Point first-time (and broke) visitors at the coin slot: while the
// landing page shows zero credits, the corner COINS button throbs
// (.needs-coin, kiosk.css) from the moment the page loads until the first
// coin goes in. Gated on .game-grid, which only exists on index.html.
function updateCoinBeckon() {
  if (!document.querySelector('.game-grid')) return;
  var btn = document.getElementById('kiosk-insert-btn');
  if (btn) btn.classList.toggle('needs-coin', getCoins() <= 0);
}
window.addEventListener('load', updateCoinBeckon);

/* ---- BLIP logo overcharge glitch ----
 * A seldom, random electrical fault on the wordmark: one letter of "BLIP"
 * arcs like it's taken a surge, the rest of the sign flickers as if power
 * got diverted, with a matching zap/crackle on the speaker. Purely a
 * decorative flourish — every page carries the logo (kiosk.js is loaded
 * on all of them), so this one setup covers the whole site.
 * Deferred to 'load': kiosk.js sits in <head> on the game-shell pages, so
 * .blip-logo doesn't exist in the DOM yet at parse time. */
(function () {
  var reduceMotion = !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  if (reduceMotion) return; // no schedule at all — this is pure motion, nothing informational

  window.addEventListener('load', function () {
    var logo = document.querySelector('.blip-logo');
    if (!logo || !logo.firstChild || logo.firstChild.nodeType !== Node.TEXT_NODE) return;

    // Split the "BLIP" text node (before the .ba-rest " ARCADE" span) into
    // one span per letter, so a single one can be singled out. "BLIP" is
    // the part of the wordmark that's always visible (ba-rest hides on
    // phones), so that's the pool the random strike is drawn from.
    var letters = logo.firstChild.textContent.split('');
    var frag = document.createDocumentFragment();
    letters.forEach(function (ch) {
      var span = document.createElement('span');
      span.className = 'lg-ch';
      span.textContent = ch;
      frag.appendChild(span);
    });
    logo.replaceChild(frag, logo.firstChild);
    var chars = logo.querySelectorAll('.lg-ch');
    if (!chars.length) return;

    // An electrical crackle (filtered noise burst) under a fast descending
    // zap (a sawtooth sweeping from a shriek down to a thud) — the same
    // procedural-audio recipe as playCoinInsert/playNoRoom above.
    function playZap() {
      var ctx = getKioskAudio();
      var t = ctx.currentTime;

      var buf = ctx.createBuffer(1, Math.ceil(ctx.sampleRate * 0.14), ctx.sampleRate);
      var data = buf.getChannelData(0);
      for (var i = 0; i < data.length; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / data.length);
      var noise = ctx.createBufferSource();
      noise.buffer = buf;
      var filt = ctx.createBiquadFilter();
      filt.type = 'highpass';
      filt.frequency.value = 2400;
      var ng = ctx.createGain();
      ng.gain.setValueAtTime(0.5, t);
      ng.gain.exponentialRampToValueAtTime(0.001, t + 0.14);
      noise.connect(filt); filt.connect(ng); ng.connect(ctx.destination);
      noise.start(t); noise.stop(t + 0.14);

      var osc = ctx.createOscillator();
      var og = ctx.createGain();
      osc.type = 'sawtooth';
      osc.frequency.setValueAtTime(2600, t);
      osc.frequency.exponentialRampToValueAtTime(80, t + 0.17);
      og.gain.setValueAtTime(0.22, t);
      og.gain.exponentialRampToValueAtTime(0.001, t + 0.19);
      osc.connect(og); og.connect(ctx.destination);
      osc.start(t); osc.stop(t + 0.2);
    }

    function zap() {
      if (document.hidden) return;
      var el = chars[Math.floor(Math.random() * chars.length)];
      logo.classList.add('lg-surge');
      el.classList.add('lg-zap');
      playZap();
      var done = function () {
        el.classList.remove('lg-zap');
        logo.classList.remove('lg-surge');
      };
      el.addEventListener('animationend', done, { once: true });
      setTimeout(done, 500); // in case the tab was hidden mid-animation and it never fired
    }

    // Seldom: the next strike lands 45s-3min out, so it reads as a rare
    // fault rather than a tic. Re-armed after every strike (and skipped,
    // then re-armed, while the tab is hidden) rather than on a fixed
    // interval — running it down while backgrounded would otherwise fire
    // a burst of overdue zaps the moment the tab regains focus.
    function scheduleNext() {
      var delay = 45000 + Math.random() * 135000;
      setTimeout(function () {
        if (document.hidden) { scheduleNext(); return; }
        zap();
        scheduleNext();
      }, delay);
    }
    scheduleNext();
  });
}());

/* ---- Shared gamepad polling ----
 * Polls the first connected gamepad every frame and reports logical button
 * state changes ('ArrowUp' | 'ArrowDown' | 'ArrowLeft' | 'ArrowRight' |
 * 'Space' | 'KeyZ') to onDown/onUp. Used both by game pages (shell.js, which
 * turns these into synthetic keyboard events on the game canvas) and the
 * kiosk landing page (which uses them to move the card selection).
 */
function pollGamepad(onDown, onUp) {
  if (!navigator.getGamepads) return;

  var DEADZONE = 0.25;
  var held = {};

  var BTN_MAP = [
    { idx: 0,  code: 'Space'      },  // A / Cross   — Button 1
    { idx: 1,  code: 'KeyZ'       },  // B / Circle  — Button 2
    { idx: 2,  code: 'KeyZ'       },  // X / Square  — Button 2
    { idx: 3,  code: 'Space'      },  // Y / Triangle— Button 1
    { idx: 4,  code: 'KeyZ'       },  // L shoulder  — Button 2
    { idx: 5,  code: 'KeyZ'       },  // R shoulder  — Button 2
    { idx: 8,  code: 'Select'     },  // Select / Back — arcade menu
    { idx: 9,  code: 'Start'      },  // Start / Options — start the game
    { idx: 12, code: 'ArrowUp'    },
    { idx: 13, code: 'ArrowDown'  },
    { idx: 14, code: 'ArrowLeft'  },
    { idx: 15, code: 'ArrowRight' },
  ];

  function findPad() {
    var pads = navigator.getGamepads();
    for (var i = 0; i < pads.length; i++) {
      if (pads[i] && pads[i].connected) return pads[i];
    }
    return null;
  }

  // Don't gate polling on the 'gamepadconnected' event — many browsers
  // (notably Chrome on Linux) never fire it for generic/non-standard
  // joysticks, even though navigator.getGamepads() reports them fine once
  // a button has been pressed. Poll unconditionally instead; it's a cheap
  // native call and requestAnimationFrame self-throttles to the display
  // refresh rate, so there's no meaningful cost while nothing is connected.
  function tick() {
    var pad = findPad();
    if (pad) {
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
    }
    requestAnimationFrame(tick);
  }

  requestAnimationFrame(tick);
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
