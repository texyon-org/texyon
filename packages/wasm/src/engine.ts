/**
 * The module bound to its host: the imports it calls back into, the request
 * call, and the buffers it leaves in linear memory. The browser Worker and the
 * Node stdio host both run the module through this.
 */

import type { LogLevel } from "./protocol.js";
import { type StreamName, Wasi } from "./wasi.js";

/** The module's exports, see `crates/wasm/src/lib.rs`. */
export interface Exports {
  memory: WebAssembly.Memory;
  _initialize?: () => void;
  texyon_request(reqPtr: number, reqLen: number, outPtr: number, outLen: number): number;
  texyon_wasm_alloc(len: number): number;
  texyon_wasm_free(ptr: number, len: number): void;
  /** Only in a module built to count for profile-guided optimization. */
  texyon_pgo_profile?(outPtr: number, outLen: number): number;
}

/** Bytes left in linear memory for the host to take and free. */
export interface Buffer {
  ptr: number;
  len: number;
}

/** What the module asks of its host while it runs. */
export interface EngineHost {
  /** A support file, or nothing when there is none. */
  readFile(name: string): Uint8Array | null | undefined;
  /** The current text of a project source the session does not hold. */
  readSource(path: string): string | null | undefined;
  /** A line the engine reports, such as a TeX error. */
  log(level: LogLevel, message: string): void;
  /** What the module writes to stdout or stderr, a Rust panic among it. `log` takes it when absent. */
  output?(stream: StreamName, text: string): void;
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();

/** A running module. */
export class Engine {
  readonly exports: Exports;

  private constructor(exports: Exports) {
    this.exports = exports;
  }

  /** Instantiate the module with `host`'s imports. `instantiate` fetches and compiles it. */
  static async start(
    instantiate: (imports: WebAssembly.Imports) => Promise<WebAssembly.Instance>,
    host: EngineHost,
  ): Promise<Engine> {
    let engine: Engine | null = null;
    const bound = () => {
      if (!engine) throw new Error("the module called its host before it was instantiated");
      return engine;
    };
    const wasi = new Wasi((stream, text) => {
      if (host.output) host.output(stream, text);
      else host.log(stream === "stderr" ? "error" : "note", text);
    });
    const imports: WebAssembly.Imports = {
      wasi_snapshot_preview1: wasi.imports,
      env: {
        texyon_host_read_file: (namePtr: number, nameLen: number, outPtr: number, outLen: number) =>
          bound().#answer(host.readFile(bound().#string(namePtr, nameLen)), outPtr, outLen),
        texyon_host_read_source: (namePtr: number, nameLen: number, outPtr: number, outLen: number) => {
          const text = host.readSource(bound().#string(namePtr, nameLen));
          return bound().#answer(text == null ? null : encoder.encode(text), outPtr, outLen);
        },
        texyon_host_log: (kind: number, msgPtr: number, msgLen: number) =>
          host.log(kind === 2 ? "error" : kind === 1 ? "warning" : "note", bound().#string(msgPtr, msgLen)),
      },
    };

    const instance = await instantiate(imports);
    engine = new Engine(instance.exports as unknown as Exports);
    wasi.bind(engine.exports.memory);
    engine.exports._initialize?.();
    return engine;
  }

  /** Copy `bytes` into linear memory, for the module to take over. */
  intoModule(bytes: Uint8Array): Buffer {
    const ptr = this.exports.texyon_wasm_alloc(bytes.length);
    if (ptr === 0 && bytes.length > 0) throw new Error("the engine is out of memory");
    new Uint8Array(this.exports.memory.buffer, ptr, bytes.length).set(bytes);
    return { ptr, len: bytes.length };
  }

  /** Copy bytes out of linear memory and free them there. */
  take({ ptr, len }: Buffer): Uint8Array<ArrayBuffer> {
    const bytes = new Uint8Array(this.exports.memory.buffer, ptr, len).slice();
    this.exports.texyon_wasm_free(ptr, len);
    return bytes;
  }

  /** Send one JSON request and return the parsed response. Its buffers are the caller's to take. */
  call(request: object): unknown {
    const input = this.intoModule(encoder.encode(JSON.stringify(request)));
    const out = this.exports.texyon_wasm_alloc(8);
    try {
      if (this.exports.texyon_request(input.ptr, input.len, out, out + 4) !== 0) {
        throw new Error("the engine could not produce a response");
      }
      const view = new DataView(this.exports.memory.buffer);
      const reply = this.take({ ptr: view.getUint32(out, true), len: view.getUint32(out + 4, true) });
      return JSON.parse(decoder.decode(reply));
    } finally {
      this.exports.texyon_wasm_free(out, 8);
      this.exports.texyon_wasm_free(input.ptr, input.len);
    }
  }

  /** The profile counters as a `.profraw` file, or null for a module that does not count. */
  profile(): Uint8Array | null {
    if (!this.exports.texyon_pgo_profile) return null;
    const out = this.exports.texyon_wasm_alloc(8);
    try {
      if (this.exports.texyon_pgo_profile(out, out + 4) !== 0) return null;
      const view = new DataView(this.exports.memory.buffer);
      return this.take({ ptr: view.getUint32(out, true), len: view.getUint32(out + 4, true) });
    } finally {
      this.exports.texyon_wasm_free(out, 8);
    }
  }

  #answer(bytes: Uint8Array | null | undefined, outPtr: number, outLen: number): number {
    if (!bytes) return 1;
    const buffer = this.intoModule(bytes);
    const view = new DataView(this.exports.memory.buffer);
    view.setUint32(outPtr, buffer.ptr, true);
    view.setUint32(outLen, buffer.len, true);
    return 0;
  }

  #string(ptr: number, len: number): string {
    return decoder.decode(new Uint8Array(this.exports.memory.buffer, ptr, len));
  }
}
