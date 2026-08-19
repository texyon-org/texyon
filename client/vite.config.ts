import { fileURLToPath } from "node:url";

import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

/**
 * Where the support bundle actually lives: this project's own TeX Live 2026
 * ttb. Pinned to an exact object so a dev server does not silently change
 * bundles underneath a project. Override with BUNDLE_HOST to develop against
 * another bundle server — pair it with VITE_BUNDLE_URL naming the file the
 * engine should ask that host for (the 2022 tar era of this default was
 * `https://data1.fullyjustified.net` + `/bundle/tlextras-2022.0r0.tar`).
 */
const BUNDLE_HOST =
  process.env.BUNDLE_HOST ??
  "https://s3.de-1.yendric.be/uploads/01a0117b-2294-7203-bb52-edb7070f93fb";

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
    // The engine package lives one level up in this repository rather than in
    // node_modules, and its Worker is fetched by URL — so the dev server has to
    // be willing to serve from outside the client's own root.
    fs: { allow: [fileURLToPath(new URL("..", import.meta.url))] },

    proxy: {
      // The support bundle — every package, font and graphic in TeX Live that
      // is not in the local pack. The engine asks for byte ranges of it, so it
      // has to be same-origin: the host serves Range requests and an index
      // sidecar but no Access-Control-Allow-Origin, which makes fetching it
      // cross-origin impossible rather than merely slow.
      //
      // Without this, every \usepackage outside the 423-file pack fails with
      // "File `x.sty' not found" and the only clue is one line in the log
      // saying the bundle index could not be loaded.
      "/bundle": {
        target: BUNDLE_HOST,
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/bundle/, ""),
      },

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
