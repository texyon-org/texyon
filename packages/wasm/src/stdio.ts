/**
 * Serves the module's session over stdio in the frames `texyon preview`
 * speaks (a little-endian u32 length, then a MessagePack map), so one client
 * drives either host. `cargo xtask` uses it to test, benchmark and profile the
 * module.
 *
 * Usage: node stdio.js <module.wasm> <pack-dir> [--profile <file>], bundled by esbuild.
 *
 * Support files come from the pack, then from the project root the session was
 * opened with. A module built for profile-guided optimization writes its
 * counters to `--profile` when the input ends.
 */

import { readFileSync, readSync, writeFileSync, writeSync } from "node:fs";
import { join } from "node:path";
import { decode, encode } from "@msgpack/msgpack";

import { Engine } from "./engine.js";

const STDIN = 0;
const STDOUT = 1;
const STDERR = 2;

const [modulePath, packDir, flag, profilePath] = process.argv.slice(2);
if (!modulePath || !packDir || (flag !== undefined && (flag !== "--profile" || !profilePath))) {
  writeSync(STDERR, "usage: stdio.js <module.wasm> <pack-dir> [--profile <file>]\n");
  process.exit(2);
}

const packFiles = (JSON.parse(readFileSync(join(packDir, "manifest.json"), "utf8")) as { files: object }).files;
let projectRoot: string | null = null;

function readOptional(path: string): Uint8Array | null {
  try {
    return readFileSync(path);
  } catch {
    return null;
  }
}

function fromProject(path: string): Uint8Array | null {
  return projectRoot === null ? null : readOptional(join(projectRoot, path));
}

const engine = await Engine.start(
  async (imports) => (await WebAssembly.instantiate(readFileSync(modulePath), imports)).instance,
  {
    readFile: (name) => (name in packFiles ? readOptional(join(packDir, "files", name)) : fromProject(name)),
    readSource: (path) => fromProject(path)?.toString(),
    // Every report carries the TeX diagnostics, so only the module's own output is shown.
    log: () => {},
    output: (_stream, text) => writeSync(STDERR, text),
  },
);

/** Read exactly `length` bytes, or null if the input ends before the first one. */
function readExactly(length: number): Uint8Array | null {
  const bytes = new Uint8Array(length);
  let filled = 0;
  while (filled < length) {
    const read = readSync(STDIN, bytes, filled, length - filled, null);
    if (read === 0) {
      if (filled === 0) return null;
      throw new Error("the input ended inside a frame");
    }
    filled += read;
  }
  return bytes;
}

function readMessage(): unknown {
  const header = readExactly(4);
  if (!header) return null;
  const length = new DataView(header.buffer).getUint32(0, true);
  return decode(readExactly(length) ?? new Uint8Array());
}

function writeMessage(message: unknown): void {
  const body = encode(message, { ignoreUndefined: true });
  const header = new Uint8Array(4);
  new DataView(header.buffer).setUint32(0, body.length, true);
  for (const chunk of [header, body]) {
    let written = 0;
    while (written < chunk.length) written += writeSync(STDOUT, chunk, written);
  }
}

type Fields = Record<string, unknown>;

/** Rebuild `value` with `swap` applied to every object in it, innermost first. */
function rebuild(value: unknown, swap: (fields: Fields) => Fields): unknown {
  if (Array.isArray(value)) return value.map((item) => rebuild(item, swap));
  if (value === null || typeof value !== "object" || value instanceof Uint8Array) return value;
  return swap(Object.fromEntries(Object.entries(value).map(([key, field]) => [key, rebuild(field, swap)])));
}

// The stream carries bytes inline as `data`, the module as `ptr` and `len` in its memory.
function toModule(fields: Fields): Fields {
  const { data, ...rest } = fields;
  return data instanceof Uint8Array ? { ...rest, ...engine.intoModule(data) } : fields;
}

function fromModule(fields: Fields): Fields {
  const { ptr, len, ...rest } = fields;
  return typeof ptr === "number" && typeof len === "number" ? { ...rest, data: engine.take({ ptr, len }) } : fields;
}

for (let request = readMessage(); request !== null; request = readMessage()) {
  const { op, root } = request as { op?: string; root?: string };
  if (op === "open" && typeof root === "string") projectRoot = root;
  let response: unknown;
  try {
    response = rebuild(engine.call(rebuild(request, toModule) as object), fromModule);
  } catch (error) {
    response = { status: "error", message: error instanceof Error ? error.message : String(error) };
  }
  writeMessage(response);
}

if (profilePath) {
  const profile = engine.profile();
  if (!profile) throw new Error(`${modulePath} does not count, build it with cargo xtask wasm --pgo`);
  writeFileSync(profilePath, profile);
}
