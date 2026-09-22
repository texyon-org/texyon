import { createHmac, timingSafeEqual } from "node:crypto";

/** The key Laravel and this server share, derived from APP_KEY the same way on both sides. */
export function deriveSecret(appKey: string): Buffer {
  const key = appKey.startsWith("base64:") ? Buffer.from(appKey.slice(7), "base64") : Buffer.from(appKey, "utf8");
  if (key.length === 0) throw new Error("APP_KEY is empty");
  return createHmac("sha256", key).update("tectonic-collab").digest();
}

export function sameSecret(presented: string | undefined, secret: Buffer): boolean {
  if (presented === undefined) return false;
  const expected = Buffer.from(secret.toString("hex"));
  const given = Buffer.from(presented);
  return given.length === expected.length && timingSafeEqual(given, expected);
}
