/**
 * Turning a server-side project into a workspace the editor can hold.
 *
 * Everything is fetched up front rather than lazily, because the compiler
 * wants the whole project on every run and the completion index reads every
 * file too — there is no moment where holding back a file would have paid.
 */

import { listFiles, readFile } from "../api/client";
import type { Project } from "../api/client";
import { Workspace } from "../workspace/Workspace";

/** What a project starts as, when it starts empty. */
const SKELETON = String.raw`\documentclass{article}

\begin{document}

Hello.

\end{document}
`;

export async function openProject(project: Project): Promise<Workspace> {
  const infos = await listFiles(project.id);
  const texts = await Promise.all(infos.map((info) => readFile(project.id, info.path)));

  const workspace = new Workspace(
    Object.fromEntries(infos.map((info, index) => [info.path, texts[index]])),
  );

  // A brand-new project has no files yet. Created through the workspace rather
  // than the constructor so it arrives dirty, and the first save writes it.
  if (!workspace.has(project.primary)) {
    workspace.create(project.primary, SKELETON);
  }

  return workspace;
}
