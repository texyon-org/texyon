import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";

import { SignIn } from "./SignIn";

export function SignInScreen({ notice = null }: { notice?: string | null }) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  return (
    <SignIn
      notice={notice}
      onSignedIn={() => {
        // Cached data belongs to whoever was signed in before.
        queryClient.clear();
        void navigate({ to: "/" });
      }}
      onSample={() => void navigate({ to: "/sample" })}
    />
  );
}
