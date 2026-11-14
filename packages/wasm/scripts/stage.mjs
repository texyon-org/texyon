import { copyFileSync, existsSync, mkdirSync } from "node:fs";

const repo = new URL("../../../", import.meta.url);
const here = new URL("../", import.meta.url);

const copies = [
  ["target/wasm-assets/texyon_wasm.wasm", "dist/texyon_wasm.wasm"],
  ["target/wasm-assets/texyon-pack.tar.gz", "dist/texyon-pack.tar.gz"],
  ["LICENSE", "LICENSE"],
  ["THIRD_PARTY_NOTICES.md", "THIRD_PARTY_NOTICES.md"],
  ["licenses/AGPL-3.0.txt", "licenses/AGPL-3.0.txt"],
  ["licenses/GPL-2.0.txt", "licenses/GPL-2.0.txt"],
  ["licenses/LGPL-2.1.txt", "licenses/LGPL-2.1.txt"],
];

mkdirSync(new URL("licenses/", here), { recursive: true });
for (const [from, to] of copies) {
  const source = new URL(from, repo);
  if (!existsSync(source)) {
    console.error(`missing ${source.pathname}, run tools/wasm/build.sh and tools/wasm/build-assets.sh`);
    process.exit(1);
  }
  copyFileSync(source, new URL(to, here));
}
