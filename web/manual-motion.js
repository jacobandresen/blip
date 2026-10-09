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
    var frame = spread.getBoundingClientRect(), rightBox = rightPage.getBoundingClientRect(), leftBox = leftPage.getBoundingClientRect();
    var book = source.cloneNode(true);
    book.removeAttribute('id'); book.removeAttribute('data-page-card');
    book.className = 'cabinet-manual manual-transit';
    book.setAttribute('aria-hidden', 'true'); book.tabIndex = -1;
    var box = function (r, extra) {
      return Object.assign({ left: r.left + 'px', top: r.top + 'px', width: r.width + 'px', height: r.height + 'px' }, extra);
    };
    var seated = box(slot, { fontSize: getComputedStyle(source.querySelector('strong')).fontSize, opacity: 1,
      transform: 'perspective(900px) rotateX(0deg) rotate(0deg)', boxShadow: '0 2px 2px #000c, inset 0 1px #b9b38c33' });
    var lifted = Object.assign({}, seated, { top: slot.top - 14 + 'px', transform: 'perspective(900px) rotateX(14deg) rotate(-3deg)',
      boxShadow: '0 9px 9px #0009, inset 0 1px #b9b38c55' });
    var held = box(rightBox, { fontSize: '30px', opacity: 1, transform: 'perspective(900px) rotateX(0deg) rotate(0deg)',
      boxShadow: '0 24px 35px #0009, inset 0 1px #b9b38c55' });
    var animations = [], stage = null;

    // The cover, hinged at the spine: its front is the closed book, its back the left-hand page.
    function hinge(closedAngle) {
      var back = strip(leftPage.cloneNode(true));
      back.querySelectorAll('.manual-tear').forEach(function (n) { n.remove(); });
      var cover = document.createElement('div');
      cover.className = 'manual-cover';
      cover.style.cssText = 'position:absolute;left:' + (rightBox.left - frame.left) + 'px;top:' + (rightBox.top - frame.top) + 'px;width:' + rightBox.width +
        'px;height:' + rightBox.height + 'px;transform-origin:-8px 50%;transform-style:preserve-3d;z-index:12;pointer-events:none;will-change:transform;transform:rotateY(' + closedAngle + 'deg)';
      var front = document.createElement('div'), rear = document.createElement('div');
      front.className = 'manual-cover-face manual-cover-front'; rear.className = 'manual-cover-face manual-cover-back';
      var closed = book.cloneNode(true);
      closed.className = 'cabinet-manual manual-cover-book';
      closed.style.cssText = 'position:absolute;inset:0;width:100%;height:100%;margin:0;left:0;top:0;transform:none;font-size:30px;box-shadow:none;';
      front.appendChild(closed);
      back.style.cssText = 'position:absolute;inset:0;margin:0;width:100%;height:100%;grid-area:auto;';
      rear.appendChild(back);
      var shadeFront = document.createElement('i'), shadeBack = document.createElement('i');
      shadeFront.className = shadeBack.className = 'manual-flip-shade';
      front.appendChild(shadeFront); rear.appendChild(shadeBack);
      cover.append(front, rear);
      spread.appendChild(cover);
      return { cover: cover, shadeFront: shadeFront, shadeBack: shadeBack };
    }
    function swing(parts, from, to, ms) {
      parts.shadeFront.animate([{ opacity: from === 0 ? 0 : .5 }, { opacity: to === 0 ? 0 : .5 }], { duration: ms, easing: 'ease-in-out', fill: 'both' });
      parts.shadeBack.animate([{ opacity: from === 0 ? .5 : 0 }, { opacity: to === 0 ? .5 : 0 }], { duration: ms, easing: 'ease-in-out', fill: 'both' });
      return parts.cover.animate([
        { transform: 'rotateY(' + from + 'deg)', offset: 0, easing: 'cubic-bezier(.4,0,.6,1)' },
        { transform: 'rotateY(' + (to < from ? to - 4 : to) + 'deg)', offset: .82, easing: 'ease-in-out' },
        { transform: 'rotateY(' + to + 'deg)', offset: 1 }
      ], { duration: ms, fill: 'forwards' });
    }
    async function ready() {
      for (var waited = 0; waited < 1500 && !overlay.querySelector('#manual-page-content .manual-leaf-sheet'); waited += 50)
        await new Promise(function (r) { setTimeout(r, 50); });
    }
    try {
      if (!returning) document.body.appendChild(book);
      if (returning) {
        // The cover is already open over the left-hand page; close it over the right.
        leftPage.style.visibility = 'hidden';
        var parts = hinge(-180);
        animations.push(overlay.animate([{ opacity: 1 }, { opacity: 1 }], { duration: 950, fill: 'both' }));
        await swing(parts, -180, 0, 950).finished;
        parts.cover.style.visibility = 'hidden';
        animations.push(spread.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 260, fill: 'both' }));
        animations.push(overlay.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 900, delay: 260, fill: 'both' }));
        var settled = Object.assign({}, seated, { top: slot.top + 1 + 'px' });
        document.body.appendChild(book);
        await book.animate([
          Object.assign({}, held, { offset: 0, easing: 'cubic-bezier(.4,0,.25,1)' }),
          Object.assign({}, lifted, { offset: .62, easing: 'ease-in-out' }),
          Object.assign({}, settled, { offset: .94 }),
          Object.assign({}, seated, { offset: 1 })
        ], { duration: 1150, fill: 'forwards' }).finished;
      } else {
        leftPage.style.visibility = 'hidden';
        animations.push(spread.animate([{ opacity: 0 }, { opacity: 0, offset: .7 }, { opacity: 1 }], { duration: 1000, fill: 'both' }));
        animations.push(overlay.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 800, delay: 150, fill: 'both' }));
        await book.animate([
          Object.assign({}, seated, { offset: 0, easing: 'cubic-bezier(.3,0,.3,1)' }),
          Object.assign({}, lifted, { offset: .22, easing: 'cubic-bezier(.3,0,.25,1)' }),
          Object.assign({}, held, { offset: 1 })
        ], { duration: 1000, fill: 'forwards' }).finished;
        await ready();
        leftPage.style.visibility = 'hidden';
        var opening = hinge(0);
        book.remove();
        await swing(opening, 0, -180, 1000).finished;
        leftPage.style.visibility = '';
        opening.cover.remove();
      }
    } finally {
      animations.forEach(function (animation) { animation.cancel(); });
      leftPage.style.visibility = '';
      spread.querySelectorAll('.manual-cover').forEach(function (n) { n.remove(); });
      book.remove();
      source.classList.toggle('manual-away', !returning);
      overlay.inert = false;
    }
  };

  window.blipMoveManual = function (overlay, returning) {
    var calm = matchMedia('(prefers-reduced-motion: reduce)').matches;
    return (!calm && matchMedia('(min-width:701px)').matches ? moveWide : moveSimple)(overlay, returning);
  };
}());
