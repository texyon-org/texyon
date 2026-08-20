import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";

import type { Account } from "../api/types";
import { BackIcon, ChevronDownIcon, GearIcon, SignOutIcon } from "./icons";
import { Wordmark } from "./Wordmark";

export interface TopBarProps {
  /** The signed-in account, or null while it loads. */
  account: Account | null;
  /** Where "back" goes, or null on the root screen. */
  onBack: (() => void) | null;
  /** Null on the account screen itself, and where there is no account. */
  onAccount: (() => void) | null;
  onSignOut: () => void;
  /** Screen-specific content after the wordmark, like the open project's name. */
  context?: ReactNode;
  /** Screen-specific controls before the account menu, like the editor's metrics. */
  actions?: ReactNode;
}

function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter((part) => part !== "");
  const letters = parts.slice(0, 2).map((part) => part[0] ?? "");
  return letters.join("").toUpperCase() || "?";
}

/** The one app bar. Every screen renders this, with its own stuff inside. */
export function TopBar({ account, onBack, onAccount, onSignOut, context, actions }: TopBarProps) {
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLDivElement>(null);

  // A click anywhere else closes the menu.
  useEffect(() => {
    if (!open) return;
    const close = (event: MouseEvent): void => {
      if (anchor.current !== null && !anchor.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, [open]);

  return (
    <header className="topbar">
      <div className="topbar-nav">
        {onBack !== null && (
          <button className="topbar-back" onClick={onBack} title="Back to projects">
            <BackIcon />
          </button>
        )}
        <Wordmark size={15} />
        {context !== undefined && <div className="topbar-context">{context}</div>}
      </div>

      <div className="topbar-end">
        {actions}

        {account !== null ? (
          <div className="menu-anchor" ref={anchor}>
            <button className="account-button" onClick={() => setOpen((was) => !was)}>
              <span className="avatar">{initials(account.name)}</span>
              <span>{account.name}</span>
              <span className="chev">
                <ChevronDownIcon />
              </span>
            </button>

            {open && (
              <div className="menu">
                <div className="menu-email">{account.email}</div>
                {onAccount !== null && (
                  <button
                    className="menu-item"
                    onClick={() => {
                      setOpen(false);
                      onAccount();
                    }}
                  >
                    <GearIcon />
                    Account settings
                  </button>
                )}
                <button
                  className="menu-item"
                  onClick={() => {
                    setOpen(false);
                    onSignOut();
                  }}
                >
                  <SignOutIcon />
                  Sign out
                </button>
              </div>
            )}
          </div>
        ) : (
          onAccount !== null && <span className="avatar skeleton" />
        )}
      </div>
    </header>
  );
}
