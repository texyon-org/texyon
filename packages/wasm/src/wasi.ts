/** WASI preview1 errno values. */
const enum Errno {
  Success = 0,
  Badf = 8,
  Nosys = 52,
  Notcapable = 76,
}

const enum ClockId {
  Monotonic = 1,
}

/** Where a module wrote. */
export type StreamName = "stdout" | "stderr";

/**
 * The WASI preview1 calls the module reaches. Files come through host imports
 * instead, so the filesystem calls report no capability.
 */
export class Wasi {
  /** The `wasi_snapshot_preview1` import object. */
  readonly imports: WebAssembly.ModuleImports;

  #memory: WebAssembly.Memory | null = null;
  #decoder = new TextDecoder();

  constructor(onOutput: (stream: StreamName, text: string) => void) {
    const view = () => new DataView(this.#bound().buffer);
    const bytes = () => new Uint8Array(this.#bound().buffer);
    const zeroCounts = (count: number, size: number) => {
      view().setUint32(count, 0, true);
      view().setUint32(size, 0, true);
      return Errno.Success;
    };
    const notcapable = () => Errno.Notcapable;
    const badf = () => Errno.Badf;
    const success = () => Errno.Success;

    this.imports = {
      fd_write: (fd: number, iovs: number, count: number, written: number) => {
        const parts: Uint8Array[] = [];
        for (let i = 0; i < count; i++) {
          const ptr = view().getUint32(iovs + i * 8, true);
          const len = view().getUint32(iovs + i * 8 + 4, true);
          parts.push(bytes().slice(ptr, ptr + len));
        }
        const chunk = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
        let at = 0;
        for (const part of parts) {
          chunk.set(part, at);
          at += part.length;
        }
        if (fd === 1 || fd === 2) {
          const text = this.#decoder.decode(chunk);
          if (text.length > 0) onOutput(fd === 1 ? "stdout" : "stderr", text);
        } else if (fd !== 0) {
          return Errno.Badf;
        }
        view().setUint32(written, chunk.length, true);
        return Errno.Success;
      },
      fd_read: (_fd: number, _iovs: number, _count: number, read: number) => {
        view().setUint32(read, 0, true);
        return Errno.Success;
      },
      fd_fdstat_get: (_fd: number, stat: number) => {
        // A character device with no rights is enough for libc's stdio setup.
        const v = view();
        v.setUint8(stat, 2);
        v.setUint16(stat + 2, 0, true);
        v.setBigUint64(stat + 8, 0n, true);
        v.setBigUint64(stat + 16, 0n, true);
        return Errno.Success;
      },
      fd_fdstat_set_flags: success,
      fd_advise: success,
      environ_sizes_get: zeroCounts,
      environ_get: success,
      args_sizes_get: zeroCounts,
      args_get: success,
      clock_time_get: (id: number, _precision: bigint, out: number) => {
        // The realtime clock is pinned so unchanged input typesets to the same bytes.
        const nanos = id === ClockId.Monotonic ? BigInt(Math.round(performance.now() * 1e6)) : 0n;
        view().setBigUint64(out, nanos, true);
        return Errno.Success;
      },
      clock_res_get: (_id: number, out: number) => {
        view().setBigUint64(out, 1_000_000n, true);
        return Errno.Success;
      },
      random_get: (ptr: number, len: number) => {
        // getRandomValues takes at most 65536 bytes per call.
        const target = bytes().subarray(ptr, ptr + len);
        for (let i = 0; i < target.length; i += 65536) {
          crypto.getRandomValues(target.subarray(i, i + 65536));
        }
        return Errno.Success;
      },
      proc_exit: (code: number) => {
        throw new WasiExit(code);
      },
      sched_yield: success,
      poll_oneoff: () => Errno.Nosys,
      fd_close: badf,
      fd_seek: badf,
      fd_tell: badf,
      fd_sync: badf,
      fd_datasync: badf,
      fd_readdir: badf,
      fd_pread: badf,
      fd_pwrite: badf,
      fd_allocate: badf,
      fd_renumber: badf,
      fd_filestat_get: badf,
      fd_filestat_set_size: badf,
      fd_filestat_set_times: badf,
      fd_prestat_get: badf,
      fd_prestat_dir_name: badf,
      sock_accept: badf,
      sock_recv: badf,
      sock_send: badf,
      sock_shutdown: badf,
      path_open: notcapable,
      path_filestat_get: notcapable,
      path_filestat_set_times: notcapable,
      path_create_directory: notcapable,
      path_remove_directory: notcapable,
      path_unlink_file: notcapable,
      path_rename: notcapable,
      path_link: notcapable,
      path_readlink: notcapable,
      path_symlink: notcapable,
    };
  }

  /** Bind to the instantiated module's memory. */
  bind(memory: WebAssembly.Memory): void {
    this.#memory = memory;
  }

  #bound(): WebAssembly.Memory {
    if (!this.#memory) throw new Error("the WASI shim was used before bind()");
    return this.#memory;
  }
}

/** Thrown when the module calls `proc_exit`, which leaves it unusable. */
export class WasiExit extends Error {
  constructor(code: number) {
    super(`the engine exited with status ${code}`);
    this.name = "WasiExit";
  }
}
