"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");

const { parseServerLine } = require("../server-line");

test("parses protocol events", () => {
  assert.deepEqual(parseServerLine('{"type":"ready","protocol_version":1}'), {
    event: { type: "ready", protocol_version: 1 }
  });
});

test("diagnoses a Tectonic binary built without live-preview support", () => {
  assert.deepEqual(
    parseServerLine('note: "version 2" Tectonic command-line interface activated'),
    {
      error:
        "configured Tectonic executable does not support live preview; " +
        "rebuild it with `cargo build --release -p tectonic --features preview-mupdf`"
    }
  );
});

test("reports other malformed server output as invalid JSON", () => {
  assert.match(parseServerLine("not json").error, /^server emitted invalid JSON:/);
});
