import { unzipSync } from "fflate";

import { isJunk } from "./import.ts";

export interface ProjectArchive {
  name: string;
  bytes: Uint8Array;
}

const ZIP = /\.zip$/i;

function nameOf(path: string): string {
  const base = path.split(/[/\\]/).pop() ?? path;
  return base.replace(ZIP, "").trim() || "Imported project";
}

/**
 * One zip per project. A zip holding only zips is a bulk export, like
 * Overleaf's "download selected projects", and each inner zip is a project.
 */
export function projectArchives(fileName: string, bytes: Uint8Array): ProjectArchive[] {
  let onlyZips = true;
  let any = false;
  const inner = unzipSync(bytes, {
    filter: ({ name }) => {
      if (name.endsWith("/") || isJunk(name)) return false;
      any = true;
      if (!ZIP.test(name)) onlyZips = false;
      return ZIP.test(name);
    },
  });

  if (!any || !onlyZips) return [{ name: nameOf(fileName), bytes }];

  return Object.entries(inner)
    .map(([path, data]) => ({ name: nameOf(path), bytes: data }))
    .sort((a, b) => a.name.localeCompare(b.name));
}
