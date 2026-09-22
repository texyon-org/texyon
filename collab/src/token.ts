import { createHmac, timingSafeEqual } from "node:crypto";

export type Role = "owner" | "editor" | "viewer";

/** Who a connection belongs to, as Laravel vouched for it a moment ago. */
export interface Claims {
  uid: number;
  pid: number;
  role: Role;
  name: string;
  exp: number;
}

const ROLES: readonly string[] = ["owner", "editor", "viewer"];

/** `base64url(json).base64url(hmac)`, minted by CollabTokenController. */
export function verifyToken(token: string, secret: Buffer, nowSeconds: number): Claims | null {
  const [body, signature, ...rest] = token.split(".");
  if (body === undefined || signature === undefined || rest.length > 0) return null;

  const expected = createHmac("sha256", secret).update(body).digest();
  const given = Buffer.from(signature, "base64url");
  if (given.length !== expected.length || !timingSafeEqual(given, expected)) return null;

  let claims: unknown;
  try {
    claims = JSON.parse(Buffer.from(body, "base64url").toString("utf8"));
  } catch {
    return null;
  }
  if (!isClaims(claims) || claims.exp < nowSeconds) return null;
  return claims;
}

function isClaims(value: unknown): value is Claims {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    Number.isInteger(v.uid) &&
    Number.isInteger(v.pid) &&
    typeof v.role === "string" &&
    ROLES.includes(v.role) &&
    typeof v.name === "string" &&
    typeof v.exp === "number"
  );
}

/** For tests, and the one place the format is written down in this package. */
export function signToken(claims: Claims, secret: Buffer): string {
  const body = Buffer.from(JSON.stringify(claims)).toString("base64url");
  const signature = createHmac("sha256", secret).update(body).digest("base64url");
  return `${body}.${signature}`;
}
