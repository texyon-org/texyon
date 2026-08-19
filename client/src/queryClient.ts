import { QueryClient } from "@tanstack/react-query";

export const queryClient = new QueryClient({
  defaultOptions: {
    // Long enough that a loader and the hook mounting behind it share one
    // request, short enough that returning to a screen asks the server again.
    queries: { retry: false, staleTime: 10_000 },
    mutations: { retry: false },
  },
});
