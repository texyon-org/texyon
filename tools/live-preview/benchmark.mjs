#!/usr/bin/env node
import assert from "node:assert/strict";
import childProcess from "node:child_process";
import { EventEmitter } from "node:events";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import readline from "node:readline";
import { fileURLToPath } from "node:url";
import { performance } from "node:perf_hooks";
import { setTimeout as delay } from "node:timers/promises";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const FIXTURE_DIR = path.join(SCRIPT_DIR, "fixtures", "report");
const PRIMARY_PATH = path.join(FIXTURE_DIR, "main.tex");
const CHILD_PATH = path.join(FIXTURE_DIR, "chapter.tex");
const EARLY_BASELINE = "Early baseline";
const LATE_BASELINE = "Late baseline";

function usage() {
  return `Usage: node tools/live-preview/benchmark.mjs --bin PATH [options]

Options:
  --bin PATH                 preview-enabled tectonic executable
  --format NAME              cached format name (default: latex)
  --output DIR               result directory (default: /tmp/...)
  --warmups N                discarded runs (default: 5)
  --samples N                retained runs (default: 30)
  --tail-idle-ms N           server tail delay (default: 200)
  --burst-spacing-ms N       delay between burst edits (default: 120)
  --timeout-ms N             per-event timeout (default: 30000)
  --verify-rewinds           enable the engine rewind oracle
  --help                     show this message
`;
}

function integer(value, name) {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 0) {
    throw new Error(`${name} must be a non-negative safe integer`);
  }
  return parsed;
}

function parseArgs(argv) {
  const options = {
    bin: process.env.TECTONIC_PREVIEW_BIN || null,
    format: "latex",
    output: path.join("/tmp", `tectonic-live-preview-benchmark-${Date.now()}`),
    warmups: 5,
    samples: 30,
    tailIdleMs: 200,
    burstSpacingMs: 120,
    timeoutMs: 30_000,
    verifyRewinds: false
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const take = () => {
      index += 1;
      if (index >= argv.length) throw new Error(`${argument} requires a value`);
      return argv[index];
    };
    if (argument === "--bin") options.bin = take();
    else if (argument === "--format") options.format = take();
    else if (argument === "--output") options.output = path.resolve(take());
    else if (argument === "--warmups") options.warmups = integer(take(), argument);
    else if (argument === "--samples") options.samples = integer(take(), argument);
    else if (argument === "--tail-idle-ms") options.tailIdleMs = integer(take(), argument);
    else if (argument === "--burst-spacing-ms") options.burstSpacingMs = integer(take(), argument);
    else if (argument === "--timeout-ms") options.timeoutMs = integer(take(), argument);
    else if (argument === "--verify-rewinds") options.verifyRewinds = true;
    else if (argument === "--help") options.help = true;
    else throw new Error(`unknown option: ${argument}`);
  }
  if (!options.help && !options.bin) throw new Error("--bin is required");
  if (options.samples === 0 && !options.help) throw new Error("--samples must be at least 1");
  return options;
}

function replaceOnce(text, before, after) {
  const first = text.indexOf(before);
  assert.notEqual(first, -1, `fixture marker ${JSON.stringify(before)} is absent`);
  assert.equal(text.indexOf(before, first + before.length), -1, `fixture marker ${JSON.stringify(before)} is not unique`);
  return `${text.slice(0, first)}${after}${text.slice(first + before.length)}`;
}

function nearestRank(values, percentile) {
  assert.ok(values.length > 0);
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.max(0, Math.ceil(percentile * sorted.length) - 1)];
}

function commandOutput(command, args) {
  const result = childProcess.spawnSync(command, args, { encoding: "utf8" });
  return result.status === 0 ? result.stdout.trim() : null;
}

class PreviewSession {
  constructor(options) {
    const args = [
      "-X", "preview",
      "--format", options.format,
      "--tail-idle-ms", String(options.tailIdleMs)
    ];
    if (options.verifyRewinds) args.push("--verify-rewinds");
    this.child = childProcess.spawn(options.bin, args, {
      cwd: FIXTURE_DIR,
      stdio: ["pipe", "pipe", "pipe"]
    });
    this.events = [];
    this.rawLines = [];
    this.requests = [];
    this.stderr = "";
    this.emitter = new EventEmitter();
    this.timeoutMs = options.timeoutMs;
    this.exited = new Promise((resolve, reject) => {
      this.child.once("error", reject);
      this.child.once("exit", (code, signal) => resolve({ code, signal }));
    });
    this.child.stderr.on("data", (data) => { this.stderr += data.toString(); });
    this.lines = readline.createInterface({ input: this.child.stdout });
    this.lines.on("line", (line) => {
      const receivedAtMs = performance.now();
      this.rawLines.push(line);
      let event;
      try {
        event = JSON.parse(line);
      } catch (error) {
        this.emitter.emit("error", new Error(`server emitted invalid JSON: ${error.message}`));
        return;
      }
      const observed = { event, receivedAtMs };
      this.events.push(observed);
      if (event.type === "fatal") this.emitter.emit("error", new Error(event.reason));
      if (event.type === "aborted") this.emitter.emit("error", new Error(`compile aborted: ${event.log_tail}`));
      if (event.type === "rejected") this.emitter.emit("error", new Error(`request rejected: ${event.reason}`));
      this.emitter.emit("event", observed);
    });
  }

  send(request) {
    const sentAtMs = performance.now();
    this.requests.push({ request, sentAtMs });
    this.child.stdin.write(`${JSON.stringify(request)}\n`);
    return sentAtMs;
  }

  async waitFor(predicate, description, afterMs = -Infinity) {
    const existing = this.events.find((observed) => observed.receivedAtMs >= afterMs && predicate(observed.event));
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
        this.emitter.off("error", onError);
      };
      this.emitter.on("event", onEvent);
      this.emitter.on("error", onError);
    });
  }

  async updateFor(revision, phase, sentAtMs) {
    const observed = await this.waitFor(
      (event) => event.type === "update" && event.source_revision === String(revision) && event.phase === phase,
      `${phase} revision ${revision}`,
      sentAtMs
    );
    assert.equal(observed.event.outcome, "complete", `${phase} revision ${revision} was not complete`);
    return observed;
  }

  async foreground(revision, sentAtMs) {
    const foreground = await this.updateFor(revision, "foreground", sentAtMs);
    let settle = null;
    if (foreground.event.tail_pending) settle = await this.updateFor(revision, "tail_settle", foreground.receivedAtMs);
    return { foreground, settle };
  }

  async shutdown() {
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
    if (result.code !== 0) {
      throw new Error(`preview exited ${result.code ?? result.signal}: ${this.stderr}`);
    }
  }
}

function observation(label, sentAtMs, observed) {
  if (!observed) return null;
  return {
    label,
    wall_ms: observed.receivedAtMs - sentAtMs,
    metrics: observed.event.metrics,
    page_count: observed.event.page_count,
    changed: observed.event.changed,
    tail_pending: observed.event.tail_pending
  };
}

function tailObservation(label, foreground, settle) {
  assert.ok(settle, `${label} was missing its tail-settle frame`);
  return {
    ...observation(label, foreground.receivedAtMs, settle),
    request_wall_ms: settle.receivedAtMs - foreground.sentAtMs
  };
}

async function runSample(options) {
  const primary = fs.readFileSync(PRIMARY_PATH, "utf8");
  const child = fs.readFileSync(CHILD_PATH, "utf8");
  const early = replaceOnce(primary, EARLY_BASELINE, "Early edited");
  const late = replaceOnce(child, LATE_BASELINE, "Late edited");
  const lateAgain = replaceOnce(child, LATE_BASELINE, "Late change");
  const session = new PreviewSession(options);
  let revision = 0;
  const observations = [];

  const ready = await session.waitFor((event) => event.type === "ready", "ready");
  const initializedAt = session.send({
    type: "initialize",
    protocol_version: ready.event.protocol_version,
    primary_path: PRIMARY_PATH,
    primary_text: primary,
    children: [{ path: "chapter.tex", text: child }]
  });
  const initial = await session.updateFor(0, "foreground", initializedAt);
  assert.ok(initial.event.page_count >= 6, "benchmark fixture did not produce enough pages");

  session.send({ type: "viewport", first: 0, last: 0 });
  const earlyAt = session.send({ type: "replace", text: early });
  const earlyResult = await session.foreground(++revision, earlyAt);
  earlyResult.foreground.sentAtMs = earlyAt;
  observations.push(observation("early_foreground", earlyAt, earlyResult.foreground));
  observations.push(tailObservation("early_tail", earlyResult.foreground, earlyResult.settle));

  const lastPage = earlyResult.settle?.event.page_count ?? earlyResult.foreground.event.page_count;
  session.send({ type: "viewport", first: lastPage - 1, last: lastPage - 1 });
  const lateAt = session.send({ type: "replace", path: "chapter.tex", text: late });
  const lateResult = await session.foreground(++revision, lateAt);
  lateResult.foreground.sentAtMs = lateAt;
  observations.push(observation("late_foreground", lateAt, lateResult.foreground));
  observations.push(tailObservation("late_tail", lateResult.foreground, lateResult.settle));

  const lateAgainAt = session.send({ type: "replace", path: "chapter.tex", text: lateAgain });
  const lateAgainResult = await session.foreground(++revision, lateAgainAt);
  lateAgainResult.foreground.sentAtMs = lateAgainAt;
  observations.push(observation("late_repeat_foreground", lateAgainAt, lateAgainResult.foreground));
  observations.push(tailObservation("late_repeat_tail", lateAgainResult.foreground, lateAgainResult.settle));
  assert.ok(
    BigInt(lateAgainResult.foreground.event.metrics.replay_commands) <
      BigInt(lateResult.foreground.event.metrics.replay_commands),
    "repeat late edit did not reduce replay work"
  );

  const revertPrimaryAt = session.send({ type: "replace", text: primary });
  await session.foreground(++revision, revertPrimaryAt);
  const revertChildAt = session.send({ type: "replace", path: "chapter.tex", text: child });
  await session.foreground(++revision, revertChildAt);

  session.send({ type: "viewport", first: 0, last: 0 });
  const burstStartMs = performance.now();
  let lastBurstAt = burstStartMs;
  const burstSends = [];
  for (let index = 0; index < 10; index += 1) {
    const burst = replaceOnce(primary, EARLY_BASELINE, `Early burst ${index}`);
    lastBurstAt = session.send({ type: "replace", text: burst });
    revision += 1;
    burstSends.push({ revision, sentAtMs: lastBurstAt });
    if (index !== 9) await delay(options.burstSpacingMs);
  }
  const burstForeground = await session.updateFor(revision, "foreground", lastBurstAt);
  observations.push(observation("burst_final_foreground", lastBurstAt, burstForeground));
  let burstSettle = null;
  if (burstForeground.event.tail_pending) {
    burstSettle = await session.updateFor(revision, "tail_settle", burstForeground.receivedAtMs);
    burstForeground.sentAtMs = lastBurstAt;
    observations.push(tailObservation("burst_final_tail", burstForeground, burstSettle));
  }
  const tailDuringBurst = session.events.filter(({ event, receivedAtMs }) =>
    event.type === "update" && event.phase === "tail_settle" &&
    receivedAtMs >= burstStartMs && receivedAtMs < lastBurstAt
  );
  assert.equal(tailDuringBurst.length, 0, "tail work started between burst replacements");
  const cancellationDelaysMs = session.events
    .filter(({ event }) => event.type === "cancelled")
    .map(({ event, receivedAtMs }) => {
      const sent = burstSends.find((candidate) => String(candidate.revision) === event.source_revision);
      return sent ? receivedAtMs - sent.sentAtMs : null;
    })
    .filter((value) => value !== null);

  await session.shutdown();
  return {
    observations: observations.filter(Boolean),
    rawLines: session.rawLines,
    requests: session.requests,
    stderr: session.stderr,
    bundleDigest: ready.event.bundle_digest,
    cancellationDelaysMs
  };
}

function summarize(samples) {
  const grouped = new Map();
  for (const sample of samples) {
    for (const observation of sample.observations) {
      if (!grouped.has(observation.label)) grouped.set(observation.label, []);
      grouped.get(observation.label).push(observation);
    }
  }
  const summary = {};
  for (const [label, observations] of grouped) {
    const metricNames = ["wall_ms", "engine_ms", "analysis_ms", "render_ms"];
    summary[label] = {};
    for (const name of metricNames) {
      const values = observations.map((item) => name === "wall_ms" ? item.wall_ms : item.metrics[name]);
      summary[label][name] = {
        median: nearestRank(values, 0.5),
        p95: nearestRank(values, 0.95)
      };
    }
  }
  const metrics = samples.flatMap((sample) => sample.observations.map((item) => item.metrics));
  const cancellationDelaysMs = samples.flatMap((sample) => sample.cancellationDelaysMs);
  summary.maxima = {
    checkpoint_bytes: Math.max(...metrics.map((item) => item.checkpoint_bytes)),
    checkpoint_rungs: Math.max(...metrics.map((item) => item.checkpoint_rungs)),
    replay_rung: Math.max(...metrics.map((item) => item.replay_rung)),
    replay_commands: metrics
      .map((item) => BigInt(item.replay_commands))
      .reduce((largest, value) => value > largest ? value : largest, 0n)
      .toString(),
    checkpoint_rungs_admitted: Math.max(...metrics.map((item) => item.checkpoint_rungs_admitted)),
    checkpoint_rungs_dropped: Math.max(...metrics.map((item) => item.checkpoint_rungs_dropped)),
    checkpoint_rungs_rejected: Math.max(...metrics.map((item) => item.checkpoint_rungs_rejected)),
    shipouts: Math.max(...metrics.map((item) => item.shipouts)),
    pages_rendered: Math.max(...metrics.map((item) => item.pages_rendered))
  };
  summary.cancellation_delay_ms = cancellationDelaysMs.length === 0 ? null : {
    count: cancellationDelaysMs.length,
    median: nearestRank(cancellationDelaysMs, 0.5),
    p95: nearestRank(cancellationDelaysMs, 0.95),
    max: Math.max(...cancellationDelaysMs)
  };
  return summary;
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
  options.bin = path.resolve(options.bin);
  if (!fs.existsSync(options.bin)) throw new Error(`binary does not exist: ${options.bin}`);
  fs.mkdirSync(options.output, { recursive: true });

  for (let index = 0; index < options.warmups; index += 1) {
    process.stderr.write(`warmup ${index + 1}/${options.warmups}\n`);
    await runSample(options);
  }

  const samples = [];
  for (let index = 0; index < options.samples; index += 1) {
    process.stderr.write(`sample ${index + 1}/${options.samples}\n`);
    const sample = await runSample(options);
    samples.push(sample);
    const stem = `sample-${String(index + 1).padStart(3, "0")}`;
    fs.writeFileSync(path.join(options.output, `${stem}.events.jsonl`), `${sample.rawLines.join("\n")}\n`);
    fs.writeFileSync(path.join(options.output, `${stem}.stderr.log`), sample.stderr);
    fs.writeFileSync(path.join(options.output, `${stem}.trace.json`), `${JSON.stringify({
      observations: sample.observations,
      requests: sample.requests,
      cancellation_delays_ms: sample.cancellationDelaysMs
    }, null, 2)}\n`);
  }

  const bundleDigests = [...new Set(samples.map((sample) => sample.bundleDigest))];
  assert.equal(bundleDigests.length, 1, "bundle digest changed during benchmark");
  const report = {
    schema_version: 2,
    generated_at: new Date().toISOString(),
    options: { ...options, bin: path.resolve(options.bin) },
    fixture: {
      primary: path.relative(process.cwd(), PRIMARY_PATH),
      child: path.relative(process.cwd(), CHILD_PATH)
    },
    environment: {
      git_commit: commandOutput("git", ["rev-parse", "HEAD"]),
      rustc: commandOutput("rustc", ["-Vv"]),
      uname: commandOutput("uname", ["-a"]),
      cpu: fs.existsSync("/proc/cpuinfo") ? commandOutput("sh", ["-c", "grep -m1 'model name' /proc/cpuinfo"]) : null,
      memory: fs.existsSync("/proc/meminfo") ? commandOutput("sh", ["-c", "grep -m1 'MemTotal' /proc/meminfo"]) : null,
      bundle_digest: bundleDigests[0]
    },
    summary: summarize(samples)
  };
  fs.writeFileSync(path.join(options.output, "summary.json"), `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(report.summary, null, 2)}\n`);
  process.stderr.write(`results: ${options.output}\n`);
}

await main();
