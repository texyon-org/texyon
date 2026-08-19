// Fortify's session endpoints, outside the generated API surface.

import { ApiError, request } from "./http";

/** Sanctum issues the CSRF cookie here. Without it the first POST is refused. */
export async function primeSession(): Promise<void> {
  await request("/sanctum/csrf-cookie");
}

export async function login(email: string, password: string): Promise<void> {
  await primeSession();
  const response = await request("/login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password }),
  });
  const body = (await response.json()) as { two_factor?: boolean };
  if (body.two_factor === true) {
    // The server supports it, this client cannot complete it yet.
    throw new ApiError(501, "this account uses two-factor authentication, which the editor cannot complete yet");
  }
}

/** Create an account. Signs the new user in as a side effect. */
export async function register(name: string, email: string, password: string): Promise<void> {
  await primeSession();
  await request("/register", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name, email, password, password_confirmation: password }),
  });
}

export async function logout(): Promise<void> {
  await request("/logout", { method: "POST" });
}
