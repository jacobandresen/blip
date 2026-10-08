/* Shared control-deck builder; game pages configure it through BLIP_GAMES.
 * Each station has a stick or pad and at most two caps; unused caps are hidden.
 * Station two is inert unless the game supports two players. */
(function () {
  var panel = document.querySelector('.deck-panel');
  if (!panel || document.getElementById('deck-p1')) return;

  var game = (typeof blipGameFromPath === 'function')
    ? blipGameFromPath(window.location.pathname) : null;
  var buttons = (game && game.buttons) || [{ key: ' ', code: 'Space' }];
  // A one-button game still gets two live caps: A and B both fire.
  var specs = buttons.length < 2 ? [buttons[0], buttons[0]] : buttons;
  var seatsTwo = !!(game && game.players === 2);
  var CAPS = 4;       // the layout's name for this deck (data-caps); CSS keys off it
  var PER_PLAYER = 2; // caps a player ever gets: no game reads more than two

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
    for (var i = 0; i < PER_PLAYER; i++) fire.appendChild(stickCap(st, who, i));
    side.appendChild(base);
    side.appendChild(fire);
    frag.appendChild(side);
  });

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

  // ---- Pads ----
  STATIONS.forEach(function (st, who) {
    var pad = el('div', 'snes-pad' + (who ? ' second' : ''), '',
      { id: 'snes-pad' + st.id, 'aria-hidden': 'true' });
    var shell = el('div', 'snes-shell');
    var dpad = el('div', 'snes-dpad', crossBody(st.id || '-p1'));
    ['up', 'right', 'down', 'left'].forEach(function (d, k) {
      dpad.appendChild(el('span', 'snes-dp-seg dp-' + d, '',
        { 'data-blip': st.dirs[k], 'data-blip-nopress': '' }));
    });
    var face = el('div', 'snes-face four');
    for (var i = 0; i < PER_PLAYER; i++) face.appendChild(padCap(st, who, i));
    shell.appendChild(dpad);
    shell.appendChild(face);
    if (!who) shell.appendChild(el('span', 'snes-wordmark', 'BLIP', { 'aria-hidden': 'true' }));
    pad.appendChild(shell);
    frag.appendChild(pad);
  });

  panel.insertBefore(frag, panel.firstChild);
  panel.parentElement.prepend(el('div', 'deck-housing', '', { 'aria-hidden':'true' }));
  var book=el(document.getElementById('manual-overlay')?'button':'a','cabinet-manual',
    '<span class="manual-book-spine" aria-hidden="true"></span><span class="manual-book-copy"><strong>MANUAL</strong><small>BLIP · OPERATOR’S COPY</small></span><span class="manual-page-edges" aria-hidden="true"></span>',
    {id:'manual-book','data-page-card':'true','aria-label':'Lift the BLIP Arcade Manual'});
  if(book.tagName==='BUTTON')book.type='button';
  else {
    book.href=(game?'../':'')+'index.html?manual=controls'+(game?'&game='+game.slug:'');
    if(game){book.target='_blank';book.rel='noopener';book.title='Manual (opens in a new tab)';book.setAttribute('aria-label','Open manual in a new tab');}
  }
  var bookPocket=el('div','manual-pocket','');bookPocket.appendChild(book);document.body.appendChild(bookPocket);
  var coin=document.getElementById('insert-coin-btn')||document.getElementById('kiosk-insert-btn');
  if(coin) {
    try {root.toggleAttribute('data-credit',Number(sessionStorage.getItem('blip-coins'))>0);}catch(e){}
    var coinAnim=coin.querySelector('#coin-drop-anim');
    coin.replaceChildren();
    coin.classList.add('deck-coin-slot');
    coin.setAttribute('aria-label','Insert coin');coin.title='Insert coin';
    coin.innerHTML='<span class="coin-plate" aria-hidden="true"><span class="coin-instruction">INSERT COIN</span><i class="coin-mouth"></i><i class="coin-return"></i><i class="coin-fastener upper"></i><i class="coin-fastener lower"></i></span>';
    if(coinAnim)coin.appendChild(coinAnim);
    panel.parentElement.appendChild(coin);
    var lamp=el('span','deck-credit-lamp','P1',{'aria-label':'Player one credit'});
    var lamp2=el('span','deck-second-lamp','P2',{'aria-label':'Player two station'});
    var bar=panel.parentElement;panel.append(lamp,lamp2);
    var placementFrame=0;
    function placeLamp() {
      placementFrame=0;
      if(root.getAttribute('data-layout')!=='landscape') {lamp.style.left='';lamp2.style.left='';return;}
      var bounds=panel.getBoundingClientRect();
      function center(selector,fallback) {
        var part=bar.querySelector(selector),rect=part&&part.getBoundingClientRect();
        var x=rect&&rect.width>0?(rect.left+rect.right)/2:bounds.left+bounds.width*fallback;
        return Math.max(22,Math.min(bounds.width-22,x-bounds.left));
      }
      lamp.style.left=center('#stick-base .stick-boot,#snes-pad .snes-dpad,#paddle-dial,#touch-strip .ts-half[data-slot="0"]',.25)+'px';
      lamp2.style.left=center('#stick-base-p2 .stick-boot,#snes-pad-p2 .snes-dpad,#paddle-dial-p2,#touch-strip .ts-half[data-slot="1"]',.75)+'px';
    }
    function scheduleLamp(){if(!placementFrame)placementFrame=requestAnimationFrame(placeLamp);}
    window.addEventListener('resize',scheduleLamp);
    new MutationObserver(scheduleLamp).observe(root,{attributes:true,attributeFilter:['data-controls','data-players','data-layout','data-dials','data-touch','data-fullscreen','data-has-touch']});
    new MutationObserver(scheduleLamp).observe(bar,{childList:true,subtree:true});
    if(typeof ResizeObserver==='function')new ResizeObserver(scheduleLamp).observe(panel);
    scheduleLamp();
    coin.addEventListener('pointerdown',function(event){event.preventDefault();event.stopPropagation();});
    coin.addEventListener('click',function(){var canvas=document.getElementById('glcanvas');if(canvas)canvas.focus();});
  }
}());
