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

/** Split a byte stream into decoded frames. */
class FrameReader {
  constructor(onMessage) {
    this.onMessage = onMessage;
    this.pending = Buffer.alloc(0);
  }

  push(chunk) {
    this.pending = this.pending.length === 0 ? chunk : Buffer.concat([this.pending, chunk]);
    while (this.pending.length >= 4) {
      const length = this.pending.readUInt32LE(0);
      if (length > MAX_FRAME_BYTES) throw new Error(`preview frame of ${length} bytes is too large`);
      if (this.pending.length < 4 + length) return;
      const body = this.pending.subarray(4, 4 + length);
      this.pending = this.pending.subarray(4 + length);
      this.onMessage(decode(body));
    }
  }
}

/**
 * One resident engine process. Requests are answered in order, except
 * `cancel`, which stops the running operation and gets no reply.
 */
class NativeEngine {
  constructor(executable, { cwd, onExit, onStderr }) {
    this.waiting = [];
    this.closed = false;
    this.process = cp.spawn(executable, ["-X", "preview", "--protocol", String(PROTOCOL_VERSION)], {
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
