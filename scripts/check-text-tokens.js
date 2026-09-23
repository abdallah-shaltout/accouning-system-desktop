// Guard against new hard-coded Tailwind arbitrary text sizes creeping back in.
// Fails (non-zero exit) if any `text-[Npx]` appears anywhere under src/.
// Use the semantic tokens in src/assets/styles/design-system.css instead
// (text-caption, text-tiny, text-label, text-body, text-ui, text-lead,
// text-heading-sm, text-heading, text-stat) — see docs/v2/14-platform.md §3.

import { readFileSync, globSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");

const pattern = /text-\[\d+px\]/g;
const files = globSync("src/**/*.{vue,ts,tsx,js}", { cwd: root }).map((f) => path.join(root, f));

let hits = 0;

for (const file of files) {
  const content = readFileSync(file, "utf8");
  const lines = content.split("\n");
  lines.forEach((line, i) => {
    const matches = line.match(pattern);
    if (matches) {
      hits += matches.length;
      console.error(`${path.relative(root, file)}:${i + 1}: ${matches.join(", ")}`);
    }
  });
}

if (hits > 0) {
  console.error(
    `\nFound ${hits} hard-coded text-[Npx] usage(s). Use the text-size tokens in design-system.css instead.`
  );
  process.exit(1);
}

console.log("check-text-tokens: no hard-coded text-[Npx] found.");
