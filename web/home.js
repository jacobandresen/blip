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
// Two can play: a second station, or a key for the second player (Rally's dials share one).
function twoPlayer(slug) {
  var g = BLIP_GAMES[slug];
  return g.players === 2 || /2(ND)? PLAYER/.test(JSON.stringify(g.controls || []));
}

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
  button.setAttribute('aria-label', name + (twoPlayer(slug) ? ', 1 or 2 players' : ''));
  button.innerHTML = '<span class="app-icon"><img src="' + slug + '/icon-192.png?v=2" alt="" draggable="false"></span>' +
    '<span class="app-label">' + name + '</span>';
  if (twoPlayer(slug) && !inDock) {
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

/* ---- The High Scores widget ---- The board's best (cached from the
 * leaderboard) or this device's own, for the games played last. */
var TROPHY_SVG = '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 1h8v1.5h2.5V5a3 3 0 0 1-3 3h-.2A4 4 0 0 1 9 10.3V12h2v2H5v-2h2v-1.7A4 4 0 0 1 4.7 8h-.2a3 3 0 0 1-3-3V2.5H4zm0 3H3v1a1.5 1.5 0 0 0 1 1.4zm8 0v2.4A1.5 1.5 0 0 0 13 5V4z"/></svg>';
function bestOf(slug) {
  try {
    var top = JSON.parse(localStorage.getItem('blip-top-' + slug) || 'null');
    if (top && (top.score | 0) > 0) return { score: top.score | 0, who: top.handle || '' };
  } catch (e) {}
  var mine = 0;
  try { mine = parseInt(localStorage.getItem('blip-best-' + slug) || '0', 10) || 0; } catch (e) {}
  return mine > 0 ? { score: mine, who: 'You' } : null;
}
function scoresWidget() {
  var li = document.createElement('li');
  li.className = 'widget';
  li.setAttribute('aria-label', 'High scores');
  var order = recent().concat(Object.keys(BLIP_GAMES));
  var rows = order.filter(function (slug, i) { return order.indexOf(slug) === i && bestOf(slug); }).slice(0, 3);
  var html = '<p class="widget-head">' + TROPHY_SVG + 'High Scores</p>';
  if (!rows.length) {
    html += '<p class="widget-empty">Play a game to put a score up here.</p>';
  } else {
    html += '<ul class="widget-list">' + rows.map(function (slug) {
      var best = bestOf(slug);
      return '<li><img src="' + slug + '/icon-192.png?v=2" alt=""><span class="widget-game">' + titleCase(BLIP_GAMES[slug].name) +
        '<small>' + (best.who ? escapeHtml(best.who) : '&nbsp;') + '</small></span><span class="widget-score">' + best.score.toLocaleString() + '</span></li>';
    }).join('') + '</ul>';
  }
  li.innerHTML = html;
  return li;
}
function escapeHtml(s) { return String(s).replace(/[&<>"']/g, function (c) { return '&#' + c.charCodeAt(0) + ';'; }); }

/* The order the icons were arranged in (edit mode), with any game added
 * since at the end. */
var ORDER_KEY = 'blip-home-order';
function savedOrder() {
  var list = [];
  try { list = JSON.parse(localStorage.getItem(ORDER_KEY) || '[]') || []; } catch (e) {}
  list = Array.isArray(list) ? list.filter(function (s, i) { return BLIP_GAMES[s] && list.indexOf(s) === i; }) : [];
  return list.concat(Object.keys(BLIP_GAMES).filter(function (s) { return list.indexOf(s) < 0; }));
}
function saveOrder() {
  var list = Array.prototype.map.call(grid.querySelectorAll('.app-button[data-slug]'), function (b) { return b.dataset.slug; });
  try { localStorage.setItem(ORDER_KEY, JSON.stringify(list)); } catch (e) {}
}

function build() {
  grid.appendChild(scoresWidget());
  savedOrder().forEach(function (slug) { grid.appendChild(gameApp(slug, false)); });
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
    if (onHold && !editing) timer = setTimeout(function () { held = true; cancel(); onHold(); }, 500);
  });
  button.addEventListener('pointermove', function (e) {
    if (start && Math.hypot(e.clientX - start.x, e.clientY - start.y) > 10) cancel();
  });
  ['pointerup', 'pointercancel', 'pointerleave'].forEach(function (type) { button.addEventListener(type, cancel); });
  button.addEventListener('click', function (e) {
    if (held) { held = false; e.preventDefault(); return; }
    if (editing) return;
    onTap();
  });
  if (onHold) button.addEventListener('contextmenu', function (e) { e.preventDefault(); cancel(); held = false; if (!editing) onHold(); });
}

/* ---- Edit mode ---- The icons jiggle and can be dragged into a new
 * order; Done, Escape or a tap on the wallpaper ends it. */
var editing = false;
var doneButton = document.getElementById('edit-done');
function setEditing(on) {
  editing = on;
  root.toggleAttribute('data-editing', on);
  doneButton.hidden = !on;
  if (on) {
    Array.prototype.forEach.call(document.querySelectorAll('.app, .widget'), function (el, i) {
      el.style.setProperty('--jiggle-delay', (-(i * 137) % 260) + 'ms');
    });
    doneButton.focus({ preventScroll: true });
  } else saveOrder();
}
doneButton.addEventListener('click', function () { setEditing(false); });

// A long press on the wallpaper starts it, as on iOS.
(function () {
  var timer = null, start = null;
  document.querySelector('.springboard').addEventListener('pointerdown', function (e) {
    if (editing || e.target.closest('.app, .widget, button')) return;
    start = { x: e.clientX, y: e.clientY };
    timer = setTimeout(function () { setEditing(true); }, 600);
  });
  document.addEventListener('pointermove', function (e) {
    if (timer && Math.hypot(e.clientX - start.x, e.clientY - start.y) > 10) { clearTimeout(timer); timer = null; }
  });
  ['pointerup', 'pointercancel'].forEach(function (t) { document.addEventListener(t, function () { clearTimeout(timer); timer = null; }); });
  document.querySelector('.springboard').addEventListener('click', function (e) {
    if (editing && !e.target.closest('.app, .widget')) setEditing(false);
  });
}());

/** Move one icon to before `ref` (or the end), the others sliding along. */
function placeApp(app, ref) {
  var apps = Array.prototype.slice.call(grid.querySelectorAll('.app'));
  var before = apps.map(function (a) { return a.getBoundingClientRect(); });
  grid.insertBefore(app, ref);
  apps.forEach(function (a, i) {
    if (a === app || REDUCED) return;
    var now = a.getBoundingClientRect(), dx = before[i].left - now.left, dy = before[i].top - now.top;
    if (dx || dy) a.animate([{ translate: dx + 'px ' + dy + 'px' }, { translate: '0 0' }], { duration: 260, easing: 'cubic-bezier(.32,.72,0,1)' });
  });
}

// Dragging an icon: it follows the finger, and the others make room.
grid.addEventListener('pointerdown', function (e) {
  var button = e.target.closest('.app-button');
  if (!editing || !button || e.button !== 0) return;
  e.preventDefault();
  var app = button.parentElement, r = app.getBoundingClientRect();
  var grab = { x: e.clientX - r.left, y: e.clientY - r.top }, dx = 0, dy = 0, moved = false;
  try { button.setPointerCapture(e.pointerId); } catch (err) {}
  // Where the grid has the icon, its drag offset taken off.
  function slot() { var b = app.getBoundingClientRect(); return { left: b.left - dx, top: b.top - dy }; }
  function follow(x, y) {
    var at = slot();
    dx = x - grab.x - at.left; dy = y - grab.y - at.top;
    app.style.translate = dx + 'px ' + dy + 'px';
  }
  function move(ev) {
    if (!moved && Math.hypot(ev.clientX - e.clientX, ev.clientY - e.clientY) < 6) return;
    if (!moved) { moved = true; app.classList.add('dragging'); }
    // The icon under the finger gives up its place; one still sliding away
    // is not where it will be, so it is no target yet.
    var target = null;
    Array.prototype.forEach.call(grid.querySelectorAll('.app:not(.dragging)'), function (other) {
      if (other.getAnimations().some(function (a) { return a.playState === 'running' && !(a instanceof CSSAnimation); })) return;
      var b = other.querySelector('.app-icon').getBoundingClientRect();
      if (ev.clientX > b.left && ev.clientX < b.right && ev.clientY > b.top && ev.clientY < b.bottom) target = other;
    });
    if (target) {
      var later = Array.prototype.indexOf.call(grid.children, target) > Array.prototype.indexOf.call(grid.children, app);
      placeApp(app, later ? target.nextSibling : target);
    }
    follow(ev.clientX, ev.clientY);
  }
  function up() {
    button.removeEventListener('pointermove', move);
    button.removeEventListener('pointerup', up);
    button.removeEventListener('pointercancel', up);
    if (!moved) return;
    var from = dx + 'px ' + dy + 'px';
    app.style.translate = '';
    app.classList.remove('dragging');
    if (!REDUCED) app.animate([{ translate: from }, { translate: '0 0' }], { duration: 220, easing: 'cubic-bezier(.32,.72,0,1)' });
    saveOrder();
  }
  button.addEventListener('pointermove', move);
  button.addEventListener('pointerup', up);
  button.addEventListener('pointercancel', up);
});

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

/* ---- Opening a game ---- The icon opens into a black window, and rides
 * to the middle of it: where the game's launch screen shows it while the
 * game loads, so the handover is seamless. */
var ICON_ON_LAUNCH = 108;
function launchFrames(slug, button) {
  var icon = button.querySelector('.app-icon').getBoundingClientRect();
  var r = icon.width * 0.2237, W = innerWidth, H = innerHeight;
  var clipIcon = 'inset(' + icon.top + 'px ' + (W - icon.right) + 'px ' + (H - icon.bottom) + 'px ' + icon.left + 'px round ' + r + 'px)';
  var clipFull = 'inset(0px 0px 0px 0px round 0px)';
  // The launch screen's icon sits 16px above the middle: half its gap and bar.
  var cx = W / 2 - ICON_ON_LAUNCH / 2, cy = H / 2 - 16 - ICON_ON_LAUNCH / 2;
  var s = icon.width / ICON_ON_LAUNCH;
  var atIcon = 'translate(' + icon.left + 'px,' + icon.top + 'px) scale(' + s + ')';
  var atMiddle = 'translate(' + cx + 'px,' + cy + 'px) scale(1)';
  var img = zoom.querySelector('img');
  img.src = slug + '/icon-512.png?v=2';
  img.style.transformOrigin = '0 0';
  zoom.style.width = W + 'px';
  zoom.style.height = H + 'px';
  zoom.hidden = false;
  return { img: img, clipIcon: clipIcon, clipFull: clipFull, atIcon: atIcon, atMiddle: atMiddle, radius: (24 * s) + 'px' };
}
var leaving = false;
function launch(slug, from, url) {
  if (leaving) return;
  leaving = true;
  remember(slug);
  try { sessionStorage.setItem('blip-freeplay', '1'); } catch (e) {}
  var target = url || (slug + '/');
  tick();
  if (REDUCED || !from || !zoom.animate) { location.href = target; return; }
  var f = launchFrames(slug, from);
  var timing = { duration: 380, easing: 'cubic-bezier(.32,.72,0,1)', fill: 'forwards' };
  zoom.animate([{ clipPath: f.clipIcon }, { clipPath: f.clipFull }], timing);
  f.img.animate([{ transform: f.atIcon }, { transform: f.atMiddle }], timing)
    .onfinish = function () { location.href = target; };
}

/* Back from a game: the window closes back into its icon. */
function arrive() {
  var back = null;
  try {
    back = JSON.parse(sessionStorage.getItem('blip-home-return') || 'null');
    sessionStorage.removeItem('blip-home-return');
  } catch (e) {}
  if (!back || !BLIP_GAMES[back.slug] || Date.now() - back.at > 15000) return;
  var button = grid.querySelector('[data-slug="' + back.slug + '"]');
  if (!button) return;
  button.focus({ preventScroll: true });
  if (REDUCED || !zoom.animate) return;
  var f = launchFrames(back.slug, button);
  var timing = { duration: 340, easing: 'cubic-bezier(.32,.72,0,1)', fill: 'forwards' };
  zoom.animate([{ clipPath: f.clipFull }, { clipPath: f.clipIcon }], timing);
  f.img.animate([{ transform: f.atMiddle }, { transform: f.atIcon }], timing)
    .onfinish = function () { zoom.hidden = true; zoom.getAnimations({ subtree: true }).forEach(function (a) { a.cancel(); }); };
}

// Back from the game by the browser's own back button: the page comes out
// of the cache with the zoom still covering it.
window.addEventListener('pageshow', function (e) {
  if (e.persisted) { leaving = false; zoom.hidden = true; zoom.getAnimations({ subtree: true }).forEach(function (a) { a.cancel(); }); }
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
  searchSheet.hidden = true; lift.hidden = true;
  if (returnFocus && returnFocus.focus) returnFocus.focus({ preventScroll: true });
}
backdrop.addEventListener('click', closeSheet);

var actionSlug = null, actionFrom = null;
var lift = document.getElementById('context-lift');
function openActions(slug, from) {
  actionSlug = slug; actionFrom = from;
  if (navigator.vibrate) try { navigator.vibrate(8); } catch (e) {}
  var game = BLIP_GAMES[slug];
  document.getElementById('action-title').textContent = titleCase(game.name) + ' \u00b7 ' + (twoPlayer(slug) ? '1 or 2 players' : '1 player');
  var icon = from.querySelector('.app-icon').getBoundingClientRect();
  lift.querySelector('img').src = slug + '/icon-192.png?v=2';
  Object.assign(lift.style, { left: icon.left + 'px', top: icon.top + 'px', width: icon.width + 'px', height: icon.height + 'px' });
  lift.hidden = false;
  showSheet(actionSheet, from);
  // Below the icon, or above where the screen runs out; never off its sides.
  var w = actionSheet.offsetWidth, h = actionSheet.offsetHeight, pad = 12, gap = icon.height * 0.12 + 10;
  var fromRight = icon.left + icon.width / 2 > innerWidth / 2;
  var left = Math.max(pad, Math.min(fromRight ? icon.right - w : icon.left, innerWidth - w - pad));
  var below = icon.bottom + gap + h < innerHeight - pad;
  var top = below ? icon.bottom + gap : Math.max(pad, icon.top - gap - h);
  actionSheet.style.left = left + 'px';
  actionSheet.style.top = top + 'px';
  actionSheet.style.setProperty('--from', (icon.left + icon.width / 2 - left) + 'px ' + (below ? '0' : h + 'px'));
}
actionSheet.addEventListener('click', function (e) {
  var b = e.target.closest('button[data-action]');
  if (!b) return;
  var action = b.dataset.action, slug = actionSlug, from = actionFrom;
  closeSheet();
  // The game's page is what a browser saves, so it shows how from there.
  if (action === 'play') launch(slug, from);
  else if (action === 'install') launch(slug, from, slug + '/?install');
  else if (action === 'edit') setEditing(true);
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
document.getElementById('set-reset').addEventListener('click', function () {
  try { localStorage.removeItem(ORDER_KEY); } catch (e) {}
  Object.keys(BLIP_GAMES).forEach(function (slug) { grid.appendChild(grid.querySelector('.app-button[data-slug="' + slug + '"]').parentElement); });
  closeSheet();
});

/* ---- Keys and pads ---- Arrows move to the nearest icon that way (the
 * widget breaks the grid's rows), Enter or the pad's first button opens it,
 * Escape (or the second) closes a sheet. */
function focusables() {
  var scope = openSheet || document;
  return Array.prototype.slice.call(scope.querySelectorAll(openSheet ? 'button:not([hidden]), input' : '.app-button'));
}
function move(dir) {
  var items = focusables();
  if (!items.length) return;
  var here = document.activeElement, i = items.indexOf(here);
  if (i < 0) { items[0].focus(); return; }
  if (openSheet) {
    var step = dir === 'up' || dir === 'left' ? -1 : 1;
    items[(i + step + items.length) % items.length].focus();
    return;
  }
  var a = here.getBoundingClientRect(), ax = a.left + a.width / 2, ay = a.top + a.height / 2;
  var best = null, bestCost = Infinity;
  items.forEach(function (el) {
    if (el === here) return;
    var b = el.getBoundingClientRect(), dx = b.left + b.width / 2 - ax, dy = b.top + b.height / 2 - ay;
    var along = dir === 'left' ? -dx : dir === 'right' ? dx : dir === 'up' ? -dy : dy;
    var across = dir === 'left' || dir === 'right' ? Math.abs(dy) : Math.abs(dx);
    if (along < 4) return;
    var cost = along + across * 2;
    if (cost < bestCost) { bestCost = cost; best = el; }
  });
  if (best) best.focus();
}
var ARROWS = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right' };
document.addEventListener('keydown', function (e) {
  if (editing && e.key === 'Escape') { e.preventDefault(); setEditing(false); return; }
  if (editing && e.shiftKey && ARROWS[e.key] && document.activeElement.closest('#app-grid .app')) {
    // Shift and an arrow carries the focused icon along the row.
    e.preventDefault();
    var app = document.activeElement.closest('.app'), apps = Array.prototype.slice.call(grid.querySelectorAll('.app')), i = apps.indexOf(app);
    var j = ARROWS[e.key] === 'left' || ARROWS[e.key] === 'up' ? i - 1 : i + 1;
    if (j >= 0 && j < apps.length) placeApp(app, j > i ? apps[j].nextSibling : apps[j]);
    document.activeElement.focus();
    saveOrder();
    return;
  }
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

/* ---- Search ---- The pill, or a letter typed on the home screen. */
var searchSheet = document.getElementById('search-sheet');
var searchInput = document.getElementById('search-input');
var searchResults = document.getElementById('search-results');
function genreOf(slug) { return twoPlayer(slug) ? '1 or 2 players' : '1 player'; }
function renderSearch() {
  var q = searchInput.value.trim().toLowerCase();
  var hits = Object.keys(BLIP_GAMES).filter(function (slug) {
    return !q || BLIP_GAMES[slug].name.toLowerCase().indexOf(q) >= 0 || slug.replace('_', ' ').indexOf(q) >= 0;
  });
  searchResults.innerHTML = hits.length ? hits.map(function (slug, i) {
    return '<li><button type="button" data-slug="' + slug + '"' + (i === 0 && q ? ' class="first"' : '') + '><img src="' + slug + '/icon-192.png?v=2" alt=""><span>' +
      titleCase(BLIP_GAMES[slug].name) + '<small>' + genreOf(slug) + '</small></span></button></li>';
  }).join('') : '<li class="search-none">No games match “' + escapeHtml(searchInput.value.trim()) + '”.</li>';
}
function openSearch(first) {
  searchInput.value = first || '';
  renderSearch();
  openSheet = searchSheet; returnFocus = document.activeElement;
  searchSheet.hidden = false;
  searchInput.focus({ preventScroll: true });
}
document.getElementById('search-pill').addEventListener('click', function () { openSearch(''); });
document.getElementById('search-cancel').addEventListener('click', function () { searchSheet.hidden = true; openSheet = null; if (returnFocus && returnFocus.focus) returnFocus.focus(); });
searchInput.addEventListener('input', renderSearch);
searchInput.addEventListener('keydown', function (e) {
  if (e.key === 'Enter') { var first = searchResults.querySelector('button'); if (first) { e.preventDefault(); first.click(); } }
  if (e.key === 'ArrowDown') { var b = searchResults.querySelector('button'); if (b) { e.preventDefault(); e.stopPropagation(); b.focus(); } }
});
searchResults.addEventListener('click', function (e) {
  var b = e.target.closest('button[data-slug]');
  if (!b) return;
  var slug = b.dataset.slug, icon = grid.querySelector('.app-button[data-slug="' + slug + '"]');
  searchSheet.hidden = true; openSheet = null;
  launch(slug, icon);
});
document.addEventListener('keydown', function (e) {
  if (openSheet || e.ctrlKey || e.metaKey || e.altKey || e.key.length !== 1 || !/[a-z]/i.test(e.key)) return;
  e.preventDefault();
  openSearch(e.key);
});

// A finger drawn down the home screen opens Search, as on iOS. Touch events,
// not pointer events: a pan cancels the pointer before it has gone far.
(function () {
  var start = null;
  document.querySelector('.springboard').addEventListener('touchstart', function (e) {
    start = e.touches.length === 1 ? { x: e.touches[0].clientX, y: e.touches[0].clientY } : null;
  }, { passive: true });
  document.addEventListener('touchmove', function (e) {
    if (!start || openSheet || leaving) return;
    var dx = e.touches[0].clientX - start.x, dy = e.touches[0].clientY - start.y;
    if (dy > 56 && Math.abs(dx) < dy / 2) { start = null; openSearch(''); }
  }, { passive: true });
  document.addEventListener('touchend', function () { start = null; }, { passive: true });
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
