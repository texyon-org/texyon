"use strict";

const cp = require("child_process");
const { decode, encode } = require("@msgpack/msgpack");

/** The session protocol this client speaks, checked by the server at startup. */
const PROTOCOL_VERSION = 2;

/** Refused before buffering, so a corrupt length cannot exhaust memory. */
const MAX_FRAME_BYTES = 256 * 1024 * 1024;

/** Encode one request as a length-prefixed MessagePack frame. */
function frame(message) {
  const body = encode(message);
  const header = Buffer.alloc(4);
  header.writeUInt32LE(body.byteLength, 0);
  return Buffer.concat([header, body]);
}

/** Split a byte stream into decoded frames, copying each byte at most once. */
class FrameReader {
  constructor(onMessage) {
    this.onMessage = onMessage;
    this.chunks = [];
    this.buffered = 0;
    this.needed = null;
  }

  push(chunk) {
    this.chunks.push(chunk);
    this.buffered += chunk.length;
    for (;;) {
      if (this.needed === null) {
        if (this.buffered < 4) return;
        this.needed = this.take(4).readUInt32LE(0);
        if (this.needed > MAX_FRAME_BYTES) throw new Error(`preview frame of ${this.needed} bytes is too large`);
      }
      if (this.buffered < this.needed) return;
      const body = this.take(this.needed);
      this.needed = null;
      this.onMessage(decode(body));
    }
  }

  take(length) {
    if (length === 0) return Buffer.alloc(0);
    const first = this.chunks[0];
    if (first.length >= length) {
      if (first.length === length) this.chunks.shift();
      else this.chunks[0] = first.subarray(length);
      this.buffered -= length;
      return first.subarray(0, length);
    }
    const out = Buffer.allocUnsafe(length);
    let filled = 0;
    while (filled < length) {
      const chunk = this.chunks[0];
      const count = Math.min(chunk.length, length - filled);
      chunk.copy(out, filled, 0, count);
      filled += count;
      if (count === chunk.length) this.chunks.shift();
      else this.chunks[0] = chunk.subarray(count);
    }
    this.buffered -= length;
    return out;
  }
}

/**
 * One resident engine process. Requests are answered in order, except
 * `cancel`, which stops the running operation and gets no reply.
 */
class NativeEngine {
  constructor(executable, { cwd, pack, onExit, onStderr }) {
    this.waiting = [];
    this.closed = false;
    const args = ["preview", "--protocol", String(PROTOCOL_VERSION)];
    if (pack) args.push("--pack", pack);
    this.process = cp.spawn(executable, args, {
      cwd,
      stdio: ["pipe", "pipe", "pipe"]
    });
    const reader = new FrameReader((message) => this.settle(message));
    this.process.stdout.on("data", (chunk) => {
      try {
        reader.push(chunk);
      } catch (error) {
        this.fail(error);
        this.process.kill();
      }
    });
    this.process.stderr.on("data", (data) => onStderr(data.toString()));
    this.process.on("error", (error) => this.fail(error));
    this.process.on("exit", (code, signal) => {
      this.fail(new Error(signal ? `preview server stopped by ${signal}` : `preview server exited with ${code}`));
      onExit(code, signal);
    });
  }

  /** Send a request and resolve with its response. */
  request(message) {
    if (this.closed) return Promise.reject(new Error("preview server is not running"));
    return new Promise((resolve, reject) => {
      this.waiting.push({ resolve, reject });
      this.process.stdin.write(frame(message));
    });
  }

  /** Stop the running operation. Only meaningful while a reply is pending. */
  cancel() {
    if (!this.closed) this.process.stdin.write(frame({ op: "cancel" }));
  }

  /** End the session cleanly, killing the process if it does not exit. */
  async close() {
    if (this.closed) return;
    const process = this.process;
    const killer = setTimeout(() => process.kill(), 1500);
    killer.unref();
    try {
      await this.request({ op: "shutdown" });
    } catch {
      // The exit handler reports why.
    }
    this.closed = true;
    process.stdin.end();
  }

  settle(message) {
    const next = this.waiting.shift();
    if (next) next.resolve(message);
  }

  fail(error) {
    this.closed = true;
    for (const { reject } of this.waiting.splice(0)) reject(error);
  }
}

module.exports = { FrameReader, MAX_FRAME_BYTES, NativeEngine, PROTOCOL_VERSION, frame };
