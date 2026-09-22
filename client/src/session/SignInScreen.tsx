import { useQueryClient } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";

import { SignIn } from "./SignIn";

export function SignInScreen({ notice = null }: { notice?: string | null }) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  // Not every screen that renders this is the sign-in route.
  const { back } = useSearch({ strict: false });

  return (
    <SignIn
      notice={notice}
      onSignedIn={() => {
        // Cached data belongs to whoever was signed in before.
        queryClient.clear();
        void navigate({ to: back ?? "/" });
      }}
      onSample={() => void navigate({ to: "/sample" })}
    />
  );
}
