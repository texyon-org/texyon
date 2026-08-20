import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useCallback } from "react";

import { logout } from "../api/auth";

/** Sign out and land on the sign-in screen with an empty cache. */
export function useSignOut(): () => void {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  return useCallback(() => {
    void (async () => {
      try {
        await logout();
      } catch {
        // Failing still means leaving.
      }
      queryClient.clear();
      void navigate({ to: "/sign-in" });
    })();
  }, [navigate, queryClient]);
}
