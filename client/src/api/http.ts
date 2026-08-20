export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }

  /** 419 is Laravel's expired-session status. */
  get isUnauthenticated(): boolean {
    return this.status === 401 || this.status === 419;
  }

  /** 423 means the password must be re-confirmed before this action. */
  get needsPasswordConfirmation(): boolean {
    return this.status === 423;
  }
}

/** Laravel's XSRF cookie, echoed back on every write. */
function csrfToken(): string | null {
  for (const cookie of document.cookie.split(";")) {
    const [name, ...rest] = cookie.trim().split("=");
    if (name === "XSRF-TOKEN") return decodeURIComponent(rest.join("="));
  }
  return null;
}

async function describe(response: Response): Promise<string> {
  try {
    const body: unknown = await response.json();
    if (typeof body === "object" && body !== null && "message" in body) {
      const message = (body as { message: unknown }).message;
      if (typeof message === "string" && message !== "") return message;
    }
  } catch {
    // Not JSON, the status is all we know.
  }
  return `HTTP ${response.status}`;
}

export async function request(path: string, init: RequestInit = {}): Promise<Response> {
  const headers = new Headers(init.headers);
  headers.set("Accept", "application/json");

  const token = csrfToken();
  if (token !== null) headers.set("X-XSRF-TOKEN", token);

  const response = await fetch(path, {
    ...init,
    headers,
    credentials: "same-origin",
  });

  if (!response.ok) {
    throw new ApiError(response.status, await describe(response));
  }

  return response;
}

/** The orval mutator, see orval.config.ts. Prepends /api, a 204 resolves to undefined. */
export async function apiFetch<T>(url: string, init: RequestInit = {}): Promise<T> {
  const response = await request(`/api${url}`, init);

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json() as Promise<T>;
}
