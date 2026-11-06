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
