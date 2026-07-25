import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      // The built package rather than its sources: the library starts its
      // Worker with `new URL("./worker.js", import.meta.url)`, which only
      // resolves next to an emitted worker.js. `npm run dev` builds it first.
      "@tectonic/wasm": fileURLToPath(new URL("../js/dist/index.js", import.meta.url)),
    },
  },
  worker: { format: "es" },
  server: {
    // The engine module is 5 MB and the format dump 24 MB; without these the
    // browser re-fetches them on every reload.
    headers: { "Cache-Control": "public, max-age=3600" },
    // The library lives in ../js, one level above the demo's root.
    fs: { allow: [fileURLToPath(new URL("..", import.meta.url))] },
    // The Tectonic bundle host serves byte ranges but no CORS headers, so a
    // browser cannot read it directly. Proxying it is what lets the demo reach
    // every package in TeX Live; a real deployment wants its own mirror.
    proxy: {
      "/bundle": {
        target: "https://data1.fullyjustified.net",
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/bundle/, ""),
      },
    },
  },
  build: { target: "es2022" },
  // .wasm and .fmt are served from public/ untouched.
  assetsInclude: ["**/*.fmt"],
});
