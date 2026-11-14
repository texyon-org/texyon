import { copyFileSync, existsSync } from "node:fs";

const from = new URL("../../../target/wasm-assets/", import.meta.url);
for (const name of ["texyon_wasm.wasm", "texyon-pack.tar.gz"]) {
  const source = new URL(name, from);
  if (!existsSync(source)) {
    console.error(`missing ${source.pathname}, run tools/wasm/build.sh and tools/wasm/build-assets.sh`);
    process.exit(1);
  }
  copyFileSync(source, new URL(`../dist/${name}`, import.meta.url));
}
