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
      // Not tsconfig.json: its allowImportingTsExtensions would put .ts on every generated import.
      tsconfig: { compilerOptions: { target: "ES2022", verbatimModuleSyntax: true } },
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
