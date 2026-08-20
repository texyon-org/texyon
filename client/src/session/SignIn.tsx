import { useState } from "react";

import { login, register, twoFactorChallenge } from "../api/auth";
import { Wordmark } from "../ui/Wordmark";

export interface SignInProps {
  /** Something worth saying before the form, like "the server could not be reached". */
  notice: string | null;
  onSignedIn: () => void;
  /** Open the sample without an account, or null where that makes no sense. */
  onSample: (() => void) | null;
  /** "page" is the full split screen, "card" fits the session-expired overlay. */
  variant?: "page" | "card";
}

type Mode = "sign-in" | "register" | "challenge";

/**
 * The way in.
 *
 * One form for signing in, creating an account and the two-factor challenge,
 * because they share fields and a person who picked the wrong one should not
 * have to retype what they already gave.
 */
export function SignIn({ notice, onSignedIn, onSample, variant = "page" }: SignInProps) {
  const [mode, setMode] = useState<Mode>("sign-in");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [useRecovery, setUseRecovery] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = (): void => {
    void (async () => {
      setBusy(true);
      setError(null);
      try {
        if (mode === "challenge") {
          await twoFactorChallenge(code.trim(), useRecovery);
          onSignedIn();
        } else if (mode === "register") {
          await register(name.trim(), email.trim(), password);
          onSignedIn();
        } else {
          const { twoFactor } = await login(email.trim(), password);
          if (twoFactor) {
            setMode("challenge");
          } else {
            onSignedIn();
          }
        }
      } catch (failure) {
        setError(failure instanceof Error ? failure.message : String(failure));
      } finally {
        setBusy(false);
      }
    })();
  };

  const heading =
    mode === "register" ? "Create an account" : mode === "challenge" ? "Two-factor code" : "Sign in";
  const sub =
    mode === "register"
      ? "Create an account to keep projects."
      : mode === "challenge"
        ? useRecovery
          ? "Enter one of your recovery codes."
          : "Enter the code from your authenticator app."
        : "Welcome back. Your projects are where you left them.";

  const form = (
    <form
      className={variant === "page" ? "auth-form" : "card"}
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      {variant === "page" ? <h2>{heading}</h2> : <Wordmark size={18} />}
      <p className={variant === "page" ? "auth-sub" : "card-sub"}>{sub}</p>

      {notice !== null && <div className="notice">{notice}</div>}
      {error !== null && <div className="notice error">{error}</div>}

      {mode === "register" && (
        <label className="field">
          Name
          <input
            value={name}
            onChange={(event) => setName(event.target.value)}
            autoComplete="name"
            required
          />
        </label>
      )}

      {mode !== "challenge" && (
        <>
          <label className="field">
            Email
            <input
              type="email"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              autoComplete="email"
              required
            />
          </label>
          <label className="field">
            Password
            <input
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              autoComplete={mode === "register" ? "new-password" : "current-password"}
              required
            />
          </label>
        </>
      )}

      {mode === "challenge" && (
        <label className="field">
          {useRecovery ? "Recovery code" : "Code"}
          <input
            value={code}
            onChange={(event) => setCode(event.target.value)}
            autoComplete="one-time-code"
            inputMode={useRecovery ? "text" : "numeric"}
            autoFocus
            required
          />
        </label>
      )}

      <button type="submit" className="btn-primary" disabled={busy}>
        {mode === "register" ? "Create account" : mode === "challenge" ? "Verify" : "Sign in"}
      </button>

      <div className="auth-links">
        {mode === "challenge" ? (
          <>
            <button
              type="button"
              className="btn-link"
              onClick={() => {
                setUseRecovery((was) => !was);
                setCode("");
              }}
            >
              {useRecovery ? "Use an authenticator code instead" : "Use a recovery code instead"}
            </button>
            <button
              type="button"
              className="btn-link quiet"
              onClick={() => {
                setMode("sign-in");
                setCode("");
              }}
            >
              Back to sign in
            </button>
          </>
        ) : (
          <button
            type="button"
            className="btn-link"
            onClick={() => setMode((was) => (was === "register" ? "sign-in" : "register"))}
          >
            {mode === "register" ? "I already have an account" : "I need an account"}
          </button>
        )}

        {onSample !== null && mode !== "challenge" && (
          <button type="button" className="btn-link quiet" onClick={onSample}>
            Just try the sample, nothing is saved
          </button>
        )}
      </div>
    </form>
  );

  if (variant === "card") {
    return <div className="gate">{form}</div>;
  }

  return (
    <div className="auth">
      <aside className="auth-brand">
        <Wordmark size={24} />

        {mode === "register" ? (
          <div className="auth-hero">
            <h1>Your projects, on every machine.</h1>
            <p>
              An account keeps every file saved as you type. Close the tab mid-sentence, the
              sentence is on the server.
            </p>
          </div>
        ) : (
          <div className="auth-hero">
            <h1>The preview keeps up with your keystrokes.</h1>
            <p>
              FlashTeX compiles as you type. TeX, page diff, and paint, in the time it takes to
              blink, and only the pages that changed are redrawn.
            </p>
          </div>
        )}

        {mode === "register" ? (
          <div className="auth-chips">
            <span className="auth-chip ok">saved</span>
            <span className="auth-chip">every keystroke synced</span>
            <span className="auth-chip">nothing to install</span>
          </div>
        ) : (
          <div className="auth-chips">
            <span className="auth-chip hot">142 ms keystroke &rarr; pixels</span>
            <span className="auth-chip">2/24 pages redrawn</span>
            <span className="auth-chip">resumed at rung 3</span>
          </div>
        )}
      </aside>

      <div className="auth-side">{form}</div>
    </div>
  );
}
