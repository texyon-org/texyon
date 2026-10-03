import { decodeText } from "../files/import";
import type { FileContent } from "../workspace/Workspace";
import { collabBlobUrl, projectFileShow } from "./generated";
import { ApiError } from "./http";

/** Fetched bare via the signed URL. UTF-8 becomes text, the rest stays bytes. */
export async function readFile(projectId: number, path: string): Promise<FileContent> {
  const { url } = await projectFileShow(projectId, { path });

  const response = await fetch(url);
  if (!response.ok) throw new ApiError(response.status, `could not fetch ${path}`);

  const bytes = new Uint8Array(await response.arrayBuffer());
  return decodeText(bytes) ?? bytes;
}

/** Bytes the project's document names, by their digest. */
export async function readBlob(projectId: number, digest: string): Promise<Uint8Array> {
  const { url } = await collabBlobUrl(projectId, digest);
  const response = await fetch(url);
  if (!response.ok) throw new ApiError(response.status, `could not fetch blob ${digest}`);
  return new Uint8Array(await response.arrayBuffer());
}
