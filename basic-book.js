(function () {
  var source = document.querySelector('.pages');
  if (!source) return;
  var body = document.getElementById('book-body');
  var opening = document.querySelector('.opening-spread');
  var chapters = Array.from(source.querySelectorAll('.chapter'));
  var STAGE_W = 964, STAGE_H = 704, CLOSED_SHIFT = -241, STACK = 8;
  var chapterIndex = -1;
  var spread = document.createElement('div');
  spread.className = 'pages reader-spread';
  spread.setAttribute('aria-live', 'polite');
  var left = document.createElement('section'), right = document.createElement('section');
  left.setAttribute('aria-label', 'Left page'); right.setAttribute('aria-label', 'Right page');
  spread.append(left, right);
  var controls = document.createElement('nav');
  controls.className = 'page-turns'; controls.setAttribute('aria-label', 'Turn pages');
  controls.innerHTML = '<button type="button" data-turn="prev" disabled aria-label="Turn to previous spread">‹ <span>PREVIOUS</span></button><output aria-live="polite">CONTENTS</output><button type="button" data-turn="next" aria-label="Turn to next spread"><span>TURN PAGE</span> ›</button><button type="button" data-close-book aria-label="Close the book and return to BLIP">▣ <span>CLOSE BOOK</span></button>';
  source.replaceWith(spread);
  document.querySelector('.book-layout').append(controls);
  var previous = controls.querySelector('[data-turn="prev"]');
  var next = controls.querySelector('[data-turn="next"]');
  var count = controls.querySelector('output');
  var closeButton = controls.querySelector('[data-close-book]');
  var cover = document.getElementById('book-cover');
  var flipping = null;

  // Handwriting is never quite level: each word sits a hair off the line, with its own tilt and pressure.
  var seed = 11;
  function jitter() { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed / 0x7fffffff; }
  var note = opening.querySelector('.inscription');
  if (note) {
    note.querySelectorAll('p').forEach(function (line) {
      line.innerHTML = line.textContent.split(' ').map(function (word) {
        return '<span style="--dy:' + ((jitter() - .5) * 2.4).toFixed(1) + 'px;--r:' + ((jitter() - .5) * 3).toFixed(1) + 'deg;--o:' + (.82 + jitter() * .16).toFixed(2) + '">' + word + '</span>';
      }).join(' ');
    });
    // The pastedown carries the note too, so it is there while the cover swings.
    document.querySelector('.pastedown').append(note.cloneNode(true));
  }

  function wide() { return matchMedia('(min-width: 861px)').matches; }

  function fitStage() {
    if (!wide()) { document.documentElement.style.removeProperty('--s'); return; }
    var nav = controls.offsetHeight + 6;
    var s = Math.min((innerWidth - 32) / STAGE_W, (innerHeight - 48 - nav) / (STAGE_H + 30));
    document.documentElement.style.setProperty('--s', Math.max(.5, Math.min(1.4, s)).toFixed(3));
  }

  // The print size fits every page; zoom is only a safety net for a page that outgrows it.
  function fitPage(slot) {
    var chapter = slot.querySelector('.chapter');
    if (!chapter || !wide()) return;
    chapter.style.zoom = '';
    for (var zoom = 1; zoom >= .7 && slot.scrollHeight > slot.clientHeight + 1; zoom -= .02) chapter.style.zoom = zoom.toFixed(2);
  }

  function side(slot, index) {
    slot.className = 'page reader-page ' + (index % 2 ? 'recto' : 'verso');
    var chapter = chapters[index];
    if (chapter) slot.append(chapter);
    else slot.classList.add('endpaper');
  }

  function stacks(index) {
    var done = index < 0 ? 0 : Math.min(1, (index + 1) / chapters.length);
    body.style.setProperty('--stack-l', (STACK * done).toFixed(1) + 'px');
    body.style.setProperty('--stack-r', (STACK * (1 - done) + 1).toFixed(1) + 'px');
  }

  function render(index) {
    chapterIndex = index;
    left.replaceChildren(); right.replaceChildren();
    left.removeAttribute('data-end'); right.removeAttribute('data-end');
    stacks(index);
    if (index < 0) {
      opening.hidden = false;
      spread.hidden = true;
      delete document.body.dataset.reading;
      count.textContent = 'CONTENTS';
      previous.disabled = true;
      next.disabled = false;
      return;
    }
    opening.hidden = true;
    spread.hidden = false;
    document.body.dataset.reading = '';
    if (wide()) { side(left, index); side(right, index + 1); }
    else side(right, index);
    fitPage(left);
    fitPage(right);
    var firstPage = index + 2, lastPage = Math.min(index + (wide() ? 3 : 2), chapters.length + 1);
    count.textContent = 'PAGES ' + firstPage + (lastPage > firstPage ? '–' + lastPage : '') + ' / ' + (chapters.length + 1);
    previous.disabled = false;
    next.disabled = index + (wide() ? 2 : 1) >= chapters.length;
    if (next.disabled) right.dataset.end = '';
  }

  var leafStyle = { ms: 640, shade: .42, cast: .26, frames: function (sign) {
    return [
      { transform: 'translateZ(0) rotateY(0deg)', offset: 0, easing: 'cubic-bezier(.3,0,.55,1)' },
      { transform: 'translateZ(12px) rotateY(' + sign * 62 + 'deg)', offset: .36, easing: 'linear' },
      { transform: 'translateZ(16px) rotateY(' + sign * 128 + 'deg)', offset: .72, easing: 'cubic-bezier(.2,.6,.3,1)' },
      { transform: 'translateZ(0) rotateY(' + sign * 180 + 'deg)', offset: 1 }
    ];
  }};

  function goTo(target) {
    if (flipping) flipping.finish();
    if (wide() && target >= 0) target -= target % 2;
    if (target >= chapters.length || target === chapterIndex) return;
    var direction = target > chapterIndex ? 1 : -1;
    var reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (!wide() || reduced || document.body.dataset.bookState !== 'open') { render(target); return; }
    // Leaving the contents lifts the contents page itself; returning lands on it.
    var fromContents = chapterIndex < 0, toContents = target < 0;
    var contents = opening.querySelector('.contents-page'), endpaper = opening.querySelector('.blank-page');
    var options = {
      book: body, direction: direction, style: leafStyle,
      update: function () { render(target); },
      onFinish: function () { flipping = null; }
    };
    if (fromContents) { options.lifted = contents; options.landing = endpaper; options.landingAfter = left; }
    else if (toContents) { options.lifted = left; options.landing = right; options.landingAfter = contents; }
    else { options.lifted = direction > 0 ? right : left; options.landing = direction > 0 ? left : right; }
    flipping = window.BlipBook.turn(options);
  }

  function turn(direction) {
    if (flipping) return;
    if (document.body.dataset.bookState === 'closed') { if (direction > 0) openBook(); return; }
    var step = wide() ? 2 : 1;
    goTo(chapterIndex < 0 ? (direction > 0 ? 0 : -1) : Math.max(-1, chapterIndex + direction * step));
  }

  previous.addEventListener('click', function () { turn(-1); });
  next.addEventListener('click', function () { turn(1); });
  opening.querySelectorAll('[data-go]').forEach(function (entry) {
    entry.addEventListener('click', function () { goTo(Number(entry.dataset.go)); });
  });

  function closeBook() {
    if (closeButton.disabled) return;
    closeButton.disabled = true;
    if (flipping) flipping.finish();
    var wasClosed = document.body.dataset.bookState === 'closed';
    render(-1);
    var reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
    var leave = function () {
      try {
        if (document.referrer && new URL(document.referrer).origin === location.origin && history.length > 1) history.back();
        else location.href = 'basic.html';
      } catch (_) { location.href = 'basic.html'; }
    };
    if (reduced || !cover || wasClosed) { leave(); return; }
    document.body.dataset.bookState = 'closing';
    if (wide()) body.animate([{ transform: 'translateX(0)' }, { transform: 'translateX(' + CLOSED_SHIFT + 'px)' }], { duration: 1050, delay: 150, easing: 'cubic-bezier(.45,0,.3,1)', fill: 'forwards' });
    cover.animate([
      { transform: 'rotateY(-180deg)', offset: 0, easing: 'cubic-bezier(.32,0,.62,.24)' },
      { transform: 'rotateY(-34deg)', offset: .76, easing: 'cubic-bezier(.2,.7,.3,1)' },
      { transform: 'rotateY(0deg)', offset: 1 }
    ], { duration: 1050, fill: 'forwards' }).finished.then(function () { setTimeout(leave, 160); });
  }
  closeButton.addEventListener('click', closeBook);

  document.addEventListener('keydown', function (event) {
    if (event.key === 'Escape') { event.preventDefault(); closeBook(); return; }
    if (event.key === 'ArrowRight' || event.key === 'PageDown') { event.preventDefault(); turn(1); }
    if (event.key === 'ArrowLeft' || event.key === 'PageUp') { event.preventDefault(); turn(-1); }
  });

  // A click on a page turns it, as a reader takes the corner; a swipe does the same on touch.
  spread.addEventListener('click', function (event) {
    var page = event.target.closest('.reader-page');
    if (!wide() || !page || page.classList.contains('endpaper') || getSelection().toString()) return;
    turn(page === left ? -1 : 1);
  });
  var pointer = null;
  spread.addEventListener('pointerdown', function (event) { pointer = event.pointerType === 'mouse' ? null : { id: event.pointerId, x: event.clientX, y: event.clientY }; });
  spread.addEventListener('pointercancel', function () { pointer = null; });
  spread.addEventListener('pointerup', function (event) {
    var start = pointer; pointer = null;
    if (!start || start.id !== event.pointerId) return;
    var dx = event.clientX - start.x, dy = event.clientY - start.y;
    if (Math.abs(dx) > 48 && Math.abs(dx) > Math.abs(dy) * 1.5) turn(dx < 0 ? 1 : -1);
  });

  var wasWide = wide();
  addEventListener('resize', function () {
    fitStage();
    if (wide() !== wasWide) { wasWide = wide(); if (flipping) flipping.finish(); if (chapterIndex >= 0) render(wide() ? chapterIndex - chapterIndex % 2 : chapterIndex); }
  });
  fitStage();
  render(-1);

  if (!cover) return;
  // The book waits closed on the desk, front cover up, until it is opened.
  document.body.dataset.bookState = 'closed';
  count.textContent = 'FRONT COVER';
  next.querySelector('span').textContent = 'OPEN BOOK';
  if (wide()) body.style.transform = 'translateX(' + CLOSED_SHIFT + 'px)';
  cover.addEventListener('click', function () { if (document.body.dataset.bookState === 'closed') openBook(); });

  function openBook() {
    document.body.dataset.bookState = 'opening';
    count.textContent = 'CONTENTS';
    next.querySelector('span').textContent = 'TURN PAGE';
    var shift = getComputedStyle(body).transform;
    body.style.transform = '';
    if (matchMedia('(prefers-reduced-motion: reduce)').matches) {
      cover.style.transform = 'rotateY(-180deg)';
      document.body.dataset.bookState = 'open';
      return;
    }
    if (wide()) body.animate([{ transform: shift === 'none' ? 'translateX(' + CLOSED_SHIFT + 'px)' : shift }, { transform: 'translateX(0)' }], { duration: 1500, delay: 120, easing: 'cubic-bezier(.45,0,.25,1)', fill: 'backwards' });
    cover.animate([
      { transform: getComputedStyle(cover).transform === 'none' ? 'rotateY(0deg)' : getComputedStyle(cover).transform, offset: 0, easing: 'cubic-bezier(.36,0,.65,.25)' },
      { transform: 'rotateY(-38deg)', offset: .16, easing: 'cubic-bezier(.2,.72,.28,1)' },
      { transform: 'rotateY(-140deg)', offset: .7, easing: 'cubic-bezier(.27,.08,.35,1)' },
      { transform: 'rotateY(-176deg)', offset: .9, easing: 'ease-out' },
      { transform: 'rotateY(-180deg)', offset: 1 }
    ], { duration: 1850, fill: 'forwards' }).finished.then(function () {
      document.body.dataset.bookState = 'open';
    });
  }
}());
