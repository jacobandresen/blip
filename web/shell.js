(function () {
'use strict';

// Game-page touches are input; iOS still pinches to zoom despite `touch-action: none`.
['gesturestart', 'gesturechange', 'gestureend'].forEach(function (t) {
  document.addEventListener(t, function (e) { e.preventDefault(); }, { passive: false });
});

// The vendored runtime reports its failures with alert(), a dialog nobody
// at a kiosk can dismiss. They go to the console instead.
window.alert = function (msg) { try { console.error('[blip] ' + msg); } catch (e) {} };

// ---- Pause ---- The games have no pause of their own. In app mode the
// controls card and the Guide stop the game as an app's menus do: the
// runtime's frame callback (named `animation`) is held back and the sound
// suspended. The engine caps a frame at 0.1s, so play resumes without a jump.
(function () {
  var paused = false, held = [];
  var raf = window.requestAnimationFrame.bind(window);
  window.requestAnimationFrame = function (cb) {
    if (paused && cb && cb.name === 'animation') { held.push(cb); return -1; }
    return raf(cb);
  };
  window.blipPause = function (on) {
    on = !!on;
    if (on === paused) return;
    paused = on;
    document.documentElement.toggleAttribute('data-paused', on);
    try {
      if (typeof Howler !== 'undefined' && Howler.ctx) { if (on) Howler.ctx.suspend(); else Howler.ctx.resume(); }
    } catch (e) {}
    if (window.BlipController && on) window.BlipController.releaseAll();
    if (!on) { var q = held; held = []; q.forEach(function (cb) { raf(cb); }); }
  };
}());

var TOPBAR_H   = 56;
var MARQUEE_H  = 94; // marquee and the card receiver beneath it
var CABINET_SCREEN = new URLSearchParams(location.search).get('cabinet') === '1';
var GAME = (typeof blipGameFromPath === 'function')
  ? blipGameFromPath(location.pathname) : null;
if (CABINET_SCREEN) document.documentElement.setAttribute('data-cabinet-screen','');
var FRAME_PAD  = 14; // matches the cabinet screen's inset below the arcade

var loader    = document.getElementById('loader');
var barInner  = document.getElementById('bar-inner');
var statusEl  = document.getElementById('status');
var canvas    = document.getElementById('glcanvas');
var overlay   = document.getElementById('need-coin-overlay');
// Launched from the home screen or the game's own icon: no coins at all.
var FREE_PLAY = blipFreePlay();

// The runtime focuses its canvas at boot; a cabinet preview must leave keys with the arcade.
if (CABINET_SCREEN) canvas.inert = true;

updateCoinsHud();

// Inside BLIP's monitor the game is a frame: its parent takes it down.
function leaveGame() {
  if (blipAmstradMode()) parent.postMessage({ type: 'blip-amstrad-exit' }, location.origin);
  else location.href = blipFrontendUrl();
}

var returningToCabinet = false;
function returnToCabinet(event) {
  if (event && (event.ctrlKey || event.metaKey || event.shiftKey || event.altKey)) return;
  if (event) event.preventDefault();
  if (blipAmstradMode()) { parent.postMessage({ type: 'blip-amstrad-exit' }, location.origin); return; }
  if (CABINET_SCREEN || returningToCabinet) return;
  returningToCabinet = true;
  var departure = { slug:GAME && GAME.slug, at:Date.now() };
  if (blipAppMode()) {
    // The game's own app has nowhere else to go: back to its title.
  } else if (blipFrontend() === 'home' && document.documentElement.hasAttribute('data-appstyle')) {
    try { sessionStorage.setItem('blip-home-return', JSON.stringify(departure)); } catch (e) {}
  } else {
    try { departure.frame = canvas.toDataURL('image/png'); } catch (e) {}
    try { sessionStorage.setItem('blip-cabinet-return', JSON.stringify(departure)); } catch (e) {}
  }
  location.href = blipFrontendUrl();
}

// ---- Per-game marquee + cabinet accent ----
// Built here rather than in each game's HTML.
(function () {
  if(CABINET_SCREEN) return;
  var game = GAME;
  if (!game) return;
  document.documentElement.style.setProperty('--cab', game.accent);
  var bar = document.createElement('div');
  bar.id = 'marquee-bar';
  bar.className = 'jukebox-marquee';
  bar.dataset.machine = game.slug;
  var sign = document.createElement('a');
  sign.href=blipFrontendUrl();sign.title=blipAppMode()?'Back to the title':'Return to the '+({home:'home screen',basic:'BLIP',cabinet:'cabinet'}[blipFrontend()]);
  sign.id = 'marquee-name';
  sign.setAttribute('aria-label', game.name);
  sign.dataset.title = game.name.charAt(0) + game.name.slice(1).toLowerCase();
  showBest(sign);
  // An app's launch screen shows its icon while the game loads.
  document.documentElement.style.setProperty('--app-icon', 'url("' + new URL('icon-512.png?v=2', location.href).href + '")');
  for (var i = 0; i < 8; i++) {
    var char = '\u00a0';
    var tile = document.createElement('span');
    tile.className = 'marquee-letter';
    tile.style.setProperty('--flap-index', i);
    tile.textContent = char;
    sign.appendChild(tile);

  }
  // The cabinet's own marquee (marquee.css), as it stands in the arcade: the same machine.
  sign.classList.add('machine-marquee');
  sign.appendChild(Object.assign(document.createElement('b'), { className: 'machine-marquee-name', textContent: game.name }));
  bar.append(
    Object.assign(document.createElement('span'), { className: 'marquee-bulbs' }),
    sign,
    Object.assign(document.createElement('span'), { className: 'marquee-bulbs' })
  );
  blipCardReceiver(bar);
  document.body.insertBefore(bar, document.body.firstChild);
  markGameLoaded(game, bar);
  sign.addEventListener('click', returnToCabinet);
  document.querySelector('.blip-logo').addEventListener('click', returnToCabinet);
  // An app's way back: "< Home" where the logo stands. A game in its own app has nowhere to go back to.
  if (document.documentElement.hasAttribute('data-appstyle') && !blipAppMode()) {
    var back = document.createElement('a');
    back.className = 'app-back';
    back.href = blipFrontendUrl();
    back.innerHTML = '<svg viewBox="0 0 12 20" aria-hidden="true"><path d="M10 2 2 10l8 8" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/></svg>' + (blipFrontend() === 'basic' ? 'BLIP' : 'Home');
    back.addEventListener('click', returnToCabinet);
    document.body.appendChild(back);
  }

  var logo = document.querySelector('.blip-logo');
  if (logo) {
    logo.classList.add('boot');
    logo.addEventListener('animationend', function () {
      logo.classList.remove('boot');
    }, { once: true });
  }
}());

// A kiosk nobody is playing goes back to the cabinet: in fullscreen, two
// minutes without a key, a touch or a pad press (deck and gamepad arrive as
// key events on the canvas); 40 s when the cabinet's attract loop opened it.
(function () {
  var attract = false;
  try { attract = sessionStorage.getItem('blip-attract') === '1'; sessionStorage.removeItem('blip-attract'); } catch (e) {}
  var IDLE_MS = attract ? 40000 : 120000, last = Date.now();
  function poke() { last = Date.now(); IDLE_MS = 120000; }
  ['keydown', 'pointerdown', 'pointermove', 'touchstart'].forEach(function (ev) {
    window.addEventListener(ev, poke, { capture: true, passive: true });
  });
  // A finger held still sends nothing more, but it is still playing.
  var down = 0;
  window.addEventListener('pointerdown', function () { down++; }, { capture: true, passive: true });
  ['pointerup', 'pointercancel'].forEach(function (ev) {
    window.addEventListener(ev, function () { down = Math.max(0, down - 1); }, { capture: true, passive: true });
  });
  // However it got there, a miscount must not hold the kiosk awake: no touches left means none down.
  ['touchend', 'touchcancel'].forEach(function (ev) {
    document.addEventListener(ev, function (e) { if (!e.touches.length) down = 0; }, { capture: true, passive: true });
  });
  window.addEventListener('blur', function () { down = 0; });
  setInterval(function () {
    if (down > 0) poke();
    if (!document.documentElement.hasAttribute('data-fullscreen') || document.hidden) return;
    // A name prompt or score board nobody has touched for a minute is put
    // away (typing counts as touching), or it would hold the kiosk here.
    var modal = document.querySelector('.blip-hs-modal');
    if (modal) {
      if (Date.now() - last < 60000) return;
      var out = Array.prototype.filter.call(modal.querySelectorAll('button'), function (b) {
        return /^(SKIP|CANCEL|CONTINUE|I SAVED IT)$/.test(b.textContent);
      })[0];
      if (out) out.click(); else modal.remove();
      return;
    }
    if (Date.now() - last >= IDLE_MS) leaveGame();
  }, 5000);
}());

// Stop all audio when navigating away (pagehide is reliable on iOS Safari / PWA)
window.addEventListener('pagehide', function () {
  if (typeof Howler !== 'undefined') Howler.stop();
});

function markGameLoaded(game, bar) {
  var receiver=bar.querySelector('.jukebox-receiver');
  receiver.classList.add('stowed');
  blipSetMarquee(game.name, false);
  document.documentElement.removeAttribute('data-card-loading');
  try {
    var selectionOrder = ['rally', 'bouncer', 'defender', 'bubbler', 'raider', 'meteors', 'serpent', 'brawler', 'adder'];
    var code = 'A' + (selectionOrder.indexOf(game.slug) + 1);
    var rom={
      slug: game.slug, name: game.name, code: code,
      art: new URL('screenshot.png', location.href).href
    };
    blipSeatRom(receiver,rom);
    sessionStorage.setItem('blip-last-game', game.slug);
    ['blip-card-insert', 'blip-card-seated', 'blip-title-seated'].forEach(function (key) {
      sessionStorage.removeItem(key);
    });
  } catch (e) {}
  if(!receiver.classList.contains('loaded'))blipSeatRom(receiver,{name:game.name,art:'screenshot.png'});
}

// Control detents use vibration where supported, otherwise a 40 ms sub-bass pulse.
var HAS_TOUCH = blipHasTouch();
function feedbackTick() {
  if (!HAS_TOUCH || overlay.classList.contains('visible')) return;
  if (navigator.vibrate) {
    try { if (navigator.vibrate(7)) return; } catch (e) {}
  }
  try {
    var ctx = getKioskAudio();
    if (!ctx) return;
    var t = ctx.currentTime;
    var osc = ctx.createOscillator(), gain = ctx.createGain();
    osc.type = 'sine';
    osc.frequency.setValueAtTime(150, t);
    osc.frequency.exponentialRampToValueAtTime(55, t + 0.026);
    gain.gain.setValueAtTime(0.0001, t);
    gain.gain.exponentialRampToValueAtTime(0.2, t + 0.004);
    gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.05);
    osc.connect(gain); gain.connect(blipOut(ctx));
    osc.start(t); osc.stop(t + 0.055);
  } catch (e) {}
}
// kiosk.js's coin drop (dropCoinAnimation), into #insert-coin-btn's slot.
(function () {
  var btn = document.getElementById('insert-coin-btn');
  if (!btn) return;
  coinDropAnim = btn.querySelector('#coin-drop-anim');
  if(!coinDropAnim) {
    coinDropAnim = document.createElement('span');
    coinDropAnim.id = 'coin-drop-anim';
    coinDropAnim.setAttribute('aria-hidden', 'true');
    btn.appendChild(coinDropAnim);
  }
}());
// Called from WASM when play starts (from the title or a game-over
// restart), once more per second player. Out of coins, the debt is
// kept and the coins that go in next pay it before the game goes on.
var coinsOwed = 0;
window.blipSpendCoin = function () {
  if (FREE_PLAY) return 1;
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
  if (!FREE_PLAY && getCoins() <= 0) overlay.classList.add('visible');
  setTimeout(showBest, 1500);
  if (!window.blipScores) return;
  window.blipScores.onGameOver(GAME ? GAME.slug : null, score);
};

// The title bar's subtitle in app mode: the score to beat.
function showBest(sign) {
  sign = sign || document.getElementById('marquee-name');
  if (!sign) return;
  var t = blipTopEntry();
  if (t) sign.dataset.best = 'Best ' + t.score.toLocaleString() + (t.name ? ' \u00b7 ' + t.name : '');
  else delete sign.dataset.best;
}

// Return the leaderboard entry, or this browser's best and claimed handle.
function blipTopEntry() {
  var game = GAME;
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
  if (coinsOwed <= 0) overlay.classList.remove('visible');
}

document.getElementById('insert-coin-btn').addEventListener('click', function () {
  if (returningToCabinet) return;
  if (getCoins() >= MAX_COINS) playNoRoom();
  else coinIn();
});

// The wall itself is a coin slot: a click on it is a credit.
overlay.addEventListener('click', function (e) {
  if (e.isTrusted && !returningToCabinet && getCoins() < MAX_COINS) coinIn();
  try { canvas.focus(); } catch (err) {}
});

// The controls card (kiosk.js builds it): this game's keys, or its touch gestures on a phone.
var cardApi = (function () {
  var noTouch = !document.documentElement.hasAttribute('data-has-touch');
  var list = GAME && (noTouch ? GAME.controls : GAME.touchControls);
  if (!list) return null;
  var appStyle = document.documentElement.hasAttribute('data-appstyle');
  var exitLabel = blipAppMode() ? 'BACK TO THE TITLE' : appStyle ? 'HOME' : 'ANOTHER GAME';
  // A game in its own app has no Home to tap, only Backspace back to its title.
  if (noTouch || !blipAppMode()) list = list.concat([[exitLabel, noTouch ? ['BACKSPACE'] : [appStyle ? 'TAP ‹ HOME' : 'TAP BLIP LOGO']]]);
  var install = blipStandalone() ? '' : '<button type="button" id="cc-install">' +
    (appStyle ? 'Add to Home Screen' : 'ADD ' + GAME.name + ' TO HOME SCREEN') + '</button>';
  // The app's Guide, open at this game's page (the cabinet has its book).
  var guide = appStyle ? '<button type="button" id="cc-guide">Guide</button>' : '';
  install = install || guide ? '<p class="cc-install">' + guide + install + '</p>' : '';
  return blipControlsCard({
    label: 'HOW TO PLAY', name: GAME.name, html: '<div class="pc-legend">' + legendHtml(list) + '</div>' + install,
    after: function (open) {
      if (document.documentElement.hasAttribute('data-appstyle')) window.blipPause(open);
      try { canvas.focus({ preventScroll: true }); } catch (e) {}
    }
  });
}());

// ---- The score prompts as iOS alerts (app style) ---- Their labels stay
// as written (code and tests match them); data-ios carries the title-case
// label an alert shows.
if (document.documentElement.hasAttribute('data-appstyle')) {
  new MutationObserver(function () {
    Array.prototype.forEach.call(document.querySelectorAll('.blip-hs-btn:not([data-ios])'), function (b) {
      var t = b.textContent.trim();
      b.dataset.ios = t === 'OK' ? t : t.toLowerCase().replace(/\b[a-z]/g, function (c) { return c.toUpperCase(); });
    });
  }).observe(document.body, { childList: true, subtree: true });
}

// ---- Saving this game to a home screen ---- Chrome and Android install it
// from their own prompt; iOS can only be told where Share is. Either way the
// saved page is ./?app, so the icon opens the game in its own app mode.
function showInstallSheet() {
  if (CABINET_SCREEN || blipStandalone() || !GAME || document.getElementById('install-sheet')) return;
  try { history.replaceState(null, '', './?app'); } catch (e) {}
  var name = GAME.name.charAt(0) + GAME.name.slice(1).toLowerCase();
  var sheet = document.createElement('div');
  sheet.id = 'install-sheet';
  sheet.setAttribute('role', 'dialog');
  sheet.setAttribute('aria-modal', 'true');
  sheet.setAttribute('aria-labelledby', 'install-title');
  var share = '<svg class="install-share" viewBox="0 0 16 20" aria-label="Share"><path d="M8 1v11M4.5 4.5 8 1l3.5 3.5M5 8H2v11h12V8h-3" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  var body = blipIOS()
    ? '<ol class="install-steps"><li>Tap ' + share + ' <b>Share</b> in Safari\'s toolbar.</li><li>Choose <b>Add to Home Screen</b>.</li></ol>'
    : '<p class="install-wait">Use your browser\'s menu: <b>Install app</b> or <b>Add to Home screen</b>.</p>';
  sheet.innerHTML = '<div class="install-card"><img class="install-icon" src="icon-180.png?v=2" alt="">' +
    '<h2 id="install-title">Add ' + name + ' to your Home Screen</h2>' +
    '<p class="install-lede">Its icon opens ' + name + ' straight away, full screen, free to play.</p>' + body +
    '<div class="install-actions"><button type="button" class="install-go" hidden>INSTALL</button><button type="button" class="install-close">NOT NOW</button></div></div>';
  document.body.appendChild(sheet);
  var go = sheet.querySelector('.install-go'), close = sheet.querySelector('.install-close');
  function offer() {
    if (!window.blipInstallPrompt) return;
    go.hidden = false;
    var wait = sheet.querySelector('.install-wait');
    if (wait) wait.hidden = true;
    go.focus();
  }
  function dismiss() {
    sheet.remove();
    try { canvas.focus({ preventScroll: true }); } catch (e) {}
  }
  go.addEventListener('click', function () {
    var prompt = window.blipInstallPrompt;
    window.blipInstallPrompt = null;
    if (!prompt) return dismiss();
    prompt.prompt();
    (prompt.userChoice || Promise.resolve()).then(dismiss, dismiss);
  });
  close.addEventListener('click', dismiss);
  sheet.addEventListener('keydown', function (e) { if (e.key === 'Escape') { e.stopPropagation(); dismiss(); } });
  document.addEventListener('blip-install-ready', offer);
  offer();
  if (go.hidden) close.focus();
}
window.blipShowInstallSheet = showInstallSheet;
document.addEventListener('click', function (e) {
  if (e.target.closest && e.target.closest('#cc-install')) showInstallSheet();
  if (e.target.closest && e.target.closest('#cc-guide')) showGuide();
});

// ---- The Guide, as a sheet over the game ---- The game keeps its place
// underneath; Done (a message from the guide) puts the sheet away.
function showGuide() {
  if (!GAME || document.getElementById('guide-sheet')) return;
  var sheet = document.createElement('div');
  sheet.id = 'guide-sheet';
  sheet.setAttribute('role', 'dialog');
  sheet.setAttribute('aria-modal', 'true');
  sheet.setAttribute('aria-label', 'Guide');
  var frame = document.createElement('iframe');
  frame.title = 'Guide';
  frame.src = '../guide.html?embed#/t/game-' + GAME.slug;
  sheet.appendChild(frame);
  document.body.appendChild(sheet);
  window.blipPause(true);
  frame.addEventListener('load', function () { try { frame.contentWindow.focus(); } catch (e) {} });
}
window.addEventListener('message', function (e) {
  if (e.origin !== location.origin || !e.data || e.data.type !== 'blip-guide-close') return;
  var sheet = document.getElementById('guide-sheet');
  if (!sheet) return;
  sheet.classList.add('closing');
  setTimeout(function () {
    sheet.remove();
    if (!document.documentElement.hasAttribute('data-card')) window.blipPause(false);
    try { canvas.focus({ preventScroll: true }); } catch (err) {}
  }, 260);
});
if (new URLSearchParams(location.search).has('install')) showInstallSheet();

// ---- App mode: the first time, how to play ---- The game's first frame
// comes up under its controls card, paused; any key, tap or Escape starts
// play. Once per game (blip-seen-<slug>).
function introduceControls() {
  var root = document.documentElement;
  if (!cardApi || !GAME || !root.hasAttribute('data-appstyle') || CABINET_SCREEN) return;
  var key = 'blip-seen-' + GAME.slug;
  try { if (localStorage.getItem(key)) return; localStorage.setItem(key, '1'); } catch (e) { return; }
  root.setAttribute('data-intro', '');
  new MutationObserver(function (list, obs) {
    if (!root.hasAttribute('data-card')) { root.removeAttribute('data-intro'); obs.disconnect(); }
  }).observe(root, { attributes: true, attributeFilter: ['data-card'] });
  cardApi.set(true);
  function onKey(e) {
    window.removeEventListener('keydown', onKey, true);
    if (!root.hasAttribute('data-card')) return;
    e.preventDefault(); e.stopImmediatePropagation();
    cardApi.set(false);
  }
  window.addEventListener('keydown', onKey, true);
}

// A hidden app is a paused one: no music from a tab in the background.
document.addEventListener('visibilitychange', function () {
  var root = document.documentElement;
  if (!root.hasAttribute('data-appstyle')) return;
  if (document.hidden) window.blipPause(true);
  else if (!root.hasAttribute('data-card') && !document.getElementById('guide-sheet')) window.blipPause(false);
});

// Mirror the overlay onto <body> (body.need-coin lights the coin button).
// When it closes, a coin just went in: give focus back to the canvas so the
// title screen answers the keyboard.
var overlayWasVisible = overlay.classList.contains('visible');
new MutationObserver(function () {
  var vis = overlay.classList.contains('visible');
  document.body.classList.toggle('need-coin', vis);
  if (vis) {
    if (window.BlipController) window.BlipController.releaseAll();
    if (window.blipReleaseTouch) window.blipReleaseTouch();
  }
  if (overlayWasVisible && !vis && canvas) {
    try { canvas.focus(); } catch (e) {}
  }
  overlayWasVisible = vis;
}).observe(overlay, { attributes: true, attributeFilter: ['class'] });

// No credits on arrival (fresh session, deep link): the coin wall is up, and
// it blocks injectKey() until a coin goes in.
if (!FREE_PLAY && getCoins() <= 0) overlay.classList.add('visible');

// ---- Canvas sizing ----
// The kiosk bar is position:fixed;bottom:0. We leave PAD px on each side
// plus full clearance for the bar so it never overlaps the canvas.

// Sideways the deck moves into the picture's side gutters (one layout rule
// for every page, in kiosk.js).
function landscape() { return blipLandscape(); }
function applyLayout() { blipApplyLayout(); }
applyLayout();

/** Reserve the panel through the highest visible control, including any part
 * that rises above its fixed box. */
function deckCover(bar) {
  var top = bar.getBoundingClientRect().top;
  var parts = bar.querySelectorAll(
    '.stick-ball, .stick-boot, .fire-buttons, .snes-dpad, .snes-face, ' +
    '#paddle-dial, #paddle-dial-p2, #touch-strip .ts-half');
  for (var i = 0; i < parts.length; i++) {
    var r = parts[i].getBoundingClientRect();
    if (r.width > 0 && r.height > 0 && r.top < top) top = r.top;
  }
  return Math.ceil(window.innerHeight - top);
}

/** Fullscreen on a PC: no deck and no cabinet frame, so the picture runs to
 * the edges under the top bar (see #fullscreen-btn in shell.css). */
function bareScreen() {
  var root = document.documentElement;
  return root.hasAttribute('data-bare') || (root.hasAttribute('data-fullscreen') && !blipHasTouch());
}

// App mode needs no deck where nothing plays on it: a PC (keys or a pad),
// or a tablet with a gamepad connected (kiosk.js sets data-pad), until a
// finger touches the screen again.
(function () {
  var root = document.documentElement;
  if (!root.hasAttribute('data-appstyle')) return;
  var touched = false;
  function update() {
    var bare = !blipHasTouch() || (root.hasAttribute('data-pad') && !touched);
    if (bare === root.hasAttribute('data-bare')) return;
    root.toggleAttribute('data-bare', bare);
    if (typeof fillCanvas === 'function') fillCanvas();
  }
  new MutationObserver(function () { if (!root.hasAttribute('data-pad')) touched = false; update(); })
    .observe(root, { attributes: true, attributeFilter: ['data-pad'] });
  document.addEventListener('touchstart', function () { if (!touched && root.hasAttribute('data-bare')) { touched = true; update(); } }, { passive: true, capture: true });
}());

var lastFit = '';
function fillCanvas() {
  if(CABINET_SCREEN) {
    canvas.style.setProperty('width',innerWidth+'px','important');
    canvas.style.setProperty('height',innerHeight+'px','important');
    canvas.style.setProperty('top','0','important');canvas.style.setProperty('left','0','important');
    canvas.style.setProperty('transform','none','important');syncBuffer();return;
  }
  applyLayout();
  var tb = document.getElementById('topbar');
  var amstrad = blipAmstradMode();
  var bare = bareScreen() || amstrad;
  // The frame is FRAME_PAD wide; without it the picture needs no margin.
  var PAD = bare ? 0 : FRAME_PAD;
  var screenInset = bare ? 0 : FRAME_PAD + 1;
  // The picture starts at the cabinet fascia and uses the same 14px inset as
  // the display below the arcade on the landing page.
  var GAP = 0;
  // Sideways the deck sits in the gutters. Upright, reserve its full panel.
  TOPBAR_H = bare || landscape() ? 0 : (tb ? Math.max(deckCover(tb), 56) : 56);
  var mb = document.getElementById('marquee-bar');
  if (mb && !amstrad) {
    var receiver = bare ? null : mb.querySelector('.jukebox-receiver');
    var receiverBottom = receiver
      ? Math.ceil(parseFloat(getComputedStyle(receiver).top) + receiver.offsetHeight)
      : 0;
    MARQUEE_H = Math.max(mb.offsetHeight, receiverBottom, 28);
  } else MARQUEE_H = 0;
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
  if (!bare && !landscape()) {
    var cabinetWidth = parseFloat(getComputedStyle(document.documentElement)
      .getPropertyValue('--cabinet-width')) || innerWidth - 32;
    var screenWidth = Math.min(innerWidth - 32, cabinetWidth) - screenInset * 2;
    w = Math.max(0, Math.min(w, screenWidth));
    gl = gr = (innerWidth - w) / 2;
  }
  var h = window.innerHeight - TOPBAR_H - MARQUEE_H - GAP - screenInset * 2;
  var screenTop = MARQUEE_H + GAP + screenInset;
  var bezel = document.getElementById('arcade-bezel');
  if (!bare && !landscape() && bezel) {
    // Match the landing page's panel boundary, including the 1px bezel border.
    bezel.style.setProperty('--cabinet-panel-depth',
      TOPBAR_H + 'px');
    var frame = bezel.getBoundingClientRect();
    gl = frame.left + screenInset;
    screenTop = frame.top + screenInset;
    w = frame.width - screenInset * 2;
    h = frame.height - screenInset * 2;
  }
  document.querySelectorAll('.cabinet-glass, #need-coin-overlay').forEach(function(surface) {
    surface.style.left = gl + 'px'; surface.style.top = screenTop + 'px';
    surface.style.width = w + 'px'; surface.style.height = h + 'px';
  });
  canvas.style.setProperty('width',  w + 'px', 'important');
  canvas.style.setProperty('height', h + 'px', 'important');
  canvas.style.setProperty('top',    screenTop + 'px', 'important');
  canvas.style.setProperty('left',   gl + 'px', 'important');
  canvas.style.setProperty('transform', 'none', 'important');
  // The controls position themselves off --topbar-h, the bar's real height.
  document.documentElement.style.setProperty('--topbar-h', TOPBAR_H + 'px');
  syncBuffer();
}

/** The macroquad runtime sizes the drawing buffer on window resize only
 * (its window.onresize); a layout change without one (fullscreen, another
 * controller) would leave the game drawn for the old size, stretched. */
function syncBuffer() {
  var dpr = window.high_dpi ? window.devicePixelRatio || 1 : 1;   // the runtime's high-DPI buffer (app style)
  if (canvas.width === Math.floor(canvas.clientWidth * dpr) && canvas.height === Math.floor(canvas.clientHeight * dpr)) return;
  if (typeof window.onresize !== 'function') return;
  // A game's own call (blipSetMode) lands here mid-frame; miniquad's resize
  // handler run now would re-enter it and panic, so it runs once the frame returns.
  queueMicrotask(function () {
    try { window.onresize(); } catch (e) { /* the game has not loaded yet */ }
  });
}

/** Re-fit only when inputs change; deck observation and `--topbar-h` writes otherwise form a resize loop. */
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

// A game that traps (a Rust panic aborts) leaves a frozen picture, and a
// kiosk has nobody to reload it: say so, then go back to the cabinet.
var gameStopped = false;
function onGameStopped(err) {
  if (gameStopped || typeof WebAssembly === 'undefined' || !(err instanceof WebAssembly.RuntimeError)) return;
  gameStopped = true;
  if (loader) loader.style.display = 'flex';
  if (statusEl) statusEl.textContent = 'GAME STOPPED';
  if (barInner && barInner.parentNode) barInner.parentNode.style.display = 'none';
  setTimeout(leaveGame, 2500);
}
window.addEventListener('error', function (e) { onGameStopped(e.error); });
window.addEventListener('unhandledrejection', function (e) { onGameStopped(e.reason); });

function hideLoader() {
  if (gameStopped) return;
  if (loader && loader.style.display !== 'none') {
    // An app's launch screen fades into the first frame.
    if (document.documentElement.hasAttribute('data-appstyle') && !matchMedia('(prefers-reduced-motion: reduce)').matches) {
      var fading = loader.cloneNode(true);
      fading.removeAttribute('id');
      fading.className = 'launch-fade';
      document.body.appendChild(fading);
      fading.addEventListener('animationend', function () { fading.remove(); });
    }
    loader.style.display = 'none';
    fillCanvas();
    introduceControls();
    if(!CABINET_SCREEN&&!document.querySelector('.blip-hs-modal'))canvas.focus();
  }
}

// The game hears keys only while the canvas has focus. A name prompt that
// closes leaves it on <body>, a click on a bar button on the button, and the
// keyboard is dead: on a kiosk with no mouse, for good. Hand it back.
function refocusGame() {
  if (document.querySelector('.blip-hs-modal')) return;
  var a = document.activeElement;
  if (a === canvas || (a && a.closest && a.closest('input, textarea'))) return;
  try { canvas.focus({ preventScroll: true }); } catch (e) {}
}
new MutationObserver(refocusGame).observe(document.body, { childList: true });
document.addEventListener('click', function (e) {
  if (e.detail > 0) setTimeout(refocusGame, 0);   // a pointer's click, not Enter on a button
}, true);
window.addEventListener('keydown', function () {
  if (document.activeElement === document.body) refocusGame();
}, true);

// Canvas dimensions exist before WASM starts; only a real draw proves readiness.
window.blipCanvasReady=function(){
  fillCanvas();
  requestAnimationFrame(function(){requestAnimationFrame(function(){
    document.documentElement.setAttribute('data-game-ready','');hideLoader();
    if(CABINET_SCREEN)parent.postMessage({type:'blip-game-ready'},location.origin);
    if(!CABINET_SCREEN && !blipAppMode()) {
      var slug=blipGameFromPath(location.pathname)?.slug;
      var cabinetStart=false;
      try {
        cabinetStart=sessionStorage.getItem('blip-cabinet-start')===slug;
        sessionStorage.removeItem('blip-cabinet-start');
      } catch(e) {}
      if(cabinetStart && getCoins()>0) {
        function pressStart() {
          canvas.dispatchEvent(new KeyboardEvent('keydown',{bubbles:true,cancelable:true,key:' ',code:'Space'}));
          setTimeout(function(){canvas.dispatchEvent(new KeyboardEvent('keyup',{bubbles:true,cancelable:true,key:' ',code:'Space'}));},80);
        }
        function beginCabinetPlay() {
          if(document.hidden) {
            document.addEventListener('visibilitychange',beginCabinetPlay,{once:true});
            return;
          }
          if(getCoins()<=0 || document.querySelector('.blip-hs-modal'))return;
          canvas.focus({preventScroll:true});
          pressStart();
          if(slug==='brawler')setTimeout(pressStart,320);
        }
        setTimeout(beginCabinetPlay,180);
      }
    }
  });});
};

// The GPU dropped the picture. An alert would hang a kiosk for good: load
// the page again, or the cabinet if that was tried in the last half minute.
canvas.addEventListener('webglcontextlost', function (e) {
  e.preventDefault();
  if (gameStopped) return;
  gameStopped = true;
  var again = true;
  try {
    again = Date.now() - (+sessionStorage.getItem('blip-gl-lost') || 0) > 30000;
    sessionStorage.setItem('blip-gl-lost', String(Date.now()));
  } catch (err) {}
  if (loader) loader.style.display = 'flex';
  if (statusEl) statusEl.textContent = 'ONE MOMENT';
  setTimeout(function () {
    if (again) window.location.reload();
    else leaveGame();
  }, 1200);
}, false);

// Pad, stick, keyboard and gamepad presses become canvas key events via BlipController; Rally uses bindDial().

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
// Fire, Enter and 5 insert a coin at the wall; controls never add credit during play.
window.addEventListener('keydown', function (e) {
  if(document.querySelector('.blip-hs-modal'))return;
  var wall = overlay.classList.contains('visible');
  var coinKey = e.key === '5' || (wall && (e.key === ' ' || e.key === 'Enter'));
  var muteKey = e.key === 'm' || e.key === 'M';
  // isTrusted: the deck and a gamepad send synthetic keys, gated elsewhere.
  if (coinKey && !e.repeat && e.isTrusted && getCoins() < MAX_COINS) coinIn();
  if (wall && !coinKey && !muteKey) e.stopImmediatePropagation();
}, true);

(function () {
  if(CABINET_SCREEN) return;
  var game = GAME;
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
    if(coinGated())return;
    // Held for a few frames: Brawler samples the key, and a press that is
    // down and up inside one frame never happened.
    dispatch(primary, 'keydown');
    setTimeout(function () { dispatch(primary, 'keyup'); }, 60);
  }
  // Backspace does it from the keyboard: a kiosk has no mouse for the logo.
  window.addEventListener('keydown', function (e) {
    if (blipAmstradMode() && e.key === 'Escape') {
      e.preventDefault(); e.stopImmediatePropagation(); leaveGame(); return;
    }
    if (e.key !== 'Backspace' || e.repeat) return;
    if (e.target && e.target.closest && e.target.closest('input, textarea')) return;
    e.preventDefault();
    leaveGame();
  }, true);

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
    onSelect: leaveGame
  });
  BlipController.bindKeyboard();
  BlipController.bindGamepad(pollGamepad);

  window.addEventListener('message', function (event) {
    if (event.origin !== location.origin || event.source !== parent || !blipAmstradMode() || !event.data || event.data.type !== 'blip-amstrad-key') return;
    var type = event.data.down ? 'keydown' : 'keyup';
    canvas.dispatchEvent(new KeyboardEvent(type, {
      bubbles: true, cancelable: true, key: event.data.key, code: event.data.code
    }));
  });

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
    var LABEL = { stick: 'joystick', pad: 'd-pad' };
    if (touchSpec && blipHasTouch()) modes.push('touch');
    if (modes.length < 2) modes = [];
    modes.forEach(function (m) {
      var b = document.createElement('button');
      b.type = 'button';
      b.setAttribute('role', 'radio');
      b.setAttribute('data-mode', m);
      b.title = { stick:'Use the joystick', pad:'Use the directional pad', touch:'Use touch gestures', dial:'Use the paddle dial' }[m];
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
  window.blipReleaseTouch = touchPlay.release;
  window.onBlipControlsChange = function () {
    BlipController.releaseAll();
    touchPlay.release();
    stations.forEach(function (st) { st.held = {}; leanStick(st); });
    syncPicker();
    // The controllers are different heights, so the picture has to be
    // re-fitted or it keeps the other one's reservation.
    if (typeof fillCanvas === 'function') fillCanvas();
  };

  // Touch drag/paddles send positions; swipe sends keys. Serpent accepts picture swipes in every mode.
  // Touch screens only; a touchscreen laptop's mouse may hover-play the trackpad.
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
      var left = cr.left + (cr.width - w) / 2;
      if (!blipLandscape()) {
        var coin = bar.querySelector('.deck-coin-slot');
        var innerLeft = Math.max(cr.left, br.left + 12);
        var innerRight = coin ? coin.getBoundingClientRect().left - 10 : br.right - 12;
        w = Math.min(w, Math.max(0, innerRight - innerLeft));
        left = Math.max(innerLeft, Math.min(left, innerRight - w));
      }
      glass.style.left = Math.round(left - br.left) + 'px';
      glass.style.width = w + 'px';
    }
    window.addEventListener('blip-picture', fitStrip);
    window.addEventListener('resize', fitStrip);
    new MutationObserver(fitStrip).observe(root, {
      attributes: true, attributeFilter: ['data-open', 'data-versus', 'data-layout']
    });
    if (typeof ResizeObserver === 'function') new ResizeObserver(fitStrip).observe(canvas);
    fitStrip();
    var thumb = strip.querySelector('.ts-thumb');
    var arrow = strip.querySelector('.ts-arrow');
    var p2tag = strip.querySelector('.ts-half[data-slot="1"] .ts-tag');

    // 0 = nothing, 1 = a PC's pointer hovering, 2 = pressed.
    window.blipTouchDown = function (slot) {
      if (coinGated()) return 0;
      var t = slots[slot];
      return t ? (t.pressed ? 2 : 1) : 0;
    };
    window.blipTouchPos = function (slot, axis) {
      if (coinGated()) return 0;
      var t = slots[slot];
      return t ? (axis ? t.y : t.x) : 0;
    };
    window.blipHaptic = function () {
      if (blipControls() === 'touch' && navigator.vibrate) {
        try { navigator.vibrate(12); } catch (e) {}
      }
    };

    // Where a finger lands is the whole picture: across the strip's width if it
    // is on the strip, one to one on the picture. Rally's bats move by how far
    // a finger drags, so they keep the picture's scale.
    function surfaceRect(e) {
      var onStrip = e.target && e.target.closest && e.target.closest('.ts-glass');
      return (kind === 'drag' && onStrip ? glass : canvas).getBoundingClientRect();
    }
    function frac(e) {
      return { x: (e.clientX - rect.left) / rect.width, y: (e.clientY - rect.top) / rect.height };
    }
    // Rally: each player drags on their own half, except against the CPU,
    // where the whole screen is player one's.
    function slotFor(e) {
      if (kind !== 'paddles' || root.hasAttribute('data-cpu')) return 0;
      var onStrip = e.target && e.target.closest && e.target.closest('.ts-glass');
      var g = glass.getBoundingClientRect();
      var middle = onStrip ? g.left + g.width / 2 : window.innerWidth / 2;
      return e.clientX < middle ? 0 : 1;
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

    // Touch-down blows a bubble (release missed drags); slides run and upward flicks jump.
    // Player-two gestures use the right half; BlipController injects the keys.
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
      rect = surfaceRect(e);
      if (kind === 'swipe') {
        if (swipe) return;               // the first finger keeps the swipe
        swipe = { id: e.pointerId, x: e.clientX, y: e.clientY, moved: false };
      } else if (e.pointerType === 'mouse') {
        placeMouse(e, true);
        if (kind === 'drag') setFire(true);
      } else {
        var slot = slotFor(e);
        // A second finger would take the control from the first, and lifting it would drop both.
        if (slots[slot] && !slots[slot].mouse) return;
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
        if (!held) rect = surfaceRect(e);
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

    if (blipHasTouch()) {
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

  // The open title screen shows both stations so player two can join in either
  // orientation; one-player matches hide the second station.
  var open = false;
  function applyPlayers() {
    var both = versus || open;
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
  // Capture starts anywhere on the screen: the ball can project over the
  // game canvas even though its untransformed base stays in the deck.
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
    // Pick the stick the pointer actually landed on, including its raised
    // ball. Otherwise use the station half, excluding its action caps.
    function isStickTouch(e) {
      if (base.getBoundingClientRect().width === 0) return false;
      var ball = base.querySelector('.stick-ball');
      var ballRect = ball && ball.getBoundingClientRect();
      var onStick = e.target === base || (e.target.closest && e.target.closest('.stick-base') === base) ||
        (ballRect && e.clientX >= ballRect.left && e.clientX <= ballRect.right &&
          e.clientY >= ballRect.top && e.clientY <= ballRect.bottom);
      if (onStick) return true;
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
    window.addEventListener('pointerdown', function (e) {
      if (blipControls() !== 'stick') return;   // pad mode — deck is the SNES pad
      if (activeId !== null) release();
      if (!isStickTouch(e)) return;
      e.preventDefault();
      activeId = e.pointerId;
      pivot = { x: e.clientX, y: e.clientY };
      window.addEventListener('pointermove', onMove, true);
      window.addEventListener('pointerup', onEnd, true);
      window.addEventListener('pointercancel', onEnd, true);
    }, true);
    window.addEventListener('blur', release);
    document.addEventListener('visibilitychange', function () {
      if (document.hidden) release();
    });
  });
}());


})();
