(function () {
  window.blipMoveManual = async function (overlay, returning) {
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
}());
