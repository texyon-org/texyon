/**
 * Turning a server-side project into a workspace the editor can hold.
 *
 * Everything is fetched up front rather than lazily, because the compiler
 * wants the whole project on every run and the completion index reads every
 * file too — there is no moment where holding back a file would have paid.
 */

import { readFile } from "../api/files";
import { projectFileIndex } from "../api/generated";
import type { Project } from "../api/types";
import { Workspace } from "../workspace/Workspace";

/** What a project starts as, when it starts empty. */
export const SKELETON = String.raw`\documentclass{article}

\begin{document}

Hello.

\end{document}
`;

export async function openProject(project: Project): Promise<Workspace> {
  const { files: infos } = await projectFileIndex(project.id);
  const contents = await Promise.all(infos.map((info) => readFile(project.id, info.path)));

  const workspace = new Workspace(
    Object.fromEntries(infos.map((info, index) => [info.path, contents[index]])),
  );

  // The server writes this file when it creates a project. Projects made
  // before that did not get one, so the editor can still start from something.
  if (!workspace.has(project.primary)) {
    workspace.create(project.primary, SKELETON);
  }

  return workspace;
}
