#!/usr/bin/env node
import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import childProcess from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import readline from "node:readline";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";
import { isDeepStrictEqual } from "node:util";

const MAX_SOURCE_FILE_BYTES = 16 * 1024 * 1024;
const MAX_SNAPSHOT_BYTES = 64 * 1024 * 1024;
const MAX_RENDERED_MUTATION_ATTEMPTS = 8;
const EDITOR_EXTENSIONS = new Set([".tex", ".ltx", ".sty", ".cls"]);

export class UnsupportedCorpusError extends Error {
  constructor(reasonCode, message) {
    super(message);
    this.name = "UnsupportedCorpusError";
    this.reasonCode = reasonCode;
  }
}

function usage() {
  return `Usage: node tools/live-preview/corpus.mjs --bin PATH --manifest PATH [options]

Manifest lines are tab-separated, with comments beginning with #:
  /absolute/or/manifest-relative/main.tex
  id<TAB>/path/to/main.tex
  id<TAB>/document/root<TAB>root-relative/main.tex

Options:
  --bin PATH                 preview-enabled tectonic executable
  --manifest PATH            corpus document manifest
  --format NAME              cached format name (default: latex)
  --output DIR               artifacts/report directory (default: /tmp/...)
  --max-documents N          stop after N manifest entries (default: all)
  --edits-per-document N     used source files to mutate (default: 3)
  --variants-per-file N      sequential marker variants per file (default: 2)
  --seed N                   deterministic candidate shuffle seed (default: 1)
  --timeout-ms N             per-event timeout (default: 60000)
  --tail-idle-ms N           server tail delay (default: 0)
  --ladder-mib N             checkpoint soft budget (default: 64)
  --verify-rewinds           enable the engine rewind oracle
  --help                     show this message
`;
}

function integer(value, name, minimum = 0) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < minimum) {
    throw new Error(`${name} must be a safe integer >= ${minimum}`);
  }
  return parsed;
}

function parseArgs(argv) {
  const options = {
    bin: process.env.TEXYON_PREVIEW_BIN || null,
    manifest: null,
    format: "latex",
    output: path.join("/tmp", `tectonic-live-preview-corpus-${Date.now()}`),
    maxDocuments: Number.POSITIVE_INFINITY,
    editsPerDocument: 3,
    variantsPerFile: 2,
    seed: 1,
    timeoutMs: 60_000,
    tailIdleMs: 0,
    ladderMiB: 64,
    verifyRewinds: false,
    help: false
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const take = () => {
      index += 1;
      if (index >= argv.length) throw new Error(`${argument} requires a value`);
      return argv[index];
    };
    if (argument === "--bin") options.bin = take();
    else if (argument === "--manifest") options.manifest = take();
    else if (argument === "--format") options.format = take();
    else if (argument === "--output") options.output = take();
    else if (argument === "--max-documents") {
      options.maxDocuments = integer(take(), argument, 1);
    } else if (argument === "--edits-per-document") {
      options.editsPerDocument = integer(take(), argument, 1);
    } else if (argument === "--variants-per-file") {
      options.variantsPerFile = integer(take(), argument, 1);
    } else if (argument === "--seed") options.seed = integer(take(), argument);
    else if (argument === "--timeout-ms") options.timeoutMs = integer(take(), argument, 1);
    else if (argument === "--tail-idle-ms") options.tailIdleMs = integer(take(), argument);
    else if (argument === "--ladder-mib") options.ladderMiB = integer(take(), argument, 1);
    else if (argument === "--verify-rewinds") options.verifyRewinds = true;
    else if (argument === "--help") options.help = true;
    else throw new Error(`unknown option: ${argument}`);
  }
  if (!options.help && !options.bin) throw new Error("--bin is required");
  if (!options.help && !options.manifest) throw new Error("--manifest is required");
  options.bin = options.bin && path.resolve(options.bin);
  options.manifest = options.manifest && path.resolve(options.manifest);
  options.output = path.resolve(options.output);
  return options;
}

function slash(value) {
  return value.split(path.sep).join("/");
}

export function parseManifest(text, manifestPath) {
  const base = path.dirname(path.resolve(manifestPath));
  const documents = [];
  const ids = new Set();
  for (const [lineIndex, raw] of text.split(/\r?\n/u).entries()) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    const fields = raw.split("\t").map((field) => field.trim());
    let id;
    let root;
    let primary;
    if (fields.length === 1) {
      const absolute = path.resolve(base, fields[0]);
      id = path.basename(absolute, path.extname(absolute));
      root = path.dirname(absolute);
      primary = path.basename(absolute);
    } else if (fields.length === 2) {
      id = fields[0];
      const absolute = path.resolve(base, fields[1]);
      root = path.dirname(absolute);
      primary = path.basename(absolute);
    } else if (fields.length === 3) {
      [id] = fields;
      root = path.resolve(base, fields[1]);
      const absolute = path.isAbsolute(fields[2])
        ? path.resolve(fields[2])
        : path.resolve(root, fields[2]);
      primary = slash(path.relative(root, absolute));
    } else {
      throw new Error(`manifest line ${lineIndex + 1} must have 1, 2, or 3 fields`);
    }
    if (!id) throw new Error(`manifest line ${lineIndex + 1} has an empty id`);
    if (ids.has(id)) throw new Error(`duplicate manifest id ${JSON.stringify(id)}`);
    const relative = path.relative(root, path.resolve(root, primary));
    if (relative.startsWith("..") || path.isAbsolute(relative)) {
      throw new Error(`manifest line ${lineIndex + 1} primary escapes its root`);
    }
    ids.add(id);
    documents.push({ id, root, primary: slash(relative) });
  }
  return documents;
}

function decodeUtf8(buffer, file) {
  if (buffer.length > MAX_SOURCE_FILE_BYTES) {
    throw new UnsupportedCorpusError(
      "source-file-size-limit",
      `source has ${buffer.length} bytes, exceeding ${MAX_SOURCE_FILE_BYTES}: ${file}`
    );
  }
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(buffer);
  } catch {
    throw new UnsupportedCorpusError("non-utf8-source", `source is not UTF-8: ${file}`);
  }
}

function walkSources(root, directory, files) {
  const entries = fs
    .readdirSync(directory, { withFileTypes: true })
    .sort((left, right) => left.name.localeCompare(right.name));
  for (const entry of entries) {
    if (entry.name === ".git" || entry.name === "node_modules" || entry.name === "target") {
      continue;
    }
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      walkSources(root, absolute, files);
    } else if (entry.isFile() && EDITOR_EXTENSIONS.has(path.extname(entry.name).toLowerCase())) {
      const text = decodeUtf8(fs.readFileSync(absolute), absolute);
      files.set(slash(path.relative(root, absolute)), text);
    }
  }
}

export function loadDocument(document) {
  const declaredRoot = fs.realpathSync(document.root);
  const primaryCandidate = path.resolve(declaredRoot, document.primary);
  const declaredRelative = path.relative(declaredRoot, primaryCandidate);
  if (declaredRelative.startsWith("..") || path.isAbsolute(declaredRelative)) {
    throw new Error(`${document.id}: primary escapes its root`);
  }
  const primaryAbsolute = fs.realpathSync(primaryCandidate);
  const resolvedRelative = path.relative(declaredRoot, primaryAbsolute);
  if (resolvedRelative.startsWith("..") || path.isAbsolute(resolvedRelative)) {
    throw new Error(`${document.id}: resolved primary escapes its root`);
  }
  // The JSONL server derives its authority root from the primary's parent.
  // Scan and key children from that exact root even when the manifest uses a
  // broader project root to locate a nested primary.
  const root = fs.realpathSync(path.dirname(primaryAbsolute));
  const primary = slash(path.relative(root, primaryAbsolute));
  const files = new Map();
  walkSources(root, root, files);
  if (!files.has(primary)) {
    files.set(primary, decodeUtf8(fs.readFileSync(primaryAbsolute), primaryAbsolute));
  }
  const totalBytes = [...files.values()].reduce(
    (total, text) => total + Buffer.byteLength(text),
    0
  );
  if (totalBytes > MAX_SNAPSHOT_BYTES) {
    throw new UnsupportedCorpusError(
      "snapshot-size-limit",
      `${document.id}: ${totalBytes} UTF-8 bytes exceed the ${MAX_SNAPSHOT_BYTES}-byte protocol limit`
    );
  }
  return { ...document, root, primary, files, totalBytes };
}

function inputNames(text) {
  const names = [];
  const pattern = /\\(?:input|include)\s*(?:\{([^}\r\n]+)\}|([^\s%{}]+))/gu;
  const uncommented = text
    .split(/\r?\n/u)
    .map((line) => {
      for (let index = 0; index < line.length; index += 1) {
        if (line[index] !== "%") continue;
        let escapes = 0;
        for (let cursor = index - 1; cursor >= 0 && line[cursor] === "\\"; cursor -= 1) {
          escapes += 1;
        }
        if (escapes % 2 === 0) return line.slice(0, index);
      }
      return line;
    })
    .join("\n");
  for (const match of uncommented.matchAll(pattern)) names.push(match[1] || match[2]);
  return names;
}

function resolveInput(files, source, requested) {
  let name = requested.replaceAll("\\", "/");
  if (!path.posix.extname(name)) name += ".tex";
  const candidates = [
    path.posix.normalize(name),
    path.posix.normalize(path.posix.join(path.posix.dirname(source), name))
  ];
  return candidates.find(
    (candidate) => !candidate.startsWith("../") && files.has(candidate)
  );
}

export function discoverInputs(primary, files) {
  return discoverInputGraph(primary, files).map(({ source }) => source);
}

export function discoverInputGraph(primary, files) {
  const seen = new Map();
  const queue = [{ source: primary, depth: 0 }];
  while (queue.length > 0) {
    const { source, depth } = queue.shift();
    if (seen.has(source)) continue;
    seen.set(source, depth);
    const text = files.get(source);
    if (text === undefined) continue;
    for (const requested of inputNames(text)) {
      const resolved = resolveInput(files, source, requested);
      if (resolved && !seen.has(resolved)) {
        queue.push({ source: resolved, depth: depth + 1 });
      }
    }
  }
  return [...seen].map(([source, depth]) => ({ source, depth }));
}

function hashText(value) {
  let hash = 2166136261;
  for (const byte of Buffer.from(value)) {
    hash ^= byte;
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function shuffle(values, seed) {
  let state = seed >>> 0 || 1;
  const next = () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return state >>> 0;
  };
  const result = [...values];
  for (let index = result.length - 1; index > 0; index -= 1) {
    const other = next() % (index + 1);
    [result[index], result[other]] = [result[other], result[index]];
  }
  return result;
}

export function instrumentSource(text, marker) {
  assert.match(marker, /^[A-Z0-9-]+$/u);
  const bom = text.startsWith("\uFEFF") ? "\uFEFF" : "";
  const body = bom ? text.slice(1) : text;
  const firstBreak = body.indexOf("\n");
  if (body.startsWith("%&") && firstBreak !== -1) {
    return `${bom}${body.slice(0, firstBreak + 1)}\\message{${marker}}\n${body.slice(firstBreak + 1)}`;
  }
  return `${bom}\\message{${marker}}\n${body}`;
}

function renderedMutationPlans(text) {
  const plans = [];
  const lines = text.matchAll(/[^\r\n]*(?:\r\n|\r|\n|$)/gu);
  for (const line of lines) {
    const value = line[0].replace(/[\r\n]+$/u, "");
    if (value.trim().length < 12 || /[\\%{}$&#^_~]/u.test(value)) continue;
    const words = [...value.matchAll(/[A-Za-z]{4,}/gu)];
    if (words.length === 0) continue;
    const word = words[Math.floor(words.length / 2)];
    const start = line.index + word.index;
    plans.push({ kind: "rendered-text", start, end: start + word[0].length });
  }
  const desired = Math.floor(text.length * 0.75);
  plans.sort(
    (left, right) =>
      Math.abs(left.start - desired) - Math.abs(right.start - desired) ||
      left.start - right.start
  );
  return plans;
}

function applyMutationPlan(text, marker, plan) {
  assert.match(marker, /^[A-Z0-9-]+$/u);
  if (plan.kind === "rendered-text") {
    return {
      text: `${text.slice(0, plan.start)}${marker}${text.slice(plan.end)}`,
      kind: plan.kind,
      offset: plan.start,
      plan
    };
  }
  const edited = instrumentSource(text, marker);
  return {
    text: edited,
    kind: "instrumentation",
    offset: edited.indexOf(marker),
    plan
  };
}

export function mutationCandidates(text, marker) {
  const plans = renderedMutationPlans(text).slice(0, MAX_RENDERED_MUTATION_ATTEMPTS);
  plans.push({ kind: "instrumentation" });
  return plans.map((plan) => applyMutationPlan(text, marker, plan));
}

export function mutateSource(text, marker) {
  return mutationCandidates(text, marker)[0];
}

export function fatalErrorForReason(reason) {
  if (/encoded frame has \d+ bytes, exceeding \d+/u.test(reason)) {
    return new UnsupportedCorpusError("frame-size-limit", `fatal: ${reason}`);
  }
  return new Error(`fatal: ${reason}`);
}

class PreviewSession {
  constructor(options, cwd) {
    const args = [
      "-X", "preview",
      "--format", options.format,
      "--tail-idle-ms", String(options.tailIdleMs),
      "--ladder-mib", String(options.ladderMiB)
    ];
    if (options.verifyRewinds) args.push("--verify-rewinds");
    this.child = childProcess.spawn(options.bin, args, {
      cwd,
      stdio: ["pipe", "pipe", "pipe"]
    });
    this.timeoutMs = options.timeoutMs;
    this.events = [];
    this.rawLines = [];
    this.stderr = "";
    this.failure = null;
    this.shuttingDown = false;
    this.emitter = new EventEmitter();
    this.exited = new Promise((resolve) => {
      this.child.once("error", (error) => {
        this.fail(error);
        resolve({ code: null, signal: null, error });
      });
      this.child.once("exit", (code, signal) => {
        if (!this.shuttingDown) {
          this.fail(new Error(`preview exited unexpectedly: ${code ?? signal}`));
        }
        resolve({ code, signal });
      });
    });
    this.child.stderr.on("data", (data) => {
      this.stderr += data.toString();
    });
    this.lines = readline.createInterface({ input: this.child.stdout });
    this.lines.on("line", (line) => {
      const receivedAtMs = performance.now();
      this.rawLines.push(line);
      let event;
      try {
        event = JSON.parse(line);
      } catch (error) {
        this.fail(new Error(`invalid server JSON: ${error.message}`));
        return;
      }
      const observed = { event, receivedAtMs };
      this.events.push(observed);
      if (event.type === "fatal") this.fail(fatalErrorForReason(event.reason));
      if (event.type === "aborted") {
        this.fail(new Error(`compile aborted: ${event.log_tail}`));
      }
      if (event.type === "rejected") {
        this.fail(new Error(`request rejected: ${event.reason}`));
      }
      this.emitter.emit("event", observed);
    });
  }

  fail(error) {
    this.failure ??= error;
    this.emitter.emit("serverError", this.failure);
  }

  send(request) {
    if (this.failure) throw this.failure;
    const sentAtMs = performance.now();
    this.child.stdin.write(`${JSON.stringify(request)}\n`);
    return sentAtMs;
  }

  async waitFor(predicate, description, afterMs = -Infinity) {
    if (this.failure) throw this.failure;
    const existing = this.events.find(
      (observed) => observed.receivedAtMs >= afterMs && predicate(observed.event)
    );
    if (existing) return existing;
    return new Promise((resolve, reject) => {
      const onEvent = (observed) => {
        if (observed.receivedAtMs >= afterMs && predicate(observed.event)) {
          cleanup();
          resolve(observed);
        }
      };
      const onError = (error) => {
        cleanup();
        reject(error);
      };
      const timer = setTimeout(() => {
        cleanup();
        reject(new Error(`timed out waiting for ${description}`));
      }, this.timeoutMs);
      const cleanup = () => {
        clearTimeout(timer);
        this.emitter.off("event", onEvent);
        this.emitter.off("serverError", onError);
      };
      this.emitter.on("event", onEvent);
      this.emitter.on("serverError", onError);
    });
  }

  async ready() {
    return this.waitFor((event) => event.type === "ready", "ready");
  }

  async update(revision, phase, afterMs) {
    return this.waitFor(
      (event) =>
        event.type === "update" &&
        event.source_revision === String(revision) &&
        event.phase === phase,
      `${phase} revision ${revision}`,
      afterMs
    );
  }

  async settle(revision, afterMs) {
    const foreground = await this.update(revision, "foreground", afterMs);
    if (foreground.event.outcome !== "complete" || !foreground.event.tail_pending) {
      return foreground;
    }
    return this.update(revision, "tail_settle", foreground.receivedAtMs);
  }

  async belief(revision) {
    const sentAtMs = this.send({ type: "resync" });
    const observed = await this.update(revision, "resync", sentAtMs);
    assert.equal(observed.event.outcome, "complete");
    assert.equal(observed.event.changed.length, observed.event.page_count);
    assert.equal(observed.event.pages_svg.length, observed.event.page_count);
    assert.deepEqual(observed.event.removed, []);
    return observed.event;
  }

  async shutdown() {
    this.shuttingDown = true;
    if (this.child.exitCode === null && this.child.stdin.writable) {
      this.child.stdin.end(`${JSON.stringify({ type: "shutdown" })}\n`);
    }
    let timer;
    const timeout = new Promise((_, reject) => {
      timer = setTimeout(
        () => reject(new Error("timed out waiting for preview shutdown")),
        this.timeoutMs
      );
    });
    let result;
    try {
      result = await Promise.race([this.exited, timeout]);
    } finally {
      clearTimeout(timer);
    }
    this.lines.close();
    if (result.error) throw result.error;
    if (result.code !== 0) {
      throw new Error(`preview exited ${result.code ?? result.signal}: ${this.stderr}`);
    }
  }

  stop() {
    this.shuttingDown = true;
    if (this.child.exitCode === null) this.child.kill("SIGKILL");
    this.lines.close();
  }
}

function initialization(document, files) {
  return {
    type: "initialize",
    protocol_version: 1,
    primary_path: path.join(document.root, document.primary),
    primary_text: files.get(document.primary),
    children: [...files]
      .filter(([source]) => source !== document.primary)
      .map(([source, text]) => ({ path: source, text }))
  };
}

export function canonicalBelief(event) {
  return {
    page_count: event.page_count,
    changed: event.changed,
    removed: event.removed,
    pages_svg: event.pages_svg,
    page_sizes: event.page_sizes,
    unsupported_pages: event.unsupported_pages
  };
}

async function openAt(options, document, files) {
  const session = new PreviewSession(options, document.root);
  try {
    const ready = await session.ready();
    const request = initialization(document, files);
    request.protocol_version = ready.event.protocol_version;
    const sentAtMs = session.send(request);
    const settled = await session.settle(0, sentAtMs);
    assert.equal(settled.event.outcome, "complete", `${document.id}: initial compile failed`);
    return session;
  } catch (error) {
    session.stop();
    throw error;
  }
}

async function oracleResult(options, document, files) {
  const session = new PreviewSession(options, document.root);
  let completed = false;
  try {
    const ready = await session.ready();
    const request = initialization(document, files);
    request.protocol_version = ready.event.protocol_version;
    const sentAtMs = session.send(request);
    const settled = await session.settle(0, sentAtMs);
    const result = {
      outcome: settled.event.outcome,
      belief:
        settled.event.outcome === "complete"
          ? canonicalBelief(await session.belief(0))
          : null
    };
    completed = true;
    return result;
  } finally {
    if (completed) await session.shutdown();
    else session.stop();
  }
}

async function withFrameLimitOracle(options, document, files, operation, description) {
  try {
    return await operation();
  } catch (error) {
    if (
      !(error instanceof UnsupportedCorpusError) ||
      error.reasonCode !== "frame-size-limit"
    ) {
      throw error;
    }
    let fresh;
    try {
      fresh = await oracleResult(options, document, files);
    } catch (freshError) {
      if (
        freshError instanceof UnsupportedCorpusError &&
        freshError.reasonCode === "frame-size-limit"
      ) {
        throw error;
      }
      throw freshError;
    }
    throw new Error(
      `${document.id}: incremental ${description} exceeded the frame limit but a fresh session returned ${fresh.outcome}`
    );
  }
}

function assertBudget(event, options) {
  const hardBytes = options.ladderMiB * 1024 * 1024 * 1.5;
  assert.ok(
    event.metrics.checkpoint_bytes <= hardBytes,
    `checkpoint ladder used ${event.metrics.checkpoint_bytes} bytes, above ${hardBytes}`
  );
}

async function runDocument(options, document) {
  const loaded = loadDocument(document);
  const inputGraph = discoverInputGraph(loaded.primary, loaded.files);
  const discovered = inputGraph.map(({ source }) => source);
  const children = inputGraph.filter(({ source }) => source !== loaded.primary);
  const candidates = shuffle(
    children,
    (options.seed ^ hashText(loaded.id)) >>> 0
  ).sort((left, right) => right.depth - left.depth);
  const selected = candidates
    .slice(0, Math.max(0, options.editsPerDocument - 1))
    .map(({ source, depth }) => ({ source, depth }));
  selected.push({ source: loaded.primary, depth: 0 });
  const originalFiles = new Map(loaded.files);
  const initialStarted = performance.now();
  const incremental = await openAt(options, loaded, originalFiles);
  const trace = [];
  const rejectedMutations = [];
  try {
    const initialEvent = await incremental.belief(0);
    assertBudget(initialEvent, options);
    const initialBelief = canonicalBelief(initialEvent);
    let revision = 0;
    for (const [candidateIndex, { source, depth }] of selected.entries()) {
      const original = originalFiles.get(source);
      assert.notEqual(original, undefined);
      let acceptedPlan = null;
      for (let variant = 0; variant < options.variantsPerFile; variant += 1) {
        const marker = `LPSTRESS-${candidateIndex.toString(16).padStart(4, "0")}-${variant
          .toString(16)
          .padStart(4, "0")}`.toUpperCase();
        const mutations = acceptedPlan
          ? [applyMutationPlan(original, marker, acceptedPlan)]
          : mutationCandidates(original, marker);
        let accepted = null;
        for (const [attempt, mutation] of mutations.entries()) {
          const edited = mutation.text;
          loaded.files.set(source, edited);
          const request = {
            type: "replace",
            text: edited,
            ...(source === loaded.primary ? {} : { path: source })
          };
          const sentAtMs = incremental.send(request);
          revision += 1;
          const settled = await withFrameLimitOracle(
            options,
            loaded,
            loaded.files,
            () => incremental.settle(revision, sentAtMs),
            `compile after editing ${source}`
          );
          if (settled.event.outcome === "error") {
            const fresh = await oracleResult(options, loaded, loaded.files);
            assert.equal(
              fresh.outcome,
              "error",
              `${loaded.id}: incremental compile errored but a fresh compile completed after editing ${source}`
            );
            rejectedMutations.push({
              source,
              depth,
              variant,
              attempt,
              mutation_kind: mutation.kind,
              mutation_offset: mutation.offset
            });
            loaded.files.set(source, original);
            const restoreAt = incremental.send({
              type: "replace",
              text: original,
              ...(source === loaded.primary ? {} : { path: source })
            });
            const restoreSettled = await incremental.settle(++revision, restoreAt);
            assert.equal(
              restoreSettled.event.outcome,
              "complete",
              `${loaded.id}: failed to recover after a rejected mutation of ${source}`
            );
            const recovered = await incremental.belief(revision);
            assert.deepEqual(
              canonicalBelief(recovered),
              initialBelief,
              `${loaded.id}: rejected mutation recovery did not restore ${source}`
            );
            continue;
          }
          assert.equal(
            settled.event.outcome,
            "complete",
            `${loaded.id}: unexpected mutation outcome ${settled.event.outcome}`
          );
          const actualEvent = await withFrameLimitOracle(
            options,
            loaded,
            loaded.files,
            () => incremental.belief(revision),
            `resync after editing ${source}`
          );
          assertBudget(actualEvent, options);
          const expected = await oracleResult(options, loaded, loaded.files);
          assert.equal(
            expected.outcome,
            "complete",
            `${loaded.id}: incremental compile completed but a fresh compile errored after editing ${source}`
          );
          const actualBelief = canonicalBelief(actualEvent);
          assert.deepEqual(
            actualBelief,
            expected.belief,
            `${loaded.id}: incremental belief diverged after editing ${source} variant ${variant}`
          );
          accepted = { mutation, settled, actualBelief, sentAtMs };
          break;
        }
        if (!accepted) {
          throw new UnsupportedCorpusError(
            "no-valid-synthetic-mutation",
            `${loaded.id}: every safe mutation of ${source} produced the same TeX error in incremental and fresh sessions`
          );
        }
        acceptedPlan ??= accepted.mutation.plan;
        trace.push({
          source,
          depth,
          variant,
          mutation_kind: accepted.mutation.kind,
          mutation_offset: accepted.mutation.offset,
          rendered_output_changed: !isDeepStrictEqual(accepted.actualBelief, initialBelief),
          nonzero_replay: accepted.settled.event.metrics.replay_rung > 0,
          wall_ms: performance.now() - accepted.sentAtMs,
          metrics: accepted.settled.event.metrics
        });
      }

      loaded.files.set(source, original);
      const restoreAt = incremental.send({
        type: "replace",
        text: original,
        ...(source === loaded.primary ? {} : { path: source })
      });
      const restoreSettled = await incremental.settle(++revision, restoreAt);
      assert.equal(
        restoreSettled.event.outcome,
        "complete",
        `${loaded.id}: restoring ${source} failed`
      );
      const restored = await incremental.belief(revision);
      assert.deepEqual(
        canonicalBelief(restored),
        initialBelief,
        `${loaded.id}: restoring ${source} did not recover the initial belief`
      );
    }
    await incremental.shutdown();
    return {
      id: loaded.id,
      status: "passed",
      source_files: loaded.files.size,
      discovered_inputs: discovered.length,
      maximum_input_depth: Math.max(...inputGraph.map(({ depth }) => depth)),
      edited_sources: selected.map(({ source }) => source),
      rendered_output_changes: trace.filter(({ rendered_output_changed }) => rendered_output_changed)
        .length,
      nonzero_replays: trace.filter(({ nonzero_replay }) => nonzero_replay).length,
      rejected_mutation_attempts: rejectedMutations.length,
      operations: trace,
      elapsed_ms: performance.now() - initialStarted
    };
  } catch (error) {
    incremental.stop();
    throw Object.assign(error, {
      livePreviewTrace: {
        id: loaded.id,
        source_files: loaded.files.size,
        discovered_inputs: discovered,
        edited_sources: selected.map(({ source }) => source),
        rejected_mutations: rejectedMutations,
        operations: trace,
        events: incremental.rawLines,
        stderr: incremental.stderr
      }
    });
  }
}

async function main() {
  let options;
  try {
    options = parseArgs(process.argv.slice(2));
  } catch (error) {
    console.error(error.message);
    console.error(usage());
    process.exitCode = 2;
    return;
  }
  if (options.help) {
    process.stdout.write(usage());
    return;
  }
  if (!fs.existsSync(options.bin)) throw new Error(`binary does not exist: ${options.bin}`);
  const documents = parseManifest(
    fs.readFileSync(options.manifest, "utf8"),
    options.manifest
  ).slice(0, options.maxDocuments);
  if (documents.length === 0) throw new Error("manifest contains no documents");
  fs.mkdirSync(options.output, { recursive: true });

  const results = [];
  for (const [index, document] of documents.entries()) {
    process.stderr.write(`[${index + 1}/${documents.length}] ${document.id}\n`);
    try {
      results.push(await runDocument(options, document));
    } catch (error) {
      const result = {
        id: document.id,
        status: error instanceof UnsupportedCorpusError ? "unsupported" : "failed",
        reason_code: error.reasonCode || null,
        message: error.stack || error.message,
        trace: error.livePreviewTrace || null
      };
      results.push(result);
      fs.writeFileSync(
        path.join(
          options.output,
          `${document.id.replaceAll(/[^A-Za-z0-9_.-]/gu, "_")}.${result.status}.json`
        ),
        `${JSON.stringify(result, null, 2)}\n`
      );
    }
  }

  const passedResults = results.filter((result) => result.status === "passed");
  const report = {
    schema_version: 2,
    generated_at: new Date().toISOString(),
    options,
    passed: passedResults.length,
    failed: results.filter((result) => result.status === "failed").length,
    unsupported: results.filter((result) => result.status === "unsupported").length,
    coverage: {
      documents_with_nested_inputs: passedResults.filter(
        ({ maximum_input_depth }) => maximum_input_depth > 1
      ).length,
      maximum_input_depth: Math.max(
        0,
        ...passedResults.map(({ maximum_input_depth }) => maximum_input_depth)
      ),
      operations: passedResults.reduce(
        (total, { operations }) => total + operations.length,
        0
      ),
      rendered_output_changes: passedResults.reduce(
        (total, result) => total + result.rendered_output_changes,
        0
      ),
      nonzero_replays: passedResults.reduce(
        (total, result) => total + result.nonzero_replays,
        0
      ),
      rejected_mutation_attempts: passedResults.reduce(
        (total, result) => total + result.rejected_mutation_attempts,
        0
      )
    },
    results
  };
  fs.writeFileSync(
    path.join(options.output, "summary.json"),
    `${JSON.stringify(report, null, 2)}\n`
  );
  process.stdout.write(
    `${JSON.stringify({
      passed: report.passed,
      failed: report.failed,
      unsupported: report.unsupported
    })}\n`
  );
  process.stderr.write(`results: ${options.output}\n`);
  if (report.failed > 0) process.exitCode = 1;
  else if (report.unsupported > 0) process.exitCode = 2;
}

const invokedPath = process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href;
if (import.meta.url === invokedPath) await main();
