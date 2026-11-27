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
}

updateCreditsDisplay(); // initialise dimming to match coins on page load

// Start opens the selected machine; its credit is spent by the game after the first frame.
var cardInFlight = false, pendingCard = null, focusArcadeCard = null, arcadeStepBack = null, arcadeStepIn = null, arcadeWalk = null, arcadeWalkOut = null;
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
  ['left','right'].forEach(function(side){var panelSide=document.createElement('span');panelSide.className='machine-side '+side;panelSide.setAttribute('aria-hidden','true');card.appendChild(panelSide);});
  var controls=document.createElement('div');controls.className='machine-controls';
  controls.innerHTML='<button type="button" class="machine-start" aria-label="Start '+name+'"><span class="machine-start-cap">START</span></button>';
  // The deck is the game page's own, the same <blip-deck>, so the cabinet's controls are the ones the player will use.
  var deck=document.createElement('div');deck.className='machine-deck';
  deck.innerHTML='<iframe src="machine-deck.html?game='+slug+'" tabindex="-1" aria-hidden="true" title="" onload="this.dataset.loaded=\'\'"></iframe>';
  controls.appendChild(deck);
  ['left','right'].forEach(function(side){var cheek=document.createElement('span');cheek.className='machine-deck-cheek '+side;cheek.setAttribute('aria-hidden','true');controls.appendChild(cheek);});
  card.appendChild(controls.querySelector('.machine-start'));
  // A sticker a small fan slapped on the cabinet, just over Start where she could reach.
  var sticker=(BLIP_GAMES[slug]||{}).sticker;
  if(sticker){var tag=document.createElement('span');tag.className='machine-sticker';tag.setAttribute('aria-hidden','true');tag.innerHTML='<b></b>';tag.firstChild.textContent=sticker;card.appendChild(tag);}
  card.querySelector('.machine-start').addEventListener('click',function(event){
    event.preventDefault();event.stopPropagation();
    if(!card.classList.contains('card-focused')){if(focusArcadeCard)focusArcadeCard(card);return;}
    attemptStart(card);
  });
  card.insertBefore(controls,card.querySelector('.machine-start'));
  var coinbox=document.createElement('button');
  coinbox.type='button';coinbox.className='machine-coinbox';coinbox.setAttribute('aria-label','Insert a coin at the '+(title?title.textContent.trim():'')+' machine');
  coinbox.innerHTML='<span class="machine-coin-mech" aria-hidden="true"><span class="machine-coin-slot"></span><span class="machine-coin-price">25¢</span></span>';
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
  var nativeTransition=blipNativePageTransitions();
  try { sessionStorage.setItem('blip-cabinet-launch',JSON.stringify({slug:machineSlug,at:Date.now(),art:new URL(art,location.href).href,native:nativeTransition})); } catch(e) {}
  // The player walks up to the machine until its picture is where the game page shows it, then the page takes
  // over: the same machine, near (a cross-document transition's crossfade, or straight on without one).
  // The machine's monitor onto the game page's.
  prerender(gameCardUrl(card));
  // Where the game page's canvas is known, the walk ends with the cabinet's picture on its picture; else monitor on monitor.
  var machine=gameMachine(),shotEl=card.querySelector('.card-shot'),aspect=shotEl&&shotEl.naturalWidth?shotEl.naturalWidth/shotEl.naturalHeight:0;
  var walk=machine.box&&aspect?[pictureBox(machine.box,aspect),function(c){return cardPicture(c,aspect);}]:[machine.monitor,cardMonitor];
  arcadeWalk(card,walk[0],walk[1],WALK_MS,false).then(function(){window.location.href=gameCardUrl(card);});
}
function pictureBox(box,a){var r=containIn({left:box.x,top:box.y,width:box.w,height:box.h},a);return {x:r.left,y:r.top,w:r.width,h:r.height};}
// 1.8 s: at 1.4 the middle of the walk read as a rush on a phone (user, 2026-11-27).
var WALK_MS=1800;
window.addEventListener('pageswap',function(event){
  if(!event.viewTransition)return;
  if(!document.querySelector('#game-grid .card-launching'))event.viewTransition.skipTransition();
});
// Back from a game: the machine's screen holds the game's last frame, and the player walks back from it into the
// aisle while the pages cross-fade.
window.addEventListener('pagereveal',function(event){
  var back=window.blipMachineReturn;window.blipMachineReturn=null;
  if(!event.viewTransition){document.documentElement.removeAttribute('data-machine-return');return;}
  var card=back&&document.querySelector('#game-grid .card[data-game-url^="'+back.slug+'/"]');
  if(!card){document.documentElement.removeAttribute('data-machine-return');event.viewTransition.skipTransition();return;}
  card.classList.add('card-launching');
  // Stand where the game page's screen was, in front of the machine, then walk back from it into the aisle.
  // Picture on picture where the game page sent its canvas (and the frame drawn on it), else monitor on monitor.
  var frameAspect=back.picture&&back.picture.w/back.picture.h,walked=Promise.resolve();
  if(typeof arcadeWalk==='function'&&(back.picture||back.screen))
    walked=arcadeWalk(card,back.picture||back.screen,back.picture?function(c){return cardPicture(c,frameAspect);}:cardMonitor,WALK_MS,true);
  var shot=card.querySelector('.card-shot');
  if(shot&&/^data:image\/png;base64,/.test(back.frame||'')){shot.dataset.still=shot.getAttribute('src');shot.src=back.frame;}
  var faded=event.viewTransition.finished.catch(function(){});
  faded.then(function(){document.documentElement.removeAttribute('data-machine-return');});
  // The screen keeps the game's frame until the walk is over: switching mid-walk was the one jump on the way back.
  Promise.all([faded,walked]).then(function(){
    card.classList.remove('card-launching');
    if(shot&&shot.dataset.still){shot.src=shot.dataset.still;delete shot.dataset.still;}
  });
});
// The game page made ready while the player walks up to it, so it is there when they arrive (where the browser
// can: speculation rules).
function prerender(url) {
  if(!(HTMLScriptElement.supports&&HTMLScriptElement.supports('speculationrules')))return;
  var rules=document.createElement('script');rules.type='speculationrules';
  rules.textContent=JSON.stringify({prerender:[{source:'list',urls:[new URL(url,location.href).href]}]});
  document.head.appendChild(rules);
}
// The game page's machine at this window size: its monitor and deck, as it last measured itself here (game.js,
// blip-screen-box), or else as it lays itself out on a computer: the deck as deep as it last was (180 at first)
// along the bottom, the marquee bar's 78 above, and between them the monitor: on a computer in the cabinet's shape, at
// most 960; on a phone the width less 16 each side.
function gameMachine() {
  var saved=null;
  try { saved=JSON.parse(localStorage.getItem('blip-screen-box')||'null'); } catch(e) {}
  if(saved&&saved.w===innerWidth&&saved.h===innerHeight&&saved.monitor&&saved.panel)
    return {monitor:{x:saved.monitor[0],y:saved.monitor[1],w:saved.monitor[2],h:saved.monitor[3]},panel:{x:saved.panel[0],y:saved.panel[1],w:saved.panel[2],h:saved.panel[3]},
      box:saved.box&&{x:saved.box[0],y:saved.box[1],w:saved.box[2],h:saved.box[3]}};
  // A deck depth that could not be the page's (a stale record) is not taken.
  var wide=innerWidth>=1017,deck=saved&&saved.deck>=100&&saved.deck<=innerHeight*.4?saved.deck:(wide?180:150);
  var top=78,h=innerHeight-top-deck,w=wide?blipCabinetWidth(h):innerWidth-32,x=(innerWidth-w)/2;
  return {monitor:{x:x,y:top,w:w,h:h},panel:{x:x,y:top+h,w:w,h:deck}};
}
// A cabinet's monitor where it is drawn now: what the walks in and back measure.
function cardMonitor(card) {return card.querySelector('.card-shot-wrap').getBoundingClientRect();}
// The picture on a cabinet's screen where it is drawn now: its image (of aspect a, or its own) contained in the tube.
// The walks end with it over the game page's picture, so the cross-fade lays one picture on the same picture.
function cardPicture(card,a) {
  var shot=card.querySelector('.card-shot'),r=(card.querySelector('.game-rom')||card.querySelector('.card-shot-wrap')).getBoundingClientRect();
  return containIn(r,a||(shot&&shot.naturalWidth?shot.naturalWidth/shot.naturalHeight:r.width/r.height));
}
function containIn(r,a) {var w=Math.min(r.width,r.height*a),h=w/a;return {left:r.left+(r.width-w)/2,top:r.top+(r.height-h)/2,width:w,height:h};}
// Back from a game through the browser's page cache: stand back from the machine.
window.addEventListener('pageshow',function(event){
  if(!event.persisted)return;
  if(typeof arcadeStepBack==='function')arcadeStepBack();
  if(typeof arcadeWalkOut==='function')arcadeWalkOut();
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
  // leanP is the step's progress, eased into lean (smoothstep over LEAN_S): an exponential set off at full speed.
  var steppedUp=false,lean=0,leanP=0,LEAN_Z=150,LEAN_LIFT=60,LEAN_S=.45;
  // #game-grid's eye, 1100px in front of the room (arcade.css perspective), and the wall 215 behind the machines'
  // fronts (room.js WALL).
  var EYE=1100,WALL_BACK=-BlipRoom.WALL;
  // How far the walk has gone, in machines. The aisle's own position wraps round the loop when it settles;
  // this keeps counting (tests and captures read it).
  var roomTravel=aisle.pos,roomFrom=aisle.pos;
  // The wall and the floor (arcade.css .room-wall, .room-floor) slide past with the machines as the walk goes.
  var roomWall=document.createElement('div'),roomFloor=document.createElement('div');
  roomWall.className='room-wall';roomFloor.className='room-floor';
  roomWall.setAttribute('aria-hidden','true');roomFloor.setAttribute('aria-hidden','true');
  grid.insertBefore(roomFloor,grid.firstChild);grid.insertBefore(roomWall,roomFloor);
  // Each cabinet is the game page's machine seen from farther: where that page's monitor has the cabinet's shape
  // (267 x 219, at 16.5, 84 on its 300 wide front), its deck is the page's deck, as wide and deep and where it is
  // under the monitor, scaled as the monitor is; the deck's page is laid out at the game page's size and cropped to it.
  function fitDecks(){
    var machine=gameMachine(),style=grid.style;
    var names=['--deck-x','--deck-y','--deck-w','--deck-height','--deck-page-w','--deck-page-h','--deck-shift','--deck-shift-x','--deck-scale','--deck-squeeze',
      '--monitor-ring','--monitor-radius','--rom-inset','--rom-radius','--rom-ring'];
    if(Math.abs(machine.monitor.w/machine.monitor.h-BLIP_MONITOR_ASPECT)>.02){names.forEach(function(n){style.removeProperty(n);});return;}
    var m=machine.monitor,p=machine.panel,k=267/m.w;
    // With its housing, which stands 8 out each side and 10 above the deck (blip.css .deck-housing).
    var sx=8,sy=10;
    style.setProperty('--deck-x',(16.5+(p.x-sx-m.x)*k).toFixed(2)+'px');
    style.setProperty('--deck-y',(84+(p.y-sy-m.y)*k).toFixed(2)+'px');
    style.setProperty('--deck-w',((p.w+2*sx)*k).toFixed(2)+'px');
    style.setProperty('--deck-height',((p.h+sy)*k).toFixed(2)+'px');
    // The deck's page is the game page's whole window (its dials are sized by it), its deck as wide as the game
    // page's machine (deckWidth), cropped to the deck.
    style.setProperty('--deck-page-w',innerWidth+'px');
    style.setProperty('--deck-page-h',innerHeight+'px');
    style.setProperty('--deck-shift',(p.h+sy-innerHeight)+'px');
    style.setProperty('--deck-shift-x',(sx-p.x)+'px');
    style.setProperty('--deck-scale',k.toFixed(4));
    style.setProperty('--deck-squeeze','1');
    deckWidth=p.w;
    document.querySelectorAll('.machine-deck iframe').forEach(sizeDeck);
    // The monitor's frame and tube in the game page's pixels (blip.css --machine-monitor-ring, game.js's 15px inset,
    // the glass's 16/20 corners and 3px edge), scaled as the monitor is.
    var px=function(n){return (n*k).toFixed(2)+'px';};
    style.setProperty('--monitor-ring','inset 0 0 0 '+px(4)+' #000,inset 0 0 0 '+px(6)+' color-mix(in srgb,var(--trim) 70%,#fff),inset 0 0 0 '+px(8)+' #050507,0 4px 10px #000c');
    style.setProperty('--monitor-radius',px(6));
    style.setProperty('--rom-inset',px(15));
    style.setProperty('--rom-radius',px(16)+' / '+px(20));
    style.setProperty('--rom-ring','0 0 0 '+px(3)+' #000');
  }
  var deckWidth=0;
  function sizeDeck(frame){
    try{var root=frame.contentDocument&&frame.contentDocument.documentElement;if(!root)return;
      if(deckWidth)root.style.setProperty('--cabinet-width',deckWidth+'px');else root.style.removeProperty('--cabinet-width');}catch(e){}
  }
  grid.addEventListener('load',function(e){if(e.target.tagName==='IFRAME')sizeDeck(e.target);},true);
  fitDecks();
  window.addEventListener('resize',fitDecks);
  // A few things in the room (web/room.js), each standing in the gap to a machine's right, in its 3D space.
  var THINGS={rally:'plant',raider:'pots',meteors:'shield',serpent:'sword'};
  cards.forEach(function(c){
    var name=THINGS[c.getAttribute('data-machine')];
    if(name)BlipRoom.build(document,c,name);
  });
  // Each machine's shadow on the wall behind it (arcade.css).
  function shadow(c){
    ['card-backlight','card-floorshadow'].forEach(function(name){
      var el=document.createElement('span');el.className=name;el.setAttribute('aria-hidden','true');c.appendChild(el);
    });
  }
  // Living screens: a machine within two plays a few seconds of its game (tools/make_attract_strips.mjs, 12 frames),
  // farther ones show their still.
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
    shadow(c);
    var slug=(c.getAttribute('data-game-url')||'').split('/')[0];
    return {strip:strip,source:source,near:false,src:slug+'/attract.webp',fallback:slug+'/attract.png'};
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
    var closeScale=Math.min(h*.95/510,(h*.97-32)/459);
    var scale=Math.max(.3,Math.min(close?closeScale:h*.78/660,(w-56)/300));
    // The wall stands 215px behind the machines' fronts (a cabinet is 200 deep), seen through #game-grid's
    // perspective, 1100px from 38% of its height: --room-floor-y is where the floor meets it.
    var feet=close?h*.03+660*scale:h*.97,origin=h*.38,rate=EYE/(EYE+WALL_BACK*scale);
    var floorY=Math.round(origin+(feet-origin)*rate)+'px';
    if(track.style.getPropertyValue('--room-floor-y')!==floorY)track.style.setProperty('--room-floor-y',floorY);
    return {width:w,scale:scale,rate:rate,feet:feet,eye:origin,height:h};
  }
  // The wall moves at its depth's rate, modulo its patterns' 288px repeat (the width it stands out past the room's
  // right side). The floor lies flat in the room, from the wall to FLOOR_FRONT in front of the machines (far enough
  // to pass under the room's bottom edge), so the perspective draws its boards going away; it moves with the machines.
  var FLOOR_FRONT=250;
  // Where the player has walked to from where they stand in the aisle: toward a machine's screen and back (walkTo).
  var cam={x:0,y:0,z:0};
  // While the eye walks in, what it has left behind is not drawn: machines two or more along once it is under way,
  // the neighbours' decks once half-way (each costs a frame at this pace; all are out of view by then).
  var walking=false;
  function walkHides(distance){return walking&&distance>=1.5&&cam.z>150;}
  function camera(){return 'translate3d('+cam.x.toFixed(1)+'px,'+cam.y.toFixed(1)+'px,'+cam.z.toFixed(1)+'px)';}
  // cam3d: this frame's camera() (poseCards).
  function slideRoom(room,cam3d) {
    var walked=roomTravel*aisleSpacing(room),back=Math.round(WALL_BACK*room.scale);
    // The wall stands back: the player's steps toward it bring it nearer by less (an element at z = 0 moved t toward
    // the eye looks as near as one at -W moved t (EYE + W) / EYE).
    var far=EYE/(EYE+back);
    roomWall.style.transform='translate3d('+(cam.x*far).toFixed(1)+'px,'+(cam.y*far).toFixed(1)+'px,'+(cam.z*far).toFixed(1)+'px) translateX('+(-(((walked*room.rate)%288+288)%288)).toFixed(1)+'px)';
    if(roomFloor.depth!==back){roomFloor.depth=back;roomFloor.style.height=(FLOOR_FRONT+back)+'px';}
    // Where the machines' feet are: the bottom of the room, or below it close up.
    var under=(room.height-room.feet).toFixed(1)+'px';
    if(roomFloor.style.bottom!==under)roomFloor.style.bottom=under;
    roomFloor.style.transform=cam3d+' translateX('+(-((walked%288+288)%288)).toFixed(1)+'px) translateZ('+FLOOR_FRONT+'px) rotateX(90deg)';
  }
  function aisleOffset(index) {
    return BlipAisle.offset(index,aisle.pos,cards.length);
  }
  // Every machine stands where the walk has got to. Transform and opacity only: this runs every frame.
  function poseCards(room,bob) {
    var closer=lean*LEAN_Z,lift=-lean*LEAN_LIFT*room.scale,cam3d=camera();
    cards.forEach(function(c,i){
      if(!c.classList.contains('card'))return;
      var offset=aisleOffset(i),pose=cardPose(c,offset,room),distance=Math.abs(offset);
      var scale=room.scale;
      c.style.transform=cam3d+' translateX(calc(-50% + '+pose.x.toFixed(1)+'px)) translateY('+((bob||0)+lift).toFixed(2)+'px) translateZ('+(pose.z+closer).toFixed(1)+'px) rotateY('+pose.turn.toFixed(2)+'deg) scale('+scale.toFixed(4)+')';
      var z=String(30-Math.min(Math.round(distance*2),25));
      if(c.style.zIndex!==z)c.style.zIndex=z;
      // Past three and a half machines a cabinet is out of sight, and crosses to the aisle's other end unseen.
      var away=distance>3.5||walkHides(distance);
      if(c.deckEl===undefined)c.deckEl=c.querySelector('.machine-deck');
      var deck=c.deckEl;
      if(deck){var hideDeck=walking&&distance>=.5&&cam.z>300;if(deck.walkHidden!==hideDeck){deck.walkHidden=hideDeck;deck.style.visibility=hideDeck?'hidden':'';}}
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
    });
    roomTravel+=BlipAisle.offset(aisle.pos,roomFrom,cards.length);roomFrom=aisle.pos;
    grid.dataset.aislePos=(((aisle.pos%cards.length)+cards.length)%cards.length).toFixed(2);
    grid.dataset.roomTravel=roomTravel.toFixed(3);
    slideRoom(room,cam3d);
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
    if(dir)queuedUps=0;
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
    leanP=leanTo>leanP?Math.min(leanTo,leanP+dt/LEAN_S):Math.max(leanTo,leanP-dt/LEAN_S);
    lean=leanP*leanP*(3-2*leanP);
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
    if(aisle.lead==null)aisle.lead=aisle.pos;
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
  // Pressed while the aisle is still settling or the room still arriving back from a game, Up waits for it rather
  // than being lost: two presses always step up and play.
  var queuedUps=0,upTimer=0;
  function stepUp() {
    if(cardInFlight||!manualOverlay.hidden)return;
    if(!canWalk()||!aisle.settled){queuedUps=Math.min(2,queuedUps+1);if(!upTimer)upTimer=setTimeout(retryUp,100);return;}
    if(steppedUp){activate();return;}
    setStepped(true);
  }
  function retryUp() {
    upTimer=0;
    if(!queuedUps||cardInFlight)return;
    if(!canWalk()||!aisle.settled){upTimer=setTimeout(retryUp,100);return;}
    var n=queuedUps;queuedUps=0;
    while(n--)stepUp();
  }
  function stepBack() {
    if(steppedUp)setStepped(false);
  }
  arcadeStepBack=stepBack;
  arcadeStepIn=function(){if(!steppedUp)setStepped(true);};
  // Walking up to a machine (or back from it): the eye moves through the room, so the machine, its neighbours, the
  // floor and the wall each grow as near things do, with a step's dip of the head. The walk ends where the machine's
  // screen covers the box 'to' (the game page's screen): #game-grid's eye is EYE out, level with 50% 38% of it,
  // so a screen D away drawn w wide is drawn k w wide at D / k, and a point moves out from that level point as it nears.
  // pick(card): the box on the machine that is to cover 'to', measured where the eye is now.
  function walkTarget(card,to,pick){
    var glass=pick(card),box=grid.getBoundingClientRect();
    var ox=box.left+box.width*.5,oy=box.top+box.height*.38,k=to.w/glass.width;
    var near=EYE-(lean*LEAN_Z+cam.z),nearer=near/k;
    return {x:cam.x+((to.x+to.w/2-ox)*nearer-(glass.left+glass.width/2-ox)*near)/EYE,
      y:cam.y+((to.y+to.h/2-oy)*nearer-(glass.top+glass.height/2-oy)*near)/EYE,
      z:Math.min(cam.z+near-nearer,880)};
  }
  // back: start at the machine and walk away from it. Resolves when the walk ends.
  arcadeWalk=function(card,to,pick,ms,back){
    var end=walkTarget(card,to,pick),start={x:cam.x,y:cam.y,z:cam.z};
    walkMarqueeFor(card);
    // Back: stand at the machine while the game page fades (blip.css, 900 ms), then walk.
    return back?walkCam(end,start,ms,true,350):walkCam(start,end,ms,false,0);
  };
  // Nearing the machine, the controls printed on its bezel fade (the game page has none) and the top bar takes its
  // marquee, title and all: set up once a walk (walkMarqueeFor), then only the two opacities change frame by frame.
  var walkWords=null,walkMarquee=null;
  function walkMarqueeFor(card){
    if(!walkMarquee){
      walkMarquee=document.createElement('div');walkMarquee.className='walk-marquee';walkMarquee.setAttribute('aria-hidden','true');
      walkMarquee.appendChild(document.createElement('b'));
      (document.querySelector('.top-marquee-bar')||document.body).appendChild(walkMarquee);
    }
    walkWords=card.querySelector('.machine-instructions');
    var name=(card.querySelector('.machine-marquee-name')||{}).textContent||'',words=walkMarquee.firstChild;
    words.textContent=name;
    placeWalkName(words,name);
    walkMarquee.style.background=getComputedStyle(card).getPropertyValue('--machine-marquee');
  }
  // The name as large and where the game page draws it (cabinet.js fitName remembers it), so the hand-over keeps its
  // size. Before that page has been seen at this width: a phone's name fits between the home switch and the game
  // page's ? button (32px left of the manual), in whole pixels, 16px at most, centred on the 60px fascia; wider screens centre it at 20px.
  function placeWalkName(words,name){
    words.removeAttribute('style');
    var bar=walkMarquee.getBoundingClientRect(),fit=null;
    try{fit=JSON.parse(sessionStorage.getItem('blip-marquee-fit')||'{}')[name];}catch(e){}
    if(!(fit&&fit.w===innerWidth)){
      // A phone on its side has no name on the game page's bar (blip.css).
      if(matchMedia('(pointer:coarse) and (min-width:621px) and (max-width:950px) and (max-height:520px)').matches){words.style.visibility='hidden';return;}
      var logo=document.querySelector('.blip-logo'),home=document.getElementById('home-switch-btn'),manual=document.querySelector('.manual-pocket');
      if(innerWidth>520||!logo||!home||!manual)return;
      // The game page squeezes the switch 12px right of the logo and pads the name 4px past it, 4px inside each end.
      var from=logo.getBoundingClientRect().right+12+home.offsetWidth+10,to=manual.getBoundingClientRect().left-38,size=16;
      words.style.fontSize=size+'px';
      while(size>7&&words.scrollWidth>to-from-4){size--;words.style.fontSize=size+'px';}
      fit={x:from+(to-from-words.scrollWidth)/2,y:bar.bottom-30-size/2,size:size,hidden:words.scrollWidth>to-from-3};
    }
    words.classList.toggle('walk-name-small',fit.size<14);
    words.style.cssText='position:absolute;left:'+(fit.x-bar.left)+'px;top:'+(fit.y-bar.top)+'px;font-size:'+fit.size+'px;line-height:1'+(fit.hidden?';visibility:hidden':'');
  }
  function nearing(near){
    if(!walkMarquee)return;
    if(walkWords){if(near)walkWords.style.opacity=(1-near).toFixed(3);else walkWords.removeAttribute('style');}
    walkMarquee.style.opacity=near.toFixed(3);
  }
  // Back to where the player stood in the aisle, from wherever the eye is (a page restored from the cache).
  arcadeWalkOut=function(){return walkCam({x:cam.x,y:cam.y,z:cam.z},{x:0,y:0,z:0},WALK_MS,true);};
  function walkCam(start,end,ms,back,hold){
    var room=aisleRoomNow||aisleRoom();
    cam=start;
    walking=true;grid.setAttribute('data-walking','');
    var nearZ=Math.max(start.z,end.z),near=function(){return nearZ?Math.max(0,Math.min(1,cam.z/nearZ)):0;};
    function arrive(){cam=end;walking=false;grid.removeAttribute('data-walking');poseCards(room);nearing(near());}
    if(stepWalk.matches||!nearZ){arrive();return Promise.resolve();}
    // A step every 260px walked, at least two; each dips the head and sounds a footstep.
    var steps=Math.max(2,Math.round(Math.abs(end.z-start.z)/260)),t0=performance.now(),heard=0;
    return new Promise(function(done){
      function frame(now){
        var t=Math.max(0,Math.min(1,(now-t0-(hold||0))/ms)),e=t*t*(3-2*t);
        // Each step lowers the head and raises it again smoothly (sin², no corner where steps meet).
        var dip=-3.5*room.scale*Math.pow(Math.sin(Math.PI*steps*e),2)*(1-Math.abs(2*t-1)*.5);
        cam={x:start.x+(end.x-start.x)*e,y:start.y+(end.y-start.y)*e+dip,z:start.z+(end.z-start.z)*e};
        poseCards(room);
        nearing(near());
        var step=Math.floor(e*steps+.5);
        if(step>heard&&step<=steps){heard=step;playMachineStep(back?-1:1,1);}
        if(t<1||now-t0<(hold||0))requestAnimationFrame(frame);
        else{arrive();done();}
      }
      poseCards(room);
      requestAnimationFrame(frame);
    });
  }
  function setStepped(on) {
    steppedUp=on;
    grid.toggleAttribute('data-stepped-up',on);
    blipArcadeSound.duck(on);
    if(stepWalk.matches){lean=leanP=on?1:0;poseCards(aisleRoomNow||aisleRoom());return;}
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
