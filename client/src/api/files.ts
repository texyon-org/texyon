import { projectFileShow } from "./generated";
import { ApiError } from "./http";

/** The file endpoint answers with a URL. It may be another origin, so it is fetched bare. */
export async function readFile(projectId: number, path: string): Promise<string> {
  const { url } = await projectFileShow(projectId, { path });

  const response = await fetch(url);
  if (!response.ok) throw new ApiError(response.status, `could not fetch ${path}`);

  return response.text();
}
