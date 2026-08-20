// Fortify's session endpoints, outside the generated API surface.

import { request } from "./http";

/** Sanctum issues the CSRF cookie here. Without it the first POST is refused. */
export async function primeSession(): Promise<void> {
  await request("/sanctum/csrf-cookie");
}

/** Sign in. A true result means a two-factor code is still owed. */
export async function login(email: string, password: string): Promise<{ twoFactor: boolean }> {
  await primeSession();
  const response = await request("/login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password }),
  });
  const body = (await response.json()) as { two_factor?: boolean };
  return { twoFactor: body.two_factor === true };
}

/** Complete a sign-in that asked for a second factor. */
export async function twoFactorChallenge(code: string, isRecoveryCode: boolean): Promise<void> {
  await request("/two-factor-challenge", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(isRecoveryCode ? { recovery_code: code } : { code }),
  });
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
