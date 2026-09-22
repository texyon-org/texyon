import { readFileSync } from "node:fs";

import { Laravel } from "./laravel.ts";
import { deriveSecret } from "./secret.ts";
import { createCollabServer } from "./server.ts";

/** APP_KEY from the environment, else from the Laravel .env named by LARAVEL_ENV_FILE. */
function appKey(): string {
  const fromEnv = process.env.APP_KEY;
  if (fromEnv !== undefined && fromEnv !== "") return fromEnv;
  const file = process.env.LARAVEL_ENV_FILE;
  if (file !== undefined) {
    const line = readFileSync(file, "utf8").split("\n").find((l) => l.startsWith("APP_KEY="));
    if (line !== undefined) return line.slice("APP_KEY=".length).trim().replace(/^"(.*)"$/, "$1");
  }
  throw new Error("APP_KEY is not set (nor LARAVEL_ENV_FILE pointing at a .env with one)");
}

const secret = deriveSecret(appKey());
const port = Number(process.env.COLLAB_PORT ?? 1234);
const origins = (process.env.COLLAB_ALLOWED_ORIGINS ?? "").split(",").map((o) => o.trim()).filter((o) => o !== "");

const server = createCollabServer({
  port,
  address: process.env.COLLAB_ADDRESS ?? "127.0.0.1",
  secret,
  laravel: new Laravel(process.env.LARAVEL_URL ?? "http://127.0.0.1", secret),
  origins,
});

await server.listen();
console.log(`collab listening on ${port}`);
