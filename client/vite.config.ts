import { fileURLToPath } from "node:url";

import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],

  resolve: {
    alias: {
      // The engine is a sibling package in this repository rather than
      // something installed, so it resolves to its build output directly.
      "@tectonic/wasm": fileURLToPath(new URL("../js/dist/index.js", import.meta.url)),
    },
  },

  // The engine's Worker is an ES module — it imports its WASI shim — so it has
  // to be emitted as one. Vite's default is an IIFE, which cannot.
  worker: { format: "es" },

  // Top-level await, in the Worker's own startup.
  build: { target: "es2022" },

  // The engine's Worker fetches the wasm module and the support pack by URL at
  // runtime; neither is something to bundle.
  optimizeDeps: {
    exclude: ["@tectonic/wasm"],
  },

  server: {
    proxy: {
      // The API and Fortify's session endpoints belong to the server. In
      // production Caddy routes these; in development they are proxied so the
      // session cookie is same-origin either way.
      "/api": { target: "http://localhost:8080", changeOrigin: false },
      "/login": { target: "http://localhost:8080", changeOrigin: false },
      "/logout": { target: "http://localhost:8080", changeOrigin: false },
      "/register": { target: "http://localhost:8080", changeOrigin: false },
      "/sanctum": { target: "http://localhost:8080", changeOrigin: false },
    },
  },
});
