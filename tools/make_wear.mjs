import { copyFile, stat } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const template = fileURLToPath(new URL('./wear.template.css', import.meta.url));
const output = fileURLToPath(new URL('../web/wear.css', import.meta.url));
await copyFile(template, output);
const { size } = await stat(output);
console.log(`wrote ${output} (${size.toLocaleString()} bytes)`);
