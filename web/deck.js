/* Shared input layer for pad, stick, keyboard and gamepad.
 * Inputs: directions, button1–4 (button2 falls back), p2up–p2button4, start/select edges.
 * `init` injects per-game key events; start/select can call callbacks. Without a canvas, callers handle logical events. */
(function () {
  'use strict';

  // Directional names — the click flavour depends on it. A second
  // player's stick carries the same four under a p2 prefix.
  var DIR = { up: 1, down: 1, left: 1, right: 1,
              p2up: 1, p2down: 1, p2left: 1, p2right: 1 };

  var cfg = {
    canvas: null,       // where synthetic KeyboardEvents go (null = don't inject)
    gate: null,         // fn -> bool ; true = swallow game input (e.g. INSERT COIN up)
    kioskHref: '../index.html',
    onInput: null,      // fn(name, down) — every logical state change, for the consumer
    onStart: null,      // fn() — overrides the default start action
    onSelect: null,     // fn() — overrides the default select action
    feedback: null,     // fn() — a haptic/detent tick, fired with the click on a real press
    click: true         // play a soft button click on every press
  };

  var keymap = null;              // logical name -> { key, code }
  var ARROW_NAME = {
    ArrowUp: 'up', KeyW: 'up', ArrowDown: 'down', KeyS: 'down',
    ArrowLeft: 'left', KeyA: 'left', ArrowRight: 'right', KeyD: 'right'
  };
  var held   = {};                // logical name -> bool
  var injected = {};             // caps can share one physical key
  var roots  = [];                // containers holding [data-blip] elements, for visual reflect

  /* ------------------------------------------------------------------ */
  /* Key map — arrows are fixed, the action buttons come per game         */
  /* ------------------------------------------------------------------ */

  function buildKeymap(buttons, extra) {
    var list = (buttons && buttons.length) ? buttons : [{ key: ' ', code: 'Space' }];
    var map = {
      up:      { key: 'ArrowUp',    code: 'ArrowUp' },
      down:    { key: 'ArrowDown',  code: 'ArrowDown' },
      left:    { key: 'ArrowLeft',  code: 'ArrowLeft' },
      right:   { key: 'ArrowRight', code: 'ArrowRight' }
    };
    // One logical name per button the game declares.
    for (var i = 0; i < list.length; i++) {
      map['button' + (i + 1)] = { key: list[i].key, code: list[i].code };
    }
    // A one-button game still answers to button2: deck.js gives it two
    // live caps that both fire.
    if (!map.button2) map.button2 = map.button1;
    // Anything else the game names itself — a second player's stick and
    // buttons (p2up … p2button4), or its own keys for player one's
    // directions when the arrows belong to player two.
    for (var n in extra) { if (extra[n]) map[n] = extra[n]; }
    return map;
  }

  /* Which logical name a physical key code belongs to. The arrows are
     fixed; the action keys are whatever this game declared, so the
     deck lights the right cap when a keyboard player presses one. */
  function nameForCode(code) {
    // What this game declared wins over the default arrows: in a game
    // where the arrows are player two's, ArrowUp must light player two's
    // stick, not player one's.
    for (var n in keymap || {}) {
      if (keymap[n] && keymap[n].code === code) return n;
    }
    return ARROW_NAME[code] || null;
  }

  function injectKey(spec, type) {
    if (!cfg.canvas || !spec) return;
    var count = injected[spec.code] || 0;
    if (type === 'keydown') {
      injected[spec.code] = count + 1;
      if (count) return;
    } else {
      if (count > 1) { injected[spec.code] = count - 1; return; }
      delete injected[spec.code];
    }
    cfg.canvas.dispatchEvent(new KeyboardEvent(type, {
      bubbles: true, cancelable: true, key: spec.key, code: spec.code
    }));
  }

  /* select is a plain navigation out — always allowed, even off the coin
     wall. Everything else is swallowed while the gate says so. */
  function swallowed(name) {
    if (name === 'select') return false;
    return cfg.gate ? !!cfg.gate() : false;
  }

  /* A raw key injection that still respects the coin wall — used by the
     rally dial and by the default start action, both of which drive
     explicit key specs rather than a logical name. */
  function emitKey(spec, type) {
    if (type === 'keydown' && cfg.gate && cfg.gate()) return;
    injectKey(spec, type);
  }

  /* ------------------------------------------------------------------ */
  /* Click sounds — a soft plastic "clack" per press                     */
  /* ------------------------------------------------------------------ */

  var _actx = null;
  function actx() {
    if (Howler.ctx) {
      if (Howler.ctx.state === 'suspended') Howler.ctx.resume();
      return Howler.ctx;
    }
    try {
      if (!_actx) _actx = new (window.AudioContext || window.webkitAudioContext)();
      if (_actx.state === 'suspended') _actx.resume();
    } catch (e) { return null; }
    return _actx;
  }

  // kind: 'face' | 'dpad' | 'shoulder' | 'menu' — each pad button has its
  // own weight of clack, the way a real controller's do.
  function tick() { if (cfg.feedback) { try { cfg.feedback(); } catch (e) {} } }

  function click(kind) {
    if (!cfg.click) { return; }
    var ctx = actx();
    if (!ctx) return;
    var t = ctx.currentTime;
    var spec = ({
      face:     { hp: 1900, ng: 0.16, nd: 0.020, thk: 128, tg: 0.10, td: 0.045 },
      dpad:     { hp: 1400, ng: 0.11, nd: 0.028, thk: 92,  tg: 0.09, td: 0.055 },
      shoulder: { hp: 1200, ng: 0.13, nd: 0.024, thk: 78,  tg: 0.11, td: 0.060 },
      menu:     { hp: 2600, ng: 0.10, nd: 0.014, thk: 180, tg: 0.05, td: 0.030 }
    })[kind] || { hp: 1800, ng: 0.14, nd: 0.020, thk: 120, tg: 0.09, td: 0.045 };

    // The plastic contact transient: a very short filtered-noise tick.
    var src = blipNoise(ctx, spec.nd, function (t, p) { return 1 - p; });
    var hp = ctx.createBiquadFilter(); hp.type = 'highpass'; hp.frequency.value = spec.hp;
    var ng = ctx.createGain();
    ng.gain.setValueAtTime(spec.ng, t);
    ng.gain.exponentialRampToValueAtTime(0.0006, t + spec.nd);
    src.connect(hp); hp.connect(ng); ng.connect(blipOut(ctx));
    src.start(t); src.stop(t + spec.nd + 0.01);

    // The muted thock of the cap bottoming out.
    var osc = ctx.createOscillator(), og = ctx.createGain();
    osc.type = 'sine'; osc.frequency.setValueAtTime(spec.thk, t);
    osc.frequency.exponentialRampToValueAtTime(spec.thk * 0.6, t + spec.td);
    og.gain.setValueAtTime(spec.tg, t);
    og.gain.exponentialRampToValueAtTime(0.0006, t + spec.td);
    osc.connect(og); og.connect(blipOut(ctx));
    osc.start(t); osc.stop(t + spec.td + 0.02);
  }

  /* ------------------------------------------------------------------ */
  /* Visual reflection — light up every [data-blip] for a logical name    */
  /* ------------------------------------------------------------------ */

  function reflect(name, down) {
    for (var r = 0; r < roots.length; r++) {
      var els = roots[r].querySelectorAll('[data-blip~="' + name + '"]');
      for (var i = 0; i < els.length; i++) els[i].classList.toggle('blip-pressed', down);
    }
  }

  /* ------------------------------------------------------------------ */
  /* The one entry point every source calls                              */
  /* ------------------------------------------------------------------ */

  function edgeAction(name) {
    if (name === 'start') {
      if (cfg.onStart) { cfg.onStart(); return; }
      // default: tap the primary action, i.e. "PRESS FIRE" on a title screen
      emitKey(keymap.button1, 'keydown');
      emitKey(keymap.button1, 'keyup');
      return;
    }
    if (name === 'select') {
      if (cfg.onSelect) { cfg.onSelect(); return; }
      // default: leave for the arcade's game grid (kioskHref is relative to
      // the current page — '../index.html' from a game, 'index.html' from
      // the info pages).
      if (cfg.kioskHref) window.location.href = cfg.kioskHref;
    }
  }

  // opts.silent — reflect + emit but don't inject a key or click. Used by
  // the keyboard mirror: the real keypress already reached the game, we
  // only want the on-screen pad/stick to light up in sympathy.
  function set(name, down, opts) {
    opts = opts || {};
    down = !!down;
    if (!keymap) return;

    if (name === 'start' || name === 'select') {
      if (down === !!held[name]) return;
      held[name] = down;
      reflect(name, down);
      if (cfg.onInput) { try { cfg.onInput(name, down); } catch (e) {} }
      if (down && !swallowed(name)) {
        if (!opts.silent) { click('menu'); tick(); }
        edgeAction(name);
      }
      return;
    }

    if (!keymap[name]) return;
    if (down === !!held[name]) return;
    held[name] = down;

    reflect(name, down);
    if (cfg.onInput) { try { cfg.onInput(name, down); } catch (e) {} }

    if (swallowed(name)) return;
    if (!opts.silent) {
      injectKey(keymap[name], down ? 'keydown' : 'keyup');
      if (down) { if (!opts.silentClick) click(DIR[name] ? 'dpad' : 'face'); tick(); }
    }
  }

  function releaseAll() {
    _tBtns.forEach(function (g) { g._pend = false; g.release(); });
    _tDpads.forEach(function (d) { d._pend = false; d.clear(); });
    _tDials.forEach(function (release) { release(); });
    Object.keys(held).forEach(function (n) { if (held[n]) set(n, false); });
  }

  /* ------------------------------------------------------------------ */
  /* Touch-list authority                                                */
  /* ------------------------------------------------------------------ */
  // iOS cancels pointers during multi-touch; derive held state from `TouchEvent.touches` and ignore `pointercancel`.

  var _tBtns = [];    // { el, on, press, release, _pend }
  var _tDpads = [];   // one { pad, calc, clear, _pend } per bound d-pad
  var _tDials = [];

  // Releases wait a frame: iOS cancels every touch when a multi-touch gesture
  // moves, then re-fires touchstart for the fingers still down.
  function _defer(o, fn) {
    if (o._pend) return;
    o._pend = true;
    requestAnimationFrame(function () { if (o._pend) { o._pend = false; fn(); } });
  }
  // Slop past a control's edge that still counts as touching it; each touch
  // belongs to the one control whose edge is nearest (0 inside), so
  // neighbours never both fire.
  var BTN_SLOP = 12, DPAD_SLOP = 14;
  function _edgeDist(r, x, y) {
    var dx = Math.max(r.left - x, 0, x - r.right);
    var dy = Math.max(r.top - y, 0, y - r.bottom);
    return Math.sqrt(dx * dx + dy * dy);
  }
  // Control rects, read on touchstart / resize only: a touchmove arrives at
  // up to 120 Hz per finger, and reading them there forced a layout per move.
  var _brects = null, _drects = null;
  function _readRects() {
    _brects = _tBtns.map(function (b) { return b.el.getBoundingClientRect(); });
    _drects = _tDpads.map(function (p) { return p.pad.getBoundingClientRect(); });
  }
  window.addEventListener('resize', function () { _brects = null; });
  function _syncTouches(list, fresh) {
    var i, j, r, x, y, d, best, bestD, bestC;
    if (fresh || !_brects || _brects.length !== _tBtns.length || _drects.length !== _tDpads.length) _readRects();
    var brects = _brects, drects = _drects;
    var bDown = [], dTouch = [];
    for (j = 0; j < list.length; j++) {
      x = list[j].clientX; y = list[j].clientY;
      best = null; bestD = Infinity; bestC = Infinity;
      for (i = 0; i < brects.length; i++) {
        r = brects[i];
        if (!r.width) continue;
        d = _edgeDist(r, x, y);
        if (d > BTN_SLOP) continue;
        var c = Math.hypot(x - (r.left + r.width / 2), y - (r.top + r.height / 2));
        if (d < bestD || (d === bestD && c < bestC)) { bestD = d; bestC = c; best = { b: i }; }
      }
      for (i = 0; i < drects.length; i++) {
        r = drects[i];
        if (!r.width) continue;
        d = _edgeDist(r, x, y);
        if (d > DPAD_SLOP) continue;
        var c2 = Math.hypot(x - (r.left + r.width / 2), y - (r.top + r.height / 2));
        if (d < bestD || (d === bestD && c2 < bestC)) { bestD = d; bestC = c2; best = { p: i }; }
      }
      if (!best) continue;
      if (best.b !== undefined) bDown[best.b] = true;
      else if (!dTouch[best.p]) dTouch[best.p] = list[j];
    }
    for (i = 0; i < _tBtns.length; i++) {
      if (!brects[i].width) continue;
      if (bDown[i]) { _tBtns[i]._pend = false; _tBtns[i].press(); }
      else if (_tBtns[i].on) _defer(_tBtns[i], _tBtns[i].release);
    }
    for (i = 0; i < _tDpads.length; i++) {
      var dp = _tDpads[i];
      if (dTouch[i]) { dp._pend = false; dp.calc(dTouch[i], drects[i]); }
      else if (drects[i].width) _defer(dp, dp.clear);
    }
  }
  ['touchstart', 'touchmove', 'touchend', 'touchcancel'].forEach(function (ev) {
    document.addEventListener(ev, function (e) { _syncTouches(e.touches, ev === 'touchstart'); }, { passive: true });
  });

  /* ------------------------------------------------------------------ */
  /* Binders — connect a source surface to set()                         */
  /* ------------------------------------------------------------------ */

  // Any container of elements with data-blip="<logical name>"
  // (space-separated for several). On touch the live touch list is the
  // authority, on a mouse the pointer events. A cross uses bindDpad().
  function bindButtons(root) {
    if (!root) return;
    if (roots.indexOf(root) === -1) roots.push(root);
    var els = root.querySelectorAll('[data-blip]');
    for (var i = 0; i < els.length; i++) (function (el) {
      var raw = el.getAttribute('data-blip');
      // data-blip-nopress: reflect-only (the d-pad arms — their input
      // comes from bindDpad's floating pivot, not a discrete tap).
      if (raw === 'dpad' || el.hasAttribute('data-blip-nopress')) return;
      var names = raw.split(/\s+/).filter(Boolean);
      // data-blip-kind overrides the click flavour (shoulder / menu); the
      // logical name picks it otherwise (face for buttons, dpad for a d-pad
      // segment bound individually).
      var kind = el.getAttribute('data-blip-kind');
      var grp = { el: el, on: false };
      grp.press = function () {
        if (grp.on) return;
        grp.on = true;
        if (kind) { click(kind); names.forEach(function (n) { set(n, true, { silentClick: true }); }); }
        else      { names.forEach(function (n) { set(n, true); }); }
      };
      grp.release = function () {
        if (!grp.on) return;
        grp.on = false;
        names.forEach(function (n) { set(n, false); });
      };
      _tBtns.push(grp);
      el.addEventListener('pointerdown', function (e) {
        e.preventDefault();
        try { el.setPointerCapture(e.pointerId); } catch (x) {}
        // A touch is resolved (nearest control wins) by _syncTouches on the
        // touchstart that follows; pressing the element under the finger
        // here would light a neighbour the touch is not nearest to.
        if (e.pointerType !== 'touch') grp.press();
      });
      el.addEventListener('pointerup', function (e) {
        // touch releases are owned by _syncTouches (a real lift comes with
        // a touchend that drops the finger from the list); the mouse has no
        // such list, so honour its up directly.
        if (e.pointerType !== 'touch') grp.release();
      });
      ['pointercancel', 'lostpointercapture'].forEach(function (type) {
        el.addEventListener(type, function (e) { if (e.pointerType !== 'touch') grp.release(); });
      });
      // pointercancel: deliberately not handled — see "Touch-list authority".
    })(els[i]);
  }

  // The cross: one pivot at the centre of `pad`, the thumb resolved to four
  // ways with a dead centre; toward a corner both axes engage, so you can
  // roll round the rim.
  function bindDpad(pad, o) {
    if (!pad) return;
    o = o || {};
    var dead = o.dead != null ? o.dead : 0.32;
    // Which logical name each arm drives — a second player's cross runs
    // the same four arms under its own names.
    var names = o.names || { up: 'up', down: 'down', left: 'left', right: 'right' };
    var pid = null;
    // Light the matching d-pad arm (.dp-up … .dp-right inside `pad`) the
    // instant a direction engages — a local, precise reflection that
    // doesn't depend on the pad being a registered [data-blip] root.
    var segs = {};
    ['up', 'down', 'left', 'right'].forEach(function (n) {
      segs[n] = pad.querySelector('.dp-' + n);
    });
    function put(n, on) {
      set(names[n], on);
      if (segs[n]) segs[n].classList.toggle('blip-pressed', on);
    }
    // e is a PointerEvent or a Touch — both carry clientX / clientY; r is
    // the pad's rect when the caller already has it.
    function calc(e, r) {
      r = r || pad.getBoundingClientRect();
      var nx = (e.clientX - (r.left + r.width  / 2)) / (r.width  / 2);
      var ny = (e.clientY - (r.top  + r.height / 2)) / (r.height / 2);
      put('left',  nx < -dead);
      put('right', nx >  dead);
      put('up',    ny < -dead);
      put('down',  ny >  dead);
      if (!pad.classList.contains('blip-touched')) pad.classList.add('blip-touched');
    }
    function clear() {
      ['up', 'down', 'left', 'right'].forEach(function (n) { put(n, false); });
      pad.classList.remove('blip-touched');
    }
    _tDpads.push({ pad: pad, calc: calc, clear: clear, _pend: false });
    pad.addEventListener('pointerdown', function (e) {
      e.preventDefault();
      pid = e.pointerId;
      try { pad.setPointerCapture(pid); } catch (x) {}
      if (e.pointerType !== 'touch') calc(e);
    });
    pad.addEventListener('pointermove', function (e) {
      if (e.pointerId !== pid) return;
      e.preventDefault();
      if (e.pointerType !== 'touch') calc(e);
    });
    pad.addEventListener('pointerup', function (e) {
      if (e.pointerId !== pid) return;
      pid = null;
      // touch: _syncTouches owns the (deferred) clear; mouse: clear now
      if (e.pointerType !== 'touch') clear();
    });
    ['pointercancel', 'lostpointercapture'].forEach(function (type) {
      pad.addEventListener(type, function (e) {
        if (e.pointerType !== 'touch' && e.pointerId === pid) { pid = null; clear(); }
      });
    });
    // pointercancel: deliberately not handled — see "Touch-list authority".
  }

  // Rally dial: angle past `dead` injects up/down; small movement calls `onTap`.
  // Options: injected `up`/`down` keys, `onTap`/`onInteract` callbacks, `dead` radians (default 0.018).
  function bindDial(dialEl, opts2) {
    if (!dialEl) return;
    opts2 = opts2 || {};
    var dead = opts2.dead != null ? opts2.dead : 0.018;
    // How long a direction keeps being held after the finger stops
    // moving. See the accumulator in pointermove for why this exists.
    var IDLE_RELEASE_MS = opts2.idleMs != null ? opts2.idleMs : 70;
    var last = null, pid = null, total = 0, upHeld = false, downHeld = false;
    var acc = 0, idleTimer = null;

    var rect = null; // read once per grab, not per pointermove
    function angleAt(x, y) {
      var r = rect || dialEl.getBoundingClientRect();
      return Math.atan2(y - (r.top + r.height / 2), x - (r.left + r.width / 2));
    }
    function dir(u, d) {
      if (u !== upHeld)   { upHeld = u;   if (u) click('dpad'); emitKey(opts2.up,   u ? 'keydown' : 'keyup'); }
      if (d !== downHeld) { downHeld = d; if (d) click('dpad'); emitKey(opts2.down, d ? 'keydown' : 'keyup'); }
    }
    function clearIdle() { if (idleTimer) { clearTimeout(idleTimer); idleTimer = null; } }
    function stop() { clearIdle(); dir(false, false); dialEl.classList.remove('active'); last = null; pid = null; acc = 0; rect = null; }
    _tDials.push(stop);

    dialEl.addEventListener('pointerdown', function (e) {
      e.preventDefault();
      if (pid !== null) return;
      if (opts2.onInteract) opts2.onInteract();
      pid = e.pointerId;
      try { dialEl.setPointerCapture(pid); } catch (x) {}
      rect = dialEl.getBoundingClientRect();
      last = angleAt(e.clientX, e.clientY);
      total = 0;
      dialEl.classList.add('active');
    });
    dialEl.addEventListener('pointermove', function (e) {
      if (e.pointerId !== pid || last === null) return;
      e.preventDefault();
      var a = angleAt(e.clientX, e.clientY);
      var d = a - last;
      if (d >  Math.PI) d -= 2 * Math.PI;
      if (d < -Math.PI) d += 2 * Math.PI;
      last = a;
      total += Math.abs(d);

      // Rotation accumulates rather than being tested per event, so a slow,
      // careful turn still crosses the dead band; the direction then holds
      // until the finger stops (IDLE_RELEASE_MS) or reverses.
      acc += d;
      if (acc > dead) { acc = 0; dir(false, true); }
      else if (acc < -dead) { acc = 0; dir(true, false); }

      clearIdle();
      idleTimer = setTimeout(function () { idleTimer = null; acc = 0; dir(false, false); }, IDLE_RELEASE_MS);
    });
    function end(e) {
      if (e.type !== 'pointercancel' && e.pointerId !== pid) return;
      if (e.type === 'pointerup' && total < 0.08) {
        if (opts2.onTap) opts2.onTap();
        if (opts2.tap) { click('menu'); emitKey(opts2.tap, 'keydown'); emitKey(opts2.tap, 'keyup'); }
      }
      stop();
    }
    dialEl.addEventListener('pointerup', end);
    dialEl.addEventListener('pointercancel', end);
    dialEl.addEventListener('lostpointercapture', function (e) { if (e.pointerId === pid) stop(); });
  }

  // Physical keyboard, reflection only: the key already reaches the game;
  // this lights the deck to match.
  function bindKeyboard() {
    document.addEventListener('keydown', function (e) {
      var n = nameForCode(e.code); if (n) set(n, true, { silent: true });
    });
    document.addEventListener('keyup', function (e) {
      var n = nameForCode(e.code); if (n) set(n, false, { silent: true });
    });
  }

  // Map gamepad codes to player one directly; Bubbler and Brawler bind arrows to player two.
  var GP_NAME = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right',
                  Space: 'button1', KeyZ: 'button2', Start: 'start', Select: 'select' };
  function bindGamepad(poll) {
    if (typeof poll !== 'function') return;
    function name(code) { return GP_NAME[code] || null; }
    poll(
      function (code) { var n = name(code); if (n) set(n, true); },
      function (code) { var n = name(code); if (n) set(n, false); }
    );
  }

  /* ------------------------------------------------------------------ */
  /* Public surface                                                      */
  /* ------------------------------------------------------------------ */

  window.BlipController = {
    init: function (opts) {
      opts = opts || {};
      ['canvas', 'gate', 'onInput', 'onStart', 'onSelect', 'kioskHref'].forEach(function (k) {
        if (opts[k] !== undefined) cfg[k] = opts[k];
      });
      if (opts.feedback !== undefined) cfg.feedback = opts.feedback;
      if (opts.click !== undefined) cfg.click = !!opts.click;
      keymap = buildKeymap(opts.buttons, opts.keys);
      return this;
    },
    set: set,
    releaseAll: releaseAll,
    registerVisual: function (root) { if (root && roots.indexOf(root) === -1) roots.push(root); },
    bindButtons: bindButtons,
    bindDpad: bindDpad,
    bindDial: bindDial,
    bindKeyboard: bindKeyboard,
    bindGamepad: bindGamepad
  };
}());

/* The control deck: two stations, each an arcade stick and a pad (the page's data-controls shows one) with two caps.
 *
 *   BlipDeck.build(panel, spec)   builds it into `panel` (a .deck-panel inside a .kiosk-bar)
 *
 * spec (all optional): buttons [{ label }] — a game's one or two actions; capLabel — the legend when a button has
 * none (FIRE); seats 1 or 2 — station two is drawn but takes no input with 1; touch { kind, hint, mouseHint,
 * actions } — a gesture strip for touch screens. Without buttons the caps carry no legend and A / B both fire.
 * Caps and arms carry data-blip="<logical name>" (up … button1, p2up … p2button2) for BlipController above. */
window.BlipDeck = (function () {
  var PER_PLAYER = 2; // caps a player ever gets: no game reads more than two

  function el(tag, cls, html, attrs) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (html) e.innerHTML = html;
    for (var k in (attrs || {})) e.setAttribute(k, attrs[k]);
    return e;
  }

  // Logical input names per station (BlipController's vocabulary).
  var STATIONS = [
    { id: '',    dirs: ['up', 'right', 'down', 'left'],         cap: function (i) { return 'button' + (i + 1); } },
    { id: '-p2', dirs: ['p2up', 'p2right', 'p2down', 'p2left'], cap: function (i) { return 'p2button' + (i + 1); } }
  ];

  // The cross, drawn as one moulded part: a rounded plus with softened inner
  // corners, one gradient over the whole of it, a rim light and a shadow.
  // The arm segments over it are the touch targets and light up when pressed.
  var PLUS = 'M38 2H62Q68 2 68 8V29Q68 32 71 32H92Q98 32 98 38V62Q98 68 92 68H71Q68 68 68 71V92Q68 98 62 98H38Q32 98 32 92V71Q32 68 29 68H8Q2 68 2 62V38Q2 32 8 32H29Q32 32 32 29V8Q32 2 38 2Z';
  function crossBody(id) {
    return '<svg class="snes-dp-body" viewBox="0 0 100 100" aria-hidden="true">' +
      '<defs><linearGradient id="dpg' + id + '" x1="0" y1="0" x2="0" y2="1">' +
      '<stop offset="0" class="dp-s0"/><stop offset=".55" class="dp-s1"/><stop offset="1" class="dp-s2"/>' +
      '</linearGradient><filter id="dps' + id + '" x="-10%" y="-10%" width="120%" height="125%">' +
      '<feDropShadow dx="0" dy="1.6" stdDeviation="1.4" flood-color="#000" flood-opacity=".75"/></filter></defs>' +
      '<path d="' + PLUS + '" fill="url(#dpg' + id + ')" filter="url(#dps' + id + ')"/>' +
      '<path class="dp-rim" d="' + PLUS + '"/>' +
      '<circle class="dp-dimple" cx="50" cy="50" r="7"/></svg>';
  }

  function touchStrip(touch, seats) {
    var kind = touch.kind;
    var twoTouch = seats === 2 && (kind === 'drag' || kind === 'action');
    var halves = kind === 'paddles' || kind === 'platform' || twoTouch ? ['P1', '2P'] : [''];
    var actions = kind === 'action' ? Array.from({ length: twoTouch ? 2 : 1 }, function (_, player) {
      return '<div class="ts-actions" data-player="' + player + '">' + touch.actions.map(function (label, i) {
        return '<div class="ts-action" data-button="' + (i + 1) + '">' + label + '</div>';
      }).join('') + '</div>';
    }).join('') : '';
    var strip = el('div', '', '<div class="ts-glass">' + halves.map(function (label, i) {
      return '<div class="ts-half" data-slot="' + i + '">' + (label ? '<span class="ts-tag">' + label + '</span>' : '') + '</div>';
    }).join('') + actions + '<span class="ts-demo"></span><span class="ts-thumb"></span><span class="ts-arrow"></span>' +
      '<span class="ts-hint for-touch">' + (kind === 'drag' ? 'Drag to move &middot; ' : '') + touch.hint + '</span>' +
      '<span class="ts-hint for-mouse">' + (touch.mouseHint || touch.hint) + '</span></div>', { id: 'touch-strip', 'aria-hidden': 'true' });
    if (kind === 'platform' || twoTouch)
      strip.querySelector('.ts-glass').appendChild(el('button', 'ts-join', '<span class="tj-plus">+</span>2P JOIN', { type: 'button' }));
    return strip;
  }

  function build(panel, spec) {
    spec = spec || {};
    var lettered = !!spec.buttons;
    var buttons = spec.buttons || [{}];
    // A one-button game still gets two live caps: A and B both fire.
    var specs = buttons.length < 2 ? [buttons[0], buttons[0]] : buttons;
    var seatsTwo = spec.seats === 2;
    var root = document.documentElement;
    root.setAttribute('data-caps', '4');  // the layout's name for this deck; CSS keys off it
    // Every deck seats two, as a two-player cabinet does; a one-player game's second station takes no input.
    root.setAttribute('data-players', '2');
    if (!seatsTwo) root.setAttribute('data-seats', '1');

    // Live caps: station two only when it seats two; without a game's buttons, A and B of station one are "fire".
    function capSpec(who, i) { return who === 1 && !seatsTwo ? null : specs[i] || null; }
    function capName(st, who, i) { return lettered ? st.cap(i) : who === 0 ? 'button1' : null; }
    // Caps carry no lettering; what each does is printed on the panel beside it.
    function legendFor(cap) { return cap && lettered ? cap.label || spec.capLabel || 'FIRE' : ''; }
    function cap(tag, cls, inner, st, who, i) {
      var c = capSpec(who, i), name = c && capName(st, who, i), text = legendFor(c);
      var b = el(tag, cls + (c ? '' : ' spare'), inner + (text ? '<small class="cap-legend" aria-hidden="true">' + text + '</small>' : ''));
      if (tag === 'button') b.type = 'button';
      if (name && (lettered || tag === 'button')) b.setAttribute('data-blip', name);
      if (text) b.setAttribute('aria-label', text);
      // A game with two different actions (Meteors: fire / hyperspace) gets a blue second cap.
      if (buttons.length === 2 && i === 1) b.classList.add('alt');
      return b;
    }

    var frag = document.createDocumentFragment();
    STATIONS.forEach(function (st, who) {
      var side = el('div', 'deck-side', '', { id: 'deck-p' + (who + 1) });
      var base = el('div', 'stick-base', '<div class="stick-boot"></div><div class="stick-handle"><div class="stick-shaft"></div><div class="stick-ball"></div></div>',
        { id: 'stick-base' + st.id });
      var fire = el('div', 'fire-buttons four', '', { id: 'fire-buttons' + st.id });
      for (var i = 0; i < PER_PLAYER; i++) fire.appendChild(cap('div', 'arcade-btn lettered', '<span class="arcade-btn-cap"></span>', st, who, i));
      side.appendChild(base);
      side.appendChild(fire);
      frag.appendChild(side);
    });
    STATIONS.forEach(function (st, who) {
      var pad = el('div', 'snes-pad' + (who ? ' second' : ''), '', { id: 'snes-pad' + st.id, 'aria-hidden': 'true' });
      var shell = el('div', 'snes-shell');
      var dpad = el('div', 'snes-dpad', crossBody(st.id || '-p1'));
      ['up', 'right', 'down', 'left'].forEach(function (d, k) {
        dpad.appendChild(el('span', 'snes-dp-seg dp-' + d, '', { 'data-blip': st.dirs[k], 'data-blip-nopress': '' }));
      });
      var face = el('div', 'snes-face four');
      for (var i = 0; i < PER_PLAYER; i++) face.appendChild(cap('button', 'snes-btn cap' + (i + 1), '<span></span>', st, who, i));
      shell.appendChild(dpad);
      shell.appendChild(face);
      if (!who) shell.appendChild(el('span', 'snes-wordmark', 'BLIP', { 'aria-hidden': 'true' }));
      pad.appendChild(shell);
      frag.appendChild(pad);
    });
    panel.insertBefore(frag, panel.firstChild);
    panel.parentElement.prepend(el('div', 'deck-housing', '', { 'aria-hidden': 'true' }));
    if (spec.touch) panel.parentElement.appendChild(touchStrip(spec.touch, spec.seats));
  }

  /* The 8-way restrictor-gate joystick: a drag that starts on the stick leans it and holds its directions through
   * BlipController. Capture starts anywhere on the screen: the ball can project over the picture although its base
   * stays in the deck. st: { stick, fire, dirs: { up, down, left, right } }; opts (all optional):
   *   zone      the element a drag must start on (default: the stick, or its station's half of the bar)
   *   bar       the deck (default #topbar); split: two stations share it, a half each; who: 0 or 1
   *   tuning    { engage, release, maxR, hyst } in pixels and degrees
   *   active()  false while the stick is not in use (the deck shows its pad)
   * Returns a function that stops listening. */
  function bindStick(st, opts) {
    opts = opts || {};
    var who = opts.who || 0;
    var base = st.stick, fire = st.fire, bar = opts.bar || document.getElementById('topbar');
    if (!base || !bar) return function () {};
    var tuning = opts.tuning || {};
    var ENGAGE  = tuning.engage  != null ? tuning.engage  : 16;
    var RELEASE = tuning.release != null ? tuning.release : 9;
    var MAX_R   = tuning.maxR    != null ? tuning.maxR    : 46;
    var HYST    = tuning.hyst    != null ? tuning.hyst    : 8;
    var GATE_KEYS = {
      0:   ['up'],            45:  ['up', 'right'],
      90:  ['right'],         135: ['down', 'right'],
      180: ['down'],          225: ['down', 'left'],
      270: ['left'],          315: ['up', 'left']
    };
    var activeId = null, engaged = false, lockDeg = null, pivot = null;
    var pendingMove = null, moveRaf = 0, nextMoveOk = 0;
    var wantDir = { up: false, down: false, left: false, right: false };

    function setDir(dir, want) {
      if (wantDir[dir] === want) return;
      wantDir[dir] = want;
      BlipController.set(st.dirs[dir], want);
    }
    function setLock(deg) {
      if (deg === lockDeg) return;
      lockDeg = deg;
      var keys = deg === null ? [] : GATE_KEYS[deg];
      ['up', 'down', 'left', 'right'].forEach(function (dir) { setDir(dir, keys.indexOf(dir) !== -1); });
    }
    function apply(e) {
      var dx = e.clientX - pivot.x, dy = e.clientY - pivot.y;
      var dist = Math.sqrt(dx * dx + dy * dy);
      if (dist > MAX_R) {
        // the centre follows a wandering thumb, so turning back acts at once
        var k = 1 - MAX_R / dist;
        pivot.x += dx * k; pivot.y += dy * k;
        dx = e.clientX - pivot.x; dy = e.clientY - pivot.y;
        dist = MAX_R;
      }
      engaged = engaged ? dist > RELEASE : dist > ENGAGE;
      if (!engaged) { setLock(null); return; }
      var ang = Math.atan2(dx, -dy) * 180 / Math.PI;
      if (ang < 0) ang += 360;
      var next = (Math.round(ang / 45) % 8) * 45;
      if (lockDeg !== null && Math.abs(((ang - lockDeg + 540) % 360) - 180) <= 22.5 + HYST) next = lockDeg;
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
    // The stick the pointer landed on, including its raised ball; else the station's half, without its caps.
    function isStickTouch(e) {
      if (opts.zone) return opts.zone.contains(e.target);
      if (base.getBoundingClientRect().width === 0) return false;
      var ball = base.querySelector('.stick-ball'), b = ball && ball.getBoundingClientRect();
      if (e.target === base || (e.target.closest && e.target.closest('.stick-base') === base) ||
          (b && e.clientX >= b.left && e.clientX <= b.right && e.clientY >= b.top && e.clientY <= b.bottom)) return true;
      if (opts.split) {
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
    function onDown(e) {
      if (opts.active && !opts.active()) return;
      if (activeId !== null) release();
      if (!isStickTouch(e)) return;
      e.preventDefault();
      activeId = e.pointerId;
      pivot = { x: e.clientX, y: e.clientY };
      window.addEventListener('pointermove', onMove, true);
      window.addEventListener('pointerup', onEnd, true);
      window.addEventListener('pointercancel', onEnd, true);
    }
    function onHidden() { if (document.hidden) release(); }
    window.addEventListener('pointerdown', onDown, true);
    window.addEventListener('blur', release);
    document.addEventListener('visibilitychange', onHidden);
    return function () {
      release();
      window.removeEventListener('pointerdown', onDown, true);
      window.removeEventListener('blur', release);
      document.removeEventListener('visibilitychange', onHidden);
    };
  }

  /** Lean a stick toward x, y (each -1, 0 or 1; a diagonal leans as far as a straight push). */
  function lean(stick, x, y) {
    if (x && y) { x *= 0.7071; y *= 0.7071; }
    stick.style.setProperty('--dx', x);
    stick.style.setProperty('--dy', y);
  }

  return { build: build, bindStick: bindStick, lean: lean };
}());

// Blip's pages: the game's deck, from BLIP_GAMES. A cabinet in the arcade's aisle shows its game's deck
// (machine-deck.html?game=<slug>).
(function () {
  var panel = document.querySelector('.deck-panel');
  // Another page that hosts the deck (no blip_core.js) builds it itself with BlipDeck.build.
  if (!panel || document.getElementById('deck-p1') || typeof blipGameFromPath !== 'function') return;
  var game = blipGameFromPath(window.BLIP_DECK_GAME ? '/' + window.BLIP_DECK_GAME + '/' : location.pathname);
  if (game && document.documentElement.hasAttribute('data-deck-preview')) document.documentElement.setAttribute('data-machine', game.slug);
  BlipDeck.build(panel, game ? { buttons: game.buttons || [{}], capLabel: game.capLabel, seats: game.players,
    touch: game.touch && blipHasTouch() ? game.touch : null } : {});
}());
