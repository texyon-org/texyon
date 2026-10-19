/**
 * The slice of WASI preview1 that the Tectonic module actually reaches.
 *
 * The module is built for `wasm32-wasip1` because that is the only Rust target
 * with a libc, and the engine is ~140k lines of C that needs one. It is not,
 * however, a program that wants an operating system: every file it reads
 * arrives through the `texyon_host_read_file` import instead, and everything
 * it writes stays in linear memory.
 *
 * So the syscalls here fall into three groups. Clocks, randomness and the
 * environment are answered for real, because libc initialization and TeX's
 * `\year`/`\time` want them. Writes to stdout and stderr are forwarded to a
 * callback. Everything filesystem-shaped reports "no such capability", which is
 * what a WASI program sees when it was granted no preopened directories — a
 * well-trodden path through wasi-libc, not an error case.
 */

/** WASI preview1 error numbers, as far as we need them. */
const enum Errno {
  Success = 0,
  /** Bad file descriptor. */
  Badf = 8,
  /** Invalid argument. */
  Inval = 28,
  /** No such file or directory. */
  Noent = 44,
  /** Function not implemented. */
  Nosys = 52,
  /** Capability insufficient — how "you have no filesystem" is spelled. */
  Notcapable = 76,
}

/** WASI clock ids. */
const enum ClockId {
  Realtime = 0,
  Monotonic = 1,
}

/** Where a `fd_write` went. */
export type StreamName = "stdout" | "stderr";

/** Everything the shim needs from its embedder. */
export interface WasiOptions {
  /** Called when the module writes to stdout or stderr. */
  onOutput?: (stream: StreamName, text: string) => void;
  /**
   * Wall-clock milliseconds since the Unix epoch.
   *
   * Overridable so a caller can pin it: TeX bakes `\today` and the PDF's
   * creation date into its output, and a fixed clock makes runs reproducible.
   */
  now?: () => number;
}

/** A live WASI instance, bound to one module. */
export class Wasi {
  /** The import object entry — pass as `wasi_snapshot_preview1`. */
  readonly imports: Record<string, (...args: never[]) => unknown>;

  #memory: WebAssembly.Memory | null = null;
  #options: WasiOptions;
  #decoder = new TextDecoder();

  constructor(options: WasiOptions = {}) {
    this.#options = options;
    this.imports = this.#build();
  }

  /** Bind the shim to the instantiated module's memory. */
  bind(memory: WebAssembly.Memory): void {
    this.#memory = memory;
  }

  get #view(): DataView {
    if (!this.#memory) throw new Error("the WASI shim was used before bind()");
    return new DataView(this.#memory.buffer);
  }

  get #bytes(): Uint8Array {
    if (!this.#memory) throw new Error("the WASI shim was used before bind()");
    return new Uint8Array(this.#memory.buffer);
  }

  #emit(stream: StreamName, chunk: Uint8Array): void {
    const text = this.#decoder.decode(chunk);
    if (text.length > 0) this.#options.onOutput?.(stream, text);
  }

  #build(): Record<string, (...args: never[]) => unknown> {
    // `iovs` is an array of (pointer, length) pairs; gathering them is the only
    // real work fd_write has to do.
    const gather = (iovs: number, count: number): Uint8Array => {
      const view = this.#view;
      const bytes = this.#bytes;
      const parts: Uint8Array[] = [];
      let total = 0;

      for (let i = 0; i < count; i++) {
        const ptr = view.getUint32(iovs + i * 8, true);
        const len = view.getUint32(iovs + i * 8 + 4, true);
        parts.push(bytes.subarray(ptr, ptr + len));
        total += len;
      }

      const out = new Uint8Array(total);
      let offset = 0;
      for (const part of parts) {
        out.set(part, offset);
        offset += part.length;
      }
      return out;
    };

    const notcapable = () => Errno.Notcapable;
    const badf = () => Errno.Badf;

    return {
      fd_write: (fd: number, iovs: number, count: number, written: number) => {
        const chunk = gather(iovs, count);
        // 1 and 2 are stdout and stderr; nothing else is open for writing.
        if (fd === 1 || fd === 2) {
          this.#emit(fd === 1 ? "stdout" : "stderr", chunk);
        } else if (fd !== 0) {
          return Errno.Badf;
        }
        this.#view.setUint32(written, chunk.length, true);
        return Errno.Success;
      },

      fd_read: (_fd: number, _iovs: number, _count: number, read: number) => {
        // stdin is permanently at end of file.
        this.#view.setUint32(read, 0, true);
        return Errno.Success;
      },

      fd_close: badf,
      fd_seek: badf,
      fd_fdstat_get: (_fd: number, stat: number) => {
        // libc calls this while setting up stdio. Reporting a character device
        // with no rights is enough to get it through initialization.
        const view = this.#view;
        view.setUint8(stat, 2 /* filetype: character device */);
        view.setUint16(stat + 2, 0, true);
        view.setBigUint64(stat + 8, 0n, true);
        view.setBigUint64(stat + 16, 0n, true);
        return Errno.Success;
      },
      fd_fdstat_set_flags: () => Errno.Success,
      fd_filestat_get: badf,
      fd_prestat_get: badf,
      fd_prestat_dir_name: badf,

      // No preopens means no path is reachable, and wasi-libc turns these into
      // the ENOENT/EPERM that the engine's I/O layer already handles — it never
      // gets here anyway, because texyon_host_read_file answers first.
      path_open: notcapable,
      path_filestat_get: notcapable,
      path_remove_directory: notcapable,
      path_unlink_file: notcapable,

      environ_sizes_get: (count: number, size: number) => {
        const view = this.#view;
        view.setUint32(count, 0, true);
        view.setUint32(size, 0, true);
        return Errno.Success;
      },
      environ_get: () => Errno.Success,

      clock_time_get: (id: number, _precision: bigint, out: number) => {
        const now = this.#options.now?.() ?? Date.now();
        const nanos =
          id === ClockId.Monotonic
            ? BigInt(Math.round(performance.now() * 1e6))
            : BigInt(Math.round(now)) * 1_000_000n;
        this.#view.setBigUint64(out, nanos, true);
        return Errno.Success;
      },

      random_get: (ptr: number, len: number) => {
        // crypto.getRandomValues caps at 65536 bytes per call.
        const target = this.#bytes.subarray(ptr, ptr + len);
        for (let i = 0; i < target.length; i += 65536) {
          crypto.getRandomValues(target.subarray(i, Math.min(i + 65536, target.length)));
        }
        return Errno.Success;
      },

      proc_exit: (code: number) => {
        // wasi-libc calls this from abort(). Throwing unwinds out of whatever
        // engine call we are inside, which the caller reports as a failed
        // compile; the module is poisoned afterwards either way.
        throw new WasiExit(code);
      },

      // Present so instantiation cannot fail on a call we did not predict.
      sched_yield: () => Errno.Success,
      poll_oneoff: () => Errno.Nosys,
      args_get: () => Errno.Success,
      args_sizes_get: (count: number, size: number) => {
        const view = this.#view;
        view.setUint32(count, 0, true);
        view.setUint32(size, 0, true);
        return Errno.Success;
      },
      fd_datasync: badf,
      fd_sync: badf,
      fd_tell: badf,
      path_create_directory: notcapable,
      path_rename: notcapable,
      fd_readdir: badf,
      fd_pread: badf,
      fd_pwrite: badf,
      fd_advise: () => Errno.Success,
      fd_allocate: badf,
      fd_renumber: badf,
      path_link: notcapable,
      path_readlink: notcapable,
      path_symlink: notcapable,
      path_filestat_set_times: notcapable,
      fd_filestat_set_size: badf,
      fd_filestat_set_times: badf,
      clock_res_get: (_id: number, out: number) => {
        this.#view.setBigUint64(out, 1_000_000n, true);
        return Errno.Success;
      },
      sock_accept: badf,
      sock_recv: badf,
      sock_send: badf,
      sock_shutdown: badf,
    } as unknown as Record<string, (...args: never[]) => unknown>;
  }
}

/** Thrown when the module calls `proc_exit`, i.e. when the engine aborted. */
export class WasiExit extends Error {
  /** The exit status the module asked for. */
  readonly code: number;

  constructor(code: number) {
    super(`the Tectonic engine exited with status ${code}`);
    this.name = "WasiExit";
    this.code = code;
  }
}
