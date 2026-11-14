/**
 * Reading the support pack, a ustar archive of flat file names.
 */

const BLOCK = 512;

/** Thrown when bytes are not a pack archive. */
export class TarError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TarError";
  }
}

function field(header: Uint8Array, start: number, length: number): string {
  const bytes = header.subarray(start, start + length);
  const end = bytes.indexOf(0);
  return new TextDecoder().decode(end < 0 ? bytes : bytes.subarray(0, end));
}

function isZero(block: Uint8Array): boolean {
  return block.every((byte) => byte === 0);
}

/** Every regular file in a ustar archive, by name, as views into `bytes`. */
export function untar(bytes: Uint8Array): Map<string, Uint8Array> {
  const files = new Map<string, Uint8Array>();
  let offset = 0;

  while (offset + BLOCK <= bytes.length) {
    const header = bytes.subarray(offset, offset + BLOCK);
    if (isZero(header)) return files;

    if (field(header, 257, 5) !== "ustar") {
      throw new TarError(`no ustar header at byte ${offset}`);
    }
    const prefix = field(header, 345, 155);
    const name = prefix ? `${prefix}/${field(header, 0, 100)}` : field(header, 0, 100);
    const size = parseInt(field(header, 124, 12).trim(), 8);
    if (!Number.isSafeInteger(size) || size < 0) {
      throw new TarError(`${name} has an unreadable size`);
    }

    const start = offset + BLOCK;
    if (start + size > bytes.length) throw new TarError(`${name} is truncated`);

    const type = String.fromCharCode(header[156]);
    if (type === "0" || type === "\0") files.set(name, bytes.subarray(start, start + size));

    offset = start + Math.ceil(size / BLOCK) * BLOCK;
  }

  throw new TarError("the archive has no end marker");
}

/** The pack's bytes, inflated if a server did not already do that on the way. */
export async function inflate(bytes: Uint8Array): Promise<Uint8Array> {
  // A host that labels .gz as Content-Encoding has the browser inflate it already.
  if (bytes[0] !== 0x1f || bytes[1] !== 0x8b) return bytes;
  const stream = new Blob([bytes.slice()]).stream().pipeThrough(new DecompressionStream("gzip"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}
