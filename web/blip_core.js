/* What every BLIP page shares, cabinet or home screen: the games, coins,
 * the layout rule, sound on/off, fullscreen and the front-end choice. Load
 * it before kiosk.js; the home screen loads it alone. */
var BLIP_CORE_SRC = document.currentScript ? document.currentScript.src : location.href;
var MAX_COINS = 5;

// A held finger must never raise a context menu (Android does, and it cancels the touch).
document.addEventListener('contextmenu', function (e) {
  if (!(e.target.closest && e.target.closest('input, textarea, [contenteditable]'))) e.preventDefault();
});
var TWO_PLAYER_KEYS = {
  up: { key: 'w', code: 'KeyW' }, down: { key: 's', code: 'KeyS' },
  left: { key: 'a', code: 'KeyA' }, right: { key: 'd', code: 'KeyD' },
  p2up: { key: 'ArrowUp', code: 'ArrowUp' }, p2down: { key: 'ArrowDown', code: 'ArrowDown' },
  p2left: { key: 'ArrowLeft', code: 'ArrowLeft' }, p2right: { key: 'ArrowRight', code: 'ArrowRight' },
  p2button1: { key: 'j', code: 'KeyJ' }, p2button2: { key: 'k', code: 'KeyK' }
};

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

/** The controller in use: 'pad', 'stick' or 'touch' (the strip, only on a
 * game that has one). */
function blipControls() {
  var root = document.documentElement;
  if (root.hasAttribute('data-touch')) return 'touch';
  return root.getAttribute('data-controls') === 'stick' ? 'stick' : 'pad';
}

/** The pad or stick chosen, which a game without a touch strip keeps using
 * while touch is chosen. */
function blipPhysicalControls() {
  var m = 'pad';
  try { m = localStorage.getItem('blip-controls') || 'pad'; } catch (e) {}
  return m === 'stick' ? 'stick' : 'pad';
}

/** A touch screen: the only place TOUCH is offered. */
function blipHasTouch() {
  return ('ontouchstart' in window) || navigator.maxTouchPoints > 0;
}

/** On a touch screen: touch was chosen, or no choice has been made yet. */
function blipTouchChosen() {
  if (!blipHasTouch()) return false;
  var v = null;
  try { v = localStorage.getItem('blip-touch'); } catch (e) {}
  return v !== '0';
}

/** Apply the stored choice as <html data-controls> (+ data-touch). Touch
 * borrows the pad's bar so the picture is fitted the same. */
function blipApplyControls() {
  var root = document.documentElement;
  var game = blipGameFromPath(window.location.pathname);
  var touch = !!(game && game.touch) && blipTouchChosen();
  root.setAttribute('data-controls', touch ? 'pad' : blipPhysicalControls());
  if (touch) root.setAttribute('data-touch', game.touch.kind);
  else root.removeAttribute('data-touch');
}

function blipSetControls(mode) {
  try {
    if (mode === 'touch') localStorage.setItem('blip-touch', '1');
    else {
      localStorage.setItem('blip-touch', '0');
      if (mode === 'pad' || mode === 'stick') localStorage.setItem('blip-controls', mode);
    }
  } catch (e) {}
  blipApplyControls();
  if (typeof window.onBlipControlsChange === 'function') {
    try { window.onBlipControlsChange(blipControls()); } catch (e) {}
  }
}

/* ---- Per-game cabinet identity ---- Card colours on the landing page,
 * marquee and bezel glow on each game page. `accent` is an "r, g, b" triple
 * so CSS can build solid and translucent colours. */
// `buttons` map deck caps; `stick` accepts engage/release and maxR pixels plus hysteresis degrees.
// `touch.kind`: drag follows a finger, swipe steers by flick, paddles split bats, platform runs/blows/jumps.
// `controls` is the key legend a PC sees after the first coin: [label, keys, alternative keys]; `hint` labels the touch strip; `mouseHint` labels a touchscreen laptop's mouse controls.
var BLIP_GAMES = {
  serpent:            { name: 'SERPENT',  accent: '50, 200, 50', touchControls: [['STEER',['SWIPE']],['START',['TAP']]], controls: [['STEER',['↑','←','↓','→'],['W','A','S','D']],['START',['SPACE']]],
                         touch: { kind: 'swipe', hint: 'Swipe to steer', mouseHint: 'Click and drag to steer' } },
  bouncer:            { name: 'BOUNCER',  accent: '0, 200, 200', touchControls: [['MOVE',['DRAG']],['LAUNCH',['TAP']]], controls: [['MOVE',['←','→'],['A','D']],['LAUNCH',['SPACE']]],
                         touch: { kind: 'drag', hint: 'Tap to launch', mouseHint: 'Point to move &middot; Click to launch' } },
  galactic_defender:  { name: 'DEFENDER', accent: '200, 50, 200', touchControls: [['MOVE',['DRAG']],['FIRE',['HOLD']]], controls: [['MOVE',['←','→'],['A','D']],['FIRE',['SPACE']]],
                         touch: { kind: 'drag', hint: 'Hold to fire', mouseHint: 'Point to move &middot; Hold the button to fire' } },
  rally:              { name: 'RALLY',    accent: '220, 50, 50', touchControls: [['1 PLAYER',['TAP LEFT']],['2 PLAYERS',['TAP RIGHT']],['MOVE BAT',['DRAG']]], controls: [['1 PLAYER',['W','S'],['↑','↓']],['2 PLAYERS',['I','K']],['START AND SERVE',['SPACE']]],
                         touch: { kind: 'paddles', hint: 'Drag up or down', mouseHint: 'Point up or down &middot; Click to serve' } },
  meteors:            { name: 'METEORS',  accent: '180, 180, 180', touchControls: [['TURN AND THRUST',['D-PAD']],['FIRE',['RED BUTTON']],['HYPERSPACE',['BLUE BUTTON']]], controls: [['TURN',['←','→'],['A','D']],['THRUST',['↑'],['W']],['FIRE',['SPACE']],['HYPERSPACE',['Z']]],
                         buttons: [{ key: ' ', code: 'Space' }, { key: 'z', code: 'KeyZ' }] },
  // Two caps; holding toward hits high. `players: 2` enables the second station; `keys` maps P1 to WASD.
  brawler:            { name: 'BRAWLER', accent: '220, 60, 40', touchControls: [['MOVE',['D-PAD']],['PUNCH',['RED BUTTON']],['KICK',['BLUE BUTTON']]], controls: [['MOVE',['←','→'],['A','D']],['PUNCH',['SPACE'],['F']],['KICK',['Z'],['G']]],
                         players: 2,
                         buttons: [{ key: 'f', code: 'KeyF', label: 'PUNCH' },
                                   { key: 'g', code: 'KeyG', label: 'KICK' }],
                         keys: TWO_PLAYER_KEYS,
                         stick: { engage: 10, release: 6, maxR: 54, hyst: 10 } },
  // A tribute to Bubble Bobble. Two caps: bubble (fire) and jump; up
  // jumps too. Like Brawler, player one is WASD so the arrows are player
  // two's, who drops in with their own bubble or jump.
  bubbler:            { name: 'BUBBLER', accent: '120, 210, 255', touchControls: [['RUN',['SLIDE']],['BUBBLE',['TAP']],['JUMP',['FLICK UP']]], controls: [['RUN',['←','→'],['A','D']],['BUBBLE',['SPACE'],['F']],['JUMP',['↑'],['W']],['2ND PLAYER',['J','K']]],
                         players: 2,
                         touch: { kind: 'platform', hint: 'Touch to bubble &middot; slide to run &middot; swipe up to jump',
                                  mouseHint: 'Drag to run &middot; click to bubble &middot; drag up to jump' },
                         buttons: [{ key: 'f', code: 'KeyF', label: 'BUBBLE' },
                                   { key: 'g', code: 'KeyG', label: 'JUMP' }],
                         keys: TWO_PLAYER_KEYS },
  // A bullet-weaving shooter: fine nudges. A small dead zone, a long pivot
  // leash so re-centring neutralises, and a firm notch so a dodge holds.
  sky_raider:         { name: 'RAIDER', accent: '50, 100, 220', touchControls: [['STEER AND THROTTLE',['D-PAD']],['FIRE',['RED BUTTON']]], controls: [['STEER',['←','→'],['A','D']],['THROTTLE',['↑','↓'],['W','S']],['FIRE',['SPACE']]],
                         stick: { engage: 11, release: 6, maxR: 58, hyst: 12 } },
  // Viper-inspired trail arena: relative left/right turns, passable dim gaps.
  // Space only starts/restarts; J joins player two. Drag steers by horizontal position.
  adder:              { name: 'ADDER', accent: '224, 160, 52', touchControls: [['STEER',['DRAG']],['START',['TAP']]], controls: [['TURN',['A','D'],['←','→']],['START',['SPACE']],['2ND PLAYER JOINS',['J']]],
                         players: 2,
                         buttons: [{ key: ' ', code: 'Space', label: 'START' }],
                         keys: TWO_PLAYER_KEYS,
                         touch: { kind: 'drag', hint: 'Drag left/right to steer &middot; tap to start',
                                  mouseHint: 'Point left/right to steer &middot; click to start' } }
};

// Pick out the game slug from a shell-page URL, e.g. "/blip/serpent/index.html"
// or "/serpent/" both resolve to "serpent". Returns null off the game pages.
function blipGameFromPath(pathname) {
  var m = /\/([a-z_]+)\/(?:index\.html)?$/i.exec(pathname || '');
  var g = m && BLIP_GAMES[m[1]];
  return g ? { slug: m[1], name: g.name, accent: g.accent, controls: g.controls, touchControls: g.touchControls, buttons: g.buttons,
               keys: g.keys, players: g.players || 1, stick: g.stick,
               touch: g.touch || null } : null;
}

// Before first paint, so no controller flashes. data-has-touch shows the
// TOUCH badges on the landing page.
blipApplyControls();
if (blipHasTouch()) document.documentElement.setAttribute('data-has-touch', '');

function getCoins() {
  try {
    var n = parseInt(sessionStorage.getItem('blip-coins') || '0', 10);
    return isNaN(n) ? 0 : Math.min(Math.max(n, 0), MAX_COINS);
  } catch (e) { return 0; }
}

function saveCoins(n) {
  try { sessionStorage.setItem('blip-coins', n); } catch (e) {}
}


/* ---- Sound on / off ---- The choice is stored ('blip-mute'). The games'
 * sounds are Howler's, muted there; the cabinet's own (coins, clicks) all
 * leave through blipOut(), one gain that is shut while muted. */
var _blipOut = null;
function blipMuted() { if(document.documentElement.hasAttribute('data-cabinet-screen')) return true; try { return localStorage.getItem('blip-mute') === '1'; } catch (e) { return false; } }
function blipOut(ctx) {
  if (!_blipOut || _blipOut.context !== ctx) {
    _blipOut = ctx.createGain();
    _blipOut.connect(ctx.destination);
  }
  _blipOut.gain.value = blipMuted() ? 0 : 1;
  return _blipOut;
}
function blipSetMuted(on) {
  try { localStorage.setItem('blip-mute', on ? '1' : '0'); } catch (e) {}
  if (typeof Howler !== 'undefined') Howler.mute(on);
  if (_blipOut) _blipOut.gain.value = on ? 0 : 1;
  document.documentElement.toggleAttribute('data-muted', on);
}
if (blipMuted()) {
  document.documentElement.setAttribute('data-muted', '');
  if (typeof Howler !== 'undefined') Howler.mute(true);
}

/* Restore the saved fullscreen layout after navigation. */
(function () {
  var root = document.documentElement;
  var KEY = 'blip-fullscreen';
  var enter = root.requestFullscreen || root.webkitRequestFullscreen;
  function wanted() { try { return localStorage.getItem(KEY) === '1'; } catch (e) { return false; } }
  function inBrowserFullscreen() {
    return !!(document.fullscreenElement || document.webkitFullscreenElement);
  }
  function request() {
    if (!enter || inBrowserFullscreen()) return;
    try {
      var p = enter.call(root);
      // Refused (an embedded page, no gesture yet): the layout still holds.
      if (p && p.catch) p.catch(function () {});
    } catch (e) {}
  }
  // In fullscreen a mouse left lying loses its pointer after three seconds.
  var cursorT = null;
  function wakeCursor() {
    root.removeAttribute('data-cursor-idle');
    clearTimeout(cursorT);
    if (!root.hasAttribute('data-fullscreen')) return;
    cursorT = setTimeout(function () { root.setAttribute('data-cursor-idle', ''); }, 3000);
  }
  ['mousemove', 'mousedown'].forEach(function (ev) {
    window.addEventListener(ev, wakeCursor, { passive: true });
  });

  function apply(on) {
    root.toggleAttribute('data-fullscreen', on);
    wakeCursor();
    if (typeof window.onBlipFullscreenChange === 'function') {
      try { window.onBlipFullscreenChange(on); } catch (e) {}
    }
  }
  function set(on) {
    try { localStorage.setItem(KEY, on ? '1' : '0'); } catch (e) {}
    apply(on);
  }
  window.blipSetFullscreen = set;
  // Before first paint where this script is in <head>.
  if (wanted()) root.setAttribute('data-fullscreen', '');

  function build() {
    window.addEventListener('keydown', function (e) {
      if ((e.key !== 'm' && e.key !== 'M') || e.repeat || e.ctrlKey || e.metaKey || e.altKey) return;
      if (e.target && e.target.closest && e.target.closest('input, textarea')) return;
      if (root.hasAttribute('data-cabinet-screen') && window.parent !== window) {
        try {
          window.parent.blipSetMuted(!window.parent.blipMuted());
          return;
        } catch (e) {}
      }
      blipSetMuted(!blipMuted());
    }, true);
    apply(wanted());
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', build);
  else build();

  // Esc, or the browser's own way out, turns the mode off. Leaving the page
  // drops real fullscreen too, and that must not count: the choice has to
  // survive into the next page.
  var leaving = false;
  ['beforeunload', 'pagehide'].forEach(function (ev) {
    window.addEventListener(ev, function () { leaving = true; });
  });
  ['fullscreenchange', 'webkitfullscreenchange'].forEach(function (ev) {
    document.addEventListener(ev, function () {
      if (leaving) return;
      if (!inBrowserFullscreen() && root.hasAttribute('data-fullscreen')) set(false);
    });
  });
  // Back in real fullscreen on the first key or click of a page.
  ['keydown', 'mousedown', 'pointerup', 'touchend'].forEach(function (ev) {
    window.addEventListener(ev, function (e) {
      if (!wanted() || inBrowserFullscreen()) return;
      if (e.key === 'Escape') return;
      request();
    }, true);
  });
}());


/* ---- Front ends ---- The rolodex cabinet (index.html) or the home screen
 * (home.html), stored as 'blip-frontend'. A game page leaves for the one
 * chosen; a game saved as its own app (blipAppMode) never leaves itself. */
function blipFrontend() {
  try { return localStorage.getItem('blip-frontend') === 'home' ? 'home' : 'cabinet'; } catch (e) { return 'cabinet'; }
}
function blipSetFrontend(which) {
  try { localStorage.setItem('blip-frontend', which === 'home' ? 'home' : 'cabinet'); } catch (e) {}
  try {
    if (which === 'home') sessionStorage.setItem('blip-freeplay', '1');
    else sessionStorage.removeItem('blip-freeplay');
  } catch (e) {}
}

/** Running from a home-screen icon: no browser chrome around the page. */
function blipStandalone() {
  if (window.navigator.standalone === true) return true;
  try { return matchMedia('(display-mode: standalone), (display-mode: fullscreen)').matches; } catch (e) { return false; }
}

/** A game page opened from its own home-screen icon. The manifest starts it
 * at ./?app; iOS saves the page's own URL, so a launch with no referrer that
 * has not come through a front end in this session counts too. */
function blipAppMode() {
  var game = blipGameFromPath(location.pathname);
  if (!game || !blipStandalone()) return false;
  if (new URLSearchParams(location.search).has('app')) return true;
  var visited = null;
  try { visited = sessionStorage.getItem('blip-frontend-visit'); } catch (e) {}
  return !document.referrer && !visited;
}

/** No coins: a game launched from the home screen or its own icon. */
function blipFreePlay() {
  if (blipAppMode()) return true;
  try { return sessionStorage.getItem('blip-freeplay') === '1'; } catch (e) { return false; }
}

/** Where a game page goes back to, relative to the game's directory. */
function blipFrontendUrl() {
  if (blipAppMode()) return './?app';
  return blipFrontend() === 'home' ? '../home.html' : '../index.html';
}

(function () {
  var root = document.documentElement;
  if (blipGameFromPath(location.pathname)) {
    // Opened as an app (home screen, or its own icon): an app's look, not the cabinet's.
    if (blipFreePlay()) { root.setAttribute('data-freeplay', ''); root.setAttribute('data-appstyle', ''); }
    if (blipAppMode()) root.setAttribute('data-app', '');
  } else {
    try { sessionStorage.setItem('blip-frontend-visit', '1'); } catch (e) {}
  }
}());

// Chrome's install prompt, kept for the game page's own Install button.
window.addEventListener('beforeinstallprompt', function (e) {
  e.preventDefault();
  window.blipInstallPrompt = e;
  document.dispatchEvent(new Event('blip-install-ready'));
});

// The worker lives beside this script, whichever page (or game app) loads it.
if ('serviceWorker' in navigator && document.querySelector('link[rel=manifest]')) {
  navigator.serviceWorker.register(new URL('sw.js', BLIP_CORE_SRC).href);
}
