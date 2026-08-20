// Fortify's signed-in user endpoints, outside the generated API surface.

import { request } from "./http";

function asJson(body: unknown): RequestInit {
  return {
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  };
}

export async function updateProfile(name: string, email: string): Promise<void> {
  await request("/user/profile-information", { method: "PUT", ...asJson({ name, email }) });
}

export async function updatePassword(current: string, next: string): Promise<void> {
  await request("/user/password", {
    method: "PUT",
    // Confirmation repeats the new password, the form asks once.
    ...asJson({ current_password: current, password: next, password_confirmation: next }),
  });
}

/** Unlocks actions guarded by the 423 confirm-password dance. */
export async function confirmPassword(password: string): Promise<void> {
  await request("/user/confirm-password", { method: "POST", ...asJson({ password }) });
}

export async function enableTwoFactor(): Promise<void> {
  await request("/user/two-factor-authentication", { method: "POST" });
}

/** The QR the authenticator app scans, as inline SVG markup. */
export async function twoFactorQrCode(): Promise<string> {
  const response = await request("/user/two-factor-qr-code");
  const body = (await response.json()) as { svg: string };
  return body.svg;
}

/** Confirm enrollment with a first code, which is what arms the factor. */
export async function confirmTwoFactor(code: string): Promise<void> {
  await request("/user/confirmed-two-factor-authentication", { method: "POST", ...asJson({ code }) });
}

export async function recoveryCodes(): Promise<string[]> {
  const response = await request("/user/two-factor-recovery-codes");
  return (await response.json()) as string[];
}

export async function disableTwoFactor(): Promise<void> {
  await request("/user/two-factor-authentication", { method: "DELETE" });
}
