// Expand one-line CSS rules longer than a screen into one declaration per line.
// Leaves every shorter line, and the meaning of the file, as it is.
//
//   node tools/format_css.mjs web/blip.css [more.css ...]

import { readFileSync, writeFileSync } from 'node:fs';

const LIMIT = 160;

/** Split at `sep` outside strings, brackets and parentheses. */
function splitTop(text, sep) {
  const parts = [];
  let depth = 0, quote = null, start = 0;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quote) { if (c === '\\') i++; else if (c === quote) quote = null; continue; }
    if (c === '"' || c === "'") quote = c;
    else if (c === '(' || c === '[') depth++;
    else if (c === ')' || c === ']') depth--;
    else if (c === sep && depth === 0) { parts.push(text.slice(start, i)); start = i + 1; }
  }
  parts.push(text.slice(start));
  return parts;
}

/** The blocks of a run of CSS: [prelude, body] pairs, where body is the text inside the braces. */
function blocks(text) {
  const out = [];
  let depth = 0, quote = null, open = -1, last = 0;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quote) { if (c === '\\') i++; else if (c === quote) quote = null; continue; }
    if (c === '"' || c === "'") quote = c;
    else if (c === '{' && depth++ === 0) open = i;
    else if (c === '}' && --depth === 0) { out.push([text.slice(last, open).trim(), text.slice(open + 1, i)]); last = i + 1; }
  }
  return text.slice(last).trim() ? null : out;
}

function format(text, indent) {
  const list = blocks(text);
  if (!list) return null;
  return list.map(([prelude, body]) => {
    if (prelude.startsWith('@')) {
      const inner = format(body, indent + '  ');
      return inner === null ? null : `${indent}${prelude} {\n${inner}\n${indent}}`;
    }
    const declarations = splitTop(body, ';').map((d) => d.trim()).filter(Boolean);
    const line = `${indent}${prelude} { ${declarations.join('; ')}; }`;
    if (line.length <= LIMIT) return line;
    return `${indent}${prelude} {\n${declarations.map((d) => `${indent}  ${d};`).join('\n')}\n${indent}}`;
  }).reduce((all, part) => (all === null || part === null ? null : all.concat(part)), []).join('\n');
}

for (const file of process.argv.slice(2)) {
  const lines = readFileSync(file, 'utf8').split('\n');
  let changed = 0;
  const out = lines.map((line) => {
    if (line.length <= LIMIT || line.includes('/*')) return line;
    const indent = line.match(/^\s*/)[0];
    const formatted = format(line.trim(), indent);
    if (formatted === null) return line;
    changed++;
    return formatted;
  });
  writeFileSync(file, out.join('\n'));
  console.log(`${file}: ${changed} long lines expanded`);
}
