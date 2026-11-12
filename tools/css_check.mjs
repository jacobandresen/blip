// Check a CSS change by what the browser computes, and find CSS that never takes effect.
//
//   node tools/css_check.mjs snapshot <out.json> [--repo <dir>] [--views desktop,phone] [--page index.html]
//   node tools/css_check.mjs compare <before.json> <after.json>
//   node tools/css_check.mjs dead <out.json> [--views ...]
//   node tools/css_check.mjs prune <dead.json> web/blip.css [more.css ...]
//   node tools/css_check.mjs fold web/blip.css [more.css ...] [--write]
//
// snapshot records every element's computed style (and ::before/::after) on each page of a matrix of views and
// page states in headless Chromium; compare lists the elements that differ. Snapshot before a refactor (or with
// --repo, a worktree of the last commit), snapshot after, compare: none should differ. --page records the full
// styles of one page, so compare shows which properties changed.
//
// dead finds declarations that never win the cascade: their rule matched an element in some state of the matrix
// and another declaration won every time. prune deletes them (keeping rules with :hover, :active or :focus, which
// the matrix never exercises), then the rules and @media blocks left empty. fold merges a rule into a later one
// with the same selector where nothing between could change the outcome. Snapshot and compare after either.
// Game wasm is not loaded; transitions and animations are not compared.
// A rule that only styles an element while script measures it (the manual's pagination) can look dead: snapshot the
// pages that measure, in every view, after a prune.
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('..', import.meta.url));
const [command, ...rest] = process.argv.slice(2);
const flag = (name) => { const i = rest.indexOf(`--${name}`); return i < 0 ? null : rest.splice(i, 2)[1]; };

const VIEWS = [
  { name: 'desktop', viewport: { width: 1280, height: 800 } },
  { name: 'kiosk', viewport: { width: 1920, height: 1080 }, fullscreen: true },
  { name: 'phone', viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true },
  { name: 'tiny', viewport: { width: 320, height: 568 }, hasTouch: true, isMobile: true },
  { name: 'landscape', viewport: { width: 844, height: 390 }, hasTouch: true, isMobile: true },
  { name: 'tablet', viewport: { width: 1024, height: 1366 }, hasTouch: true, isMobile: true },
];
// [page, with a credit, two players]
function pages(games) {
  return [['index.html'], ['index.html', 1], ['index.html?room=lite'], ['home.html'], ['guide.html'],
    ['index.html?manual=controls'], ...games.flatMap((g) => [[`${g}/index.html`], [`${g}/index.html`, 1, 1]]),
    ['serpent/index.html?app'], ['bubbler/index.html?app', 0, 1], ['rally/index.html?controls=pad'],
    ['brawler/index.html?controls=pad', 1, 1], ['bubbler/index.html?controls=stick', 1], ['warlock/index.html?controls=touch', 1, 1],
    ['machine-deck.html?game=brawler'], ['machine-deck.html?game=rally']];
}

/** Visit every page of the matrix in every view, calling visit(page, key) on each. */
async function walk(repo, views, onlyPage, visit, reduceMotion = true) {
  const { startPage } = await import(path.join(repo, 'test/lib/harness.mjs'));
  const { GAME_SLUGS } = await import(path.join(repo, 'test/lib/games.mjs'));
  for (const v of VIEWS.filter((v) => !views || views.split(',').includes(v.name))) {
    const h = await startPage('chromium', { viewport: v.viewport, hasTouch: v.hasTouch, isMobile: v.isMobile });
    try {
      await h.page.route('**/*.wasm', (r) => r.abort());
      // A snapshot holds still; a search for dead CSS must see the motion rules that reduced motion turns off.
      await h.page.emulateMedia({ reducedMotion: reduceMotion ? 'reduce' : 'no-preference' });
      for (const [p, credit, two] of pages(GAME_SLUGS)) {
        if (onlyPage && p !== onlyPage) continue;
        // Each page from a clean slate: a visit to the home screen would put later pages in app style.
        await h.page.goto(`${h.origin}/blank.html`).catch(() => {});
        await h.page.evaluate(([fs, credit]) => {
          localStorage.clear(); sessionStorage.clear();
          if (fs) localStorage.setItem('blip-fullscreen', '1');
          if (credit) sessionStorage.setItem('blip-coins', '2');
        }, [!!v.fullscreen, !!credit]);
        await h.page.goto(`${h.origin}/${p}`);
        await h.page.waitForTimeout(900);
        if (two) { await h.page.evaluate(() => { try { blipSetMode(1); } catch (e) {} }); await h.page.waitForTimeout(250); }
        await visit(h.page, `${v.name} ${p}${credit ? ' credit' : ''}${two ? ' 2P' : ''}`);
      }
    } finally { await h.close(); }
    console.error(`${v.name} done`);
  }
}

async function snapshot(out) {
  const repo = path.resolve(flag('repo') || ROOT), views = flag('views'), page = flag('page');
  const result = {};
  await walk(repo, views, page, async (p, key) => {
    result[key] = await p.evaluate((full) => {
      const r = {}, skip = /^(transition|animation|will-change|view-transition|--)/;
      // An element by its path of tag, id, classes and place among like siblings.
      const name = (el) => {
        let i = 0, s = el;
        while ((s = s.previousElementSibling)) if (s.tagName === el.tagName && s.className === el.className) i++;
        const cls = typeof el.className === 'string' && el.className.trim() ? '.' + el.className.trim().split(/\s+/).sort().join('.') : '';
        return el.tagName + (el.id ? '#' + el.id : '') + cls + ':' + i;
      };
      const pathOf = (el) => { const a = []; for (; el && el !== document.documentElement; el = el.parentElement) a.unshift(name(el)); return a.join('>'); };
      for (const el of document.querySelectorAll('body, body *')) for (const pseudo of [null, '::before', '::after']) {
        const cs = getComputedStyle(el, pseudo);
        if (pseudo && (cs.content === 'none' || cs.content === 'normal')) continue;
        const parts = [];
        for (let i = 0; i < cs.length; i++) if (!skip.test(cs[i])) parts.push(cs[i] + ':' + cs.getPropertyValue(cs[i]));
        const text = parts.sort().join(';').split(location.origin).join('');
        let h = 2166136261;
        for (let i = 0; i < text.length; i++) { h ^= text.charCodeAt(i); h = Math.imul(h, 16777619); }
        r[pathOf(el) + (pseudo || '')] = full ? text : (h >>> 0).toString(36);
      }
      return r;
    }, !!page);
  });
  writeFileSync(out, JSON.stringify(result));
}

function compare(a, b) {
  const [A, B] = [a, b].map((f) => JSON.parse(readFileSync(f, 'utf8')));
  const changed = {};
  const props = (t) => Object.fromEntries(t.split(';').map((p) => [p.slice(0, p.indexOf(':')), p.slice(p.indexOf(':') + 1)]));
  for (const state in A) for (const el in A[state]) {
    const x = A[state][el], y = B[state]?.[el];
    if (x === y) continue;
    const where = el.split('>').slice(-2).join('>');
    let what = y === undefined ? 'missing' : 'changed';
    if (y && x.includes(':')) {
      const X = props(x), Y = props(y);
      what = Object.keys(X).filter((k) => X[k] !== Y[k]).map((k) => `${k}: ${X[k].slice(0, 50)} -> ${(Y[k] || '').slice(0, 50)}`).join('; ');
    }
    (changed[`${where} ${what}`] ??= []).push(state);
  }
  for (const state in B) for (const el in B[state]) if (!(el in (A[state] || {}))) (changed[`${el.split('>').slice(-2).join('>')} new`] ??= []).push(state);
  const keys = Object.keys(changed);
  for (const k of keys) console.log(`${k}  [${changed[k].length}: ${changed[k].slice(0, 2).join(', ')}]`);
  console.log(`${keys.length} differences`);
  process.exitCode = keys.length ? 1 : 0;
}

async function dead(out) {
  const views = flag('views');
  const won = new Set(), declared = {};
  await walk(ROOT, views, null, async (p) => {
    const cdp = await p.context().newCDPSession(p), urls = {};
    cdp.on('CSS.styleSheetAdded', ({ header }) => { urls[header.styleSheetId] = path.basename(header.sourceURL.replace(/\?.*$/, '')); });
    await cdp.send('DOM.enable'); await cdp.send('CSS.enable');
    const { root } = await cdp.send('DOM.getDocument', { depth: -1, pierce: true });
    const ids = [];
    (function add(n) { if (n.nodeType === 1) ids.push(n.nodeId); for (const c of [...(n.children || []), ...(n.shadowRoots || [])]) add(c); if (n.contentDocument) add(n.contentDocument); })(root);
    for (const nodeId of ids) {
      let m; try { m = await cdp.send('CSS.getMatchedStylesForNode', { nodeId }); } catch { continue; }
      [m.matchedCSSRules || [], ...(m.pseudoElements || []).map((pe) => pe.matches || [])].forEach((rules, own) => {
        const win = {};
        for (const r of rules) {
          const st = r.rule.style;
          if (r.rule.origin !== 'regular' || !st.range) continue;
          for (const d of st.cssProperties) {
            if (!d.range || d.disabled || d.parsedOk === false) continue;
            const key = `${urls[st.styleSheetId]}:${d.range.startLine}:${d.range.startColumn}:${d.range.endLine}:${d.range.endColumn}`;
            declared[key] = { selector: r.rule.selectorList.text, text: d.text };
            for (const l of d.longhandProperties || [{ name: d.name }]) if (!win[l.name] || d.important || !win[l.name].important) win[l.name] = { key, important: !!d.important };
          }
        }
        if (!own && m.inlineStyle) for (const d of m.inlineStyle.cssProperties) {
          if (!d.range) continue;
          for (const l of d.longhandProperties || [{ name: d.name }]) if (!win[l.name]?.important || d.important) win[l.name] = { key: 'inline', important: !!d.important };
        }
        for (const w of Object.values(win)) won.add(w.key);
      });
    }
    await cdp.detach();
  }, false);
  const list = Object.entries(declared).filter(([k]) => !won.has(k)).map(([key, d]) => ({ key, ...d }));
  writeFileSync(out, JSON.stringify(list, null, 1));
  console.log(`${Object.keys(declared).length} declarations in matched rules, ${list.length} never win`);
}

// ---- prune ----
/** CSS as items: comment, ws, atline ("@import …;"), rule { prelude, body } or block { prelude, items } (@media and kin). */
function parse(s, i = 0, nested = false) {
  const items = [];
  const skipString = (j) => { const q = s[j]; j++; while (s[j] !== q) j += s[j] === '\\' ? 2 : 1; return j + 1; };
  while (i < s.length) {
    if (s.startsWith('/*', i)) { const j = s.indexOf('*/', i) + 2; items.push({ type: 'comment', text: s.slice(i, j) }); i = j; continue; }
    if (/\s/.test(s[i])) { let j = i; while (j < s.length && /\s/.test(s[j])) j++; items.push({ type: 'ws', text: s.slice(i, j) }); i = j; continue; }
    if (s[i] === '}') { if (nested) return [items, i + 1]; throw new Error(`stray } at line ${s.slice(0, i).split('\n').length}`); }
    let j = i;
    while (j < s.length && s[j] !== '{' && s[j] !== ';') {
      if (s.startsWith('/*', j)) { j = s.indexOf('*/', j) + 2; continue; }
      j = s[j] === '"' || s[j] === "'" ? skipString(j) : j + 1;
    }
    if (s[j] === ';') { items.push({ type: 'atline', text: s.slice(i, j + 1) }); i = j + 1; continue; }
    const prelude = s.slice(i, j);
    if (/^@(media|supports|container|layer|scope|starting-style)\b/.test(prelude.trim())) {
      const [inner, k] = parse(s, j + 1, true);
      items.push({ type: 'block', prelude, items: inner }); i = k; continue;
    }
    let depth = 1, k = j + 1;
    while (depth) {
      if (s.startsWith('/*', k)) { k = s.indexOf('*/', k) + 2; continue; }
      if (s[k] === '"' || s[k] === "'") { k = skipString(k); continue; }
      if (s[k] === '{') depth++; else if (s[k] === '}') depth--;
      k++;
    }
    items.push({ type: 'rule', prelude, body: s.slice(j + 1, k - 1) }); i = k;
  }
  return [items, i];
}
function render(items) {
  return items.map((it) => it.type === 'rule' ? `${it.prelude}{${it.body}}` : it.type === 'block' ? `${it.prelude}{${render(it.items)}}` : it.text).join('');
}
/** Drop rules with no declarations and blocks with no rules, each with the comment just before it. */
function dropEmpty(items) {
  const out = [];
  const drop = () => { while (out.at(-1)?.type === 'ws') out.pop(); if (out.at(-1)?.type === 'comment') out.pop(); };
  for (let it of items) {
    if (it.type === 'rule' && !it.body.replace(/\/\*[\s\S]*?\*\//g, '').trim()) { drop(); continue; }
    if (it.type === 'block') {
      it = { ...it, items: dropEmpty(it.items) };
      if (!it.items.some((x) => x.type === 'rule' || x.type === 'block')) { drop(); continue; }
    }
    out.push(it);
  }
  return out;
}
function prune(deadFile, files) {
  const list = JSON.parse(readFileSync(deadFile, 'utf8'));
  const interaction = /:(hover|active|focus|focus-visible|focus-within)\b/;
  for (const file of files) {
    const lines = readFileSync(file, 'utf8').split('\n');
    const spans = [...new Set(list.filter((d) => d.key.startsWith(path.basename(file) + ':') && !interaction.test(d.selector))
      .map((d) => d.key.split(':').slice(1).join(':')))].map((k) => k.split(':').map(Number))
      .sort((x, y) => y[0] - x[0] || y[1] - x[1]);
    for (const [l0, c0, l1, c1] of spans) lines.splice(l0, l1 - l0 + 1, lines[l0].slice(0, c0) + lines[l1].slice(c1));
    let text = lines.join('\n').replace(/\n[ \t]+(?=\n)/g, '\n');
    text = render(dropEmpty(parse(text)[0])).replace(/\n{3,}/g, '\n\n');
    writeFileSync(file, text);
    console.log(`${file}: ${spans.length} declarations removed`);
  }
}

/** Fold a rule into a later rule with the same selector in the same @media when no rule between them sets a property
 * of the same family: the cascade is then unchanged. */
function fold(file, write) {
  const css = readFileSync(file, 'utf8');

  // Parse: rules with their context, selector, the span of the whole rule and its declarations.
  const rules = [];
  {
    let i = 0, last = 0; const ctx = [];
    const skipComment = () => { const e = css.indexOf('*/', i + 2); i = e + 2; };
    while (i < css.length) {
      if (css.startsWith('/*', i)) { skipComment(); if (ctx.length === 0 || ctx.at(-1).type === 'at') last = i; continue; }
      const c = css[i];
      if (c === '"' || c === "'") { i = css.indexOf(c, i + 1) + 1; continue; }
      if (c === '{') {
        const prelude = css.slice(last, i).trim();
        if (prelude.startsWith('@')) { ctx.push({ type: 'at', prelude }); last = i + 1; i++; continue; }
        // the block of a plain rule: find its end
        let j = i + 1, depth = 1;
        while (depth) { if (css.startsWith('/*', j)) { j = css.indexOf('*/', j) + 2; continue; } if (css[j] === '{') depth++; if (css[j] === '}') depth--; j++; }
        const body = css.slice(i + 1, j - 1);
        const nested = body.includes('{');
        // the rule starts at its selector's first character
        let start = last; while (/\s/.test(css[start])) start++;
        const decls = nested ? null : splitDecls(body);
        rules.push({ ctx: ctx.map((c) => c.prelude).join(' / '), selector: prelude.replace(/\s+/g, ' '), start, open: i, end: j, decls });
        i = j; last = j; continue;
      }
      if (c === '}') { ctx.pop(); last = i + 1; }
      i++;
    }
  }
  function splitDecls(body) {
    const out = []; let depth = 0, quote = null, cur = '';
    // A comment may hold a semicolon: comments are not declarations.
    for (const c of body.replace(/\/\*[\s\S]*?\*\//g, '')) {
      if (quote) { cur += c; if (c === quote) quote = null; continue; }
      if (c === '"' || c === "'") { quote = c; cur += c; continue; }
      if (c === '(') depth++; if (c === ')') depth--;
      if (c === ';' && depth === 0) { if (cur.trim()) out.push(cur.trim()); cur = ''; } else cur += c;
    }
    if (cur.trim()) out.push(cur.trim());
    return out.map((text) => { const k = text.indexOf(':'); return { prop: text.slice(0, k).trim().toLowerCase(), value: text.slice(k + 1).trim(), text }; });
  }
  const family = (prop) => prop.startsWith('--') ? prop : prop.replace(/^-(webkit|moz|ms)-/, '').split('-')[0];
  const important = (d) => /!important\s*$/.test(d.value);

  const merges = [];
  const removed = new Set();
  for (let a = 0; a < rules.length; a++) {
    const A = rules[a];
    if (!A.decls || removed.has(a)) continue;
    const b = rules.findIndex((r, k) => k > a && r.decls && r.ctx === A.ctx && r.selector === A.selector);
    if (b < 0) continue;
    const B = rules[b];
    const bProps = new Set(B.decls.map((d) => d.prop));
    // an !important in A that B does not also make important would stop winning: leave the pair
    if (A.decls.some((d) => important(d) && bProps.has(d.prop) && !B.decls.some((e) => e.prop === d.prop && important(e)))) continue;
    // a later value in newer CSS may be an override that older browsers skip, leaving the earlier one as the fallback
    const modern = /[sdl]v[hw]|env\(|color-mix|:has|@container/;
    if (A.decls.some((d) => bProps.has(d.prop) && B.decls.some((e) => e.prop === d.prop && modern.test(e.value) && e.value !== d.value))) continue;
    const moved = A.decls.filter((d) => !bProps.has(d.prop));
    const families = new Set(moved.map((d) => family(d.prop)));
    const between = rules.slice(a + 1, b);
    const clash = between.some((r) => !r.decls || r.decls.some((d) => families.has(family(d.prop))));
    if (clash) continue;
    merges.push({ a, b, moved, dropped: A.decls.length - moved.length });
    removed.add(a);
    B.decls = moved.concat(B.decls);
  }
  console.log(`${file}: ${merges.length} rules folded into a later one, ${merges.reduce((n, m) => n + m.dropped, 0)} overridden declarations dropped`);
  if (write) {
    // rewrite from the end so earlier spans stay valid
    const edits = [];
    for (const m of merges) {
      const A = rules[m.a];
      // take the rule's own line(s) with it
      let s = A.start; while (s > 0 && css[s - 1] !== '\n' && /\s/.test(css[s - 1])) s--;
      let e = A.end; while (css[e] === ' ') e++; if (css[e] === '\n') e++;
      edits.push({ from: s, to: e, text: '' });
    }
    for (const [k, R] of rules.entries()) {
      const target = merges.filter((m) => m.b === k);
      if (!target.length) continue;
      const original = css.slice(R.open + 1, R.end - 1);
      const multi = original.includes('\n');
      const indent = multi ? (original.match(/\n([ \t]*)\S/) || [, '  '])[1] : ' ';
      const movedText = target.flatMap((m) => m.moved).map((d) => d.text + ';');
      if (!movedText.length) continue;
      const text = multi ? '\n' + movedText.map((t) => indent + t).join('\n') + original.replace(/^\s*\n?/, '\n').replace(/^\n/, '\n') : ' ' + movedText.join(' ') + ' ' + original.trim() + ' ';
      edits.push({ from: R.open + 1, to: R.end - 1, text: multi ? '\n' + movedText.map((t) => indent + t).join('\n') + '\n' + original.replace(/^\s*\n/, '') : text });
    }
    edits.sort((x, y) => y.from - x.from);
    let out = css;
    for (const ed of edits) out = out.slice(0, ed.from) + ed.text + out.slice(ed.to);
    writeFileSync(file, out);
  }
}

if (command === 'snapshot') await snapshot(rest[0]);
else if (command === 'compare') compare(rest[0], rest[1]);
else if (command === 'dead') await dead(rest[0]);
else if (command === 'prune') prune(rest[0], rest.slice(1));
else if (command === 'fold') for (const f of rest.filter((a) => a !== '--write')) fold(f, rest.includes('--write'));
else console.log(readFileSync(fileURLToPath(import.meta.url), 'utf8').split('\n').filter((l) => l.startsWith('//')).join('\n'));
