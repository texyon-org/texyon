import { defineConfig } from "orval";

// Input comes from `composer openapi` in server/. Regenerate with
// `npm run generate:api`. CI fails if the committed output drifts.
export default defineConfig({
  editor: {
    input: "../server/openapi.json",
    output: {
      target: "src/api/generated.ts",
      schemas: "src/api/model",
      client: "react-query",
      httpClient: "fetch",
      mode: "split",
      override: {
        mutator: {
          path: "src/api/http.ts",
          name: "apiFetch",
        },
        // The mutator returns the parsed body, not a {status, data} wrapper.
        fetch: {
          includeHttpResponseReturnType: false,
        },
      },
    },
  },
});
