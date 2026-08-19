import assert from "node:assert/strict";
import { test } from "node:test";

import { parseTtbHeader, TtbIndex, TTB_HEADER_BYTES } from "./ttb.ts";

function header(fields: { version?: number; offset: bigint; gz: number; real: number }): Uint8Array {
  const bytes = new Uint8Array(TTB_HEADER_BYTES);
  const view = new DataView(bytes.buffer);
  new TextEncoder().encodeInto("tectonicbundle", bytes);
  view.setUint32(14, fields.version ?? 1, true);
  view.setBigUint64(18, fields.offset, true);
  view.setUint32(26, fields.gz, true);
  view.setUint32(30, fields.real, true);
  return bytes;
}

test("the header yields the index location", () => {
  const parsed = parseTtbHeader(header({ offset: 76_845_637n, gz: 6_065_990, real: 17_331_559 }));
  assert.deepEqual(parsed, {
    indexOffset: 76_845_637,
    indexGzLength: 6_065_990,
    indexRealLength: 17_331_559,
  });
});

test("wrong magic and wrong version are refused with a reason", () => {
  const wrongMagic = header({ offset: 1n, gz: 1, real: 1 });
  wrongMagic[0] = 0x58;
  assert.throws(() => parseTtbHeader(wrongMagic), /magic/);

  assert.throws(() => parseTtbHeader(header({ version: 2, offset: 1n, gz: 1, real: 1 })), /version 2/);
});

/** An index with two same-named files and one path with a space in it. */
const INDEX = `
this line precedes any section and is ignored
[DEFAULTSEARCH]
IGNORED
MAIN
[SEARCH:MAIN]
/
/texlive/tex/latex//
/texlive/tex/plain//
[SEARCH:IGNORED]
/nowhere//
[FILELIST]
10 20 30 nohash SHA256SUM
100 200 300 abc123 texlive/tex/latex/booktabs/booktabs.sty
400 500 600 abc456 texlive/tex/plain/plain/plain.tex
700 800 900 abc789 texlive/tex/latex/zzcls/duplicate.sty
1000 1100 1200 abcabc texlive/tex/latex/aacls/duplicate.sty
1300 1400 1500 nohash texlive/tex/latex/spacey/a file with spaces.tex
1600 1700 1800 abcdef texlive/tex/latex/fithesis/fithesis4.sty
1900 2000 2100 abcdef texlive/fonts/unreachable/hidden.sty
[UNKNOWN SECTION]
this is ignored too
`;

test("a bare name resolves through the search order", () => {
  const index = TtbIndex.parse(INDEX);
  assert.equal(index.size, 8);

  assert.deepEqual(index.resolve("booktabs.sty"), { offset: 100, gzLength: 200, realLength: 300 });
  assert.deepEqual(index.resolve("plain.tex"), { offset: 400, gzLength: 500, realLength: 600 });
  // `/` is an exact rule: it matches only top-level files.
  assert.deepEqual(index.resolve("SHA256SUM"), { offset: 10, gzLength: 20, realLength: 30 });
});

test("several matches at the same priority pick the alphabetically first path", () => {
  const index = TtbIndex.parse(INDEX);
  // aacls sorts before zzcls, whatever order the FILELIST listed them in.
  assert.deepEqual(index.resolve("duplicate.sty"), { offset: 1000, gzLength: 1100, realLength: 1200 });
});

test("a file outside every search path is not found", () => {
  const index = TtbIndex.parse(INDEX);
  // It exists in the FILELIST, but no rule reaches texlive/fonts.
  assert.equal(index.resolve("hidden.sty"), null);
});

test("paths may contain spaces, because the path field is last", () => {
  const index = TtbIndex.parse(INDEX);
  assert.deepEqual(index.resolve("a file with spaces.tex"), {
    offset: 1300,
    gzLength: 1400,
    realLength: 1500,
  });
});

test("a relative path matches the tail of exactly one entry", () => {
  const index = TtbIndex.parse(INDEX);
  assert.deepEqual(index.resolve("fithesis/fithesis4.sty"), {
    offset: 1600,
    gzLength: 1700,
    realLength: 1800,
  });
  // Ambiguous tails are refused rather than guessed at.
  const ambiguous = TtbIndex.parse(
    INDEX.replace("[UNKNOWN SECTION]", "2200 2300 2400 x other/fithesis/fithesis4.sty\n[UNKNOWN SECTION]"),
  );
  assert.equal(ambiguous.resolve("fithesis/fithesis4.sty"), null);
});

test("absolute names and unknown names resolve to nothing", () => {
  const index = TtbIndex.parse(INDEX);
  assert.equal(index.resolve("/etc/passwd"), null);
  assert.equal(index.resolve("no-such-file.sty"), null);
  // And the cached answer is the same answer.
  assert.equal(index.resolve("no-such-file.sty"), null);
});

test("an index without its default search order is broken, not sparse", () => {
  assert.throws(
    () => TtbIndex.parse("[DEFAULTSEARCH]\nMISSING\n[FILELIST]\n1 2 3 h a.tex\n"),
    /SEARCH:MISSING/,
  );
});
