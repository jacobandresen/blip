/* The guide: the cabinet manual's chapters (controls.html, history.html,
 * about.html) read here and set as an iOS guide. The text has one home,
 * those pages; this file only chooses and restyles it. Anything marked
 * data-guide="cabinet" is about the cabinet alone and is left out. */
(function () {
'use strict';

var guide = document.getElementById('guide');
var toc = document.getElementById('toc');
var article = document.getElementById('article');
var search = document.getElementById('guide-search');
var rootPane = document.getElementById('pane-root');
var detailPane = document.getElementById('pane-detail');
var inlineTitle = document.getElementById('detail-inline-title');
var EMBED = new URLSearchParams(location.search).has('embed');
// Opened from its own home-screen icon: an app of its own, with nowhere to go back to.
var OWN_APP = !EMBED && blipStandalone() && !document.referrer;
var SOURCES = ['controls.html', 'history.html', 'about.html'];
var GROUPS = ['Getting started', 'Games', 'History', 'About'];
var topics = [], byId = {};

function titleCase(s) { return s.toLowerCase().replace(/\b[a-z]/g, function (c) { return c.toUpperCase(); }); }
function sentence(s) {
  s = s.replace(/[►▶►]/g, '').replace(/\s+/g, ' ').trim();
  return /^[^a-z]*$/.test(s) ? s.charAt(0) + s.slice(1).toLowerCase() : s;
}
function slugify(s) { return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, ''); }
function el(tag, cls, html) { var e = document.createElement(tag); if (cls) e.className = cls; if (html != null) e.innerHTML = html; return e; }
function gameSlugByName(name) {
  name = name.trim().toLowerCase();
  return Object.keys(BLIP_GAMES).filter(function (s) {
    var n = BLIP_GAMES[s].name.toLowerCase();
    return name === n || name.split(' ').indexOf(n) >= 0;
  })[0] || null;
}

/* ---- Restyling ---- The chapter's markup, cleaned to plain prose and
 * turned into the guide's parts: key tables into info rows, studios into
 * headed paragraphs, the operators into a grid. */
var TIP_SVG = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 2a7 7 0 0 0-4 12.7V17a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1v-2.3A7 7 0 0 0 12 2zm-3 18h6v1a1 1 0 0 1-1 1h-4a1 1 0 0 1-1-1z"/></svg>';
function convert(node) {
  if (node.nodeType === 3) return document.createTextNode(node.textContent);
  if (node.nodeType !== 1) return null;
  var tag = node.tagName.toLowerCase();
  if (/^(script|style|figure|aside)$/.test(tag) || node.matches('.divider, [data-guide="cabinet"], .manual-tip')) return null;
  if (node.matches('.section-label, .thoughts-label, .impl-label')) return el('h3', null, sentence(node.textContent));
  if (tag === 'table' && node.matches('.keytable')) {
    var ul = el('ul', 'group');
    Array.prototype.forEach.call(node.querySelectorAll('tr'), function (tr) {
      if (tr.matches('[data-guide="cabinet"]')) return;
      var cells = tr.querySelectorAll('td');
      if (cells.length < 2) return;
      var li = el('li', 'info');
      li.appendChild(el('span', null, cleanHtml(cells[0])));
      li.appendChild(el('span', null, cleanHtml(cells[1])));
      ul.appendChild(li);
    });
    return ul;
  }
  if (tag === 'table' && node.matches('.operators')) {
    var people = el('ul', 'people');
    Array.prototype.forEach.call(node.querySelectorAll('.operator'), function (td) {
      var img = td.querySelector('img');
      people.appendChild(el('li', null, (img ? '<img src="' + img.getAttribute('src') + '" alt="">' : '') +
        '<b>' + text(td, '.operator-name') + '</b><span>' + text(td, '.operator-role') + '</span><span>' + text(td, '.operator-job') + '</span>'));
    });
    return people;
  }
  if (node.matches('.studio')) {
    var frag = document.createDocumentFragment();
    frag.appendChild(el('h3', null, text(node, '.studio-name')));
    frag.appendChild(el('span', 'studio-dates', text(node, '.studio-dates')));
    Array.prototype.forEach.call(node.querySelectorAll('p'), function (p) { frag.appendChild(convert(p)); });
    return frag;
  }
  // Wrappers and layout boxes give way to their contents.
  var keep = /^(p|ul|ol|li|a|b|strong|i|em|kbd|br|q|code|h3|h4|img)$/.test(tag);
  var out = keep ? document.createElement(tag === 'h4' ? 'h3' : tag) : document.createDocumentFragment();
  if (tag === 'a') {
    var href = node.getAttribute('href') || '';
    var internal = /^(history|controls|about)\.html/.test(href);
    out.setAttribute('href', internal ? '#/t/' + (href.indexOf('history') === 0 ? 'history-intro' : href.indexOf('about') === 0 ? 'about-intro' : 'getting-started') : href);
    if (/^https?:/.test(href)) { out.target = '_blank'; out.rel = 'noopener'; }
  }
  if (tag === 'img') { out.src = node.getAttribute('src'); out.alt = node.getAttribute('alt') || ''; return out; }
  Array.prototype.forEach.call(node.childNodes, function (child) { var c = convert(child); if (c) out.appendChild(c); });
  return out;
}
function cleanHtml(node) { var box = document.createElement('div'); Array.prototype.forEach.call(node.childNodes, function (c) { var x = convert(c); if (x) box.appendChild(x); }); return box.innerHTML; }
function text(node, sel) { var n = node.querySelector(sel); return n ? n.textContent.trim() : ''; }
function prose(nodes) {
  var box = el('div', 'prose');
  nodes.forEach(function (n) { var c = convert(n); if (c) box.appendChild(c); });
  return box;
}
function tipCard(aside) {
  if (!aside) return null;
  var q = aside.querySelector('q');
  return el('div', 'tip', TIP_SVG + '<div><b>Tip</b><p>' + (q ? q.textContent.trim() : '') + '</p></div>');
}
// A section's body: its .section-body/.thoughts-body, or what follows its heading.
function bodyOf(section) {
  var inner = section.querySelector(':scope > .section-body, :scope > .thoughts-body');
  var kids = Array.prototype.slice.call((inner || section).children);
  return kids.filter(function (k) { return !k.matches('h1, h2, h3, .section-label, .thoughts-label, .game-header'); });
}

/* ---- The topics ---- */
function add(topic) { topics.push(topic); byId[topic.id] = topic; }

function readControls(doc) {
  Array.prototype.forEach.call(doc.querySelectorAll('.page > section'), function (s) {
    if (s.matches('[data-manual-game], [data-guide="cabinet"]')) return;
    var head = s.querySelector('h3, .section-label');
    if (!head) return;
    var title = sentence(head.textContent);
    // The first is the chapter's entry point, which other chapters link to.
    add({ id: topics.length === 0 ? 'getting-started' : slugify(title), group: 'Getting started', title: title,
      body: function () { return prose(bodyOf(s)); } });
  });
}

function readGames(controls, history) {
  var origins = {};
  Array.prototype.forEach.call(history.querySelectorAll('.game-section'), function (s) {
    var slug = gameSlugByName(text(s, '.game-title'));
    if (slug) origins[slug] = s;
  });
  Object.keys(BLIP_GAMES).map(function (slug) { return { slug: slug, name: titleCase(BLIP_GAMES[slug].name) }; })
    .sort(function (a, b) { return a.name.localeCompare(b.name); })
    .forEach(function (g) {
      var left = controls.querySelector('[data-manual-game="' + g.slug + '"][data-side="left"]');
      var right = controls.querySelector('[data-manual-game="' + g.slug + '"][data-side="right"]');
      var past = origins[g.slug];
      if (!left && !right) return;
      add({
        id: 'game-' + g.slug, group: 'Games', title: g.name, slug: g.slug,
        sub: past ? text(past, '.game-origin') : '',
        icon: g.slug + '/icon-192.png?v=2',
        body: function () {
          var box = document.createDocumentFragment();
          var meta = el('div', 'hero-meta', '<img src="' + g.slug + '/icon-192.png?v=2" alt=""><span><b>' + (past ? text(past, '.game-origin') : g.name) + '</b><span class="row-sub">' +
            (BLIP_GAMES[g.slug].players === 2 || /2(ND)? PLAYER/.test(JSON.stringify(BLIP_GAMES[g.slug].controls || [])) ? '1 or 2 players' : '1 player') + '</span></span>');
          if (!EMBED && !OWN_APP) {
            var play = meta.appendChild(el('a', 'play', 'Play'));
            play.href = g.slug + '/';
            // From the guide, as from the home screen: free play, back to the home screen.
            play.addEventListener('click', function () { blipSetFrontend('home'); });
          }
          box.appendChild(meta);
          var shot = left && left.querySelector('figure img');
          if (shot) box.appendChild(el('div', 'hero', '<img src="' + shot.getAttribute('src') + '" alt="' + (shot.getAttribute('alt') || '') + '">'));
          var p = el('div', 'prose');
          var blurb = left && left.querySelector('.manual-game-blurb');
          if (blurb) p.appendChild(el('p', 'lede', cleanHtml(blurb)));
          if (right) {
            p.appendChild(el('h2', null, 'How to play'));
            Array.prototype.forEach.call(right.children, function (c) { if (!c.matches('h4, .manual-tip')) { var x = convert(c); if (x) p.appendChild(x); } });
            var tip = tipCard(right.querySelector('.manual-tip'));
            if (tip) p.appendChild(tip);
          }
          if (past) {
            p.appendChild(el('h2', null, 'History'));
            p.appendChild(el('p', 'eyebrow', text(past, '.game-origin')));
            var sub = past.querySelector(':scope > h2');
            if (sub) p.appendChild(el('h3', null, sentence(sub.textContent)));
            bodyOf(past).forEach(function (n) { var x = convert(n); if (x) p.appendChild(x); });
          }
          box.appendChild(p);
          return box;
        }
      });
    });
}

function readChapter(doc, group, introId) {
  var subtitle = doc.querySelector('.page > h2');
  Array.prototype.forEach.call(doc.querySelectorAll('.page > section'), function (s) {
    if (s.matches('.game-section, [data-guide="cabinet"]')) return;
    var head = s.querySelector('h3, .section-label, .thoughts-label');
    var intro = s.matches('.intro');
    var title = intro ? sentence(subtitle ? subtitle.textContent : group) : head ? sentence(head.textContent) : null;
    if (!title) return;
    add({ id: intro ? introId : slugify(title), group: group, title: title,
      body: function () {
        if (s.matches('.manual-operators')) {
          return prose(Array.prototype.slice.call(s.children).filter(function (k) { return !k.matches('h3'); }));
        }
        return prose(intro ? Array.prototype.slice.call(s.children) : bodyOf(s));
      } });
  });
}

/* ---- The contents ---- */
function rowFor(topic) {
  var li = el('li', topic.icon ? 'with-icon' : null);
  var a = el('a', 'row nav');
  a.href = '#/t/' + topic.id;
  a.dataset.topic = topic.id;
  a.innerHTML = (topic.icon ? '<img src="' + topic.icon + '" alt="">' : '') +
    '<span class="row-text">' + topic.title + (topic.sub ? '<span class="row-sub">' + topic.sub + '</span>' : '') + '</span>';
  li.appendChild(a);
  return li;
}
function renderToc(query) {
  toc.textContent = '';
  var q = (query || '').trim().toLowerCase();
  if (q) {
    var hits = topics.filter(function (t) { return t.title.toLowerCase().indexOf(q) >= 0 || searchText(t).indexOf(q) >= 0; });
    toc.appendChild(el('h2', 'group-title', 'Results'));
    var ul = el('ul', 'group');
    if (hits.length) hits.forEach(function (t) { ul.appendChild(rowFor(t)); });
    else ul.appendChild(el('li', 'empty', 'No results for “' + q.replace(/[<&]/g, '') + '”'));
    toc.appendChild(ul);
  } else {
    GROUPS.forEach(function (g) {
      var list = topics.filter(function (t) { return t.group === g; });
      if (!list.length) return;
      toc.appendChild(el('h2', 'group-title', g));
      var ul = el('ul', 'group');
      list.forEach(function (t) { ul.appendChild(rowFor(t)); });
      toc.appendChild(ul);
    });
  }
  markCurrent();
}
function searchText(t) {
  if (t._text == null) { var box = el('div'); box.appendChild(t.body()); t._text = box.textContent.toLowerCase(); }
  return t._text;
}
search.addEventListener('input', function () { renderToc(search.value); });

/* ---- Navigation ---- #/t/<id> opens a page; #/ the contents. A split
 * view (iPad) always has a page open beside the contents. */
var SPLIT = matchMedia('(min-width: 768px) and (min-aspect-ratio: 4/5)');
function current() { var m = /^#\/t\/(.+)$/.exec(location.hash); return m && byId[m[1]] ? m[1] : null; }
function markCurrent() {
  var id = current();
  Array.prototype.forEach.call(toc.querySelectorAll('.row'), function (r) {
    if (r.dataset.topic === id) r.setAttribute('aria-current', 'page'); else r.removeAttribute('aria-current');
  });
}
function show() {
  var id = current() || (SPLIT.matches && topics[0] ? topics[0].id : null);
  guide.classList.toggle('reading', !!current());
  if (id) {
    var t = byId[id];
    article.textContent = '';
    if (t.group !== 'Games') article.appendChild(el('p', 'eyebrow', t.group));
    article.appendChild(el('h1', 'large-title', t.title));
    article.appendChild(t.body());
    inlineTitle.textContent = t.title;
    document.title = t.title + ' – BLIP Guide';
    detailPane.querySelector('.scroller').scrollTop = 0;
    detailPane.classList.remove('scrolled');
    if (current()) detailPane.querySelector('.scroller').focus({ preventScroll: true });
  } else document.title = 'BLIP Guide';
  markCurrent();
}
window.addEventListener('hashchange', show);
SPLIT.addEventListener('change', show);
document.getElementById('nav-back').addEventListener('click', function (e) {
  // Back from a page opened in the guide is the browser's back, so the
  // contents keep their scroll.
  if (history.length > 1 && document.referrer.indexOf(location.pathname) >= 0) { e.preventDefault(); history.back(); }
});
document.addEventListener('keydown', function (e) {
  if (e.key === 'Escape' && current() && !SPLIT.matches) { e.preventDefault(); location.hash = '#/'; }
});

// The large title folds into the bar once it has scrolled under it.
[rootPane, detailPane].forEach(function (pane) {
  var scroller = pane.querySelector('.scroller');
  scroller.tabIndex = -1;
  scroller.addEventListener('scroll', function () { pane.classList.toggle('scrolled', scroller.scrollTop > 40); }, { passive: true });
});

// Opened over a game (its controls card): Done closes the sheet.
if (OWN_APP) document.getElementById('nav-home').hidden = true;
if (EMBED) {
  document.getElementById('nav-home').hidden = true;
  Array.prototype.forEach.call(document.querySelectorAll('.nav-done'), function (done) {
    done.hidden = false;
    done.addEventListener('click', function () { parent.postMessage({ type: 'blip-guide-close' }, location.origin); });
  });
  document.addEventListener('keydown', function (e) { if (e.key === 'Escape' && !current()) parent.postMessage({ type: 'blip-guide-close' }, location.origin); });
}

/* ---- The Guide on a home screen of its own ---- A row at the end of the
 * contents, unless it is there already or open over a game. */
var installSheet = document.getElementById('install-sheet');
function iosSafari() { return /iP(hone|ad|od)/.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1); }
function offerInstall() {
  if (EMBED || blipStandalone() || search.value) return;
  toc.appendChild(el('h2', 'group-title', 'This guide'));
  var ul = el('ul', 'group');
  var li = el('li', 'with-icon');
  var b = el('button', 'row install-row', '<img src="guide-icon-192.png?v=1" alt=""><span class="row-text">Add Guide to Home Screen</span>');
  b.type = 'button';
  b.addEventListener('click', openInstall);
  li.appendChild(b); ul.appendChild(li); toc.appendChild(ul);
}
function openInstall() {
  var go = document.getElementById('install-go');
  var ios = iosSafari();
  document.getElementById('install-steps').hidden = !ios;
  document.getElementById('install-other').hidden = ios || !!window.blipInstallPrompt;
  go.hidden = !window.blipInstallPrompt;
  installSheet.hidden = false;
  (go.hidden ? document.getElementById('install-close') : go).focus();
}
function closeInstall() { installSheet.hidden = true; }
document.getElementById('install-close').addEventListener('click', closeInstall);
document.getElementById('install-go').addEventListener('click', function () {
  var prompt = window.blipInstallPrompt;
  window.blipInstallPrompt = null;
  closeInstall();
  if (prompt) prompt.prompt();
});
document.addEventListener('keydown', function (e) { if (e.key === 'Escape' && !installSheet.hidden) closeInstall(); });
var _renderToc = renderToc;
renderToc = function (q) { _renderToc(q); if (!q && topics.length) offerInstall(); };

/* ---- Load ---- */
Promise.all(SOURCES.map(function (url) {
  return fetch(url).then(function (r) { if (!r.ok) throw new Error(url); return r.text(); });
})).then(function (pages) {
  var docs = pages.map(function (html) { return new DOMParser().parseFromString(html, 'text/html'); });
  readControls(docs[0]);
  readGames(docs[0], docs[1]);
  readChapter(docs[1], 'History', 'history-intro');
  readChapter(docs[2], 'About', 'about-intro');
  renderToc('');
  show();
  guide.dataset.ready = '';
}).catch(function () {
  toc.innerHTML = '<p class="empty">The guide could not be loaded. Check your connection and try again.</p>';
});
}());
