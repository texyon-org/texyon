import { decodeText } from "../files/import";
import type { FileContent } from "../workspace/Workspace";
import { collabBlobUrl, projectFileShow } from "./generated";
import { ApiError, apiFetch } from "./http";
import type { Project } from "./types";

/** Fetched bare via the signed URL. UTF-8 becomes text, the rest stays bytes. */
export async function readFile(projectId: number, path: string): Promise<FileContent> {
  const { url } = await projectFileShow(projectId, { path });

  const response = await fetch(url);
  if (!response.ok) throw new ApiError(response.status, `could not fetch ${path}`);

  const bytes = new Uint8Array(await response.arrayBuffer());
  return decodeText(bytes) ?? bytes;
}

/** Upload bytes a project's document can then name. Hand-written, raw bodies are not in the OpenAPI export. */
export async function uploadBlob(projectId: number, bytes: Uint8Array): Promise<{ digest: string; size: number }> {
  return apiFetch(`/projects/${projectId}/blobs`, {
    method: "PUT",
    headers: { "Content-Type": "application/octet-stream" },
    body: bytes as BodyInit,
  });
}

/** Bytes the project's document names, by their digest. */
export async function readBlob(projectId: number, digest: string): Promise<Uint8Array> {
  const { url } = await collabBlobUrl(projectId, digest);
  const response = await fetch(url);
  if (!response.ok) throw new ApiError(response.status, `could not fetch blob ${digest}`);
  return new Uint8Array(await response.arrayBuffer());
}

export interface ImportedProject {
  project: Project;
  skipped: { path: string; reason: string }[];
}

/** A new project from one zip. Hand-written, raw bodies are not in the OpenAPI export. */
export async function importProject(name: string, zip: Uint8Array): Promise<ImportedProject> {
  return apiFetch(`/projects/import?name=${encodeURIComponent(name)}`, {
    method: "POST",
    headers: { "Content-Type": "application/zip" },
    body: zip as BodyInit,
  });
}
