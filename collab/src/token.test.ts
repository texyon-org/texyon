import assert from "node:assert/strict";
import { test } from "node:test";

import { deriveSecret } from "./secret.ts";
import { signToken, verifyToken } from "./token.ts";

const secret = deriveSecret("base64:" + Buffer.alloc(32, 7).toString("base64"));
const claims = { uid: 3, pid: 9, role: "editor" as const, name: "Ada", exp: 1000 };

test("a token signed with the shared secret verifies until it expires", () => {
  const token = signToken(claims, secret);
  assert.deepEqual(verifyToken(token, secret, 999), claims);
  assert.equal(verifyToken(token, secret, 1001), null);
});

test("a tampered or foreign token does not verify", () => {
  const token = signToken(claims, secret);
  const [body, signature] = token.split(".");
  const forged = Buffer.from(JSON.stringify({ ...claims, role: "owner" })).toString("base64url");
  assert.equal(verifyToken(`${forged}.${signature}`, secret, 0), null);
  assert.equal(verifyToken(`${body}.${signature}x`, secret, 0), null);
  assert.equal(verifyToken(token, deriveSecret("another key"), 0), null);
  assert.equal(verifyToken("garbage", secret, 0), null);
});

test("the derived secret matches what PHP computes", () => {
  // php -r 'echo bin2hex(hash_hmac("sha256", "tectonic-collab", str_repeat(chr(7), 32), true));'
  const php = "d70b5687ba7948b25c5e87fa03efb5ac96f4f6c6ecb2a22efd37228300904d84";
  assert.equal(secret.toString("hex"), php);
});
