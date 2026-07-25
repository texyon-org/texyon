// End-to-end check that the wasm module typesets a document.
//
// Node stands in for the browser here: the host imports are the same two the
// Worker provides, but backed by synchronous filesystem reads instead of
// synchronous XHR. If this passes, everything on the Rust side of the boundary
// works and anything still broken is in the page.
//
// Usage: node tools/wasm/smoke-test.mjs [support-dir]

import { readFileSync, existsSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..");
const support = process.argv[2] ?? join(repo, "demo", "public", "tectonic");
const modulePath = join(support, "tectonic_wasm.wasm");

for (const path of [modulePath, join(support, "manifest.json")]) {
  if (!existsSync(path)) {
    console.error(`missing ${path}`);
    console.error("run tools/wasm/build.sh and tools/wasm/build-assets.sh first");
    process.exit(1);
  }
}

const manifest = JSON.parse(readFileSync(join(support, "manifest.json"), "utf8"));
const encoder = new TextEncoder();
const decoder = new TextDecoder();

let engine = null;
const reads = [];

/** Serve a support file from the pack. */
function readFile(name) {
  if (!(name in manifest.files)) return null;
  try {
    return readFileSync(join(support, "files", name));
  } catch {
    return null;
  }
}

function intoModule(bytes) {
  const ptr = engine.tectonic_wasm_alloc(bytes.length);
  new Uint8Array(engine.memory.buffer, ptr, bytes.length).set(bytes);
  return { ptr, len: bytes.length };
}

/** The WASI surface the module touches. See js/src/wasi.ts for the real one. */
function wasiImports() {
  const view = () => new DataView(engine.memory.buffer);
  const ok = () => 0;
  const badf = () => 8;
  const notcapable = () => 76;

  return {
    fd_write(fd, iovs, count, written) {
      const v = view();
      const bytes = new Uint8Array(engine.memory.buffer);
      let total = 0;
      for (let i = 0; i < count; i++) {
        const ptr = v.getUint32(iovs + i * 8, true);
        const len = v.getUint32(iovs + i * 8 + 4, true);
        if (fd === 1 || fd === 2) {
          process.stderr.write(decoder.decode(bytes.subarray(ptr, ptr + len)));
        }
        total += len;
      }
      view().setUint32(written, total, true);
      return 0;
    },
    fd_read: (_fd, _iovs, _count, read) => (view().setUint32(read, 0, true), 0),
    fd_close: badf,
    fd_seek: badf,
    fd_fdstat_get(_fd, stat) {
      const v = view();
      v.setUint8(stat, 2);
      v.setUint16(stat + 2, 0, true);
      v.setBigUint64(stat + 8, 0n, true);
      v.setBigUint64(stat + 16, 0n, true);
      return 0;
    },
    fd_fdstat_set_flags: ok,
    fd_filestat_get: badf,
    fd_prestat_get: badf,
    fd_prestat_dir_name: badf,
    path_open: notcapable,
    path_filestat_get: notcapable,
    path_remove_directory: notcapable,
    path_unlink_file: notcapable,
    environ_sizes_get(count, size) {
      const v = view();
      v.setUint32(count, 0, true);
      v.setUint32(size, 0, true);
      return 0;
    },
    environ_get: ok,
    // Realtime stays at the epoch so runs are reproducible; the monotonic
    // clock has to be real or the engine's own timings read as zero.
    clock_time_get: (id, _p, out) => {
      const ns = id === 1 ? BigInt(Math.round(performance.now() * 1e6)) : 0n;
      view().setBigUint64(out, ns, true);
      return 0;
    },
    random_get(ptr, len) {
      const target = new Uint8Array(engine.memory.buffer, ptr, len);
      for (let i = 0; i < len; i += 65536) {
        crypto.getRandomValues(target.subarray(i, Math.min(i + 65536, len)));
      }
      return 0;
    },
    proc_exit(code) {
      throw new Error(`the engine called proc_exit(${code})`);
    },
    sched_yield: ok,
    poll_oneoff: () => 52,
  };
}

const imports = {
  wasi_snapshot_preview1: wasiImports(),
  env: {
    tectonic_host_read_file(namePtr, nameLen, outPtr, outLen) {
      const name = decoder.decode(new Uint8Array(engine.memory.buffer, namePtr, nameLen));
      const bytes = readFile(name);
      reads.push({ name, found: Boolean(bytes) });
      if (!bytes) return 1;
      const buffer = intoModule(bytes);
      const v = new DataView(engine.memory.buffer);
      v.setUint32(outPtr, buffer.ptr, true);
      v.setUint32(outLen, buffer.len, true);
      return 0;
    },
    tectonic_host_log(kind, msgPtr, msgLen) {
      const message = decoder.decode(new Uint8Array(engine.memory.buffer, msgPtr, msgLen));
      const label = kind === 2 ? "error" : kind === 1 ? "warning" : "note";
      console.log(`  [${label}] ${message.trim()}`);
    },
  },
};

function call(request) {
  const input = intoModule(encoder.encode(JSON.stringify(request)));
  const out = engine.tectonic_wasm_alloc(8);
  try {
    const rc = engine.tectonic_request(input.ptr, input.len, out, out + 4);
    if (rc !== 0) throw new Error("the engine could not produce a response");
    const v = new DataView(engine.memory.buffer);
    const ptr = v.getUint32(out, true);
    const len = v.getUint32(out + 4, true);
    const text = decoder.decode(new Uint8Array(engine.memory.buffer, ptr, len).slice());
    engine.tectonic_wasm_free(ptr, len);
    return JSON.parse(text);
  } finally {
    engine.tectonic_wasm_free(out, 8);
    engine.tectonic_wasm_free(input.ptr, input.len);
  }
}

// A two-file project, because a single buffer would not exercise the source
// authority the engine now runs on.
const MAIN = String.raw`\documentclass{article}
\usepackage{amsmath}
\begin{document}
\section{Smoke}
\input{chapters/one}
\begin{equation}
  \int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}
\end{equation}
\end{document}
`;

const CHILD = (extra) => `Hello from WebAssembly.${extra}\n`;

const PROJECT = (extra) => ({
  "main.tex": MAIN,
  "chapters/one.tex": CHILD(extra),
});

const { instance } = await WebAssembly.instantiate(readFileSync(modulePath), imports);
engine = instance.exports;
engine._initialize?.();
console.log("instantiated");

let reply = call({ op: "open", format: manifest.format, primary: "main.tex" });
if (reply.status === "error") throw new Error(reply.message);
console.log("session opened");

// First compile: cold, reads the format dump and the whole class hierarchy.
let started = performance.now();
reply = call({ op: "compile", files: PROJECT("") });
const cold = performance.now() - started;
if (reply.status !== "compiled") throw new Error(JSON.stringify(reply).slice(0, 400));

if (reply.pageCount === 0) {
  throw new Error(`no pages were produced; outcome=${reply.outcome}\n${reply.log}`);
}
if (reply.pages.length !== reply.changed.length) {
  throw new Error("the renderer returned a different page set than it reported changed");
}
for (const page of reply.pages) {
  if (!page.svg.includes("<svg")) throw new Error(`page ${page.index} is not SVG`);
}

console.log(
  `cold: ${cold.toFixed(0)} ms total ` +
    `(tex ${reply.texMs.toFixed(0)}, analyze ${reply.analyzeMs.toFixed(0)}, ` +
    `render ${reply.renderMs.toFixed(0)}), ` +
    `${reply.pageCount} page(s), ${reply.pages.length} drawn, outcome=${reply.outcome}`,
);

writeFileSync(join(repo, "target", "smoke-cold.svg"), reply.pages[0].svg);

// Second compile: one word changed near the end, which is what the incremental
// path exists for.
started = performance.now();
reply = call({ op: "compile", files: PROJECT(" Edited.") });
const warm = performance.now() - started;
if (reply.status !== "compiled") throw new Error(JSON.stringify(reply).slice(0, 400));

console.log(
  `warm: ${warm.toFixed(0)} ms total ` +
    `(tex ${reply.texMs.toFixed(0)}, analyze ${reply.analyzeMs.toFixed(0)}, ` +
    `render ${reply.renderMs.toFixed(0)}), ` +
    `rung ${reply.replayRung}, ${reply.replayCommands} commands, ` +
    `${reply.changed.length}/${reply.pageCount} pages redrawn, ` +
    `${reply.pagesRendered}/${reply.pagesFingerprinted} drawn of fingerprinted`,
);

if (reply.pages.length > 0) {
  writeFileSync(join(repo, "target", "smoke-warm.svg"), reply.pages[0].svg);
}

// Third compile: identical text. Nothing about the document changed, so the
// renderer should draw nothing at all.
reply = call({ op: "compile", files: PROJECT(" Edited.") });
if (reply.status !== "compiled") throw new Error(JSON.stringify(reply).slice(0, 400));
console.log(
  `no-op edit: ${reply.changed.length} pages redrawn ` +
    `(expected 0 — the fingerprints did not move)`,
);

// Export: the same assembled XDV the viewer is showing, run through xdvipdfmx.
// A download therefore cannot disagree with what was on screen.
started = performance.now();
reply = call({ op: "export" });
const exportMs = performance.now() - started;
if (reply.status !== "exported") throw new Error(JSON.stringify(reply).slice(0, 400));

// The bytes are in linear memory, not in the JSON. Copy them out before the
// next allocation can move the heap, then hand them back.
const pdf = new Uint8Array(engine.memory.buffer, reply.ptr, reply.len).slice();
engine.tectonic_wasm_free(reply.ptr, reply.len);

if (decoder.decode(pdf.subarray(0, 5)) !== "%PDF-") {
  throw new Error("the export does not start with a PDF header");
}
if (!decoder.decode(pdf.subarray(-32)).includes("%%EOF")) {
  throw new Error("the exported PDF has no trailer, so it was truncated");
}

writeFileSync(join(repo, "target", "smoke.pdf"), pdf);
console.log(
  `export: ${exportMs.toFixed(0)} ms, ${(pdf.length / 1024).toFixed(0)} KB PDF`,
);

// Editing after an export is the real test of the loan xdvipdfmx takes on
// pdf_io's open-file registry: TeX owns that registry for the life of a
// resident session, and a run that failed to hand it back would abort here.
reply = call({ op: "compile", files: PROJECT(" Edited again.") });
if (reply.status !== "compiled") throw new Error(JSON.stringify(reply).slice(0, 400));
console.log(
  `post-export edit: ${reply.changed.length}/${reply.pageCount} pages redrawn, ` +
    `outcome=${reply.outcome}`,
);

// SyncTeX, both ways. The child file is the interesting one: a forward search
// that only knew about the primary would find nothing here.
const forward = call({ op: "forwardSearch", path: "chapters/one.tex", line: 1 });
if (forward.status !== "located") throw new Error(JSON.stringify(forward).slice(0, 200));
if (!forward.rect) {
  throw new Error("forward search found nothing for the child file's first line");
}
console.log(
  `forward: chapters/one.tex:1 -> page ${forward.rect.pageIndex} ` +
    `at (${forward.rect.hBp.toFixed(0)}, ${forward.rect.vBp.toFixed(0)}) bp`,
);

const backward = call({
  op: "backwardSearch",
  pageIndex: forward.rect.pageIndex,
  hBp: forward.rect.hBp,
  vBp: forward.rect.vBp,
});
if (backward.status !== "located") throw new Error(JSON.stringify(backward).slice(0, 200));
if (!backward.source) throw new Error("backward search found nothing where forward search pointed");
console.log(`backward: that point -> ${backward.source.path}:${backward.source.line}`);

// Round-tripping has to land back in the file we started from, or the index is
// keyed by something other than what the caller asked about.
// And it comes back spelled the way it went in, not anchored under the
// engine's virtual root.
if (backward.source.path !== "chapters/one.tex") {
  throw new Error(`backward search did not round-trip the path: ${backward.source.path}`);
}

const misses = reads.filter((r) => !r.found).length;
console.log(
  `support: ${reply.supportFiles} files resident ` +
    `(${(reply.supportBytes / 1e6).toFixed(1)} MB), ` +
    `${reads.length} host reads, ${misses} misses`,
);

call({ op: "shutdown" });
console.log(`\nspeedup: ${(cold / warm).toFixed(1)}x`);
console.log("SVG written to target/smoke-cold.svg and target/smoke-warm.svg");
