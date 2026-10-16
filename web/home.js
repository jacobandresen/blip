/* The home screen: one icon per game, a dock, and the sheets behind a long
 * press and the Settings icon. Needs only blip_core.js. */
(function () {
'use strict';

var root = document.documentElement;
var grid = document.getElementById('app-grid');
var dockRow = document.getElementById('dock-row');
var zoom = document.getElementById('launch-zoom');
var backdrop = document.getElementById('sheet-backdrop');
var actionSheet = document.getElementById('action-sheet');
var settingsSheet = document.getElementById('settings-sheet');
var REDUCED = matchMedia('(prefers-reduced-motion: reduce)').matches;
var RECENT_KEY = 'blip-recent';
var DOCK_GAMES = 2;   // beside the arcade and Settings: four in a phone's dock

// Being here is the choice: a game played from here comes back here.
blipSetFrontend('home');

function titleCase(s) { return s.toLowerCase().replace(/\b[a-z]/g, function (c) { return c.toUpperCase(); }); }
function accentOf(slug) { return 'rgb(' + BLIP_GAMES[slug].accent + ')'; }

function recent() {
  try {
    var list = JSON.parse(localStorage.getItem(RECENT_KEY) || '[]');
    return Array.isArray(list) ? list.filter(function (s) { return BLIP_GAMES[s]; }) : [];
  } catch (e) { return []; }
}
function remember(slug) {
  var list = [slug].concat(recent().filter(function (s) { return s !== slug; })).slice(0, 6);
  try { localStorage.setItem(RECENT_KEY, JSON.stringify(list)); } catch (e) {}
}

/* ---- Icons ---- */
function gameApp(slug, inDock) {
  var game = BLIP_GAMES[slug], name = titleCase(game.name);
  var li = document.createElement('li');
  li.className = 'app';
  var button = document.createElement('button');
  button.type = 'button';
  button.className = 'app-button';
  button.dataset.slug = slug;
  button.setAttribute('aria-label', name + (game.players === 2 ? ', 1 or 2 players' : ''));
  button.innerHTML = '<span class="app-icon"><img src="' + slug + '/icon-192.png?v=1" alt="" draggable="false"></span>' +
    '<span class="app-label">' + name + '</span>';
  if (game.players === 2 && !inDock) {
    var badge = document.createElement('span');
    badge.className = 'app-badge';
    badge.textContent = '2P';
    badge.setAttribute('aria-hidden', 'true');
    li.appendChild(badge);
  }
  li.appendChild(button);
  wirePress(button, function () { launch(slug, button); }, function () { openActions(slug, button); });
  return li;
}

var ARCADE_SVG = '<svg viewBox="0 0 24 24" aria-hidden="true"><path fill="#e8c47e" d="M6 2h12l1 4v16H5V6z"/><path fill="#1b130c" d="M7 7h10v7H7z"/><path fill="#7fe08e" d="M8 8h8v5H8z" opacity=".75"/><path fill="#2a1a10" d="M6 16h12v2H6z"/><circle cx="9" cy="17" r="1" fill="#ed684d"/><circle cx="14" cy="17" r="1" fill="#ffd16a"/><circle cx="16.5" cy="17" r="1" fill="#ffd16a"/><path fill="#c79a52" d="M6.5 3h11l.5 2H6z"/></svg>';
var GEAR_SVG = '<svg viewBox="0 0 24 24" aria-hidden="true"><path fill="#f2f2f7" d="M13.6 2l.5 2.4c.6.2 1.2.5 1.7.9l2.3-.8 1.6 2.8-1.8 1.6c.1.6.1 1.3 0 1.9l1.8 1.6-1.6 2.8-2.3-.8c-.5.4-1.1.7-1.7.9l-.5 2.4h-3.2l-.5-2.4c-.6-.2-1.2-.5-1.7-.9l-2.3.8-1.6-2.8 1.8-1.6a6 6 0 010-1.9L3.3 7.3l1.6-2.8 2.3.8c.5-.4 1.1-.7 1.7-.9L9.4 2zM12 8.2a3.3 3.3 0 100 6.6 3.3 3.3 0 000-6.6z" transform="translate(0 1)"/></svg>';

function systemApp(kind, label, svg, act) {
  var li = document.createElement('li');
  li.className = 'app';
  var button = document.createElement('button');
  button.type = 'button';
  button.className = 'app-button';
  button.id = 'dock-' + kind;
  button.setAttribute('aria-label', label);
  button.innerHTML = '<span class="app-icon icon-' + kind + '">' + svg + '</span><span class="app-label">' + label + '</span>';
  wirePress(button, act, null);
  li.appendChild(button);
  return li;
}

function build() {
  Object.keys(BLIP_GAMES).forEach(function (slug) { grid.appendChild(gameApp(slug, false)); });
  dockRow.appendChild(systemApp('arcade', 'Arcade', ARCADE_SVG, toCabinet));
  recent().slice(0, DOCK_GAMES).forEach(function (slug) { dockRow.appendChild(gameApp(slug, true)); });
  dockRow.appendChild(systemApp('settings', 'Settings', GEAR_SVG, openSettings));
}

/* ---- Press, long press ---- A tap opens; a finger held half a second, or
 * a right click, opens the action sheet instead. */
function wirePress(button, onTap, onHold) {
  var timer = null, held = false, start = null;
  function cancel() { clearTimeout(timer); timer = null; button.classList.remove('pressed'); }
  button.addEventListener('pointerdown', function (e) {
    if (e.button !== 0) return;
    held = false; start = { x: e.clientX, y: e.clientY };
    button.classList.add('pressed');
    if (onHold) timer = setTimeout(function () { held = true; cancel(); onHold(); }, 500);
  });
  button.addEventListener('pointermove', function (e) {
    if (start && Math.hypot(e.clientX - start.x, e.clientY - start.y) > 10) cancel();
  });
  ['pointerup', 'pointercancel', 'pointerleave'].forEach(function (type) { button.addEventListener(type, cancel); });
  button.addEventListener('click', function (e) {
    if (held) { held = false; e.preventDefault(); return; }
    onTap();
  });
  if (onHold) button.addEventListener('contextmenu', function (e) { e.preventDefault(); cancel(); held = false; onHold(); });
}

/* ---- Sound ---- One soft tick when a game opens, through blipOut so the
 * stored mute holds. */
var audio = null;
function tick() {
  if (blipMuted()) return;
  try {
    audio = audio || new (window.AudioContext || window.webkitAudioContext)();
    var t = audio.currentTime, osc = audio.createOscillator(), gain = audio.createGain();
    osc.type = 'sine';
    osc.frequency.setValueAtTime(660, t);
    osc.frequency.exponentialRampToValueAtTime(990, t + 0.06);
    gain.gain.setValueAtTime(0.12, t);
    gain.gain.exponentialRampToValueAtTime(0.001, t + 0.12);
    osc.connect(gain); gain.connect(blipOut(audio));
    osc.start(t); osc.stop(t + 0.13);
  } catch (e) {}
}

/* ---- Opening a game ---- The icon grows to fill the screen, the way an
 * app opens, then the game's page loads under it. */
var leaving = false;
function launch(slug, from, url) {
  if (leaving) return;
  leaving = true;
  remember(slug);
  try { sessionStorage.setItem('blip-freeplay', '1'); } catch (e) {}
  var target = url || (slug + '/');
  tick();
  if (REDUCED || !from || !zoom.animate) { location.href = target; return; }
  var icon = from.querySelector('.app-icon').getBoundingClientRect();
  zoom.hidden = false;
  zoom.style.backgroundColor = accentOf(slug);
  zoom.style.backgroundImage = 'url(' + slug + '/icon-512.png?v=1)';
  zoom.style.width = innerWidth + 'px';
  zoom.style.height = innerHeight + 'px';
  var sx = icon.width / innerWidth, sy = icon.height / innerHeight;
  var anim = zoom.animate([
    { transform: 'translate(' + icon.left + 'px,' + icon.top + 'px) scale(' + sx + ',' + sy + ')', borderRadius: (icon.width * 0.225 / sx) + 'px / ' + (icon.height * 0.225 / sy) + 'px' },
    { transform: 'translate(0,0) scale(1,1)', borderRadius: '0px' }
  ], { duration: 320, easing: 'cubic-bezier(.2,.8,.2,1)', fill: 'forwards' });
  anim.onfinish = function () { zoom.style.backgroundImage = 'none'; location.href = target; };
}

/* Back from a game: the screen shrinks into its icon. */
function arrive() {
  var back = null;
  try {
    back = JSON.parse(sessionStorage.getItem('blip-home-return') || 'null');
    sessionStorage.removeItem('blip-home-return');
  } catch (e) {}
  if (!back || !BLIP_GAMES[back.slug] || Date.now() - back.at > 15000 || REDUCED || !zoom.animate) return;
  var button = grid.querySelector('[data-slug="' + back.slug + '"]');
  if (!button) return;
  var icon = button.querySelector('.app-icon').getBoundingClientRect();
  var sx = icon.width / innerWidth, sy = icon.height / innerHeight;
  zoom.hidden = false;
  zoom.style.backgroundColor = accentOf(back.slug);
  zoom.style.backgroundImage = 'url(' + back.slug + '/icon-512.png?v=1)';
  zoom.style.width = innerWidth + 'px';
  zoom.style.height = innerHeight + 'px';
  var anim = zoom.animate([
    { transform: 'translate(0,0) scale(1,1)', borderRadius: '0px', opacity: 1 },
    { transform: 'translate(' + icon.left + 'px,' + icon.top + 'px) scale(' + sx + ',' + sy + ')', borderRadius: (icon.width * 0.225 / sx) + 'px / ' + (icon.height * 0.225 / sy) + 'px', opacity: 0.6 }
  ], { duration: 300, easing: 'cubic-bezier(.3,.7,.2,1)' });
  anim.onfinish = function () { zoom.hidden = true; button.focus({ preventScroll: true }); };
}

// Back from the game by the browser's own back button: the page comes out
// of the cache with the zoom still covering it.
window.addEventListener('pageshow', function (e) {
  if (e.persisted) { leaving = false; zoom.hidden = true; zoom.getAnimations().forEach(function (a) { a.cancel(); }); }
});

function toCabinet() {
  if (leaving) return;
  leaving = true;
  blipSetFrontend('cabinet');
  location.href = 'index.html?cabinet';
}

/* ---- Sheets ---- */
var openSheet = null, returnFocus = null;
function showSheet(sheet, from) {
  openSheet = sheet; returnFocus = from || document.activeElement;
  backdrop.hidden = false; sheet.hidden = false;
  var first = sheet.querySelector('button, input');
  if (first) first.focus({ preventScroll: true });
}
function closeSheet() {
  if (!openSheet) return;
  openSheet.hidden = true; backdrop.hidden = true; openSheet = null;
  if (returnFocus && returnFocus.focus) returnFocus.focus({ preventScroll: true });
}
backdrop.addEventListener('click', closeSheet);

var actionSlug = null, actionFrom = null;
function openActions(slug, from) {
  actionSlug = slug; actionFrom = from;
  document.getElementById('action-title').textContent = titleCase(BLIP_GAMES[slug].name);
  document.getElementById('action-icon').src = slug + '/icon-192.png?v=1';
  showSheet(actionSheet, from);
}
actionSheet.addEventListener('click', function (e) {
  var b = e.target.closest('button[data-action]');
  if (!b) return;
  var action = b.dataset.action, slug = actionSlug, from = actionFrom;
  closeSheet();
  // The game's page is what a browser saves, so it shows how from there.
  if (action === 'play') launch(slug, from);
  else if (action === 'install') launch(slug, from, slug + '/?install');
});

var setSound = document.getElementById('set-sound');
var setFullscreen = document.getElementById('set-fullscreen');
var setCabinet = document.getElementById('set-cabinet');
function openSettings() {
  setSound.checked = !blipMuted();
  setFullscreen.checked = root.hasAttribute('data-fullscreen');
  setCabinet.checked = false;
  showSheet(settingsSheet, document.getElementById('dock-settings'));
}
setSound.addEventListener('change', function () { blipSetMuted(!setSound.checked); });
setFullscreen.addEventListener('change', function () {
  blipSetFullscreen(setFullscreen.checked);
  var el = document.documentElement;
  try {
    if (setFullscreen.checked) { var p = (el.requestFullscreen || el.webkitRequestFullscreen).call(el); if (p && p.catch) p.catch(function () {}); }
    else if (document.fullscreenElement && document.exitFullscreen) document.exitFullscreen();
  } catch (e) {}
});
setCabinet.addEventListener('change', function () { if (setCabinet.checked) toCabinet(); });
document.getElementById('settings-done').addEventListener('click', closeSheet);

/* ---- Keys and pads ---- Arrows move between icons, Enter or the pad's
 * first button opens one, Escape (or the second) closes a sheet. */
function focusables() {
  var scope = openSheet || document;
  return Array.prototype.slice.call(scope.querySelectorAll(openSheet ? 'button, input' : '.app-button'));
}
function columns() {
  return getComputedStyle(grid).gridTemplateColumns.split(' ').length;
}
function move(dir) {
  var items = focusables();
  if (!items.length) return;
  var i = items.indexOf(document.activeElement);
  if (i < 0) { items[0].focus(); return; }
  if (openSheet) {
    var step = dir === 'up' || dir === 'left' ? -1 : 1;
    items[(i + step + items.length) % items.length].focus();
    return;
  }
  var gridItems = Array.prototype.slice.call(grid.querySelectorAll('.app-button'));
  var dockItems = Array.prototype.slice.call(dockRow.querySelectorAll('.app-button'));
  var gi = gridItems.indexOf(document.activeElement), di = dockItems.indexOf(document.activeElement), cols = columns();
  var next = null;
  if (gi >= 0) {
    if (dir === 'left') next = gridItems[gi - 1];
    else if (dir === 'right') next = gridItems[gi + 1];
    else if (dir === 'up') next = gridItems[gi - cols];
    else next = gridItems[gi + cols] || (Math.floor(gi / cols) === Math.floor((gridItems.length - 1) / cols) ? dockItems[Math.min(gi % cols, dockItems.length - 1)] : gridItems[gridItems.length - 1]);
  } else if (di >= 0) {
    if (dir === 'left') next = dockItems[di - 1];
    else if (dir === 'right') next = dockItems[di + 1];
    else if (dir === 'up') {
      var lastRow = Math.floor((gridItems.length - 1) / cols) * cols;
      next = gridItems[Math.min(lastRow + di, gridItems.length - 1)];
    }
  }
  if (next) next.focus();
}
var ARROWS = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
document.addEventListener('keydown', function (e) {
  if (ARROWS[e.key]) { e.preventDefault(); move(ARROWS[e.key]); }
  else if (e.key === 'Escape' && openSheet) { e.preventDefault(); closeSheet(); }
  else if ((e.key === 'Enter' || e.key === ' ') && !document.activeElement.matches('button, input')) {
    var first = focusables()[0];
    if (first) { e.preventDefault(); first.focus(); }
  }
});

(function pads() {
  if (!navigator.getGamepads) return;
  var was = {}, nextRepeat = 0;
  function poll(now) {
    var pads = navigator.getGamepads(), pad = null;
    for (var i = 0; i < pads.length; i++) if (pads[i]) { pad = pads[i]; break; }
    if (pad) {
      var b = function (n) { return !!(pad.buttons[n] && pad.buttons[n].pressed); };
      var ax = pad.axes[0] || 0, ay = pad.axes[1] || 0;
      var dir = b(12) || ay < -0.6 ? 'up' : b(13) || ay > 0.6 ? 'down' : b(14) || ax < -0.6 ? 'left' : b(15) || ax > 0.6 ? 'right' : null;
      if (dir && (dir !== was.dir || now > nextRepeat)) { move(dir); nextRepeat = now + (dir !== was.dir ? 380 : 140); }
      if (b(0) && !was.a && document.activeElement && document.activeElement.click) document.activeElement.click();
      if (b(1) && !was.b && openSheet) closeSheet();
      was = { dir: dir, a: b(0), b: b(1) };
    }
    requestAnimationFrame(poll);
  }
  window.addEventListener('gamepadconnected', function start() {
    window.removeEventListener('gamepadconnected', start);
    requestAnimationFrame(poll);
  });
}());

/* ---- Clock ---- */
var clock = document.getElementById('home-clock');
function tickClock() {
  var d = new Date();
  clock.textContent = d.getHours() + ':' + String(d.getMinutes()).padStart(2, '0');
  clock.dateTime = d.toISOString();
}
tickClock();
setInterval(tickClock, 15000);

build();
arrive();
}());
