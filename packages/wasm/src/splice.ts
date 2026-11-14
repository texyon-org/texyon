/** Replace `before[from, to)` with `insert`, in UTF-16 code units. */
export interface SourceEdit {
  from: number;
  to: number;
  insert: string;
}

/** Substring equality runs at memcmp speed, so compare in chunks first. */
const CHUNK = 256;

/** The splice between the common prefix and suffix of `before` and `after`. */
export function spliceBetween(before: string, after: string): SourceEdit {
  const shorter = Math.min(before.length, after.length);

  let prefix = 0;
  while (
    prefix + CHUNK <= shorter &&
    before.slice(prefix, prefix + CHUNK) === after.slice(prefix, prefix + CHUNK)
  ) {
    prefix += CHUNK;
  }
  while (prefix < shorter && before.charCodeAt(prefix) === after.charCodeAt(prefix)) prefix++;
  // Never split a surrogate pair: the engine counts in whole characters.
  if (prefix > 0 && isHighSurrogate(before.charCodeAt(prefix - 1))) prefix--;

  const room = shorter - prefix;
  let suffix = 0;
  while (
    suffix + CHUNK <= room &&
    before.slice(before.length - suffix - CHUNK, before.length - suffix) ===
      after.slice(after.length - suffix - CHUNK, after.length - suffix)
  ) {
    suffix += CHUNK;
  }
  while (
    suffix < room &&
    before.charCodeAt(before.length - 1 - suffix) === after.charCodeAt(after.length - 1 - suffix)
  ) {
    suffix++;
  }
  if (suffix > 0 && isLowSurrogate(before.charCodeAt(before.length - suffix))) suffix--;

  return {
    from: prefix,
    to: before.length - suffix,
    insert: after.slice(prefix, after.length - suffix),
  };
}

/** Apply `edit` to `text`. */
export function applyEdit(text: string, edit: SourceEdit): string {
  return text.slice(0, edit.from) + edit.insert + text.slice(edit.to);
}

function isHighSurrogate(unit: number): boolean {
  return unit >= 0xd800 && unit <= 0xdbff;
}

function isLowSurrogate(unit: number): boolean {
  return unit >= 0xdc00 && unit <= 0xdfff;
}
