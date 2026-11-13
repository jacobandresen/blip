/* The arcade room (index.html): the manual book, then the aisle of cabinets, coins and launching a game. */
blipManualPocket();
(function () {
  function copyFace(page) {
    var copy = page.cloneNode(true);
    copy.removeAttribute('id');
    // The copy covers its whole leaf: a page sized by its grid shrinks to its text out of it, and the leaf shows through.
    copy.style.cssText = 'position:absolute;inset:0;margin:0;grid-area:auto;height:auto;';
    copy.querySelectorAll('[id]').forEach(function (node) { node.removeAttribute('id'); });
    return copy;
  }

  // k undoes a transform: scale() on the book, whose rects are scaled but whose children are not.
  function place(node, box, origin, k) {
    node.style.cssText = 'position:absolute;margin:0;left:' + (box.left - origin.left) * k + 'px;top:' + (box.top - origin.top) * k + 'px;width:' + box.width * k + 'px;height:' + box.height * k + 'px;';
  }

  function turn(options) {
    var book = options.book, forward = options.direction > 0;
    var lifted = options.lifted, landing = options.landing;
    var frame = book.getBoundingClientRect(), liftedBox = lifted.getBoundingClientRect(), landingBox = landing.getBoundingClientRect();
    var k = frame.width ? book.offsetWidth / frame.width : 1;
    if (Math.abs(k - 1) < .01) k = 1;
    var cover = document.createElement('div');
    cover.className = 'manual-flip-cover';
    place(cover, landingBox, frame, k);
    cover.appendChild(copyFace(landing));
    var leaf = document.createElement('div');
    leaf.className = 'manual-flip';
    place(leaf, liftedBox, frame, k);
    leaf.style.transformOrigin = forward ? '-8px 50%' : 'calc(100% + 8px) 50%';
    var front = document.createElement('div'), back = document.createElement('div');
    front.className = 'manual-flip-face manual-flip-front';
    back.className = 'manual-flip-face manual-flip-back';
    front.appendChild(copyFace(lifted));
    if (options.update) options.update();
    back.appendChild(copyFace(options.landingAfter || landing));
    var frontShade = document.createElement('i'), backShade = document.createElement('i');
    frontShade.className = backShade.className = 'manual-flip-shade';
    front.appendChild(frontShade); back.appendChild(backShade);
    leaf.append(front, back); book.append(cover, leaf);
    var sign = forward ? -1 : 1, style = options.style;
    var cast = document.createElement('i');
    cast.className = 'manual-flip-cast ' + (forward ? 'manual-flip-cast-left' : 'manual-flip-cast-right');
    cover.appendChild(cast);
    var animation = leaf.animate(style.frames(sign), { duration: style.ms, fill: 'forwards' });
    frontShade.animate([{ opacity: 0 }, { opacity: style.shade }], { duration: style.ms, easing: 'ease-in', fill: 'forwards' });
    backShade.animate([{ opacity: style.shade }, { opacity: 0 }], { duration: style.ms, easing: 'ease-out', fill: 'forwards' });
    cast.animate([{ opacity: 0 }, { opacity: style.cast, offset: .55 }, { opacity: 0 }], { duration: style.ms, easing: 'ease-in-out', fill: 'forwards' });
    var finish = function () { leaf.remove(); cover.remove(); if (options.onFinish) options.onFinish(); };
    animation.onfinish = finish;
    animation.oncancel = finish;
    return animation;
  }

  window.BlipBook = { turn: turn };
}());

(function () {
  var moveSimple = async function (overlay, returning) {
    var source = document.getElementById('manual-book');
    var spread = overlay.querySelector('.manual-open-book');
    overlay.inert = true;
    source.classList.add('manual-away');
    if (matchMedia('(prefers-reduced-motion: reduce)').matches) {
      source.classList.toggle('manual-away', !returning);
      overlay.inert = false;
      return;
    }

    var slot = source.getBoundingClientRect();
    var spreadRect = spread.getBoundingClientRect();
    var width = spreadRect.width / 2, height = spreadRect.height;
    var center = { left: spreadRect.left + width / 2, top: spreadRect.top };
    var book = source.cloneNode(true);
    book.removeAttribute('id');
    book.removeAttribute('data-page-card');
    book.className = 'cabinet-manual manual-transit';
    book.setAttribute('aria-hidden', 'true');
    book.tabIndex = -1;
    document.body.appendChild(book);

    var seated = { left: slot.left + 'px', top: slot.top + 'px', width: slot.width + 'px', height: slot.height + 'px',
      fontSize: getComputedStyle(source.querySelector('strong')).fontSize,
      transform: 'perspective(900px) rotateX(0deg) rotate(0deg)', opacity: 1,
      boxShadow: '0 2px 2px #000c, inset 0 1px #b9b38c33' };
    var lifted = Object.assign({}, seated, { top: slot.top - 11 + 'px', transform: 'perspective(900px) rotateX(12deg) rotate(-3deg)',
      boxShadow: '0 8px 8px #0009, inset 0 1px #b9b38c55' });
    var held = { left: center.left + 'px', top: center.top + 'px', width: width + 'px', height: height + 'px',
      fontSize: '20px', transform: 'perspective(900px) rotateX(0deg) rotate(0deg)', opacity: 1,
      boxShadow: '0 24px 35px #0009, inset 0 1px #b9b38c55' };
    var expanded = Object.assign({}, held, { left: spreadRect.left + 'px', width: spreadRect.width + 'px' });
    var animations = [];
    try {
      if (returning) {
        animations.push(spread.animate([
          { opacity: 1, transform: 'scale(1) rotateX(0deg)' },
          { opacity: 0, transform: 'scaleX(.5) rotateX(0deg)' }
        ], { duration: 650, easing: 'ease-in-out', fill: 'both' }));
        animations.push(overlay.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 1100, delay: 500, fill: 'both' }));
        var settled = Object.assign({}, seated, { top: slot.top + 1 + 'px' });
        await book.animate([
          Object.assign({}, expanded, { opacity: 0, offset: 0, easing: 'ease-in-out' }),
          Object.assign({}, held, { offset: 650 / 1850, easing: 'cubic-bezier(.4,0,.25,1)' }),
          Object.assign({}, lifted, { offset: .88, easing: 'ease-in-out' }),
          Object.assign({}, settled, { offset: .97 }),
          Object.assign({}, seated, { offset: 1 })
        ], { duration: 1850, fill: 'forwards' }).finished;
      } else {
        animations.push(spread.animate([{ opacity: 0 }, { opacity: 0 }], { duration: 1100, fill: 'both' }));
        animations.push(overlay.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 1000, delay: 150, fill: 'both' }));
        await book.animate([
          Object.assign({}, seated, { offset: 0, easing: 'ease-in-out' }),
          Object.assign({}, lifted, { offset: .2, easing: 'cubic-bezier(.3,0,.3,1)' }),
          Object.assign({}, held, { offset: 1 })
        ], { duration: 1100, fill: 'forwards' }).finished;
        animations[0].cancel();
        var reveal = spread.animate([
          { opacity: 0, transform: 'scaleX(.5) rotateX(0deg)' },
          { opacity: 1, transform: 'scale(1) rotateX(0deg)' }
        ], { duration: 650, easing: 'ease-in-out', fill: 'both' });
        animations.push(reveal);
        animations.push(book.animate([held, Object.assign({}, expanded, { opacity: 0 })],
          { duration: 650, easing: 'ease-in-out', fill: 'both' }));
        await reveal.finished;
      }
    } finally {
      animations.forEach(function (animation) { animation.cancel(); });
      book.remove();
      source.classList.toggle('manual-away', !returning);
      overlay.inert = false;
    }
  };

  // On a wide screen the book is a book: it leaves its pocket, lands closed over the right-hand page and its cover swings open about the spine
  // (the inside of the cover is the left-hand page); putting it away is the same in reverse.
  function strip(node) {
    node.removeAttribute('id');
    node.querySelectorAll('[id]').forEach(function (n) { n.removeAttribute('id'); });
    return node;
  }
  var moveWide = async function (overlay, returning) {
    var source = document.getElementById('manual-book');
    var spread = overlay.querySelector('.manual-open-book');
    var leftPage = overlay.querySelector('.manual-open-left'), rightPage = overlay.querySelector('.manual-open-right');
    overlay.inert = true;
    source.classList.add('manual-away');
    var slot = source.getBoundingClientRect();
    var frame = spread.getBoundingClientRect(), box = rightPage.getBoundingClientRect();
    // The closed book has its printed cover's proportions (96 × 140), its spine at the spread's.
    var W = box.width, H = box.height, CW = Math.min(W, H * 96 / 140), hx = box.left + CW / 2, hy = box.top + H / 2;
    var sx = slot.left + slot.width / 2, sy = slot.top + slot.height / 2, sc = slot.width / CW;
    // Seen in its slot the book lies tipped far back: its foreshortened height is the slot's.
    var tip = Math.acos(Math.min(1, slot.height / (H * sc))) * 180 / Math.PI;
    function pose(x, y, s, rx, rz) {
      return 'translate(' + (x - hx) + 'px,' + (y - hy) + 'px) perspective(1800px) rotateX(' + rx + 'deg) rotateZ(' + rz + 'deg) scale(' + s + ')';
    }
    // Picked up: drawn up out of the slot by its top edge, tipped toward the reader as it comes closer,
    // held a little above the table, then set down; the shadow follows the height.
    var path = [
      { offset: 0, transform: pose(sx, sy, sc, tip, 0), opacity: 0, boxShadow: '0 1px 2px #0000' },
      { offset: .12, transform: pose(sx, sy - 18, sc * 1.06, tip - 8, -2), opacity: 1, boxShadow: '0 4px 6px #0006', easing: 'cubic-bezier(.33,0,.2,1)' },
      { offset: .5, transform: pose(sx + (hx - sx) * .6, Math.max(sy + (hy - sy) * .4, 14 + H * .58 * .45), .58, 26, -5), opacity: 1, boxShadow: '0 40px 60px #000a', easing: 'cubic-bezier(.3,.1,.3,1)' },
      { offset: .84, transform: pose(hx, hy - 12, 1.035, -3, .6), opacity: 1, boxShadow: '0 46px 70px #000b', easing: 'cubic-bezier(.45,0,.55,1)' },
      { offset: 1, transform: pose(hx, hy, 1, 0, 0), opacity: 1, boxShadow: '0 6px 14px #0009' }
    ];
    var book = document.createElement('div');
    book.className = 'manual-transit';
    book.setAttribute('aria-hidden', 'true');
    book.style.cssText = 'left:' + box.left + 'px;top:' + box.top + 'px;width:' + CW + 'px;height:' + H + 'px;transform-origin:50% 50%;opacity:0';
    // The spine in the pocket rises as the book is drawn out, and gives way to the book itself.
    var spine = source.cloneNode(true);
    spine.removeAttribute('id'); spine.removeAttribute('data-page-card');
    spine.className = 'cabinet-manual manual-lift';
    spine.setAttribute('aria-hidden', 'true'); spine.tabIndex = -1;
    spine.style.cssText = 'left:' + slot.left + 'px;top:' + slot.top + 'px;width:' + slot.width + 'px;height:' + slot.height + 'px';
    var spineRise = [
      { transform: 'translateY(0) perspective(400px) rotateX(0deg)', opacity: 1 },
      { transform: 'translateY(-12px) perspective(400px) rotateX(-24deg)', opacity: 1, offset: .55 },
      { transform: 'translateY(-20px) perspective(400px) rotateX(-40deg)', opacity: 0 }
    ];
    // The cover swings on its own layer over the spread, so the spread can be cut back to what the open
    // cover uncovers; until then a closed book has no left-hand page.
    var stage = document.createElement('div');
    var view = getComputedStyle(spread);
    stage.style.cssText = 'position:fixed;left:' + frame.left + 'px;top:' + frame.top + 'px;width:' + frame.width + 'px;height:' + frame.height +
      'px;z-index:12;pointer-events:none;perspective:' + view.perspective + ';perspective-origin:' + view.perspectiveOrigin;
    overlay.appendChild(stage);
    var spineX = box.left - frame.left - 8, P = parseFloat(view.perspective) || 2600;
    function uncovered(angle) {
      var m = Math.abs(angle) * Math.PI / 180;
      if (m <= Math.PI / 2) return spineX;
      return Math.max(0, spineX - W * -Math.cos(m) * P / Math.max(1, P - W * Math.sin(m)));
    }
    function clip(left) { return 'inset(-80px -80px -80px ' + left + 'px)'; }
    var animations = [];
    var ease = function (t) { return t < .5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2; };

    function hinge(closedAngle) {
      var back = strip(leftPage.cloneNode(true));
      back.querySelectorAll('.manual-tear').forEach(function (n) { n.remove(); });
      var cover = document.createElement('div');
      cover.className = 'manual-cover';
      cover.style.cssText = 'position:absolute;left:' + (box.left - frame.left) + 'px;top:' + (box.top - frame.top) + 'px;width:' + W +
        'px;height:' + H + 'px;transform-origin:-8px 50%;transform-style:preserve-3d;pointer-events:none;will-change:transform;transform:rotateY(' + closedAngle + 'deg)';
      var front = document.createElement('div'), rear = document.createElement('div');
      front.className = 'manual-cover-face manual-cover-front'; rear.className = 'manual-cover-face manual-cover-back';
      front.style.right = 'auto'; front.style.width = CW + 'px';
      cover.style.setProperty('--cw', CW + 'px');
      var closed = document.createElement('div');
      closed.className = 'manual-cover-book';
      closed.style.cssText = 'position:absolute;inset:0';
      front.appendChild(closed);
      back.style.cssText = 'position:absolute;inset:0;margin:0;width:100%;height:100%;grid-area:auto;';
      rear.appendChild(back);
      var shadeFront = document.createElement('i'), shadeBack = document.createElement('i');
      shadeFront.className = shadeBack.className = 'manual-flip-shade';
      front.appendChild(shadeFront); rear.appendChild(shadeBack);
      cover.append(front, rear);
      stage.appendChild(cover);
      return { cover: cover, shadeFront: shadeFront, shadeBack: shadeBack };
    }
    // One clock for the cover, its shading and the cut-back spread, sampled so they cannot drift apart.
    function swing(parts, from, to, ms) {
      var turn = [], shadeF = [], shadeB = [], cut = [];
      for (var i = 0; i <= 30; i++) {
        var t = i / 30, k = ease(t), a = from + (to - from) * k;
        var lift = Math.sin(Math.PI * k);
        turn.push({ offset: t, transform: 'rotateY(' + a + 'deg) translateZ(' + (lift * 6) + 'px)' });
        shadeF.push({ offset: t, opacity: Math.min(.55, Math.abs(a) / 90 * .55) });
        shadeB.push({ offset: t, opacity: Math.max(0, (180 - Math.abs(a)) / 90 * .45) });
        cut.push({ offset: t, clipPath: clip(uncovered(a)) });
      }
      parts.shadeFront.animate(shadeF, { duration: ms, fill: 'both' });
      parts.shadeBack.animate(shadeB, { duration: ms, fill: 'both' });
      animations.push(spread.animate(cut, { duration: ms, fill: 'both' }));
      return parts.cover.animate(turn, { duration: ms, fill: 'forwards' });
    }
    var reading = ready();
    function coverClosed(cover) {
      return new Promise(function (resolve) {
        var close = overlay.querySelector('#manual-close');
        cover.classList.add('manual-cover-closed');
        cover.setAttribute('role', 'button'); cover.tabIndex = 0;
        cover.setAttribute('aria-label', 'Open the BLIP manual');
        stage.style.pointerEvents = 'none'; cover.style.pointerEvents = 'auto';
        overlay.inert = false;
        cover.focus({ preventScroll: true });
        function done(choice) {
          cover.removeEventListener('click', onClick); window.removeEventListener('keydown', onKey, true);
          close.removeEventListener('click', onClose, true);
          cover.classList.remove('manual-cover-closed'); cover.style.pointerEvents = 'none';
          overlay.inert = true;
          resolve(choice);
        }
        function onClick() { done('open'); }
        function onClose() { done('close'); }
        function onKey(event) {
          if (event.key === 'Escape') { done('close'); return; }
          if (event.key === 'Enter' || event.key === ' ' || event.key === 'ArrowRight') {
            event.preventDefault(); event.stopPropagation(); done('open');
          } else if (event.key === 'ArrowLeft') { event.preventDefault(); event.stopPropagation(); }
        }
        cover.addEventListener('click', onClick);
        window.addEventListener('keydown', onKey, true);
        close.addEventListener('click', onClose, true);
      });
    }
    async function ready() {
      for (var waited = 0; waited < 1500 && !overlay.querySelector('#manual-page-content .manual-leaf-sheet'); waited += 50)
        await new Promise(function (r) { setTimeout(r, 50); });
    }
    try {
      leftPage.style.visibility = 'hidden';
      if (returning) {
        // Close the cover over the right-hand page (unless it was never opened), pick the book up and put it back.
        var closing = hinge(overlay.hasAttribute('data-cover-closed') ? 0 : -180);
        if (!overlay.hasAttribute('data-cover-closed')) await swing(closing, -180, 0, 950).finished;
        delete overlay.dataset.coverClosed;
        document.body.appendChild(book);
        book.style.opacity = '1';
        book.style.transform = pose(hx, hy, 1, 0, 0);
        closing.cover.remove();
        animations.push(spread.animate([{ opacity: 1, clipPath: clip(spineX) }, { opacity: 0, clipPath: clip(spineX) }], { duration: 300, fill: 'both' }));
        animations.push(overlay.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 900, delay: 300, fill: 'both' }));
        var flight = book.animate(path.slice().reverse().map(function (k) { return Object.assign({}, k, { offset: 1 - k.offset }); }),
          { duration: 1150, easing: 'linear', fill: 'forwards' });
        document.body.appendChild(spine);
        animations.push(spine.animate(spineRise.slice().reverse(), { duration: 380, delay: 1150 - 380 * .6, easing: 'cubic-bezier(.3,0,.2,1)', fill: 'both' }));
        await flight.finished;
        source.classList.remove('manual-away');
        await new Promise(requestAnimationFrame);
      } else {
        document.body.append(book, spine);
        animations.push(spread.animate([{ opacity: 0, clipPath: clip(spineX) }, { opacity: 0, clipPath: clip(spineX), offset: .8 }, { opacity: 1, clipPath: clip(spineX) }],
          { duration: 1150, fill: 'both' }));
        animations.push(overlay.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 800, delay: 100, fill: 'both' }));
        animations.push(spine.animate(spineRise, { duration: 380, easing: 'cubic-bezier(.3,0,.2,1)', fill: 'forwards' }));
        await book.animate(path, { duration: 1150, easing: 'linear', fill: 'forwards' }).finished;
        await reading;
        var opening = hinge(0);
        await new Promise(requestAnimationFrame);
        book.remove();
        // It lies closed, its front cover up, until the reader opens it: a click on the cover, or Enter, Space or
        // the right arrow. Closing it now puts it straight back.
        // Closed, it is only the book: the open spread around it is hidden until the cover swings.
        var hidden = spread.animate([{ opacity: 0 }, { opacity: 0 }], { duration: 1, fill: 'forwards' });
        animations.push(hidden);
        var choice = await coverClosed(opening.cover);
        if (choice === 'open') {
          hidden.cancel();
          animations.push(spread.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 260, easing: 'ease-out', fill: 'both' }));
          await swing(opening, 0, -180, 1050).finished;
          leftPage.style.visibility = '';
          await new Promise(requestAnimationFrame);
        } else overlay.dataset.coverClosed = '';
      }
    } finally {
      animations.forEach(function (animation) { animation.cancel(); });
      leftPage.style.visibility = '';
      stage.remove(); book.remove(); spine.remove();
      source.classList.toggle('manual-away', !returning);
      overlay.inert = false;
    }
  };

  window.blipMoveManual = function (overlay, returning) {
    var calm = matchMedia('(prefers-reduced-motion: reduce)').matches;
    return (!calm && matchMedia('(min-width:701px)').matches ? moveWide : moveSimple)(overlay, returning);
  };
}());

/* ------------------------------------------------------------------ */
/* Coin / credits                                                       */
/* ------------------------------------------------------------------ */
// ---- Startup chime ----
// Once, when the machine gets its first coin (0 -> 1): a relay thunk and
// a rising flyback whine settling on the hum's 58 Hz. The cards light
// when it finishes, a beat of booting between paying and playing.
function playStartupSound(onDone) {
  var ctx = getKioskAudio();
  var t = ctx.currentTime;

  var noise = blipNoise(ctx, 0.14);
  var noiseFilter = ctx.createBiquadFilter();
  noiseFilter.type = 'lowpass';
  noiseFilter.frequency.setValueAtTime(2200, t);
  noiseFilter.frequency.exponentialRampToValueAtTime(80, t + 0.14);
  var noiseGain = ctx.createGain();
  noiseGain.gain.setValueAtTime(0.5, t);
  noiseGain.gain.exponentialRampToValueAtTime(0.001, t + 0.16);
  noise.connect(noiseFilter); noiseFilter.connect(noiseGain); noiseGain.connect(blipOut(ctx));
  noise.start(t); noise.stop(t + 0.16);

  var osc  = ctx.createOscillator();
  var gain = ctx.createGain();
  osc.type = 'sine';
  osc.frequency.setValueAtTime(60, t + 0.06);
  osc.frequency.exponentialRampToValueAtTime(920, t + 0.4);
  osc.frequency.exponentialRampToValueAtTime(58, t + 0.9);
  gain.gain.setValueAtTime(0.0001, t + 0.06);
  gain.gain.exponentialRampToValueAtTime(0.2, t + 0.32);
  gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.95);
  osc.connect(gain); gain.connect(blipOut(ctx));
  osc.start(t + 0.06); osc.stop(t + 0.96);

  if (typeof onDone === 'function') setTimeout(onDone, 900);
}

window.onCoinInserted = function() {
  // The corner button stops flashing the instant the coin lands — that's
  // the payoff for inserting it. The cards still wait for the power-up.
  updateCoinLock();
  updateCreditReadouts();
  var selected=document.querySelector('.card-focused');
  if(selected)selected.classList.add('card-ready');
  if (getCoins() === 1) {
    // This coin just brought the machine from empty to funded — power
    // up first, light up the cards once that finishes.
    playStartupSound(function () {
      updateCardDimming();
    });
  } else {
    updateCardDimming();
  }
};

// No credits: every card reads as unlit (still fully visible — just a
// light dim + desaturate) instead of a separate "NEED COIN" label —
// same gate as the card click itself, shown on the cards directly.
function updateCreditReadouts() {
  var credits=getCoins();
  document.querySelectorAll('.machine-credit').forEach(function(readout){readout.textContent='CREDIT '+credits;});
}
function updateCardDimming() {
  var enabled = getCoins() > 0;
  document.querySelectorAll('.card:not([data-page-card])').forEach(function(card) {
    card.classList.toggle('no-credit', !enabled);
    card.classList.toggle('card-ready',enabled&&card.classList.contains('card-focused'));
  });
  updateCreditReadouts();
  updateCoinLock();
}

function updateCoinLock() {
  document.body.classList.toggle('need-coin', getCoins() <= 0);
}

function updateCreditsDisplay() {
  updateCoinsHud();
  updateCardDimming();
  updateCoinPrompt();
}

updateCreditsDisplay(); // initialise dimming to match coins on page load

// Start opens the selected machine; its credit is spent by the game after the first frame.
var cardInFlight = false, pendingCard = null, focusArcadeCard = null, arcadeStepBack = null, arcadeStepIn = null;
document.querySelectorAll('#game-grid .card').forEach(function(card){var tray=card.querySelector('.card-shot-wrap'),image=tray.querySelector('.card-shot'),rom=document.createElement('div');rom.className='game-rom';rom.append(image);tray.appendChild(rom);});
document.querySelectorAll('#game-grid .card').forEach(function(card){
  var title=card.querySelector('.card-title'),slug=(card.getAttribute('data-game-url')||'').split('/')[0];
  // Rally seats two at its own dials; the others that do say so in BLIP_GAMES.
  var players=slug==='rally'||BLIP_GAMES[slug]&&BLIP_GAMES[slug].players===2?2:1,name=title?title.textContent.trim():'';
  card.dataset.machine=slug;title.classList.add('machine-marquee');
  title.textContent='';var titleName=document.createElement('b');titleName.className='machine-marquee-name';titleName.textContent=name;title.appendChild(titleName);
  var screen=card.querySelector('.card-shot-wrap');
  var instructions=document.createElement('span');instructions.className='machine-instructions';instructions.setAttribute('aria-hidden','true');instructions.textContent=(players>1?'1 OR 2 PLAYERS · ':'')+((BLIP_GAMES[slug]||{}).legend||'');screen.appendChild(instructions);
  var attract=document.createElement('span');attract.className='machine-attract';attract.setAttribute('aria-hidden','true');attract.innerHTML='<b>PRESS START</b><span class="machine-credit">CREDIT 0</span>';card.querySelector('.game-rom').appendChild(attract);
  var speaker=document.createElement('span');speaker.className='machine-speaker';speaker.setAttribute('aria-hidden','true');card.appendChild(speaker);
  ['left','right'].forEach(function(side){var panelSide=document.createElement('span');panelSide.className='machine-side '+side;panelSide.setAttribute('aria-hidden','true');panelSide.innerHTML='<b>'+name+'</b>';card.appendChild(panelSide);});
  var controls=document.createElement('div');controls.className='machine-controls';
  controls.innerHTML='<button type="button" class="machine-start" aria-label="Start '+name+'"><span class="machine-start-cap">START</span></button>';
  // The deck is the game page's own, the same <blip-deck>, so the cabinet's controls are the ones the player will use.
  var deck=document.createElement('div');deck.className='machine-deck';
  deck.innerHTML='<iframe src="machine-deck.html?game='+slug+'" tabindex="-1" aria-hidden="true" title="" onload="this.dataset.loaded=\'\'"></iframe>';
  controls.appendChild(deck);
  ['left','right'].forEach(function(side){var cheek=document.createElement('span');cheek.className='machine-deck-cheek '+side;cheek.setAttribute('aria-hidden','true');controls.appendChild(cheek);});
  card.appendChild(controls.querySelector('.machine-start'));
  card.querySelector('.machine-start').addEventListener('click',function(event){
    event.preventDefault();event.stopPropagation();
    if(!card.classList.contains('card-focused')){if(focusArcadeCard)focusArcadeCard(card);return;}
    attemptStart(card);
  });
  card.insertBefore(controls,card.querySelector('.machine-start'));
  var coinbox=document.createElement('button');
  coinbox.type='button';coinbox.className='machine-coinbox';coinbox.setAttribute('aria-label','Insert a coin at the '+(title?title.textContent.trim():'')+' machine');
  coinbox.innerHTML=('<span class="machine-coin-mech" aria-hidden="true"><span class="machine-coin-slot"></span><span class="machine-coin-price">25¢</span></span>').repeat(2)+'<span class="machine-coin-return" aria-hidden="true"></span>'.repeat(2)+'<span class="machine-coin-lock" aria-hidden="true"></span>';
  coinbox.addEventListener('click',function(event){
    event.preventDefault();event.stopPropagation();
    if(focusArcadeCard)focusArcadeCard(card);
    var sharedSlot=document.getElementById('kiosk-insert-btn');
    var before=getCoins();
    if(sharedSlot)sharedSlot.click();
    if(getCoins()>before){
      coinbox.classList.remove('coin-inserting');
      void coinbox.offsetWidth;
      coinbox.classList.add('coin-inserting');
      window.setTimeout(function(){coinbox.classList.remove('coin-inserting');},300);
    } else coinbox.classList.remove('coin-inserting');
  });
  card.appendChild(coinbox);
});
updateCardDimming();
var cabinetDeparture = window.blipCabinetReturn;
if (cabinetDeparture && /^data:image\/png;base64,/.test(cabinetDeparture.frame || '')) {
  document.querySelector('.jukebox-game-screen').style.backgroundImage = 'url(' + JSON.stringify(cabinetDeparture.frame) + ')';
}
function rollOutArcade() {
  document.documentElement.removeAttribute('data-cabinet-arranging');
  if (!cabinetDeparture) return;
  var selector = document.getElementById('jukebox-front');
  selector.inert = true;
  document.body.classList.add('cabinet-return');
  document.documentElement.removeAttribute('data-cabinet-return');
  var reduced = matchMedia('(prefers-reduced-motion:reduce)').matches;
  var drive = reduced ? null : blipCardDrive(1100);
  var began = performance.now(), motionFrame, finished = false, fallback;
  function move(now) {
    if (drive) drive.update(Math.min(1, (now - began) / 1100));
    if (!finished) motionFrame = requestAnimationFrame(move);
  }
  function finish(event) {
    if (event && (event.target !== selector || event.animationName !== 'arcade-return')) return;
    if (finished) return;
    finished = true;
    clearTimeout(fallback);
    cancelAnimationFrame(motionFrame);
    if (drive) drive.stop();
    selector.removeEventListener('animationend', finish);
    document.body.classList.remove('cabinet-return');
    document.querySelector('.jukebox-game-screen').style.backgroundImage = 'none';
    selector.inert = false;
    cabinetDeparture = null;
    delete window.blipCabinetReturn;
    playArcadeDock();
    var selected = document.querySelector('.card-focused');
    if (selected) selected.focus({preventScroll:true});
    if (pendingCard) { var card = pendingCard; pendingCard = null; attemptStart(card); }
  }
  selector.addEventListener('animationend', finish);
  motionFrame = requestAnimationFrame(move);
  fallback = setTimeout(function () { finish(); }, reduced ? 50 : 1400);
}
var manualChapters=['controls.html','history.html','about.html'], manualPages=[], manualPageIndex=0, manualLoadPromise=null, manualChapterStarts=[],manualGameStarts={},manualGameNames={},manualGameNumbers={},manualMeasureSize='';
var manualOverlay=document.getElementById('manual-overlay');
// Handwriting is never quite level or quite even: each word sits a hair off the line, with its own tilt and pressure.
function manualHand(text) {
  var seed=11;
  function next(){seed=(seed*1103515245+12345)&0x7fffffff;return seed/0x7fffffff;}
  return text.split(' ').map(function(word){
    return '<span style="--dy:'+((next()-.5)*3.6).toFixed(1)+'px;--r:'+((next()-.5)*3).toFixed(1)+'deg;--o:'+(.84+next()*.15).toFixed(2)+'">'+word+'</span>';
  }).join(' ');
}
// Sizes a page that must stay whole until it fits: its type, and so its photos and cards, shrink half a pixel at a
// time; on a very short screen the intro and the role lines go too.
function manualFitWhole(section,sheet,room) {
  section.style.fontSize='';section.removeAttribute('data-compact');
  for(var size=16;size>=7&&sheet.scrollHeight>room()+1;size-=.5) {
    section.style.fontSize=size+'px';
    if(size<11)section.dataset.compact='';
  }
}
// The measured size can differ from the shown one (a font arriving late), so the shown page is fitted again.
function manualFitShown() {
  manualOverlay.querySelectorAll('.manual-operators').forEach(function(section){
    if(section.closest('.manual-flip,.manual-flip-cover'))return;
    var sheet=section.closest('.manual-leaf-sheet');
    if(sheet&&sheet.clientHeight>0)manualFitWhole(section,sheet,function(){return sheet.clientHeight;});
  });
}
if(document.fonts&&document.fonts.ready)document.fonts.ready.then(function(){if(!manualOverlay.hidden)manualFitShown();});
function manualAtoms(node,parents,out) {
  // Written for the app's Guide (guide.js), not this book.
  if(node.nodeType!==1||node.classList.contains('divider')||node.matches('[data-guide="app"]'))return;
  if(node.tagName==='TABLE') {
    Array.from(node.rows).forEach(function(row){var table=node.cloneNode(false);table.appendChild(row.cloneNode(true));out.push({parents:parents,leaf:table});});return;
  }
  if(/^(H1|H2|H3|P|IMG|UL|OL)$/.test(node.tagName)||node.matches('.studio,.profile,.game-section,.game-header,.game-title,.game-origin,.manual-first-step,[data-manual-game],[data-manual-sheet]')) {out.push({parents:parents,leaf:node.cloneNode(true)});return;}
  if(!node.children.length) {if(node.textContent.trim())out.push({parents:parents,leaf:node.cloneNode(true)});return;}
  var next=parents.concat(node.cloneNode(false));
  Array.from(node.children).forEach(function(child){manualAtoms(child,next,out);});
}
function prepareManualPages() {
  var viewport=document.getElementById('manual-page-content'), width=viewport.clientWidth,height=viewport.clientHeight;
  if(!width||!height)return Promise.resolve([]);
  var size=width+'x'+height;if(size===manualMeasureSize&&manualLoadPromise)return manualLoadPromise;
  manualMeasureSize=size;
  var measure=document.createElement('article');measure.className='manual-page-content manual-measure';measure.style.width=width+'px';measure.style.height=height+'px';document.body.appendChild(measure);
  manualLoadPromise=Promise.all(manualChapters.map(function(url){return fetch(url).then(function(response){if(!response.ok)throw new Error('Manual page unavailable');return response.text();});})).then(function(markups){
    var result=[];
    markups.unshift(document.getElementById('manual-first-use').innerHTML);
    markups.forEach(function(markup,chapterIndex){
      chapterIndex--;
      var doc=new DOMParser().parseFromString(markup,'text/html'),source=doc.querySelector('.manual-leaf .page');
      if(!source)return;
      source.querySelectorAll('[data-manual-game]').forEach(function(game){var h3=game.querySelector('h3');if(!h3)return;var heading=h3.textContent,number=heading.match(/^\s*(\d+\.\d+)/);manualGameNumbers[game.dataset.manualGame]=number?number[1]:'';manualGameNames[game.dataset.manualGame]=heading.replace(/^\s*\d+\.\d+\s*/, '').trim();});
      source.querySelectorAll('a').forEach(function(link){var print=document.createElement('span');print.className=link.className;print.innerHTML=link.innerHTML;link.replaceWith(print);});
      var titles=['PLAYING THE GAMES','THE GOLDEN AGE OF ARCADE GAMES','ABOUT BLIP'];
      var title=chapterIndex<0?'FIRST TIME USE':['CONTROLS','HISTORY','ABOUT BLIP'][chapterIndex], atoms=[];
      var sourceHead=source.querySelector('h1');
      if(sourceHead)sourceHead.remove();
      var sourceSubhead=source.querySelector('h2');
      if(sourceSubhead&&chapterIndex>=0){var deck=document.createElement('p');deck.className='manual-chapter-deck';deck.innerHTML=sourceSubhead.innerHTML;sourceSubhead.replaceWith(deck);}
      var chapter=document.createElement('h1');
      chapter.className='manual-chapter-number';chapter.textContent='CHAPTER '+(chapterIndex+1);
      var subchapter=document.createElement('h2');
      subchapter.className='manual-subchapter-title';subchapter.textContent=titles[chapterIndex];
      if(chapterIndex>=0){source.insertBefore(subchapter,source.firstChild);source.insertBefore(chapter,subchapter);}
      source.querySelectorAll('.section-label,.thoughts-label,.impl-label').forEach(function(label){label.textContent=label.textContent.replace(/^[\s►▸→]+/,'').trim();});
      source.querySelectorAll('h2').forEach(function(heading){if(chapterIndex<0||heading.classList.contains('manual-subchapter-title'))return;var sectionHeading=document.createElement('h3');sectionHeading.className=heading.className;sectionHeading.innerHTML=heading.innerHTML;heading.replaceWith(sectionHeading);});
      Array.from(source.children).forEach(function(node){manualAtoms(node,[],atoms);});
      var sheet=document.createElement('div');sheet.className='page manual-leaf-sheet';measure.appendChild(sheet);var count=0;
      function appendAtom(atom){var wrapper=sheet;atom.parents.forEach(function(parent){var copy=parent.cloneNode(false);wrapper.appendChild(copy);wrapper=copy;});wrapper.appendChild(atom.leaf);}
      function saveSheet(){if(sheet.childElementCount){var game=sheet.querySelector('[data-manual-game]'),slug=game?game.dataset.manualGame:null,sided=sheet.querySelector('[data-side]');result.push({chapter:chapterIndex,game:slug,side:sided?sided.dataset.side||'':'',operators:!!sheet.querySelector('.manual-operators'),title:slug?manualGameNames[slug]:title,html:sheet.innerHTML});}sheet.remove();}
      function newSheet(){sheet=document.createElement('div');sheet.className='page manual-leaf-sheet';measure.appendChild(sheet);count=0;}
      function placeAtom(atom){
        if(count&&atom.leaf.matches('[data-manual-game],[data-manual-sheet]')){saveSheet();newSheet();}
        appendAtom(atom);var wrapper=sheet.lastElementChild;
        // A page made to stay whole (the operators) is never split: it is sized to fit.
        if(atom.leaf.matches('[data-manual-sheet]')) {
          manualFitWhole(wrapper,sheet,function(){return height;});
          count++;return;
        }
        if(sheet.scrollHeight>height+1){
          wrapper.remove();
          if(count){saveSheet();newSheet();}
          appendAtom(atom);wrapper=sheet.lastElementChild;
          if(sheet.scrollHeight>height+1&&atom.leaf.children.length){
            wrapper.remove();var parts=[],parents=atom.parents.concat(atom.leaf.cloneNode(false));
            Array.from(atom.leaf.children).forEach(function(child){manualAtoms(child,parents,parts);});
            if(parts.length>1){parts.forEach(placeAtom);return;}
            appendAtom(atom);
          }
          if(sheet.scrollHeight>height+1&&atom.leaf.tagName==='P'){
            sheet.lastElementChild.remove();
            var words=atom.leaf.textContent.trim().split(/\s+/),chunks=[],chunk='';
            words.forEach(function(word){
              var candidate=chunk?chunk+' '+word:word,probe=atom.leaf.cloneNode(false);probe.textContent=candidate;sheet.appendChild(probe);
              if(sheet.scrollHeight>height+1&&chunk){probe.remove();chunks.push(chunk);chunk=word;}
              else {chunk=candidate;probe.remove();}
            });
            if(chunk)chunks.push(chunk);
            chunks.forEach(function(text){var paragraph=atom.leaf.cloneNode(false);paragraph.textContent=text;placeAtom({parents:atom.parents,leaf:paragraph});});
            return;
          }
        }
        count++;
      }
      // A heading never ends a page alone: it moves with what follows unless a few lines of that fit.
      atoms.forEach(function(atom,i){
        var after=atoms[i+1];
        if(count&&after&&atom.leaf.matches('h1,h2,h3,.section-label,.thoughts-label,.impl-label')&&!after.leaf.matches('[data-manual-game],[data-manual-sheet]')) {
          appendAtom(atom);var headed=sheet.scrollHeight;appendAtom(after);
          var fits=sheet.scrollHeight<=height+1||height-headed>=72;
          sheet.lastElementChild.remove();sheet.lastElementChild.remove();
          if(!fits){saveSheet();newSheet();}
        }
        placeAtom(atom);
      });
      saveSheet();
    });
    // Chapters open on a right-hand page and each game's left page lands on a left-hand one,
    // with a ruled notes page wherever that leaves a gap, as a printed book does.
    var aligned=[];
    result.forEach(function(leaf,i){
      var before=result[i-1],opens=!before||before.chapter!==leaf.chapter;
      var sideStarts=leaf.side&&(!before||before.game!==leaf.game||before.side!==leaf.side);
      var odd=aligned.length%2===1;
      var gap=leaf.side==='left'?sideStarts&&!odd:leaf.side==='right'?sideStarts&&odd:opens&&odd;
      if(gap) {
        var jotting=['','coin box emptied Friday','left stool still wobbles','','player 2 button sticks - oiled','','free game if you beat MAX','','cleaned the glass, 3rd time this month',''][aligned.length%10];
        var letter=aligned.filter(function(page){return page.filler;}).length===1;
        aligned.push({chapter:-9,game:null,side:'',filler:true,note:letter,title:'',html:letter?
          '<div class="manual-notes"><p>NOTES</p><figure class="manual-note-card"><i class="manual-doodle" aria-hidden="true"></i>'+
          '<div class="manual-note-body">'+manualHand('Blip Arcade is a tribute to the golden age of arcade games. I hope you enjoy this as much as I enjoyed playing the old games.')+'</div>'+
          '<div class="manual-note-regards">'+manualHand('Kind regards,')+'</div>'+
          '<span class="manual-signature">Jacob Andresen</span><i class="manual-heart" aria-hidden="true"></i></figure></div>':
          '<div class="manual-notes" aria-hidden="true"><p>NOTES</p>'+(jotting?'<q>'+jotting+'</q>':'')+'</div>'});
      }
      aligned.push(leaf);
    });
    // One leaf has lost its outer corner at the foot, as a well-read leaf will.
    // Its back is the next page, torn at the same place: the other foot corner as the leaf turns over.
    var torn=aligned.filter(function(leaf){return leaf.game==='meteors'&&leaf.side==='right';})[0],tornAt=aligned.indexOf(torn);
    if(torn){torn.torn='br';if(aligned[tornAt+1])aligned[tornAt+1].torn='bl';}
    result=aligned;
    var operatorsAt=result.findIndex(function(page){return page.operators;}),operatorsEntry=document.querySelector('[data-chapter="operators"]');
    if(operatorsAt>=0){operatorsEntry.disabled=false;operatorsEntry.dataset.page=operatorsAt;operatorsEntry.querySelector('b').textContent=String(operatorsAt+1).padStart(2,'0');}
    var firstUse=document.querySelector('[data-chapter="first-use"]');firstUse.disabled=false;firstUse.dataset.page=0;firstUse.querySelector('b').textContent='01';
    var starts={};result.forEach(function(page,index){if(starts[page.chapter]===undefined)starts[page.chapter]=index;});
    manualChapterStarts=[0,1,2].map(function(index){return starts[index]===undefined?0:starts[index];});
    ['controls','history','about'].forEach(function(key,index){
      var entry=document.querySelector('.manual-contents [data-chapter="'+key+'"]');entry.disabled=false;entry.dataset.page=manualChapterStarts[index];
      entry.querySelector('b').textContent=String(manualChapterStarts[index]+1).padStart(2,'0');
    });
    manualGameStarts={};result.forEach(function(page,index){if(page.game&&manualGameStarts[page.game]===undefined)manualGameStarts[page.game]=index;});
    var contents=document.getElementById('manual-game-contents');contents.replaceChildren();
    Object.keys(manualGameStarts).sort(function(a,b){return manualGameNames[a].localeCompare(manualGameNames[b]);}).forEach(function(slug){
      var row=document.createElement('li'),button=document.createElement('button'),name=document.createElement('span'),leader=document.createElement('i'),number=document.createElement('b');
      button.type='button';button.dataset.game=slug;button.dataset.page=manualGameStarts[slug];
      name.textContent=manualGameNumbers[slug]+'  '+manualGameNames[slug];name.appendChild(leader);number.textContent=String(manualGameStarts[slug]+1).padStart(2,'0');
      button.append(name,number);row.appendChild(button);contents.appendChild(row);
    });
    return result;
  }).catch(function(error){manualMeasureSize='';manualLoadPromise=null;throw error;}).finally(function(){measure.remove();});
  return manualLoadPromise;
}
function manualWide() {return matchMedia('(min-width:701px)').matches;}
function manualSheet(leaf) {return '<div class="page manual-leaf-sheet'+(leaf?'':' manual-endpaper')+'">'+(leaf?leaf.html:'')+'</div>';}
// Which pages the book shows for a spread whose right-hand leaf is `right`; the first spread has the contents on its left.
function manualFill(pages,right) {
  var wide=manualWide(),left=wide&&right>0?pages[right-1]:null,leaf=pages[right]||null;
  var leftPage=manualOverlay.querySelector('.manual-open-left');
  leftPage.dataset.face=left?'leaf':'contents';
  [[leftPage,left],[manualOverlay.querySelector('.manual-open-right'),leaf]].forEach(function(entry){
    if(entry[1]&&entry[1].torn)entry[0].setAttribute('data-torn',entry[1].torn);else entry[0].removeAttribute('data-torn');
  });
  document.getElementById('manual-left-content').innerHTML=left?manualSheet(left):'';
  document.getElementById('manual-left-title').textContent=left&&!left.filler?left.title.toUpperCase():'';
  document.getElementById('manual-left-folio').textContent=left?String(right):'';
  document.getElementById('manual-page-content').innerHTML=manualSheet(leaf);
  document.getElementById('manual-book-title').textContent=leaf&&!leaf.filler?leaf.title.toUpperCase():'';
  document.getElementById('manual-right-folio').textContent=leaf?String(right+1):'';
  document.getElementById('manual-page-count').textContent=String(right+1).padStart(2,'0')+' / '+String(pages.length).padStart(2,'0');
  var last=pages.length-1;
  document.getElementById('manual-prev').disabled=right===0;
  document.getElementById('manual-next').disabled=right>=(wide&&last%2?last+1:last);
  manualOverlay.querySelector('.manual-open-book').style.setProperty('--read',String(last?right/last:0));
  manualOverlay.querySelector('.manual-open-book').dataset.smudge=String((wide?right/2:right)%4);
  manualFitShown();
  manualOverlay.querySelectorAll('.manual-contents button').forEach(function(entry){
    var shown=[leaf,left].filter(Boolean),current=shown.some(function(page){return entry.dataset.game?entry.dataset.game===page.game:Number(entry.dataset.page)===pages.indexOf(page);});
    entry.toggleAttribute('aria-current',current);if(current)entry.setAttribute('aria-current','page');
  });
}
// The lifted page turns about the spine: its front is the page it was, its back the page it becomes.
var manualFlipping=null;
// Three ways a page is turned, never the same twice running: a plain sweep, a lazy curl that lifts off the page, a quick flick that overshoots and settles.
var manualFlipLast=-1,manualFlipUsed=[],manualFlipTurns=0;
var MANUAL_FLIPS=[
  {ms:720,shade:.45,cast:.3,frames:function(s){return [{transform:'rotateY(0deg)',easing:'cubic-bezier(.45,.05,.25,1)'},{transform:'rotateY('+s*180+'deg)'}];}},
  {ms:920,shade:.55,cast:.38,frames:function(s){return [
    {transform:'translateZ(0) rotateX(0deg) rotateY(0deg) skewY(0deg)',offset:0,easing:'cubic-bezier(.3,0,.55,1)'},
    {transform:'translateZ(12px) rotateX(-1.5deg) rotateY('+s*62+'deg) skewY('+s*-2+'deg)',offset:.36,easing:'linear'},
    {transform:'translateZ(16px) rotateX(-1deg) rotateY('+s*128+'deg) skewY('+s*1.5+'deg)',offset:.7,easing:'cubic-bezier(.2,.6,.3,1)'},
    {transform:'translateZ(0) rotateX(0deg) rotateY('+s*180+'deg) skewY(0deg)',offset:1}];}},
  {ms:540,shade:.35,cast:.22,frames:function(s){return [
    {transform:'rotateY(0deg) rotateX(0deg)',offset:0,easing:'cubic-bezier(.25,.7,.3,1)'},
    {transform:'rotateY('+s*187+'deg) rotateX(1.6deg)',offset:.78,easing:'ease-in-out'},
    {transform:'rotateY('+s*180+'deg) rotateX(0deg)',offset:1}];}}
];
function manualFlipStyle() {
  // The style used longest ago, with ties settled at random: every style comes round before any repeats.
  manualFlipUsed.length||MANUAL_FLIPS.forEach(function(){manualFlipUsed.push(-1);});
  var oldest=Math.min.apply(null,manualFlipUsed.filter(function(_,i){return i!==manualFlipLast;}));
  var choices=MANUAL_FLIPS.map(function(_,i){return i;}).filter(function(i){return i!==manualFlipLast&&manualFlipUsed[i]===oldest;});
  var pick=manualFlipLast<0?0:choices[Math.floor(Math.random()*choices.length)];
  manualFlipUsed[pick]=++manualFlipTurns;manualFlipLast=pick;return MANUAL_FLIPS[pick];
}
function manualFlip(pages,right,direction) {
  if(manualFlipping)manualFlipping.finish();
  var book=manualOverlay.querySelector('.manual-open-book'),leftPage=book.querySelector('.manual-open-left'),rightPage=book.querySelector('.manual-open-right');
  var forward=direction>0,lifted=forward?rightPage:leftPage,landing=forward?leftPage:rightPage;
  manualFlipping=window.BlipBook.turn({
    book:book,lifted:lifted,landing:landing,direction:direction,style:manualFlipStyle(),
    update:function(){manualFill(pages,right);},
    onFinish:function(){manualFlipping=null;}
  });
}
var manualShown=false;
function showManualPage(index,direction) {
  prepareManualPages().then(function(pages){
    if(!pages.length)return;
    var wide=manualWide(),last=pages.length-1,limit=wide&&last%2?last+1:last;
    var right=Math.max(0,Math.min(index,limit));
    if(wide&&right%2)right=Math.min(right+1,limit);
    if(!wide&&pages[right]&&pages[right].filler&&!pages[right].note) {
      var step=direction<0?-1:1,probe=right;
      while(pages[probe]&&pages[probe].filler&&!pages[probe].note)probe+=step;
      if(!pages[probe]){probe=right;while(pages[probe]&&pages[probe].filler&&!pages[probe].note)probe-=step;}
      right=Math.max(0,probe);
    }
    var turning=direction&&manualShown&&wide&&right!==manualPageIndex&&Math.abs(right-manualPageIndex)===2&&!matchMedia('(prefers-reduced-motion: reduce)').matches;
    manualPages=pages;
    if(turning){manualPageIndex=right;manualFlip(pages,right,direction);}
    else {manualPageIndex=right;if(manualFlipping)manualFlipping.finish();manualFill(pages,right);}
    manualShown=true;
    if(direction)playManualPaper(false);
  }).catch(function(){document.getElementById('manual-page-content').innerHTML='<div class="page manual-leaf-sheet"><h1>PAGE MISSING</h1><p>This leaf could not be read. Please turn back and try again.</p></div>';});
}
function turnManualPage(direction) {showManualPage(manualPageIndex+direction*(manualWide()?2:1),direction);}
var manualMoving=false, manualCloseRequested=false, manualPreviousOverflow='';
async function openFieldManual(chapterIndex,gameSlug) {
  if(manualMoving||!manualOverlay.hidden)return;
  manualMoving=true;manualCloseRequested=false;
  manualPreviousOverflow=document.documentElement.style.overflow;
  manualPageIndex=0;manualShown=false;manualFlipLast=-1;manualFlipUsed=[];manualFlipTurns=0;
  manualOverlay.hidden=false;
  document.body.classList.add('manual-open');
  document.documentElement.style.overflow='hidden';
  playManualPaper(true);
  setManualContents(false);
  prepareManualPages().then(function(){showManualPage(manualGameStarts[gameSlug]===undefined?(chapterIndex===undefined?0:manualChapterStarts[chapterIndex]||0):manualGameStarts[gameSlug],false);});
  await window.blipMoveManual(manualOverlay,false);
  manualMoving=false;
  if(manualCloseRequested){closeFieldManual();return;}
  document.getElementById('manual-close').focus({preventScroll:true});
}
async function closeFieldManual() {
  if(manualOverlay.hidden)return;
  if(manualMoving){manualCloseRequested=true;return;}
  manualMoving=true;manualCloseRequested=false;
  playManualPaper(true);
  await window.blipMoveManual(manualOverlay,true);
  manualOverlay.hidden=true;
  manualOverlay.classList.remove('manual-page-turning');
  document.body.classList.remove('manual-open');
  document.documentElement.style.overflow=manualPreviousOverflow;
  manualMoving=false;
  cardInFlight=false;
  if(new URLSearchParams(location.search).has('manual'))history.replaceState(null,'',location.pathname);
  document.getElementById('manual-book').focus({preventScroll:true});
}
function setManualContents(show) {
  manualOverlay.querySelector('.manual-open-book').toggleAttribute('data-show-contents',show);
  var tab=document.getElementById('manual-contents-tab');tab.setAttribute('aria-expanded',String(show));tab.textContent=show?'Read page':'Contents';
}
document.getElementById('manual-contents-tab').addEventListener('click',function(){
  if(manualWide()){showManualPage(0,-1);return;}
  setManualContents(!manualOverlay.querySelector('.manual-open-book').hasAttribute('data-show-contents'));
});
manualOverlay.querySelector('.manual-open-left').addEventListener('click',function(event){
  var entry=event.target.closest('button[data-page]');if(!entry)return;
  setManualContents(false);showManualPage(Number(entry.dataset.page),Number(entry.dataset.page)>=manualPageIndex?1:-1);document.getElementById('manual-page-content').focus({preventScroll:true});
});
new ResizeObserver(function(){
  if(manualOverlay.hidden)return;
  var viewport=document.getElementById('manual-page-content'),size=viewport.clientWidth+'x'+viewport.clientHeight;
  if(viewport.clientWidth&&size!==manualMeasureSize){var current=manualPages[manualPageIndex];if(current&&current.filler)current=manualPages[manualPageIndex+1]||manualPages[manualPageIndex-1];prepareManualPages().then(function(){showManualPage(current&&current.game?manualGameStarts[current.game]||0:manualChapterStarts[current&&current.chapter>=-1?current.chapter:0]||0,false);});}
}).observe(document.getElementById('manual-page-content'));
document.getElementById('manual-prev').addEventListener('click',function(){turnManualPage(-1);});
document.getElementById('manual-next').addEventListener('click',function(){turnManualPage(1);});
document.getElementById('manual-close').addEventListener('click',closeFieldManual);
document.addEventListener('keydown',function(event){
  if(manualOverlay.hidden)return;
  if(event.key==='Tab') {
    var stops=Array.from(manualOverlay.querySelectorAll('button:not(:disabled),[tabindex="0"]')).filter(function(node){return node.offsetWidth&&node.offsetHeight&&getComputedStyle(node).visibility!=='hidden';});
    var index=stops.indexOf(document.activeElement),next=(index+(event.shiftKey?-1:1)+stops.length)%stops.length;
    event.preventDefault();event.stopImmediatePropagation();if(stops[next])stops[next].focus();return;
  }
  if(event.key==='Escape'){event.preventDefault();event.stopImmediatePropagation();closeFieldManual();return;}
  if(event.key==='ArrowRight'||event.key==='PageDown'){event.preventDefault();event.stopImmediatePropagation();var contents=manualOverlay.querySelector('.manual-open-book').hasAttribute('data-show-contents')&&matchMedia('(max-width:700px)').matches;setManualContents(false);if(manualWide())turnManualPage(1);else showManualPage(manualPageIndex+(contents?0:1),1);}
  else if(event.key==='ArrowLeft'||event.key==='PageUp'){event.preventDefault();event.stopImmediatePropagation();setManualContents(false);turnManualPage(-1);}
  else event.stopImmediatePropagation();
},true);
// A swipe across the open book turns the page, as the arrow keys do.
(function(){
  var book=manualOverlay.querySelector('.manual-open-book'),start=null;
  book.addEventListener('pointerdown',function(e){start=e.pointerType==='mouse'?null:{id:e.pointerId,x:e.clientX,y:e.clientY};});
  book.addEventListener('pointercancel',function(){start=null;});
  book.addEventListener('pointerup',function(e){
    var s=start;start=null;
    if(!s||s.id!==e.pointerId)return;
    var dx=e.clientX-s.x,dy=e.clientY-s.y;
    if(Math.abs(dx)<48||Math.abs(dx)<Math.abs(dy)*1.5)return;
    document.dispatchEvent(new KeyboardEvent('keydown',{key:dx<0?'ArrowRight':'ArrowLeft',bubbles:true,cancelable:true}));
  });
}());
function gameCardUrl(card) {return card.getAttribute('data-game-url')||'';}
function attemptStart(card) {
  if (!card || cardInFlight) return;
  if (document.body.classList.contains('cabinet-return')) { pendingCard=card; return; }
  if (card.id==='manual-book') { openFieldManual(); return; }
  if (!card.classList.contains('card')) return;
  var machineSlug=(gameCardUrl(card)||'').split('/')[0];
  if (!machineSlug) return;
  // Start on an empty machine pays for itself: the coin drops through this door, then the game opens.
  var paying=getCoins()<=0;
  if (paying) {
    var door=card.querySelector('.machine-coinbox');
    if (door) door.click();
    if (getCoins()<=0) return;
  }
  cardInFlight=true;
  if (paying) { window.setTimeout(function(){openMachine(card,machineSlug);},420); return; }
  openMachine(card,machineSlug);
}
async function openMachine(card,machineSlug) {
  try {
    sessionStorage.setItem('blip-last-game',machineSlug);
    sessionStorage.setItem('blip-cabinet-start',machineSlug);
  } catch(e) {}
  if (matchMedia('(prefers-reduced-motion:reduce)').matches) { window.location.href=gameCardUrl(card); return; }
  // Freeze the frame actually on the CRT, rather than jumping back to a different still.
  var shot=card.querySelector('.card-shot'),art=machineSlug+'/screenshot.png';
  var strip=card.querySelector('.card-loop-strip'),loop=card.querySelector('.card-loop-frame');
  if(card.hasAttribute('data-loop-ready')&&strip&&strip.naturalWidth){
    try {
      var frozen=document.createElement('canvas');frozen.width=strip.naturalWidth;frozen.height=strip.naturalHeight/12;
      var index=Math.max(0,Math.min(11,Math.round(parseFloat(getComputedStyle(loop).backgroundPositionY)*11/100)));
      frozen.getContext('2d').drawImage(strip,0,index*frozen.height,frozen.width,frozen.height,0,0,frozen.width,frozen.height);
      art=frozen.toDataURL('image/png');
    } catch(e) {}
  }
  if(shot&&shot.getAttribute('src')!==art){
    shot.dataset.still=shot.getAttribute('src');shot.src=art;
    try {await shot.decode();} catch(e) {}
  }
  card.classList.add('card-launching');
  var nativeTransition='onpageswap' in window&&'onpagereveal' in window&&CSS.supports('view-transition-name','machine-monitor');
  try { sessionStorage.setItem('blip-cabinet-launch',JSON.stringify({slug:machineSlug,at:Date.now(),art:new URL(art,location.href).href,native:nativeTransition})); } catch(e) {}
  // Cross-document transitions snapshot the same cabinet parts on both pages, with no estimated endpoint.
  if(nativeTransition){
    // Lean in at once while the game page loads; the move carries on from wherever the lean has got to.
    if(typeof arcadeStepIn==='function')arcadeStepIn();
    requestAnimationFrame(function(){window.location.href=gameCardUrl(card);});
    return;
  }
  // The player steps in until the picture on this screen is where the game page will show it, then the page
  // takes over: the same machine, near.
  var room=document.getElementById('jukebox-front'),frame=room.getBoundingClientRect(),glass=card.querySelector('.game-rom').getBoundingClientRect();
  var aspect=shot&&shot.naturalWidth?shot.naturalWidth/shot.naturalHeight:480/540;
  var from=fitPicture(glass,aspect),to=fitPicture(launchScreen(),aspect),zoom=to.width/from.width;
  room.style.transformOrigin=(from.left+from.width/2-frame.left)+'px '+(from.top+from.height/2-frame.top)+'px';
  room.style.setProperty('--launch-x',(to.left+to.width/2-from.left-from.width/2)+'px');
  room.style.setProperty('--launch-y',(to.top+to.height/2-from.top-from.height/2)+'px');
  room.style.setProperty('--launch-zoom',zoom.toFixed(3));
  document.body.classList.add('machine-launching');
  var navigated=false;
  function navigate(){if(navigated)return;navigated=true;window.location.href=gameCardUrl(card);}
  room.addEventListener('transitionend',function(event){if(event.target===room&&event.propertyName==='transform')navigate();},{once:true});
  window.setTimeout(navigate,LAUNCH_MS+100);
}
var LAUNCH_MS=760;
window.addEventListener('pageswap',function(event){
  if(!event.viewTransition)return;
  var launching=document.querySelector('#game-grid .card-launching .card-shot-wrap');
  if(!launching){event.viewTransition.skipTransition();return;}
  var r=launching.getBoundingClientRect();
  try{sessionStorage.setItem('blip-machine-screen',JSON.stringify({x:r.left,y:r.top,w:r.width,h:r.height,vw:innerWidth,vh:innerHeight}));}catch(e){}
});
// Back from a game: the machine's parts are named, its screen holds the game's last frame, and the page's
// view steps back from it into the aisle.
window.addEventListener('pagereveal',function(event){
  var back=window.blipMachineReturn;window.blipMachineReturn=null;
  if(!event.viewTransition){document.documentElement.removeAttribute('data-machine-return');return;}
  var card=back&&document.querySelector('#game-grid .card[data-game-url^="'+back.slug+'/"]');
  if(!card){document.documentElement.removeAttribute('data-machine-return');event.viewTransition.skipTransition();return;}
  card.classList.add('card-launching');
  if(back.screen){var r=card.querySelector('.card-shot-wrap').getBoundingClientRect();blipMachineZoom({x:r.left,y:r.top,w:r.width,h:r.height},back.screen,true);}
  var shot=card.querySelector('.card-shot');
  if(shot&&/^data:image\/png;base64,/.test(back.frame||'')){shot.dataset.still=shot.getAttribute('src');shot.src=back.frame;}
  event.viewTransition.finished.catch(function(){}).then(function(){
    document.documentElement.removeAttribute('data-machine-return');
    card.classList.remove('card-launching');
    if(shot&&shot.dataset.still){shot.src=shot.dataset.still;delete shot.dataset.still;}
  });
});
// A picture of this shape, letterboxed in a box, as the game draws itself.
function fitPicture(box,aspect) {
  var width=Math.min(box.width,box.height*aspect),height=width/aspect;
  return {left:box.left+(box.width-width)/2,top:box.top+(box.height-height)/2,width:width,height:height};
}
// Where the game page's screen will be: as it last measured itself at this window size (game.js,
// blip-screen-box), or else inside this window's frame above a deck of the depth it last had.
function launchScreen() {
  var saved=null;
  try { saved=JSON.parse(localStorage.getItem('blip-screen-box')||'null'); } catch(e) {}
  if(saved&&saved.w===innerWidth&&saved.h===innerHeight&&saved.box)return {left:saved.box[0],top:saved.box[1],width:saved.box[2],height:saved.box[3]};
  var bezel=document.getElementById('screen-bezel').getBoundingClientRect(),inset=15;
  var deck=saved&&saved.deck>0?saved.deck:Math.min(170,Math.round(innerHeight*.16));
  return {left:bezel.left+inset,top:bezel.top+inset,width:bezel.width-inset*2,height:innerHeight-deck-bezel.top-inset*2};
}
// Back from a game through the browser's page cache: stand back from the machine.
window.addEventListener('pageshow',function(event){
  if(!event.persisted)return;
  if(typeof arcadeStepBack==='function')arcadeStepBack();
  document.body.classList.remove('machine-launching');
  document.querySelectorAll('#game-grid .card-launching').forEach(function(card){
    card.classList.remove('card-launching');
    var shot=card.querySelector('.card-shot');
    if(shot&&shot.dataset.still){shot.src=shot.dataset.still;delete shot.dataset.still;}
  });
  cardInFlight=false;
});

var selectorPlane=document.getElementById('jukebox-front');
// The instructions printed on each bezel step down a whole pixel at a time until they fit; the pixel font blurs between.
var cardFitKey='';
function fitCardText() {
  var notes=document.querySelectorAll('#game-grid .machine-instructions');
  if(!notes.length||!notes[0].clientWidth)return;
  var key=document.fonts&&document.fonts.status;
  if(key===cardFitKey)return;
  cardFitKey=key;
  notes.forEach(function(note){
    note.style.fontSize='';
    for(var size=parseFloat(getComputedStyle(note).fontSize);size>5&&note.scrollWidth>note.clientWidth;size--)note.style.fontSize=(size-1)+'px';
  });
}
if(document.fonts&&document.fonts.ready)document.fonts.ready.then(function(){cardFitKey='';fitCardText();});
// The room drops back in from above when a game returns to it (arcade-return), by its own height.
function fitRoom(){
  fitCardText();
  var bezel=document.getElementById('screen-bezel');
  bezel.style.setProperty('--arcade-travel',(selectorPlane.offsetHeight+30)+'px');
}
new ResizeObserver(fitRoom).observe(selectorPlane);
window.addEventListener('resize',fitRoom);
fitRoom();

/* Light up the card of the game you're currently in (last started). */
(function () {
  try {
    var last = sessionStorage.getItem('blip-last-game');
    if (!last) return;
    var el = document.querySelector('.card[data-game-url^="' + last + '/"]');
    if (el) el.classList.add('card-active');
  } catch (e) {}
}());

document.getElementById('manual-book').addEventListener('click',function(event){event.preventDefault();attemptStart(this);});


// ---- Card navigation: keyboard arrows + joystick/gamepad ----
(function () {
  var cards = Array.prototype.slice.call(document.querySelectorAll('#game-grid .card'));
  if (!cards.length) return;
  // Start the selection on the game you're currently in (last started),
  // so the first arrow press steps away from there.
  var lastGame = null;
  try { lastGame = sessionStorage.getItem('blip-last-game'); } catch (e) {}
  var activeIdx = lastGame ? cards.findIndex(function (c) {
    return (gameCardUrl(c) || '').indexOf(lastGame + '/') === 0;
  }) : -1;
  var idx = activeIdx >= 0 ? activeIdx : 0;
  var navActive = true;
  var walkQueue=[],walking=false;
  var grid=document.getElementById('game-grid');
  // Reduced motion keeps whole-machine steps (move, walkNext); otherwise the aisle walks (BlipAisle).
  var stepWalk=matchMedia('(prefers-reduced-motion:reduce)');
  var aisle=BlipAisle.create(cards.length,idx),aisleFrame=0,aisleLast=0,aisleRoomNow=null;
  var held={},walked=0,stride=0,pendingStart=false;
  // Stepped up to the faced machine: lean 0 standing in the aisle, 1 at the machine.
  var steppedUp=false,lean=0,LEAN_Z=150,LEAN_LIFT=60;
  // The room's layers (arcade.css): the carpet nearer than the machines, the wall's joints farther.
  function roomLayer(tag,className,layer,zone){
    var el=document.createElement(tag);el.className=className;el.setAttribute('aria-hidden','true');
    el.dataset.roomLayer=layer;el.dataset.zone=zone;return el;
  }
  var wall=roomLayer('i','aisle-wall','wall','wall'),trim=roomLayer('i','aisle-trim','trim','trim'),carpet=roomLayer('i','aisle-carpet','carpet','floor');
  grid.insertBefore(carpet,grid.firstChild);grid.insertBefore(trim,carpet);grid.insertBefore(wall,trim);
  var tiles=document.getElementById('room-tiles').content.firstElementChild.cloneNode(true);
  tiles.dataset.roomLayer='tiles';tiles.dataset.zone='wall';grid.insertBefore(tiles,trim);
  var stepEdge=roomLayer('i','aisle-step','step','floor');grid.insertBefore(stepEdge,carpet);
  var signs=document.getElementById('room-signs').content.cloneNode(true);
  var roomSignEls=[['.aisle-neon','neon'],['.aisle-open','open']].map(function(sign){
    var el=signs.querySelector(sign[0]);el.dataset.roomLayer=sign[1];el.dataset.zone='sign';
    grid.insertBefore(el,carpet);return el;
  });
  // How far the room has scrolled, in machines. The aisle's own position wraps round the loop when it
  // settles; this keeps counting, so the room never jumps between the last machine and the first.
  var roomTravel=aisle.pos,roomFrom=aisle.pos;
  // Round each machine: its shadow on the wall behind, its power cord to a socket in the gap beside it, and
  // whatever visitors left behind (a can on Defender, a sweet's wrapper by Serpent).
  var dressing=document.getElementById('room-dressing').content;
  function dressCabinet(c) {
    c.appendChild(roomLayer('span','card-backlight','shadow','wall'));
    powerCord(c);
    var props=[];
    if(c.dataset.machine==='defender')props.push(['.room-can','prop','wall'],['.room-can-shadow','prop','wall']);
    if(c.dataset.machine==='serpent')props.push(['.room-wrapper','prop','floor']);
    props.forEach(function(prop){
      var el=dressing.querySelector(prop[0]).cloneNode(true);
      el.dataset.roomLayer=prop[1];el.dataset.zone=prop[2];c.appendChild(el);
    });
  }
  // Each machine's cord, laid its own way: out of a small junction box on the cabinet's right side by the deck,
  // down the side to the floor, slack across the floor of the gap and back to the wall's foot, then up into a
  // socket. The machine's name seeds the shape, so a cabinet keeps its cord. Each piece lies on its own plane
  // (arcade.css): the drop 52px behind the front, the floor seen from above (0 the cabinets' fronts, 215 the
  // wall, x 0 at 92% of the cabinet's width) and the wall (0 the floor); they meet where one ends.
  function powerCord(c) {
    var seed=0,name=c.dataset.machine||'';
    for(var i=0;i<name.length;i++)seed=(seed*31+name.charCodeAt(i))>>>0;
    function rand(){seed=(seed+0x6D2B79F5)>>>0;var t=seed;t=Math.imul(t^t>>>15,t|1);t^=t+Math.imul(t^t>>>7,t|61);return((t^t>>>14)>>>0)/4294967296;}
    function between(a,b){return a+(b-a)*rand();}
    var startX=36,startZ=52,socketX=between(52,96),socketUp=between(30,46),reach=between(26,60);
    var bendX=startX+between(10,30),loop=rand()<.25;
    var p=[[startX,startZ],[bendX,reach+between(-8,8)],[(bendX+socketX)/2+between(-12,12),reach+between(30,80)],[socketX+between(-6,6),between(198,207)],[socketX,214]];
    if(loop)p.splice(2,0,[bendX+between(10,18),reach-between(4,10)],[bendX+between(-4,4),reach-between(12,18)]);
    var d='M'+p[0][0].toFixed(1)+' '+p[0][1].toFixed(1);
    for(var k=0;k<p.length-1;k++){
      var a=p[k-1]||p[k],b=p[k],e=p[k+1],f=p[k+2]||e;
      d+='C'+(b[0]+(e[0]-a[0])/6).toFixed(1)+' '+(b[1]+(e[1]-a[1])/6).toFixed(1)+' '+(e[0]-(f[0]-b[0])/6).toFixed(1)+' '+(e[1]-(f[1]-b[1])/6).toFixed(1)+' '+e[0].toFixed(1)+' '+e[1].toFixed(1);
    }
    var floor=document.createElementNS('http://www.w3.org/2000/svg','svg');
    floor.setAttribute('viewBox','0 0 168 215');floor.setAttribute('class','cord-floor');floor.setAttribute('aria-hidden','true');
    floor.innerHTML='<path class="cord-shadow" d="'+d+'" transform="translate(1.5 3)"/><path class="cord-wire" d="'+d+'"/><path class="cord-shine" d="'+d+'" transform="translate(-.6 -.8)"/>';
    var bow=between(-7,7),up='M'+socketX.toFixed(1)+' 100C'+(socketX+bow).toFixed(1)+' '+(100-socketUp*.35).toFixed(1)+' '+(socketX+bow*.4).toFixed(1)+' '+(100-socketUp*.8).toFixed(1)+' '+socketX.toFixed(1)+' '+(100-socketUp+6).toFixed(1);
    var wallPiece=document.createElementNS('http://www.w3.org/2000/svg','svg');
    wallPiece.setAttribute('viewBox','0 0 168 100');wallPiece.setAttribute('class','cord-wall');wallPiece.setAttribute('aria-hidden','true');
    var sy=100-socketUp;
    wallPiece.innerHTML='<g class="cord-socket" transform="translate('+socketX.toFixed(1)+' '+sy.toFixed(1)+')"><rect class="socket-shadow" x="-8" y="-9" width="18" height="21" rx="2.6"/><rect x="-9" y="-11" width="18" height="21" rx="2.6"/><circle cx="-3" cy="-2" r="1.5"/><circle cx="3" cy="-2" r="1.5"/></g>'+
      '<path class="cord-shadow" d="'+up+'" transform="translate(2.5 1.5)"/><path class="cord-wire" d="'+up+'"/><path class="cord-shine" d="'+up+'" transform="translate(-.8 0)"/>'+
      '<rect class="cord-plug-shadow" x="'+(socketX-3.5).toFixed(1)+'" y="'+(sy-3).toFixed(1)+'" width="9" height="13" rx="1.8"/><rect class="cord-plug" x="'+(socketX-4.5).toFixed(1)+'" y="'+(sy-5).toFixed(1)+'" width="9" height="13" rx="1.8"/>';
    var sway=between(3,9),drop=document.createElementNS('http://www.w3.org/2000/svg','svg');
    drop.setAttribute('viewBox','0 0 24 262');drop.setAttribute('class','cord-drop');drop.setAttribute('aria-hidden','true');
    var hang='M6 0C'+(6+sway).toFixed(1)+' 70 '+(12+sway).toFixed(1)+' 170 12 262';
    drop.innerHTML='<path class="cord-wire" d="'+hang+'"/><path class="cord-shine" d="'+hang+'" transform="translate(-.7 0)"/>';
    var box=document.createElement('span');box.className='cord-box';
    var boxFront=document.createElement('span');boxFront.className='cord-box-front';
    [[box,'wall'],[boxFront,'wall'],[drop,'wall'],[floor,'floor'],[wallPiece,'wall']].forEach(function(piece){
      piece[0].setAttribute('aria-hidden','true');piece[0].dataset.roomLayer='cord';piece[0].dataset.zone=piece[1];c.appendChild(piece[0]);
    });
  }
  // Living screens: a machine within two plays a few seconds of its game (tools/make_attract_strips.mjs, 12 frames),
  // farther ones show their still; each casts a pool of its colour on the carpet, brighter as you come near.
  var LOOP_NEAR=2;
  var screens=cards.map(function(c){
    var rom=c.querySelector('.game-rom');
    if(!rom)return null;
    var box=document.createElement('span'),frame=document.createElement('span'),picture=document.createElement('picture'),source=document.createElement('source'),strip=document.createElement('img');
    box.className='card-loop';box.setAttribute('aria-hidden','true');frame.className='card-loop-frame';
    picture.className='card-loop-picture';source.type='image/webp';
    strip.className='card-loop-strip';strip.alt='';strip.decoding='async';
    var fallbackUsed=false;
    function fallbackLoop(){
      if(fallbackUsed)return;
      fallbackUsed=true;
      if(source.parentNode)source.remove();
    }
    // Brawler fills its CRT; the other cabinets preserve their stills' aspect ratio.
    function showLoop(){
      var shape=strip.naturalWidth/(strip.naturalHeight/12),glass=rom.clientWidth/Math.max(1,rom.clientHeight);
      if(c.dataset.machine==='brawler') { frame.style.width='100%'; frame.style.height='100%'; }
      else {
        frame.style.width=(shape<glass?100*shape/glass:100).toFixed(2)+'%';
        frame.style.height=(shape<glass?100:100*glass/shape).toFixed(2)+'%';
      }
      frame.style.backgroundImage='url('+JSON.stringify(strip.currentSrc||strip.src)+')';
      c.setAttribute('data-loop-ready','');
    }
    strip.addEventListener('load',function(){
      if(strip.decode)strip.decode().then(showLoop,fallbackLoop);
      else showLoop();
    });
    strip.addEventListener('error',fallbackLoop);
    picture.appendChild(source);picture.appendChild(strip);frame.appendChild(picture);box.appendChild(frame);
    var still=rom.querySelector('.card-shot');
    rom.insertBefore(box,still?still.nextSibling:rom.firstChild);
    var pool=roomLayer('span','card-pool','pool','floor');c.appendChild(pool);
    dressCabinet(c);
    var slug=(c.getAttribute('data-game-url')||'').split('/')[0];
    return {strip:strip,source:source,pool:pool,near:false,glow:'',src:slug+'/attract.webp',fallback:slug+'/attract.png'};
  });
  // Room between the machines for two players at each.
  function aisleSpacing(room) {
    return 300*room.scale*1.6;
  }
  // The machines stand in a row against the wall, side by side and all alike: walking slides the row past.
  function cardPose(c,delta,room) {
    return {x:delta*aisleSpacing(room),y:0,z:0,turn:0,tilt:0,pitch:0};
  }
  // The cabinet's scale for the room. Short and wide (a phone on its side): the player stands close, cut off below Start.
  function aisleRoom() {
    var track=document.getElementById('game-grid'),w=track.clientWidth,h=track.clientHeight;
    var close=w>h*1.4&&h<520;
    track.toggleAttribute('data-close',close);
    // Start sits 459 authored pixels down, with a minimum 30.4px touch target.
    var css=getComputedStyle(track),closeScale=Math.min(h*.95/510,(h*.97-32)/459);
    var scale=Math.max(.3,Math.min(close?closeScale:h*.78/660,(w-56)/300));
    var trimHeight=Math.max(parseFloat(css.getPropertyValue('--room-trim-min'))||24,
      Math.min(h*(parseFloat(css.getPropertyValue('--room-trim-bottom'))-parseFloat(css.getPropertyValue('--room-trim-top')))/100,
        parseFloat(css.getPropertyValue('--room-trim-max'))||96));
    var depth=roomGeometry(track,w,h,scale,close,trimHeight);
    // An integer number of mosaic repeats per circuit keeps both wall and aisle wraps seamless.
    var wallLap=cards.length*aisleSpacing({scale:scale})*depth.rate;
    var tileSize=Math.max(14,Math.min(32,300*scale*.08));
    var tilesPeriod=wallLap/Math.max(1,Math.round(wallLap/(32*tileSize)));
    track.style.setProperty('--room-tiles-period',tilesPeriod+'px');
    track.style.setProperty('--room-tile-size',tilesPeriod/32+'px');
    var tileCopies=Math.ceil(w/tilesPeriod)+2;
    while(tiles.children.length<tileCopies){
      var copy=tiles.firstElementChild.cloneNode(true),suffix='-'+tiles.children.length;
      copy.querySelectorAll('[id]').forEach(function(def){
        var id=def.id;def.id=id+suffix;
        copy.querySelectorAll('[fill="url(#'+id+')"]').forEach(function(shape){shape.setAttribute('fill','url(#'+def.id+')');});
      });
      tiles.appendChild(copy);
    }
    while(tiles.children.length>tileCopies)tiles.lastElementChild.remove();
    return {width:w,scale:scale,wallRate:depth.rate,wallZ:depth.z,tilesPeriod:tilesPeriod,
      wallPeriod:parseFloat(css.getPropertyValue('--room-wall-period'))||120,
      carpetPeriod:parseFloat(css.getPropertyValue('--room-carpet-period'))||86};
  }
  // The room in depth: the machines' fronts stand at the grid's plane and the wall 215px behind them (a cabinet
  // is 200 deep), seen through #game-grid's perspective, 1100px from 38% of its height. The wall therefore
  // scrolls at wallRate of the machines' pace, and its foot, where the floor meets it, is --room-floor-y.
  var WALL_BACK=215;
  function roomGeometry(track,w,h,scale,close,trimHeight) {
    var feet=close?h*.03+660*scale:h*.97,top=feet-660*scale,wallZ=WALL_BACK*scale,wallRate=1100/(1100+wallZ),origin=h*.38;
    var place={'--room-cab-w':Math.round(300*scale)+'px','--room-cab-top':Math.round(top)+'px','--room-side-top':Math.round(top)+'px',
      '--room-trim-px':trimHeight+'px','--room-headroom-px':Math.max(0,top-trimHeight)+'px',
      '--room-feet-y':Math.round(feet)+'px','--room-wall-z':Math.round(wallZ)+'px','--room-floor-y':Math.round(origin+(feet-origin)*wallRate)+'px'};
    for(var name in place)if(track.style.getPropertyValue(name)!==place[name])track.style.setProperty(name,place[name]);
    track.dataset.wallRate=wallRate.toFixed(4);
    return {rate:wallRate,z:Math.round(wallZ)};
  }
  function aisleOffset(index) {
    return BlipAisle.offset(index,aisle.pos,cards.length);
  }
  // Every machine stands where the walk has got to. Transform and opacity only: this runs every frame.
  function poseCards(room,bob) {
    var spacing=aisleSpacing(room),closer=lean*LEAN_Z,lift=-lean*LEAN_LIFT*room.scale;
    cards.forEach(function(c,i){
      if(!c.classList.contains('card'))return;
      var offset=aisleOffset(i),pose=cardPose(c,offset,room),distance=Math.abs(offset);
      var scale=room.scale;
      c.style.transform='translateX(calc(-50% + '+pose.x.toFixed(1)+'px)) translateY('+((bob||0)+lift).toFixed(2)+'px) translateZ('+(pose.z+closer).toFixed(1)+'px) rotateY('+pose.turn.toFixed(2)+'deg) scale('+scale.toFixed(4)+')';
      var z=String(30-Math.min(Math.round(distance*2),25));
      if(c.style.zIndex!==z)c.style.zIndex=z;
      // Past three and a half machines a cabinet is out of sight, and crosses to the aisle's other end unseen.
      var away=distance>3.5;
      if(c.aisleAway!==away){c.aisleAway=away;c.style.visibility=away?'hidden':'';}
      var screen=screens[i];
      if(!screen)return;
      var near=distance<=LOOP_NEAR;
      if(screen.near!==near){
        screen.near=near;
        if(near&&!screen.strip.getAttribute('src')){
          screen.source.srcset=screen.src;
          screen.strip.src=screen.fallback;
        }
        c.toggleAttribute('data-loop',near);
      }
      var glow=BlipAisle.machineLevel(distance).toFixed(2);
      if(screen.glow!==glow){screen.glow=glow;screen.pool.style.opacity=glow;}
    });
    roomTravel+=BlipAisle.offset(aisle.pos,roomFrom,cards.length);roomFrom=aisle.pos;
    // The floor lies from the wall's foot towards you and moves with the machines; the perspective makes its
    // near side sweep past faster than its far one.
    var carpetX=BlipAisle.layerOffset(roomTravel,spacing,1,room.carpetPeriod),wallX=BlipAisle.layerOffset(roomTravel,spacing,room.wallRate,room.wallPeriod);
    carpet.style.transform='translateZ(-'+room.wallZ+'px) rotateX(90deg) translate('+carpetX.toFixed(1)+'px,0px)';
    wall.style.transform='translate('+wallX.toFixed(1)+'px,'+((bob||0)*.4).toFixed(2)+'px)';
    // The signs hang on the wall, so they scroll with it, coming round once a lap of the aisle: never more
    // than half a lap from where they hang beside the first machine, so the wrap happens out of sight.
    var lap=cards.length*spacing*room.wallRate,signX=BlipAisle.layerOffset(roomTravel,spacing,room.wallRate,lap);
    if(signX<-lap/2)signX+=lap;
    var signMove='translate('+signX.toFixed(1)+'px,'+((bob||0)*.4).toFixed(2)+'px)';
    roomSignEls.forEach(function(el){el.style.transform=signMove;});
    var tileX=BlipAisle.layerOffset(roomTravel,spacing,room.wallRate,room.tilesPeriod);
    // Avoid rounding a completed repeat onto a different subpixel raster position.
    if(Math.abs(tileX)<.1||Math.abs(tileX+room.tilesPeriod)<.1)tileX=0;
    tiles.style.transform='translate('+tileX.toFixed(1)+'px,'+((bob||0)*.4).toFixed(2)+'px)';
    grid.dataset.aislePos=(((aisle.pos%cards.length)+cards.length)%cards.length).toFixed(2);
    grid.dataset.roomTravel=roomTravel.toFixed(3);
    blipArcadeSound.place(aisle.pos);
  }
  function highlight(force) {
    if (cardInFlight&&!force) return;
    grid.scrollLeft=0;
    var cardHadFocus=cards.indexOf(document.activeElement)>=0,room=aisleRoomNow=aisleRoom();
    cards.forEach(function (c, i) {
      c.tabIndex = i === idx ? 0 : -1;
      c.classList.toggle('card-focused', navActive && i === idx);
      if(c.classList.contains('card'))c.classList.toggle('card-ready',getCoins()>0&&navActive&&i===idx);
      c.querySelectorAll('.machine-start,.machine-coinbox').forEach(function(button){button.tabIndex=i===idx?0:-1;});
      // Once you're navigating, the focus glow is the only glow —
      // hand it over from the "last played" card.
      if (navActive) c.classList.remove('card-active');
      if(!c.classList.contains('card'))return;
      var offset=Math.round(aisleOffset(i));
      c.dataset.aisleOffset=String(offset);
      c.style.setProperty('--arc-scale',room.scale.toFixed(3));
    });
    if(!aisleFrame)poseCards(room);
    if(cardHadFocus)cards[idx].focus({preventScroll:true});
    fitRoom();
  }
  window.addEventListener('resize',function(){highlight(true);});
  focusArcadeCard=function(card){var selected=cards.indexOf(card);if(selected<0)return;walkTo(selected);};
  blipArcadeSound.enable(cards.map(function(c){return (c.getAttribute('data-game-url')||'').split('/')[0];}),aisle.pos);
  highlight();
  cards[idx].focus({preventScroll:true});

  function canWalk() {
    return !(cardInFlight||!manualOverlay.hidden||document.body.classList.contains('cabinet-return'));
  }
  // Who is holding the aisle (a key, the deck's stick, a gamepad) and which way; the latest press leads.
  function hold(source,dir) {
    if(dir)held[source]={dir:dir,at:performance.now()};
    else delete held[source];
    if(dir)wakeAisle();
  }
  function heldNow() {
    var lead=null;
    for(var source in held)if(!lead||held[source].at>lead.at)lead=held[source];
    if(!lead||!canWalk())return {dir:0,heldFor:0,speed:1};
    return {dir:lead.dir,heldFor:(performance.now()-lead.at)/1000,speed:lead===held.pad?padSpeed():1};
  }
  // A gamepad's stick sets the pace: just past its dead zone a stroll, pushed right over a brisk walk.
  function padSpeed() {
    var pads=navigator.getGamepads?navigator.getGamepads():[];
    for(var i=0;i<pads.length;i++){
      var pad=pads[i];if(!pad||!pad.connected)continue;
      var x=Math.abs(pad.axes[0]||0);
      return x>.25?.3+(Math.min(x,1)-.25)/.75*1.2:1;
    }
    return 1;
  }
  function wakeAisle() {
    if(aisleFrame)return;
    aisleRoomNow=aisleRoomNow||aisleRoom();
    aisleLast=0;
    if(aisle.settled)walked=.45;
    grid.setAttribute('data-aisle-moving','');
    aisleFrame=requestAnimationFrame(aisleTick);
  }
  function aisleTick(now) {
    aisleFrame=0;
    var dt=aisleLast?(now-aisleLast)/1000:1/60,before=aisle.pos;
    aisleLast=now;
    BlipAisle.step(aisle,heldNow(),dt);
    // A footstep every half machine walked, the view dipping a little on each.
    var moved=Math.abs(aisle.pos-before);
    if(moved<1){
      walked+=moved;stride+=moved;
      while(walked>=.5){walked-=.5;playMachineStep(Math.sign(aisle.pos-before)||1,1);}
    }
    var bob=-2.5*Math.min(1,Math.abs(aisle.v))*Math.abs(Math.sin(stride*2*Math.PI));
    var leanTo=steppedUp?1:0;
    lean+=(leanTo-lean)*(1-Math.exp(-dt/.12));
    if(Math.abs(leanTo-lean)<.003)lean=leanTo;
    poseCards(aisleRoomNow,bob);
    if(aisle.settled&&!aisle.drag&&!heldNow().dir&&lean===leanTo){arriveAisle();return;}
    aisleFrame=requestAnimationFrame(aisleTick);
  }
  function arriveAisle() {
    grid.removeAttribute('data-aisle-moving');
    stride=0;
    var previous=idx;
    idx=BlipAisle.index(aisle.pos,cards.length);
    navActive=true;
    highlight();
    if(pendingStart){pendingStart=false;activate();}
  }
  // One machine along the aisle: a tap, an arrow button, a short swipe.
  function step(dir) {
    if(!canWalk())return;
    stepBack();
    if(stepWalk.matches){move(dir);return;}
    pendingStart=false;
    BlipAisle.press(aisle,dir);
    wakeAisle();
  }
  function walkTo(index) {
    if(!canWalk())return;
    stepBack();
    if(stepWalk.matches){idx=index;aisle=BlipAisle.create(cards.length,idx);navActive=true;highlight(true);return;}
    var base=aisle.target!=null?aisle.target:Math.round(aisle.pos);
    aisle.target=base+Math.round(BlipAisle.offset(index,base,cards.length));
    aisle.settled=false;
    wakeAisle();
  }

  function move(delta) {
    if (!canWalk()) return;
    stepBack();
    if(walkQueue.length<cards.length*2)walkQueue.push(delta<0?-1:1);
    if(!walking)walkNext();
  }
  function walkNext() {
    if(cardInFlight||!manualOverlay.hidden) {
      walkQueue.length=0;walking=false;return;
    }
    if(!walkQueue.length){walking=false;return;}
    walking=true;
    var delta=walkQueue.shift();
    navActive = true;
    var previous=idx;
    idx = (idx + delta + cards.length) % cards.length;
    aisle=BlipAisle.create(cards.length,idx);
    if(idx!==previous) {
      highlight();
      playMachineStep(delta);
    }
    window.setTimeout(walkNext,2240);
  }

  // Up steps up to the machine faced: the view leans in, its controls light and the room quietens. Up again
  // plays it; Down, or walking on, steps back.
  function stepUp() {
    if(!canWalk()||!aisle.settled)return;
    if(steppedUp){activate();return;}
    setStepped(true);
  }
  function stepBack() {
    if(steppedUp)setStepped(false);
  }
  arcadeStepBack=stepBack;
  arcadeStepIn=function(){if(!steppedUp)setStepped(true);};
  function setStepped(on) {
    steppedUp=on;
    grid.toggleAttribute('data-stepped-up',on);
    blipArcadeSound.duck(on);
    if(stepWalk.matches){lean=on?1:0;poseCards(aisleRoomNow||aisleRoom());return;}
    wakeAisle();
  }

  function activate() {
    if(cardInFlight||!manualOverlay.hidden)return;
    if(aisleFrame){pendingStart=true;return;}
    if (!navActive) { navActive = true; highlight(); }
    attemptStart(cards[idx]);
  }

  // The aisle arrows stay beside the machine feet, clear of each coin door.
  (function () {
    var bezel=document.getElementById('screen-bezel');
    if(!bezel)return;
    var bar=document.createElement('div');
    bar.className='card-pick';bar.id='card-pick';
    bar.innerHTML='<button type="button" class="pick-arrow pick-prev" aria-label="Walk to previous machine"><i></i></button>'+
      '<button type="button" class="pick-arrow pick-next" aria-label="Walk to next machine"><i></i></button>';
    bezel.appendChild(bar);
    [['.pick-prev',-1],['.pick-next',1]].forEach(function(entry){
      bar.querySelector(entry[0]).addEventListener('click',function(){step(entry[1]);this.blur();});
    });
  }());

  // A horizontal swipe walks one or more machines down the aisle.
  var track = document.getElementById('game-grid');
  var touchStart = null, suppressClick = false, suppressClickTimer = 0;
  // The chosen machine's screen says PRESS START; pressing it does. A cabinet's parts take no pointer events, so this is by place.
  function onScreen(e) {
    var glass=cards[idx]&&cards[idx].querySelector('.card-shot-wrap'),r=glass&&glass.getBoundingClientRect();
    return !!r&&e.clientX>=r.left&&e.clientX<=r.right&&e.clientY>=r.top&&e.clientY<=r.bottom;
  }
  function selectByTouch(card) {
    if(!card || cardInFlight || !manualOverlay.hidden)return;
    var selected=cards.indexOf(card);
    if(selected<0)return;
    walkTo(selected);
  }
  function suppressPointerClick() {
    suppressClick=true;window.clearTimeout(suppressClickTimer);
    suppressClickTimer=window.setTimeout(function(){suppressClick=false;},0);
  }
  // Past this a drag counts as walking; short of it the aisle springs back to where it was.
  function swipeEnough(dx,dy) {
    return Math.abs(dx)>Math.max(38,track.clientWidth*.09) && Math.abs(dx)>Math.abs(dy)*1.2;
  }
  track.addEventListener('pointerdown', function (e) {
    if (e.button !== undefined && e.button !== 0) return;
    if(e.target.closest&&e.target.closest('.machine-coinbox,.machine-start'))return;
    touchStart = {id:e.pointerId, x:e.clientX, y:e.clientY, card:e.target.closest && e.target.closest('.card'), screen:onScreen(e), unit:track.clientWidth*.34, dragging:false};
    if(e.pointerType==='touch' && track.setPointerCapture)track.setPointerCapture(e.pointerId);
  });
  // The aisle follows the finger, a machine for each third of the screen it travels.
  track.addEventListener('pointermove', function (e) {
    var start=touchStart;
    if(!start || start.id!==e.pointerId || stepWalk.matches)return;
    var dx=e.clientX-start.x,dy=e.clientY-start.y;
    if(!start.dragging) {
      if(!canWalk() || Math.abs(dx)<=12 || Math.abs(dx)<=Math.abs(dy)*1.2)return;
      start.dragging=true;start.dx0=dx;
      pendingStart=false;
      stepBack();
      if(track.setPointerCapture)try{track.setPointerCapture(e.pointerId);}catch(err){}
      BlipAisle.dragStart(aisle,performance.now());
      start.from=aisle.pos;
      wakeAisle();
    }
    BlipAisle.dragTo(aisle,start.from-(dx-start.dx0)/start.unit,performance.now());
  });
  function endDrag(start,e) {
    var dx=e.clientX-start.x,dy=e.clientY-start.y;
    BlipAisle.dragTo(aisle,start.from-(dx-start.dx0)/start.unit,performance.now());
    BlipAisle.dragEnd(aisle,performance.now(),!swipeEnough(dx,dy));
    wakeAisle();
    suppressPointerClick();
  }
  // Walking a step at a time, a swipe moves one machine per machine-width it covered.
  function stepSwipe(start,dx,dy) {
    if(!stepWalk.matches || !swipeEnough(dx,dy))return false;
    for(var n=Math.max(1,Math.round(Math.abs(dx)/start.unit));n>0;n--)move(dx<0?1:-1);
    suppressPointerClick();
    return true;
  }
  track.addEventListener('pointerup', function (e) {
    var start=touchStart;
    touchStart=null;
    if(!start || start.id!==e.pointerId)return;
    if(start.dragging){endDrag(start,e);e.preventDefault();return;}
    if(cardInFlight || !manualOverlay.hidden)return;
    var dx=e.clientX-start.x,dy=e.clientY-start.y;
    if(stepSwipe(start,dx,dy)){e.preventDefault();return;}
    if(Math.hypot(dx,dy)>12 || e.pointerType!=='touch')return;
    // A tap on a machine's screen plays it, once that machine is the one in front.
    if(start.screen && !(start.card&&!start.card.classList.contains('card-focused')))activate();
    else selectByTouch(start.card);
    e.preventDefault();
    suppressPointerClick();
  });
  track.addEventListener('pointercancel', function(e){
    var start=touchStart;
    touchStart=null;
    if(!start || start.id!==e.pointerId)return;
    if(start.dragging){endDrag(start,e);return;}
    if(e.pointerType!=='touch' || cardInFlight || !manualOverlay.hidden)return;
    stepSwipe(start,e.clientX-start.x,e.clientY-start.y);
  });
  track.addEventListener('click',function(e){
    if(suppressClick){suppressClick=false;window.clearTimeout(suppressClickTimer);return;}
    var card=e.target.closest&&e.target.closest('.card');
    if(card&&!card.classList.contains('card-focused')){if(focusArcadeCard)focusArcadeCard(card);return;}
    if(onScreen(e)){activate();return;}
  });

  // A press is a tap of one machine; held, it walks until let go. Reduced motion keeps the key's own repeat.
  function walkKey(e,dir) {
    if(stepWalk.matches){move(dir);return;}
    if(e.repeat)return;
    step(dir);
    hold('key:'+e.code,dir);
  }
  document.addEventListener('keyup',function(e){if(held['key:'+e.code])hold('key:'+e.code,0);});
  window.addEventListener('blur',function(){held={};});
  document.addEventListener('keydown', function (e) {
    if(e.target.closest&&e.target.closest('input,textarea,select,[contenteditable]'))return;
    var control=e.target.closest&&e.target.closest('button,a');
    if(control&&(e.code==='Space'||e.code==='Enter'))return;
    if(control&&/^(Arrow|Key[WASD])/.test(e.code))control.blur();
    switch (e.code) {
      case 'KeyD': case 'ArrowRight': walkKey(e,1);  e.preventDefault(); break;
      case 'KeyA': case 'ArrowLeft':  walkKey(e,-1); e.preventDefault(); break;
      case 'KeyS': case 'ArrowDown': case 'Escape': stepBack(); e.preventDefault(); break;
      case 'KeyW': case 'ArrowUp':    if(!e.repeat)stepUp(); e.preventDefault(); break;
      case 'KeyF': case 'KeyG': case 'Space': case 'Enter': activate(); e.preventDefault(); break;
    }
  });

  pollGamepad(
    function (code) {
      if (code === 'ArrowRight') { step(1); if(!stepWalk.matches)hold('pad',1); }
      else if (code === 'ArrowLeft') { step(-1); if(!stepWalk.matches)hold('pad',-1); }
      else if (code === 'ArrowDown') stepBack();
      else if (code === 'ArrowUp') stepUp();
      else if (code === 'Select') insertFocusedCoin();
      else if (code === 'Space'||code === 'KeyZ'||code === 'Start') activate();
    },
    function (code) {
      if ((code === 'ArrowRight'||code === 'ArrowLeft') && held.pad) hold('pad',0);
    }
  );
}());

function insertFocusedCoin() {
  if(cardInFlight || !manualOverlay.hidden || document.body.classList.contains('cabinet-return'))return;
  var coinbox=document.querySelector('.card-focused .machine-coinbox');
  if(coinbox)coinbox.click();
}
document.addEventListener('keydown', function(e) {
  if ((e.key === '5' || e.key === 'c' || e.key === 'C')&&!e.repeat &&
      !(e.target.closest&&e.target.closest('input,textarea,select,[contenteditable]'))) {
    insertFocusedCoin();
    e.preventDefault();
  }
  konamiCheck(e.key);
});

var KONAMI = ['ArrowUp','ArrowUp','ArrowDown','ArrowDown','ArrowLeft','ArrowRight','ArrowLeft','ArrowRight','b','a'];
var konamiIdx = 0;
function konamiCheck(key) {
  konamiIdx = (key === KONAMI[konamiIdx]) ? konamiIdx + 1 : (key === KONAMI[0] ? 1 : 0);
  if (konamiIdx === KONAMI.length) {
    konamiIdx = 0;
    konamiActivate();
  }
}
function konamiActivate() {
  var n = getCoins();
  var added = Math.min(5, MAX_COINS - n);
  saveCoins(Math.min(n + 5, MAX_COINS));
  updateCoinsHud();
  updateCardDimming();
  for (var i = 0; i < added; i++) {
    setTimeout(playCoinInsert, i * 90);
  }
  document.body.classList.remove('konami-shake');
  void document.body.offsetWidth;
  document.body.classList.add('konami-shake');
  document.body.addEventListener('animationend', function() {
    document.body.classList.remove('konami-shake');
  }, { once: true });
}

var directManualChapter=new URLSearchParams(location.search).get('manual');
if(directManualChapter){var directIndex=manualChapters.indexOf(directManualChapter+'.html');if(directIndex>=0)requestAnimationFrame(function(){openFieldManual(directIndex,new URLSearchParams(location.search).get('game'));});}

// Reveal the assembled rack after its artwork and shared controls are ready.
document.addEventListener('DOMContentLoaded', function () {
  Promise.all(Array.from(document.querySelectorAll('#game-grid .card-shot'), function (image) {
    return image.decode().catch(function () {});
  })).then(function () {
    requestAnimationFrame(function () {
      cardFitKey='';
      fitRoom();
      requestAnimationFrame(rollOutArcade);
    });
  });
}, {once:true});
