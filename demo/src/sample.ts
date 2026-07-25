// The document the editor starts with.
//
// It lives as a real .tex file so that tools/wasm/build-assets.sh can compile it
// natively and guarantee every font and package it needs is in the support pack
// the demo ships. Keep them in sync by editing the .tex, never this file.
import source from "../sample.tex?raw";

/** What the editor starts with. */
export const SAMPLE: string = source;
