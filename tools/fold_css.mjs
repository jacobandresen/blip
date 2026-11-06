// Fold a CSS rule into a later rule with the same selector in the same @media,
// when no rule between them sets a property of the same family: the cascade is
// then unchanged. Declarations the later rule overrides are dropped, and a rule
// left with none is removed.
//
//   node tools/fold_css.mjs web/kiosk.css          report what would fold
//   node tools/fold_css.mjs web/kiosk.css write    fold it
//
// Then run tools/format_css.mjs on the file, and compare the pages' computed
// styles before and after; a comment above a removed rule may need removing too.
import { readFileSync, writeFileSync } from 'node:fs';
const [file, write] = process.argv.slice(2);
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
  for (const c of body) {
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
