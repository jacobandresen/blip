/* blip_controller.js — one input abstraction for the whole web shell.
 *
 * Every control surface in BLIP funnels through here: the on-screen Super
 * Nintendo pad (the default), the classic arcade joystick + fire buttons,
 * a physical keyboard, and a physical gamepad. Consumers — shell.js on a
 * game page, index.html on the kiosk — call BlipController.init() once and
 * subscribe; they never care which surface is actually driving.
 *
 * Logical inputs (the shared vocabulary):
 *
 *   up / down / left / right   directional, held
 *   button1                    primary action  (fire / launch / serve — Space)
 *   button2                    secondary       (Meteors hyperspace     — Z)
 *   start                      edge — "start the selected game"
 *   select                     edge — "go to the game kiosk"
 *
 * The Super Nintendo pad's physical buttons fold onto that vocabulary:
 *
 *   D-pad            -> up / down / left / right
 *   B, A             -> button1
 *   Y, X, L, R       -> button2
 *   START            -> start
 *   SELECT           -> select
 *
 * button1 / button2 are dispatched as synthetic KeyboardEvents on the game
 * canvas (per-game key/code from BLIP_GAMES), so the WASM games need no
 * change — it is the exact path the old touch code used. When init() gets
 * no `canvas` (the kiosk landing page) nothing is injected; the consumer
 * wires the logical events straight to its own navigation.
 */
(function () {
  'use strict';

  var DIR = { up: 1, down: 1, left: 1, right: 1 };

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
  var held   = {};                // logical name -> bool
  var roots  = [];                // containers holding [data-blip] elements, for visual reflect

  /* ------------------------------------------------------------------ */
  /* Key map — arrows are fixed, the two action buttons come per-game     */
  /* ------------------------------------------------------------------ */

  function buildKeymap(buttons) {
    var b0 = (buttons && buttons[0]) || { key: ' ', code: 'Space' };
    var b1 = (buttons && buttons[1]) || b0;
    return {
      up:      { key: 'ArrowUp',    code: 'ArrowUp' },
      down:    { key: 'ArrowDown',  code: 'ArrowDown' },
      left:    { key: 'ArrowLeft',  code: 'ArrowLeft' },
      right:   { key: 'ArrowRight', code: 'ArrowRight' },
      button1: { key: b0.key, code: b0.code },
      button2: { key: b1.key, code: b1.code }
    };
  }

  function injectKey(spec, type) {
    if (!cfg.canvas || !spec) return;
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
    if (cfg.gate && cfg.gate()) return;
    injectKey(spec, type);
  }

  /* ------------------------------------------------------------------ */
  /* Click sounds — a soft plastic "clack" per press                     */
  /* ------------------------------------------------------------------ */

  var _actx = null;
  function actx() {
    if (typeof Howler !== 'undefined' && Howler.ctx) {
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
    var len = Math.ceil(ctx.sampleRate * spec.nd);
    var buf = ctx.createBuffer(1, len, ctx.sampleRate);
    var d = buf.getChannelData(0);
    for (var i = 0; i < len; i++) d[i] = (Math.random() * 2 - 1) * (1 - i / len);
    var src = ctx.createBufferSource(); src.buffer = buf;
    var hp = ctx.createBiquadFilter(); hp.type = 'highpass'; hp.frequency.value = spec.hp;
    var ng = ctx.createGain();
    ng.gain.setValueAtTime(spec.ng, t);
    ng.gain.exponentialRampToValueAtTime(0.0006, t + spec.nd);
    src.connect(hp); hp.connect(ng); ng.connect(ctx.destination);
    src.start(t); src.stop(t + spec.nd + 0.01);

    // The muted thock of the cap bottoming out.
    var osc = ctx.createOscillator(), og = ctx.createGain();
    osc.type = 'sine'; osc.frequency.setValueAtTime(spec.thk, t);
    osc.frequency.exponentialRampToValueAtTime(spec.thk * 0.6, t + spec.td);
    og.gain.setValueAtTime(spec.tg, t);
    og.gain.exponentialRampToValueAtTime(0.0006, t + spec.td);
    osc.connect(og); og.connect(ctx.destination);
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
    Object.keys(held).forEach(function (n) { if (held[n]) set(n, false); });
  }

  /* ------------------------------------------------------------------ */
  /* Binders — connect a source surface to set()                         */
  /* ------------------------------------------------------------------ */

  // Any container of elements carrying data-blip="<logical name>" (may list
  // several, space-separated). Discrete press / release, with the pointer
  // captured so a slide off the cap still releases cleanly. A d-pad cross
  // is bound with bindDpad() instead (mark its hub data-blip="dpad").
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
      var pid = null;
      el.addEventListener('pointerdown', function (e) {
        e.preventDefault();
        pid = e.pointerId;
        try { el.setPointerCapture(pid); } catch (x) {}
        if (kind) { click(kind); names.forEach(function (n) { set(n, true, { silentClick: true }); }); }
        else      { names.forEach(function (n) { set(n, true); }); }
      });
      function up(e) {
        if (pid !== null && e.pointerId !== pid) return;
        pid = null;
        names.forEach(function (n) { set(n, false); });
      }
      el.addEventListener('pointerup', up);
      el.addEventListener('pointercancel', up);
    })(els[i]);
  }

  // The d-pad cross: one floating pivot at the centre of `pad`, thumb
  // position resolved to a simple 4-way cross with a dead centre — press
  // toward a corner and both axes engage (a real diagonal), the classic
  // "roll" you can sweep around the rim.
  function bindDpad(pad, o) {
    if (!pad) return;
    o = o || {};
    var dead = o.dead != null ? o.dead : 0.32;
    var pid = null;
    // Light the matching d-pad arm (.dp-up … .dp-right inside `pad`) the
    // instant a direction engages — a local, precise reflection that
    // doesn't depend on the pad being a registered [data-blip] root.
    var segs = {};
    ['up', 'down', 'left', 'right'].forEach(function (n) {
      segs[n] = pad.querySelector('.dp-' + n);
    });
    function put(n, on) {
      set(n, on);
      if (segs[n]) segs[n].classList.toggle('blip-pressed', on);
    }
    function calc(e) {
      var r = pad.getBoundingClientRect();
      var nx = (e.clientX - (r.left + r.width  / 2)) / (r.width  / 2);
      var ny = (e.clientY - (r.top  + r.height / 2)) / (r.height / 2);
      put('left',  nx < -dead);
      put('right', nx >  dead);
      put('up',    ny < -dead);
      put('down',  ny >  dead);
    }
    function clear() {
      ['up', 'down', 'left', 'right'].forEach(function (n) { put(n, false); });
      pad.classList.remove('blip-touched');
    }
    pad.addEventListener('pointerdown', function (e) {
      e.preventDefault();
      pid = e.pointerId;
      try { pad.setPointerCapture(pid); } catch (x) {}
      // whole cross glows while a thumb is on it — so an engaged arm never
      // looks like it's floating away from a dark centre
      pad.classList.add('blip-touched');
      calc(e);
    });
    pad.addEventListener('pointermove', function (e) {
      if (e.pointerId !== pid) return;
      e.preventDefault();
      calc(e);
    });
    function end(e) {
      if (e.type !== 'pointercancel' && e.pointerId !== pid) return;
      pid = null;
      clear();
    }
    pad.addEventListener('pointerup', end);
    pad.addEventListener('pointercancel', end);
  }

  // The rally rotary dial: a knob you spin with a thumb. Each frame's
  // angular delta past a small dead band drives an up / down key (the
  // paddle it's wired to), and a spin that barely moved is treated as a
  // tap (mode select on the title screen). The knob's *visible* rotation
  // is not set here — rally drives that from the game via window.blipPaddles
  // so it tracks the real paddle, not the raw gesture.
  //
  //   opts.up / opts.down / opts.tap : { key, code } specs to inject
  //   opts.onTap / opts.onInteract   : callbacks (title-screen mode select)
  //   opts.dead                      : rad/frame dead band (default 0.018)
  function bindDial(dialEl, opts2) {
    if (!dialEl) return;
    opts2 = opts2 || {};
    var dead = opts2.dead != null ? opts2.dead : 0.018;
    var last = null, pid = null, total = 0, upHeld = false, downHeld = false;

    function angleAt(x, y) {
      var r = dialEl.getBoundingClientRect();
      return Math.atan2(y - (r.top + r.height / 2), x - (r.left + r.width / 2));
    }
    function dir(u, d) {
      if (u !== upHeld)   { upHeld = u;   if (u) click('dpad'); emitKey(opts2.up,   u ? 'keydown' : 'keyup'); }
      if (d !== downHeld) { downHeld = d; if (d) click('dpad'); emitKey(opts2.down, d ? 'keydown' : 'keyup'); }
    }
    function stop() { dir(false, false); dialEl.classList.remove('active'); last = null; pid = null; }

    dialEl.addEventListener('pointerdown', function (e) {
      e.preventDefault();
      if (pid !== null) return;
      if (opts2.onInteract) opts2.onInteract();
      pid = e.pointerId;
      try { dialEl.setPointerCapture(pid); } catch (x) {}
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
      if      (d >  dead) dir(false, true);
      else if (d < -dead) dir(true, false);
      else                dir(false, false);
    });
    function end(e) {
      if (e.type !== 'pointercancel' && e.pointerId !== pid) return;
      if (total < 0.08) {
        if (opts2.onTap) opts2.onTap();
        if (opts2.tap) { click('menu'); emitKey(opts2.tap, 'keydown'); emitKey(opts2.tap, 'keyup'); }
      }
      stop();
    }
    dialEl.addEventListener('pointerup', end);
    dialEl.addEventListener('pointercancel', end);
  }

  // Physical keyboard — reflection only. The real keypress already reaches
  // the game (macroquad reads it straight); this just leans the on-screen
  // stick / lights the on-screen pad so the deck doubles as an input
  // read-out for a keyboard player.
  var KEY_NAME = {
    ArrowUp: 'up', KeyW: 'up', ArrowDown: 'down', KeyS: 'down',
    ArrowLeft: 'left', KeyA: 'left', ArrowRight: 'right', KeyD: 'right',
    Space: 'button1', KeyZ: 'button2'
  };
  function bindKeyboard() {
    document.addEventListener('keydown', function (e) {
      var n = KEY_NAME[e.code]; if (n) set(n, true, { silent: true });
    });
    document.addEventListener('keyup', function (e) {
      var n = KEY_NAME[e.code]; if (n) set(n, false, { silent: true });
    });
  }

  // Physical gamepad — kiosk.js's pollGamepad() reports logical codes
  // (ArrowUp… / Space / KeyZ / Start / Select); fold them onto our names
  // and drive the game for real (not silent).
  var GP_NAME = {
    ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right',
    Space: 'button1', KeyZ: 'button2', Start: 'start', Select: 'select'
  };
  function bindGamepad(poll) {
    if (typeof poll !== 'function') return;
    poll(
      function (code) { var n = GP_NAME[code]; if (n) set(n, true); },
      function (code) { var n = GP_NAME[code]; if (n) set(n, false); }
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
      keymap = buildKeymap(opts.buttons);
      return this;
    },
    set: set,
    press:   function (n) { set(n, true); },
    release: function (n) { set(n, false); },
    isHeld:  function (n) { return !!held[n]; },
    releaseAll: releaseAll,
    reflect: reflect,
    registerVisual: function (root) { if (root && roots.indexOf(root) === -1) roots.push(root); },
    bindButtons: bindButtons,
    bindDpad: bindDpad,
    bindDial: bindDial,
    bindKeyboard: bindKeyboard,
    bindGamepad: bindGamepad,
    click: click
  };
}());
