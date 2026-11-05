"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const { encode } = require("@msgpack/msgpack");

const { FrameReader, MAX_FRAME_BYTES, frame } = require("../engine");

test("frames carry their length and survive arbitrary chunking", () => {
  const messages = [{ status: "ok" }, { status: "exported", data: new Uint8Array([0, 255, 7]) }];
  const wire = Buffer.concat(messages.map(frame));
  assert.equal(wire.readUInt32LE(0), encode(messages[0]).byteLength);

  const received = [];
  const reader = new FrameReader((message) => received.push(message));
  for (let offset = 0; offset < wire.length; offset += 3) reader.push(wire.subarray(offset, offset + 3));
  assert.equal(received.length, 2);
  assert.deepEqual(received[0], messages[0]);
  assert.deepEqual([...received[1].data], [0, 255, 7]);
});

test("an oversized length is refused before buffering the body", () => {
  const header = Buffer.alloc(4);
  header.writeUInt32LE(MAX_FRAME_BYTES + 1, 0);
  const reader = new FrameReader(() => assert.fail("no frame expected"));
  assert.throws(() => reader.push(header), /too large/);
});
