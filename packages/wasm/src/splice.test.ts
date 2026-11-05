import assert from "node:assert/strict";
import { test } from "node:test";

import { applyEdit, spliceBetween } from "./splice.ts";

test("a typed character is a zero-width insert", () => {
  assert.deepEqual(spliceBetween("abcdef", "abcXdef"), { from: 3, to: 3, insert: "X" });
});

test("a deletion inserts nothing", () => {
  assert.deepEqual(spliceBetween("abcdef", "abef"), { from: 2, to: 4, insert: "" });
});

test("identical text is an empty splice", () => {
  assert.deepEqual(spliceBetween("same", "same"), { from: 4, to: 4, insert: "" });
});

test("a repeated run does not let prefix and suffix overlap", () => {
  assert.deepEqual(spliceBetween("aaaa", "aaaaa"), { from: 4, to: 4, insert: "a" });
  assert.deepEqual(spliceBetween("aaaaa", "aaaa"), { from: 4, to: 5, insert: "" });
});

test("edits far into long text are found past the chunked compare", () => {
  const body = "x".repeat(10_000);
  const before = `${body}middle${body}`;
  const after = `${body}MIDDLE${body}`;
  assert.deepEqual(spliceBetween(before, after), { from: 10_000, to: 10_006, insert: "MIDDLE" });
});

test("a splice never cuts a surrogate pair in half", () => {
  // Both emoji share their high surrogate.
  const edit = spliceBetween("a😀b", "a😁b");
  assert.deepEqual(edit, { from: 1, to: 3, insert: "😁" });
});

test("applying the splice always reproduces the new text", () => {
  const alphabet = ["a", "b", "\n", "é", "😀", "😁", "\\"];
  let seed = 7;
  const random = (limit: number): number => {
    seed = (seed * 1_103_515_245 + 12_345) % 2 ** 31;
    return seed % limit;
  };
  const text = (length: number): string =>
    Array.from({ length }, () => alphabet[random(alphabet.length)]).join("");

  for (let round = 0; round < 2000; round++) {
    const before = text(random(600));
    const cut = random(before.length + 1);
    const pairSafe = (at: number): number =>
      at > 0 && at < before.length && /[\udc00-\udfff]/.test(before[at]) ? at - 1 : at;
    const from = pairSafe(cut);
    const to = pairSafe(Math.min(before.length, from + random(20)));
    const after = before.slice(0, from) + text(random(8)) + before.slice(Math.max(from, to));

    const edit = spliceBetween(before, after);
    assert.equal(applyEdit(before, edit), after);
    for (const at of [edit.from, edit.to]) {
      assert.ok(!/[\udc00-\udfff]/.test(before[at] ?? ""), `offset ${at} splits a pair`);
    }
  }
});
