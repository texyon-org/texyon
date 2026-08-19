import { useState } from "react";

import { login, register } from "../api/auth";

export interface SignInProps {
  /** Something worth saying before the form — "the server could not be reached". */
  notice: string | null;
  onSignedIn: () => void;
  /** Open the sample without an account, or null where that makes no sense. */
  onSample: (() => void) | null;
}

/**
 * The way in.
 *
 * One form for both signing in and creating an account, because they differ by
 * a single field and a person who picked the wrong one should not have to
 * retype what they already gave.
 */
export function SignIn({ notice, onSignedIn, onSample }: SignInProps) {
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = (): void => {
    void (async () => {
      setBusy(true);
      setError(null);
      try {
        if (creating) {
          await register(name.trim(), email.trim(), password);
        } else {
          await login(email.trim(), password);
        }
        onSignedIn();
      } catch (failure) {
        setError(failure instanceof Error ? failure.message : String(failure));
      } finally {
        setBusy(false);
      }
    })();
  };

  return (
    <div className="gate">
      <form
        className="card"
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <h1>Tectonic</h1>
        <p className="card-sub">
          {creating ? "Create an account to keep projects." : "Sign in to your projects."}
        </p>

        {notice !== null && <div className="card-notice">{notice}</div>}
        {error !== null && <div className="card-error">{error}</div>}

        {creating && (
          <label>
            Name
            <input
              value={name}
              onChange={(event) => setName(event.target.value)}
              autoComplete="name"
              required
            />
          </label>
        )}
        <label>
          Email
          <input
            type="email"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            autoComplete="email"
            required
          />
        </label>
        <label>
          Password
          <input
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            autoComplete={creating ? "new-password" : "current-password"}
            required
          />
        </label>

        <button type="submit" className="card-primary" disabled={busy}>
          {creating ? "Create account" : "Sign in"}
        </button>

        <button type="button" className="card-link" onClick={() => setCreating((was) => !was)}>
          {creating ? "I already have an account" : "I need an account"}
        </button>

        {onSample !== null && (
          <button type="button" className="card-link" onClick={onSample}>
            Just try the sample — nothing is saved
          </button>
        )}
      </form>
    </div>
  );
}
