/**
 * The server, as this application sees it.
 *
 * A thin, typed face over the endpoints in `server/routes/api.php`. It exists
 * so that the rest of the client never handles an untyped JSON body, and so
 * that the one thing every request has in common — the session cookie and the
 * CSRF token that goes with it — is written once.
 */

/** A project, as the server describes one. */
export interface Project {
  id: number;
  name: string;
  primary: string;
  createdAt: string;
  updatedAt: string;
}

/** A file's metadata. Its contents are fetched separately. */
export interface ProjectFileInfo {
  path: string;
  digest: string;
  size: number;
  updatedAt: string;
}

/** The server said no, and this is what it said. */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }

  /** Whether this means "sign in", as opposed to something being wrong. */
  get isUnauthenticated(): boolean {
    return this.status === 401 || this.status === 419;
  }
}

/** Read the XSRF cookie Laravel sets, which every write has to echo back. */
function csrfToken(): string | null {
  for (const cookie of document.cookie.split(";")) {
    const [name, ...rest] = cookie.trim().split("=");
    if (name === "XSRF-TOKEN") return decodeURIComponent(rest.join("="));
  }
  return null;
}

async function request(path: string, init: RequestInit = {}): Promise<Response> {
  const headers = new Headers(init.headers);
  headers.set("Accept", "application/json");

  const token = csrfToken();
  if (token !== null) headers.set("X-XSRF-TOKEN", token);

  const response = await fetch(path, {
    ...init,
    headers,
    // The session is a cookie on this origin, so it has to be sent.
    credentials: "same-origin",
  });

  if (!response.ok) {
    throw new ApiError(response.status, await describe(response));
  }

  return response;
}

/** The most useful sentence available about a failed response. */
async function describe(response: Response): Promise<string> {
  try {
    const body: unknown = await response.json();
    if (typeof body === "object" && body !== null && "message" in body) {
      const message = (body as { message: unknown }).message;
      if (typeof message === "string" && message !== "") return message;
    }
  } catch {
    // A response that is not JSON tells us nothing beyond its status.
  }
  return `HTTP ${response.status}`;
}

async function json<T>(path: string, init?: RequestInit): Promise<T> {
  return (await request(path, init)).json() as Promise<T>;
}

/**
 * Establish a session cookie before the first write.
 *
 * Sanctum issues the CSRF cookie from its own endpoint; without it, the first
 * POST is rejected for a token it was never given.
 */
export async function primeSession(): Promise<void> {
  await request("/sanctum/csrf-cookie");
}

/**
 * Sign in, establishing the session cookie everything else rides on.
 *
 * Primes the CSRF cookie first, because on a fresh browser there is nothing to
 * echo back yet and Fortify would refuse the attempt for it.
 */
export async function login(email: string, password: string): Promise<void> {
  await primeSession();
  const body = await json<{ two_factor?: boolean }>("/login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password }),
  });
  if (body.two_factor === true) {
    // The server supports it; this client does not challenge for it yet.
    throw new ApiError(501, "this account uses two-factor authentication, which the editor cannot complete yet");
  }
}

/** Create an account. Signs the new user in as a side effect. */
export async function register(name: string, email: string, password: string): Promise<void> {
  await primeSession();
  await request("/register", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    // The confirmation is the same string because the form asks once; typing a
    // password twice guards transcription on paper, not in a password manager.
    body: JSON.stringify({ name, email, password, password_confirmation: password }),
  });
}

export async function logout(): Promise<void> {
  await request("/logout", { method: "POST" });
}

export async function listProjects(): Promise<Project[]> {
  return (await json<{ projects: Project[] }>("/api/projects")).projects;
}

export async function createProject(name: string): Promise<Project> {
  const body = await json<{ project: Project }>("/api/projects", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name }),
  });
  return body.project;
}

export async function deleteProject(id: number): Promise<void> {
  await request(`/api/projects/${id}`, { method: "DELETE" });
}

export async function listFiles(projectId: number): Promise<ProjectFileInfo[]> {
  return (await json<{ files: ProjectFileInfo[] }>(`/api/projects/${projectId}/files`)).files;
}

/**
 * One file's contents.
 *
 * The server either sends the bytes or, where its store can sign a URL, sends
 * the URL instead so the bytes never pass through it. Both are handled here so
 * no caller has to know which store is configured.
 */
export async function readFile(projectId: number, path: string): Promise<string> {
  const response = await request(`/api/projects/${projectId}/files/${encodePath(path)}`);

  if (response.headers.get("content-type")?.includes("application/json") === true) {
    const body = (await response.json()) as { url?: string };
    if (typeof body.url === "string") {
      const direct = await fetch(body.url);
      if (!direct.ok) throw new ApiError(direct.status, `could not fetch ${path}`);
      return direct.text();
    }
  }

  return response.text();
}

export async function writeFile(
  projectId: number,
  path: string,
  text: string,
): Promise<ProjectFileInfo> {
  const body = await json<{ file: ProjectFileInfo }>(
    `/api/projects/${projectId}/files/${encodePath(path)}`,
    {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text }),
    },
  );
  return body.file;
}

export async function deleteFile(projectId: number, path: string): Promise<void> {
  await request(`/api/projects/${projectId}/files/${encodePath(path)}`, { method: "DELETE" });
}

/**
 * Escape a path for a URL without escaping its separators.
 *
 * The route matches the whole remainder as one parameter, so `chapters/one.tex`
 * has to arrive with its slash intact — but everything else in a segment still
 * needs encoding.
 */
function encodePath(path: string): string {
  return path.split("/").map(encodeURIComponent).join("/");
}
