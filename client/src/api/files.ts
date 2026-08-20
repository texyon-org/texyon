import { decodeText } from "../files/import";
import type { FileContent } from "../workspace/Workspace";
import { projectFileShow } from "./generated";
import { ApiError, apiFetch } from "./http";

/** Fetched bare via the signed URL. UTF-8 becomes text, the rest stays bytes. */
export async function readFile(projectId: number, path: string): Promise<FileContent> {
  const { url } = await projectFileShow(projectId, { path });

  const response = await fetch(url);
  if (!response.ok) throw new ApiError(response.status, `could not fetch ${path}`);

  const bytes = new Uint8Array(await response.arrayBuffer());
  return decodeText(bytes) ?? bytes;
}

/** Hand-written, the raw-body endpoint is not in the OpenAPI export. */
export async function writeFileBytes(
  projectId: number,
  path: string,
  bytes: Uint8Array,
): Promise<void> {
  await apiFetch(`/projects/${projectId}/file/bytes?path=${encodeURIComponent(path)}`, {
    method: "PUT",
    headers: { "Content-Type": "application/octet-stream" },
    body: bytes as BodyInit,
  });
}
