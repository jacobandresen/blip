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
