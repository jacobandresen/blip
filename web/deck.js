/* The control deck: one builder for every page (landing, info pages and
 * games), so the bar is the same equipment everywhere. Pages carry an empty
 * <div class="deck-panel"> and load this after the bar; a game page gets its
 * caps, labels and player count from BLIP_GAMES, the others get the default
 * two-cap deck. Wiring (touch, keyboard, card navigation) stays with the page.
 *
 * Every deck has two stations: an arcade stick with four caps, and a pad
 * (cross + four caps). A cap the game does not use is `spare` and inert;
 * station two is inert unless the game seats two players. */
(function () {
  var panel = document.querySelector('.deck-panel');
  if (!panel || document.getElementById('deck-p1')) return;

  var game = (typeof blipGameFromPath === 'function')
    ? blipGameFromPath(window.location.pathname) : null;
  var buttons = (game && game.buttons) || [{ key: ' ', code: 'Space' }];
  // A one-button game still gets two live caps: A and B both fire.
  var specs = buttons.length < 2 ? [buttons[0], buttons[0]] : buttons;
  var seatsTwo = !!(game && game.players === 2);
  var CAPS = 4;

  var root = document.documentElement;
  root.setAttribute('data-caps', String(CAPS));
  root.setAttribute('data-players', '1');
  if (!seatsTwo) root.setAttribute('data-seats', '1');

  function el(tag, cls, html, attrs) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (html) e.innerHTML = html;
    for (var k in (attrs || {})) e.setAttribute(k, attrs[k]);
    return e;
  }

  // Logical input names per station (BlipController's vocabulary).
  var STATIONS = [
    { id: '',   dirs: ['up', 'right', 'down', 'left'],
      cap: function (i) { return 'button' + (i + 1); } },
    { id: '-p2', dirs: ['p2up', 'p2right', 'p2down', 'p2left'],
      cap: function (i) { return 'p2button' + (i + 1); } }
  ];

  // Live caps for a station: a game's own specs (station two only when it
  // seats two); elsewhere the deck is cabinet flavour and A / B are "fire".
  function specFor(who, i) {
    if (who === 1 && !seatsTwo) return null;
    return specs[i] || null;
  }
  function capName(st, who, i) {
    if (game) return st.cap(i);
    return who === 0 && i < 2 ? 'button1' : null;
  }

  // Caps carry no lettering (a label is kept as the accessible name only).
  function stickCap(st, who, i) {
    var spec = specFor(who, i), name = spec && capName(st, who, i);
    var b = el('div', 'arcade-btn lettered' + (spec ? '' : ' spare'),
      '<span class="arcade-btn-cap"></span>');
    if (name && game) b.setAttribute('data-blip', name);
    if (spec && spec.label) b.setAttribute('aria-label', spec.label);
    if (buttons.length === 2 && i === 1) b.classList.add('alt');
    return b;
  }
  function padCap(st, who, i) {
    var spec = specFor(who, i), name = spec && capName(st, who, i);
    var b = el('button', 'snes-btn cap' + (i + 1) + (spec ? '' : ' spare'),
      '<span></span>', { type: 'button' });
    if (name) b.setAttribute('data-blip', name);
    if (spec && spec.label) b.setAttribute('aria-label', spec.label);
    // A game with two different actions (Meteors: fire / hyperspace) gets a
    // blue second cap; a one-button game's two caps are both fire.
    if (buttons.length === 2 && i === 1) b.classList.add('alt');
    return b;
  }

  var frag = document.createDocumentFragment();

  // ---- Arcade stick stations ----
  STATIONS.forEach(function (st, who) {
    var side = el('div', 'deck-side', '', { id: 'deck-p' + (who + 1) });
    var base = el('div', 'stick-base', '', { id: 'stick-base' + st.id });
    base.appendChild(el('div', 'stick-boot'));
    var handle = el('div', 'stick-handle');
    handle.appendChild(el('div', 'stick-shaft'));
    handle.appendChild(el('div', 'stick-ball'));
    base.appendChild(handle);
    var fire = el('div', 'fire-buttons four', '', { id: 'fire-buttons' + st.id });
    for (var i = 0; i < CAPS; i++) fire.appendChild(stickCap(st, who, i));
    side.appendChild(base);
    side.appendChild(fire);
    frag.appendChild(side);
  });

  // ---- Pads ----
  STATIONS.forEach(function (st, who) {
    var pad = el('div', 'snes-pad' + (who ? ' second' : ''), '',
      { id: 'snes-pad' + st.id, 'aria-hidden': 'true' });
    var shell = el('div', 'snes-shell');
    var dpad = el('div', 'snes-dpad');
    ['up', 'right', 'down', 'left'].forEach(function (d, k) {
      dpad.appendChild(el('span', 'snes-dp-seg dp-' + d, '',
        { 'data-blip': st.dirs[k], 'data-blip-nopress': '' }));
    });
    dpad.appendChild(el('span', 'snes-dp-hub'));
    var face = el('div', 'snes-face four');
    for (var i = 0; i < CAPS; i++) face.appendChild(padCap(st, who, i));
    shell.appendChild(dpad);
    shell.appendChild(face);
    if (!who) shell.appendChild(el('span', 'snes-wordmark', 'BLIP', { 'aria-hidden': 'true' }));
    pad.appendChild(shell);
    frag.appendChild(pad);
  });

  panel.insertBefore(frag, panel.firstChild);
}());
