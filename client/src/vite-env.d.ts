/// <reference types="vite/client" />

interface ImportMetaEnv {
  /**
   * Which bundle the engine reads its long tail from, as a same-origin path.
   * Unset, the pinned TeX Live 2022 tar; a deployment that ships its own
   * bundle sets this at build time (see the Dockerfile's build arg).
   */
  readonly VITE_BUNDLE_URL?: string;
}
