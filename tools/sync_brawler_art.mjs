import { readFile, writeFile } from 'node:fs/promises';

const atlas = JSON.parse(await readFile(new URL('../crates/brawler/art/atlas.json', import.meta.url), 'utf8'));
const tape = new URL('../web/TAPE/BRAWLER.BLIP', import.meta.url);
const source = await readFile(tape, 'utf8');
const next = source.replace(/^(62\d{3} DATA "([^"]+)"[^\n]*)$/gm, (line, data, name) => {
  const cell = atlas.find((entry) => entry.name === name);
  if (!cell) throw new Error(`No Rust pose sheet for ${name}`);
  const values = data.match(/"[^"]*"|[^,]+/g);
  return `${values.slice(0, 11).join(',')},"${cell.trim}",${cell.width},${cell.height},${cell.feet},${cell.rate},${cell.bulk},${Number(cell.boss)}`;
});
await writeFile(tape, next);
console.log(`Updated BASIC's ${atlas.length} pose-sheet anchors.`);
