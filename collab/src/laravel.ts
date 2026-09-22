import type { Contents } from "./model.ts";

/** What Laravel holds for a project: the saved document, or the files to seed one from. */
export interface Stored extends Contents {
  state: Uint8Array | null;
  epoch: string | null;
}

export class ProjectGone extends Error {}

/** The internal endpoints of CollabDocumentController. */
export class Laravel {
  readonly #base: string;
  readonly #secret: string;

  constructor(base: string, secret: Buffer) {
    this.#base = base.replace(/\/$/, "");
    this.#secret = secret.toString("hex");
  }

  async fetch(projectId: number): Promise<Stored> {
    const response = await this.#call("GET", projectId);
    const body = (await response.json()) as Contents & { state: string | null; epoch: string | null };
    return {
      state: body.state === null ? null : new Uint8Array(Buffer.from(body.state, "base64")),
      epoch: body.epoch,
      files: body.files,
      binaries: body.binaries,
    };
  }

  /** Store the first state of a document. False when another seed got there first. */
  async create(projectId: number, state: Uint8Array, epoch: string, contents: Contents): Promise<boolean> {
    const response = await this.#call("PUT", projectId, { state, epoch, contents }, "?create=1", [409]);
    return response.status !== 409;
  }

  async store(projectId: number, state: Uint8Array, epoch: string | null, contents: Contents): Promise<void> {
    await this.#call("PUT", projectId, { state, epoch, contents });
  }

  async #call(
    method: string,
    projectId: number,
    payload?: { state: Uint8Array; epoch: string | null; contents: Contents },
    query = "",
    allowed: number[] = [],
  ): Promise<Response> {
    const response = await fetch(`${this.#base}/api/internal/collab/projects/${projectId}${query}`, {
      method,
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        "X-Collab-Secret": this.#secret,
      },
      body:
        payload === undefined
          ? undefined
          : JSON.stringify({
              state: Buffer.from(payload.state).toString("base64"),
              epoch: payload.epoch,
              files: payload.contents.files,
              binaries: payload.contents.binaries,
            }),
    });
    if (response.status === 404) throw new ProjectGone(`project ${projectId} is gone`);
    if (!response.ok && !allowed.includes(response.status)) {
      throw new Error(`${method} project ${projectId}: HTTP ${response.status} ${await response.text()}`);
    }
    return response;
  }
}
