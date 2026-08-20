import { unzipSync } from "fflate";

export interface ImportEntry {
  path: string;
  bytes: Uint8Array;
}

export type SkipReason = "invalid path" | "too large";

export interface ImportPlan {
  files: { path: string; content: string | Uint8Array }[];
  skipped: { path: string; reason: SkipReason }[];
}

/** Mirrors blobs.max_file_bytes on the server. */
export const MAX_FILE_BYTES = 32 * 1024 * 1024;

const MAX_PATH_BYTES = 512;

const pathBytes = new TextEncoder();

/** Mirrors the server's ProjectPath::parse, null if it would refuse. */
export function normalizePath(raw: string): string | null {
  if (raw === "" || raw.includes("\0")) return null;

  const segments = raw.split(/[/\\]+/).filter((segment) => segment !== "" && segment !== ".");
  if (segments.length === 0 || segments.includes("..")) return null;

  const path = segments.join("/");
  if (pathBytes.encode(path).length > MAX_PATH_BYTES) return null;
  return path;
}

/** Filesystem litter that travels inside zips and folders. */
export function isJunk(path: string): boolean {
  return path
    .split(/[/\\]+/)
    .some(
      (segment) =>
        segment === "__MACOSX" ||
        segment === ".DS_Store" ||
        segment === "Thumbs.db" ||
        segment.startsWith("._"),
    );
}

export function decodeText(bytes: Uint8Array): string | null {
  if (bytes.includes(0)) return null;
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    return null;
  }
}

export function expandZip(bytes: Uint8Array): ImportEntry[] {
  const unzipped = unzipSync(bytes);
  const entries = Object.entries(unzipped)
    .filter(([path]) => !path.endsWith("/") && !isJunk(path))
    .map(([path, data]) => ({ path, bytes: data }));
  return stripCommonRoot(entries);
}

/** Drop a top-level directory that every entry lives under, if there is one. */
export function stripCommonRoot(entries: ImportEntry[]): ImportEntry[] {
  let root: string | null = null;
  for (const entry of entries) {
    const at = entry.path.indexOf("/");
    if (at <= 0) return entries;
    const first = entry.path.slice(0, at);
    if (root === null) root = first;
    else if (root !== first) return entries;
  }
  if (root === null) return entries;
  return entries.map((entry) => ({ ...entry, path: entry.path.slice(root.length + 1) }));
}

/** Later entries win when two name the same path. */
export function planImport(entries: ImportEntry[]): ImportPlan {
  const files = new Map<string, string | Uint8Array>();
  const skipped: ImportPlan["skipped"] = [];

  for (const entry of entries) {
    if (isJunk(entry.path)) continue;

    const path = normalizePath(entry.path);
    if (path === null) {
      skipped.push({ path: entry.path, reason: "invalid path" });
      continue;
    }
    if (entry.bytes.length > MAX_FILE_BYTES) {
      skipped.push({ path, reason: "too large" });
      continue;
    }
    files.set(path, decodeText(entry.bytes) ?? entry.bytes);
  }

  return { files: [...files].map(([path, content]) => ({ path, content })), skipped };
}
