import { expandZip } from "./import";
import type { ImportEntry } from "./import";

export interface UploadFile {
  path: string;
  file: File;
}

export function uploadsFromInput(files: FileList | null): UploadFile[] {
  return [...(files ?? [])].map((file) => ({
    path: file.webkitRelativePath !== "" ? file.webkitRelativePath : file.name,
    file,
  }));
}

/** Dropped directories only surface through webkitGetAsEntry. */
export async function uploadsFromDataTransfer(data: DataTransfer): Promise<UploadFile[]> {
  const entries = [...data.items]
    .map((item) => item.webkitGetAsEntry?.())
    .filter((entry): entry is FileSystemEntry => entry !== null && entry !== undefined);

  if (entries.length === 0) {
    return [...data.files].map((file) => ({ path: file.name, file }));
  }

  const out: UploadFile[] = [];
  for (const entry of entries) await walk(entry, out);
  return out;
}

async function walk(entry: FileSystemEntry, out: UploadFile[]): Promise<void> {
  if (entry.isFile) {
    const file = await new Promise<File>((resolve, reject) =>
      (entry as FileSystemFileEntry).file(resolve, reject),
    );
    out.push({ path: entry.fullPath.replace(/^\//, ""), file });
    return;
  }

  const reader = (entry as FileSystemDirectoryEntry).createReader();
  // readEntries hands out batches until it answers empty
  for (;;) {
    const batch = await new Promise<FileSystemEntry[]>((resolve, reject) =>
      reader.readEntries(resolve, reject),
    );
    if (batch.length === 0) return;
    for (const child of batch) await walk(child, out);
  }
}

export async function entriesFromUploads(
  uploads: readonly UploadFile[],
): Promise<{ entries: ImportEntry[]; errors: string[] }> {
  const entries: ImportEntry[] = [];
  const errors: string[] = [];

  for (const { path, file } of uploads) {
    const bytes = new Uint8Array(await file.arrayBuffer());
    if (/\.zip$/i.test(path)) {
      try {
        entries.push(...expandZip(bytes));
      } catch {
        errors.push(`${file.name} could not be read as a zip`);
      }
    } else {
      entries.push({ path, bytes });
    }
  }

  return { entries, errors };
}
