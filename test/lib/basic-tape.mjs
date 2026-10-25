/** Parse numbered BLIP source into the mutable line map used by tape fixtures. */
export function numberedTape(source) {
  let previous = -1;
  const entries = source.split(/\r?\n/).filter((line) => line.trim()).map((line) => {
    const match = line.match(/^\s*(\d+)\s+(.*)$/);
    if (!match) throw new Error(`invalid numbered tape line: ${line}`);
    const number = Number(match[1]);
    if (!Number.isSafeInteger(number) || number <= previous) {
      throw new Error(`tape line numbers must be unique and ascending: ${number}`);
    }
    previous = number;
    return [number, match[2]];
  });
  return new Map(entries);
}
