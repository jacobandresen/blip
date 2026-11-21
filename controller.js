/* BlipController: the game page's input layer. The <blip-deck>'s presses, Rally's dials, the keyboard and a gamepad
 * all set logical names (up … right, button1 … button4, p2up … p2button4, start, select); a held name becomes a key
 * event on the game's canvas. The deck lights itself; input from elsewhere is mirrored on it with deck.press(). */
(function () {
  var DIR = { up: 1, down: 1, left: 1, right: 1, p2up: 1, p2down: 1, p2left: 1, p2right: 1 };
  var ARROW_NAME = { ArrowUp: 'up', KeyW: 'up', ArrowDown: 'down', KeyS: 'down', ArrowLeft: 'left', KeyA: 'left',
    ArrowRight: 'right', KeyD: 'right' };
  var GP_NAME = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right', Space: 'button1',
    KeyZ: 'button2', Start: 'start', Select: 'select' };

  var cfg = { canvas: null, gate: null, onInput: null, onStart: null, onSelect: null, feedback: null, click: null };
  var keymap = null, held = {}, injected = {}, decks = [], dials = [];

  function buildKeymap(buttons, extra) {
    var list = buttons && buttons.length ? buttons : [{ key: ' ', code: 'Space' }], map = {};
    ['up', 'down', 'left', 'right'].forEach(function (n) {
      var key = 'Arrow' + n[0].toUpperCase() + n.slice(1);
      map[n] = { key: key, code: key };
    });
    list.forEach(function (b, i) { map['button' + (i + 1)] = { key: b.key, code: b.code }; });
    if (!map.button2) map.button2 = map.button1; // a one-button game: a gamepad's second button fires too
    for (var n in extra) if (extra[n]) map[n] = extra[n];
    return map;
  }

  // What a game declares wins over the arrows: where the arrows are player two's, ArrowUp lights player two's stick.
  function nameForCode(code) {
    for (var n in keymap) if (keymap[n].code === code) return n;
    return ARROW_NAME[code] || null;
  }

  // Two sources can hold one key (a deck button and a gamepad): it goes up when the last lets go.
  function injectKey(spec, type) {
    if (!cfg.canvas || !spec) return;
    var count = injected[spec.code] || 0;
    if (type === 'keydown') {
      injected[spec.code] = count + 1;
      if (count) return;
    } else if (count > 1) {
      injected[spec.code] = count - 1;
      return;
    } else delete injected[spec.code];
    cfg.canvas.dispatchEvent(new KeyboardEvent(type, { bubbles: true, cancelable: true, key: spec.key, code: spec.code }));
  }

  // select always leaves; everything else waits while the gate is shut (the coin wall).
  function swallowed(name) { return name !== 'select' && !!(cfg.gate && cfg.gate()); }
  function emitKey(spec, type) { if (type !== 'keydown' || !swallowed()) injectKey(spec, type); }
  // A consumer's sound or haptic that throws (no audio) never stops input.
  function click(kind) { try { if (cfg.click) cfg.click(kind); } catch (e) {} }
  function tick() { try { if (cfg.feedback) cfg.feedback(); } catch (e) {} }

  /** Hold or let go of `name`. opts.silent: only light the deck (the real key already reached the game);
   *  opts.silentClick: no press sound; opts.fromDeck: the deck lit it itself. */
  function set(name, down, opts) {
    opts = opts || {};
    down = !!down;
    var edge = name === 'start' || name === 'select';
    if (!keymap || (!edge && !keymap[name]) || down === !!held[name]) return;
    held[name] = down;
    if (!opts.fromDeck) decks.forEach(function (deck) { deck.press(name, down); });
    try { if (cfg.onInput) cfg.onInput(name, down); } catch (e) {}
    if (swallowed(name)) return;
    if (edge) {
      if (!down) return;
      if (!opts.silent) { click('menu'); tick(); }
      var action = name === 'start' ? cfg.onStart : cfg.onSelect;
      if (action) action();
      return;
    }
    if (opts.silent) return;
    injectKey(keymap[name], down ? 'keydown' : 'keyup');
    if (down) { if (!opts.silentClick) click(DIR[name] ? 'dpad' : 'face'); tick(); }
  }

  function releaseAll() {
    decks.forEach(function (deck) { deck.release(); });
    dials.forEach(function (stop) { stop(); });
    for (var n in held) if (held[n]) set(n, false);
  }

  /** The deck's presses become key events. */
  function bindDeck(deck) {
    if (!deck || decks.indexOf(deck) >= 0) return;
    decks.push(deck);
    deck.addEventListener('blip-input', function (e) { set(e.detail.name, e.detail.down, { fromDeck: true }); });
  }

  // A spinner (Rally's paddle): turning past `dead` radians holds o.up or o.down, until the turn stops for idleMs or
  // reverses; a press that hardly turns is a tap (o.tap key, o.onTap). o.onInteract fires on every grab.
  function bindDial(dial, o) {
    if (!dial) return;
    o = o || {};
    var dead = o.dead == null ? 0.018 : o.dead, idleMs = o.idleMs == null ? 70 : o.idleMs;
    var last = null, pid = null, total = 0, acc = 0, idle = null, rect = null, hold = { up: false, down: false };
    function angleAt(x, y) { return Math.atan2(y - (rect.top + rect.height / 2), x - (rect.left + rect.width / 2)); }
    function dir(up, down) {
      [['up', up], ['down', down]].forEach(function (p) {
        if (p[1] === hold[p[0]]) return;
        hold[p[0]] = p[1];
        if (p[1]) click('dpad');
        emitKey(o[p[0]], p[1] ? 'keydown' : 'keyup');
      });
    }
    function clearIdle() { clearTimeout(idle); idle = null; }
    function stop() { clearIdle(); dir(false, false); dial.classList.remove('active'); last = pid = rect = null; acc = 0; }
    dials.push(stop);
    dial.addEventListener('pointerdown', function (e) {
      e.preventDefault();
      if (pid !== null) return;
      if (o.onInteract) o.onInteract();
      pid = e.pointerId;
      try { dial.setPointerCapture(pid); } catch (err) {}
      rect = dial.getBoundingClientRect();
      last = angleAt(e.clientX, e.clientY);
      total = 0;
      dial.classList.add('active');
    });
    dial.addEventListener('pointermove', function (e) {
      if (e.pointerId !== pid) return;
      e.preventDefault();
      var a = angleAt(e.clientX, e.clientY), d = ((a - last + 3 * Math.PI) % (2 * Math.PI)) - Math.PI;
      last = a;
      total += Math.abs(d);
      // the turn accumulates, so a slow, careful one still crosses the dead band
      acc += d;
      if (acc > dead) { acc = 0; dir(false, true); } else if (acc < -dead) { acc = 0; dir(true, false); }
      clearIdle();
      idle = setTimeout(function () { idle = null; acc = 0; dir(false, false); }, idleMs);
    });
    function end(e) {
      if (e.type !== 'pointercancel' && e.pointerId !== pid) return;
      if (e.type === 'pointerup' && total < 0.08) {
        if (o.onTap) o.onTap();
        if (o.tap) { click('menu'); emitKey(o.tap, 'keydown'); emitKey(o.tap, 'keyup'); }
      }
      stop();
    }
    dial.addEventListener('pointerup', end);
    dial.addEventListener('pointercancel', end);
    dial.addEventListener('lostpointercapture', function (e) { if (e.pointerId === pid) stop(); });
  }

  // The physical keyboard already reaches the game: this only lights the deck to match.
  function bindKeyboard() {
    [['keydown', true], ['keyup', false]].forEach(function (p) {
      document.addEventListener(p[0], function (e) { var n = nameForCode(e.code); if (n) set(n, p[1], { silent: true }); });
    });
  }

  // poll(press, release) reports gamepad codes (GP_NAME); they drive player one.
  function bindGamepad(poll) {
    if (typeof poll !== 'function') return;
    function to(down) { return function (code) { if (GP_NAME[code]) set(GP_NAME[code], down); }; }
    poll(to(true), to(false));
  }

  /** opts: canvas (gets the key events), buttons [{ key, code }], keys { name: { key, code } } (more names, e.g.
   *  p2up), gate() (true holds input back), onInput(name, down), onStart(), onSelect(), feedback() (a haptic tick on
   *  each press), click(kind) (a press sound: 'face', 'dpad' or 'menu'). */
  function init(opts) {
    opts = opts || {};
    for (var k in cfg) if (opts[k] !== undefined) cfg[k] = opts[k];
    keymap = buildKeymap(opts.buttons, opts.keys);
    return window.BlipController;
  }

  window.BlipController = { init: init, set: set, releaseAll: releaseAll, bindDeck: bindDeck, bindDial: bindDial,
    bindKeyboard: bindKeyboard, bindGamepad: bindGamepad };
}());
