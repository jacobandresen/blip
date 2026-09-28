var MAX_COINS = 5;

/* ---- Controller choice ---- The pad by default; #control-toggle switches to
 * the stick. Stored as 'pad' | 'stick' in localStorage and applied as <html
 * data-controls> early, so neither control flashes. */
(function () {
  var m = 'pad';
  try { m = localStorage.getItem('blip-controls') || 'pad'; } catch (e) {}
  document.documentElement.setAttribute('data-controls', m === 'stick' ? 'stick' : 'pad');
}());

/* ---- Layout: upright or landscape ----
 * One rule for every page's deck, applied as <html data-layout> and kept
 * current on resize, so the landing page and a game lay the bar out alike. */
function blipLandscape() {
  return window.innerHeight <= 520 && window.innerWidth > window.innerHeight * 1.25;
}
function blipApplyLayout() {
  document.documentElement.setAttribute('data-layout', blipLandscape() ? 'landscape' : 'upright');
}
blipApplyLayout();
window.addEventListener('resize', blipApplyLayout);
window.addEventListener('orientationchange', blipApplyLayout);

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

/* ---- Per-game cabinet identity ---- Card colours on the landing page,
 * marquee and bezel glow on each game page. `accent` is an "r, g, b" triple
 * so CSS can build solid and translucent colours. */
// `buttons`: the deck's caps, one per button (key/code as injectKey() and the
// keyboard listener match them); omitted = one fire button (Space).
// `stick`: per-game stick geometry, all optional: engage / release (px from
// the floating pivot to catch a detent / fall back to neutral), maxR (how far
// the pivot trails the thumb), hyst (degrees past the 22.5 midline before the
// lock jumps a detent).
var BLIP_GAMES = {
  serpent:            { name: 'SERPENT',  accent: '50, 200, 50'   },
  bouncer:            { name: 'BOUNCER',  accent: '0, 200, 200'   },
  galactic_defender:  { name: 'DEFENDER', accent: '200, 50, 200'  },
  rally:              { name: 'RALLY',    accent: '220, 50, 50'   },
  meteors:            { name: 'METEORS',  accent: '180, 180, 180',
                         buttons: [{ key: ' ', code: 'Space' }, { key: 'z', code: 'KeyZ' }] },
  // A fighter: two caps, punch and kick (holding toward the opponent hits
  // high), and a stick that answers at once, since a sloppy neutral is a
  // dropped guard. `players: 2` builds the second station, revealed by
  // blip_set_mode; `keys` gives P1 WASD so the arrows are P2's.
  brawler:            { name: 'BRAWLER', accent: '220, 60, 40',
                         players: 2,
                         buttons: [{ key: 'f', code: 'KeyF', label: 'PUNCH' },
                                   { key: 'g', code: 'KeyG', label: 'KICK' }],
                         keys: { up:    { key: 'w', code: 'KeyW' },
                                 down:  { key: 's', code: 'KeyS' },
                                 left:  { key: 'a', code: 'KeyA' },
                                 right: { key: 'd', code: 'KeyD' },
                                 p2up:    { key: 'ArrowUp',    code: 'ArrowUp' },
                                 p2down:  { key: 'ArrowDown',  code: 'ArrowDown' },
                                 p2left:  { key: 'ArrowLeft',  code: 'ArrowLeft' },
                                 p2right: { key: 'ArrowRight', code: 'ArrowRight' },
                                 p2button1: { key: 'j', code: 'KeyJ' },
                                 p2button2: { key: 'k', code: 'KeyK' } },
                         stick: { engage: 10, release: 6, maxR: 54, hyst: 10 } },
  // A tribute to Bubble Bobble. Two caps: bubble (fire) and jump; up
  // jumps too. Like Brawler, player one is WASD so the arrows are player
  // two's, who drops in with their own bubble or jump.
  bubbler:            { name: 'BUBBLER', accent: '120, 210, 255',
                         players: 2,
                         buttons: [{ key: 'f', code: 'KeyF', label: 'BUBBLE' },
                                   { key: 'g', code: 'KeyG', label: 'JUMP' }],
                         keys: { up:    { key: 'w', code: 'KeyW' },
                                 down:  { key: 's', code: 'KeyS' },
                                 left:  { key: 'a', code: 'KeyA' },
                                 right: { key: 'd', code: 'KeyD' },
                                 p2up:    { key: 'ArrowUp',    code: 'ArrowUp' },
                                 p2down:  { key: 'ArrowDown',  code: 'ArrowDown' },
                                 p2left:  { key: 'ArrowLeft',  code: 'ArrowLeft' },
                                 p2right: { key: 'ArrowRight', code: 'ArrowRight' },
                                 p2button1: { key: 'j', code: 'KeyJ' },
                                 p2button2: { key: 'k', code: 'KeyK' } } },
  // A bullet-weaving shooter: fine nudges. A small dead zone, a long pivot
  // leash so re-centring neutralises, and a firm notch so a dodge holds.
  sky_raider:         { name: 'RAIDER', accent: '50, 100, 220',
                         stick: { engage: 11, release: 6, maxR: 58, hyst: 12 } }
};

// Pick out the game slug from a shell-page URL, e.g. "/blip/serpent/index.html"
// or "/serpent/" both resolve to "serpent". Returns null off the game pages.
function blipGameFromPath(pathname) {
  var m = /\/([a-z_]+)\/(?:index\.html)?$/i.exec(pathname || '');
  var g = m && BLIP_GAMES[m[1]];
  return g ? { slug: m[1], name: g.name, accent: g.accent, buttons: g.buttons,
               keys: g.keys, players: g.players || 1, stick: g.stick } : null;
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

// A coin is two sounds: the inharmonic clink through the chute, then the
// register's "credit accepted" chime. Keep in sync with shell.js's copy.
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

// The coin that drops into the slot (kiosk.css #coin-drop-anim): a real
// element so a class toggle can animate it, injected once for every non-game
// page.
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
  // Measured, like shell.js's copy: the slot's centre is padding-right plus
  // half its width, and padding changes across breakpoints.
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

// While the landing page (.game-grid) shows zero credits, the COINS button
// throbs (.needs-coin) until the first coin.
function updateCoinBeckon() {
  if (!document.querySelector('.game-grid')) return;
  var btn = document.getElementById('kiosk-insert-btn');
  if (btn) btn.classList.toggle('needs-coin', getCoins() <= 0);
}
window.addEventListener('load', updateCoinBeckon);

/* ---- BLIP logo overcharge glitch ---- Now and then one letter arcs as if
 * surged, the rest flicker, with a zap on the speaker. Every page loads
 * kiosk.js; deferred to 'load' because game pages load it in <head>. */
(function () {
  var reduceMotion = !!(window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  if (reduceMotion) return; // no schedule at all — this is pure motion, nothing informational

  window.addEventListener('load', function () {
    var logo = document.querySelector('.blip-logo');
    if (!logo || !logo.firstChild || logo.firstChild.nodeType !== Node.TEXT_NODE) return;

    // One span per letter of "BLIP" (the part of the wordmark always shown)
    // so one can be struck.
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

      // A crackle burst (filtered noise), sharper and louder than the
      // coin sfx's — this has to carry across a whole room, not just
      // confirm a tap.
      function crackle(start, dur, gain) {
        var buf = ctx.createBuffer(1, Math.ceil(ctx.sampleRate * dur), ctx.sampleRate);
        var data = buf.getChannelData(0);
        for (var i = 0; i < data.length; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / data.length);
        var noise = ctx.createBufferSource();
        noise.buffer = buf;
        var filt = ctx.createBiquadFilter();
        filt.type = 'highpass';
        filt.frequency.value = 2200;
        var ng = ctx.createGain();
        ng.gain.setValueAtTime(gain, start);
        ng.gain.exponentialRampToValueAtTime(0.001, start + dur);
        noise.connect(filt); filt.connect(ng); ng.connect(ctx.destination);
        noise.start(start); noise.stop(start + dur);
      }
      crackle(t, 0.16, 0.65);
      crackle(t + 0.22, 0.09, 0.4);   // a second, smaller sputter as it flickers back
      crackle(t + 0.36, 0.06, 0.25);

      // The main arc: a fast descending sawtooth sweep, shriek to thud.
      var osc = ctx.createOscillator();
      var og = ctx.createGain();
      osc.type = 'sawtooth';
      osc.frequency.setValueAtTime(3200, t);
      osc.frequency.exponentialRampToValueAtTime(70, t + 0.2);
      og.gain.setValueAtTime(0.3, t);
      og.gain.exponentialRampToValueAtTime(0.001, t + 0.22);
      osc.connect(og); og.connect(ctx.destination);
      osc.start(t); osc.stop(t + 0.23);

      // A low undertone thump, like the sign's transformer took the hit.
      var thump = ctx.createOscillator();
      var tg = ctx.createGain();
      thump.type = 'triangle';
      thump.frequency.setValueAtTime(110, t);
      thump.frequency.exponentialRampToValueAtTime(45, t + 0.3);
      tg.gain.setValueAtTime(0.28, t + 0.02);
      tg.gain.exponentialRampToValueAtTime(0.001, t + 0.32);
      thump.connect(tg); tg.connect(ctx.destination);
      thump.start(t + 0.02); thump.stop(t + 0.33);
    }

    function zap(forced) {
      if (document.hidden && !forced) return;
      var el = chars[Math.floor(Math.random() * chars.length)];
      logo.classList.add('lg-surge');
      el.classList.add('lg-zap');
      playZap();
      var done = function () {
        el.classList.remove('lg-zap');
        logo.classList.remove('lg-surge');
      };
      el.addEventListener('animationend', done, { once: true });
      setTimeout(done, 900); // in case the tab was hidden mid-animation and it never fired
    }

    // The next strike is 45s-3min out, re-armed after each one (and while
    // hidden) rather than on an interval, so a backgrounded tab does not fire
    // a burst of overdue zaps.
    function scheduleNext() {
      var delay = 45000 + Math.random() * 135000;
      setTimeout(function () {
        if (document.hidden) { scheduleNext(); return; }
        zap();
        scheduleNext();
      }, delay);
    }
    scheduleNext();

    // Manual trigger for testing/demoing, since the real thing is 45s-3min
    // apart by design: run `blipZapLogo()` in the console to fire one now.
    window.blipZapLogo = function () { zap(true); };
  });
}());

/* ---- Shared gamepad polling ---- Reports logical button changes ('ArrowUp'
 * | 'ArrowDown' | 'ArrowLeft' | 'ArrowRight' | 'Space' | 'KeyZ') from the
 * first gamepad to onDown / onUp; game pages turn them into key events, the
 * landing page moves the card selection. */
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

  // Polled unconditionally: Chrome on Linux never fires 'gamepadconnected'
  // for many generic joysticks, and rAF polling costs nothing with none
  // attached. Station one's lamp: off with no pad, dim when idle for IDLE_MS,
  // lit in use.
  var IDLE_MS = 10000;
  var root = document.documentElement;
  var lastInput = -Infinity, lamp = '';
  function setLamp(state) {
    if (state === lamp) return;
    lamp = state;
    if (state) root.setAttribute('data-pad', state);
    else root.removeAttribute('data-pad');
  }

  function tick() {
    var pad = findPad();
    if (!pad) setLamp('');
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

      var now = performance.now();
      for (var any in want) { lastInput = now; break; }
      setLamp(now - lastInput < IDLE_MS ? 'active' : 'idle');
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

/* The deck itself is built by deck.js, the same on every page. */
