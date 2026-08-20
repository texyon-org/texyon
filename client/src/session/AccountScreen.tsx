import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import {
  confirmPassword,
  confirmTwoFactor,
  disableTwoFactor,
  enableTwoFactor,
  recoveryCodes,
  twoFactorQrCode,
  updatePassword,
  updateProfile,
} from "../api/account";
import { accountDestroy, getAccountShowQueryKey, useAccountShow } from "../api/generated";
import { ApiError } from "../api/http";
import type { Account } from "../api/types";
import { AccountSectionsSkeleton } from "../ui/skeletons";
import { TopBar } from "../ui/TopBar";
import { useSignOut } from "./useSignOut";

/** Per-section busywork: one pending action, one error, one success note. */
function useSection() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);

  const run = (work: () => Promise<string | null>): void => {
    void (async () => {
      setBusy(true);
      setError(null);
      setDone(null);
      try {
        setDone(await work());
      } catch (failure) {
        setError(failure instanceof Error ? failure.message : String(failure));
      } finally {
        setBusy(false);
      }
    })();
  };

  return { busy, error, done, run };
}

function needsConfirmation(failure: unknown): boolean {
  return failure instanceof ApiError && failure.needsPasswordConfirmation;
}

/** A password prompt for the 423 confirm-password dance. */
function ConfirmPassword({
  busy,
  onConfirm,
  onCancel,
}: {
  busy: boolean;
  onConfirm: (password: string) => void;
  onCancel: () => void;
}) {
  const [password, setPassword] = useState("");

  return (
    <>
      <label className="field">
        Confirm your password to continue
        <input
          type="password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          autoComplete="current-password"
          autoFocus
        />
      </label>
      <div className="section-actions">
        <button type="button" className="btn-link quiet" onClick={onCancel}>
          Cancel
        </button>
        <button
          type="button"
          className="btn-secondary"
          disabled={busy || password === ""}
          onClick={() => onConfirm(password)}
        >
          Confirm
        </button>
      </div>
    </>
  );
}

function ProfileSection({ account, onSaved }: { account: Account; onSaved: () => Promise<void> }) {
  const [name, setName] = useState(account.name);
  const [email, setEmail] = useState(account.email);
  const { busy, error, done, run } = useSection();

  // Reseed from the server without remounting, which would eat the saved note.
  useEffect(() => {
    setName(account.name);
    setEmail(account.email);
  }, [account.name, account.email]);

  return (
    <form
      className="account-section"
      onSubmit={(event) => {
        event.preventDefault();
        run(async () => {
          await updateProfile(name.trim(), email.trim());
          await onSaved();
          return "Saved.";
        });
      }}
    >
      <div className="section-head">
        <h2>Profile</h2>
      </div>
      <label className="field">
        Name
        <input value={name} onChange={(event) => setName(event.target.value)} autoComplete="name" required />
      </label>
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
      {error !== null && <div className="notice error">{error}</div>}
      <div className="section-actions">
        {done !== null && <span className="notice ok">{done}</span>}
        <button type="submit" className="btn-primary" disabled={busy}>
          Save changes
        </button>
      </div>
    </form>
  );
}

function PasswordSection() {
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const { busy, error, done, run } = useSection();

  return (
    <form
      className="account-section"
      onSubmit={(event) => {
        event.preventDefault();
        run(async () => {
          await updatePassword(current, next);
          setCurrent("");
          setNext("");
          return "Password updated.";
        });
      }}
    >
      <div className="section-head">
        <h2>Password</h2>
      </div>
      <label className="field">
        Current password
        <input
          type="password"
          value={current}
          onChange={(event) => setCurrent(event.target.value)}
          autoComplete="current-password"
          required
        />
      </label>
      <label className="field">
        New password
        <input
          type="password"
          value={next}
          onChange={(event) => setNext(event.target.value)}
          autoComplete="new-password"
          placeholder="At least 8 characters"
          required
        />
      </label>
      {error !== null && <div className="notice error">{error}</div>}
      <div className="section-actions">
        {done !== null && <span className="notice ok">{done}</span>}
        <button type="submit" className="btn-secondary" disabled={busy}>
          Update password
        </button>
      </div>
    </form>
  );
}

type TwoFactorStep =
  | { at: "idle" }
  | { at: "confirming"; then: "enable" | "disable" }
  | { at: "enrolling"; qr: string }
  | { at: "codes"; codes: string[] };

function TwoFactorSection({ account, onChanged }: { account: Account; onChanged: () => Promise<void> }) {
  const [step, setStep] = useState<TwoFactorStep>({ at: "idle" });
  const [code, setCode] = useState("");
  const { busy, error, done, run } = useSection();

  const enable = (): void =>
    run(async () => {
      try {
        await enableTwoFactor();
      } catch (failure) {
        if (!needsConfirmation(failure)) throw failure;
        setStep({ at: "confirming", then: "enable" });
        return null;
      }
      setStep({ at: "enrolling", qr: await twoFactorQrCode() });
      return null;
    });

  const disable = (): void =>
    run(async () => {
      try {
        await disableTwoFactor();
      } catch (failure) {
        if (!needsConfirmation(failure)) throw failure;
        setStep({ at: "confirming", then: "disable" });
        return null;
      }
      await onChanged();
      setStep({ at: "idle" });
      return "Two-factor authentication is off.";
    });

  return (
    <div className="account-section">
      <div className="section-head">
        <h2>Two-factor authentication</h2>
        <span className={account.twoFactorEnabled ? "pill on" : "pill"}>
          {account.twoFactorEnabled ? "enabled" : "not enabled"}
        </span>
      </div>
      <p className="section-note">Ask for a one-time code from an authenticator app at every sign-in.</p>

      {step.at === "confirming" && (
        <ConfirmPassword
          busy={busy}
          onCancel={() => setStep({ at: "idle" })}
          onConfirm={(password) => {
            const then = step.then;
            run(async () => {
              await confirmPassword(password);
              if (then === "enable") {
                await enableTwoFactor();
                setStep({ at: "enrolling", qr: await twoFactorQrCode() });
                return null;
              }
              await disableTwoFactor();
              await onChanged();
              setStep({ at: "idle" });
              return "Two-factor authentication is off.";
            });
          }}
        />
      )}

      {step.at === "enrolling" && (
        <>
          <p className="section-note">Scan the code with your authenticator app, then enter its first code.</p>
          {/* Server-rendered SVG from this origin's Fortify endpoint. */}
          <div className="qr" dangerouslySetInnerHTML={{ __html: step.qr }} />
          <label className="field">
            Code
            <input
              value={code}
              onChange={(event) => setCode(event.target.value)}
              inputMode="numeric"
              autoComplete="one-time-code"
              autoFocus
            />
          </label>
          <div className="section-actions">
            <button type="button" className="btn-link quiet" onClick={() => setStep({ at: "idle" })}>
              Cancel
            </button>
            <button
              type="button"
              className="btn-primary"
              disabled={busy || code.trim() === ""}
              onClick={() =>
                run(async () => {
                  await confirmTwoFactor(code.trim());
                  setCode("");
                  const codes = await recoveryCodes();
                  await onChanged();
                  setStep({ at: "codes", codes });
                  return null;
                })
              }
            >
              Confirm
            </button>
          </div>
        </>
      )}

      {step.at === "codes" && (
        <>
          <p className="section-note">
            Keep these recovery codes somewhere safe. Each one signs you in once if you lose the
            authenticator.
          </p>
          <ul className="recovery-codes">
            {step.codes.map((recovery) => (
              <li key={recovery}>{recovery}</li>
            ))}
          </ul>
          <div className="section-actions">
            <button type="button" className="btn-secondary" onClick={() => setStep({ at: "idle" })}>
              Done
            </button>
          </div>
        </>
      )}

      {error !== null && <div className="notice error">{error}</div>}

      {step.at === "idle" && (
        <div className="section-actions">
          {done !== null && <span className="notice ok">{done}</span>}
          {account.twoFactorEnabled ? (
            <button type="button" className="btn-secondary" disabled={busy} onClick={disable}>
              Disable two-factor
            </button>
          ) : (
            <button type="button" className="btn-secondary" disabled={busy} onClick={enable}>
              Enable two-factor
            </button>
          )}
        </div>
      )}
    </div>
  );
}

function DangerSection({ onDeleted }: { onDeleted: () => void }) {
  const [confirming, setConfirming] = useState(false);
  const { busy, error, run } = useSection();

  const destroy = (): void =>
    run(async () => {
      try {
        await accountDestroy();
      } catch (failure) {
        if (!needsConfirmation(failure)) throw failure;
        setConfirming(true);
        return null;
      }
      onDeleted();
      return null;
    });

  return (
    <div className="account-section danger">
      <div className="section-head">
        <h2>Danger zone</h2>
      </div>
      <p className="section-note">
        Deleting your account removes every project and every file in it. This cannot be undone.
      </p>

      {confirming ? (
        <ConfirmPassword
          busy={busy}
          onCancel={() => setConfirming(false)}
          onConfirm={(password) =>
            run(async () => {
              await confirmPassword(password);
              await accountDestroy();
              onDeleted();
              return null;
            })
          }
        />
      ) : (
        <div className="section-actions">
          <button
            type="button"
            className="btn-danger"
            disabled={busy}
            onClick={() => {
              if (window.confirm("Delete your account and every project in it?")) destroy();
            }}
          >
            Delete account
          </button>
        </div>
      )}

      {error !== null && <div className="notice error">{error}</div>}
    </div>
  );
}

export function AccountScreen() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const signOut = useSignOut();

  const account = useAccountShow();
  const user = account.data?.user ?? null;

  const refresh = async (): Promise<void> => {
    await queryClient.invalidateQueries({ queryKey: getAccountShowQueryKey() });
  };

  return (
    <div className="screen">
      <TopBar
        account={user}
        onBack={() => void navigate({ to: "/" })}
        onAccount={null}
        onSignOut={signOut}
      />

      <main className="screen-main">
        {user === null ? (
          <AccountSectionsSkeleton />
        ) : (
          <div className="screen-column narrow">
            <div>
              <div className="screen-title">
                <h1>Account</h1>
              </div>
              <span className="screen-sub">{user.email}</span>
            </div>

            <ProfileSection account={user} onSaved={refresh} />
            <PasswordSection />
            <TwoFactorSection account={user} onChanged={refresh} />
            <DangerSection
              onDeleted={() => {
                queryClient.clear();
                void navigate({ to: "/sign-in" });
              }}
            />
          </div>
        )}
      </main>
    </div>
  );
}
