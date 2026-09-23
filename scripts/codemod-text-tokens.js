// One-off codemod: replaces Tailwind arbitrary-value text sizes (`text-[Npx]`) with the
// semantic text-size tokens defined in src/assets/styles/design-system.css.
//
// Usage: node scripts/codemod-text-tokens.js [--dry]
//
// Maps each `text-[Npx]` to the token whose token covers that exact px value (see
// docs/v2/14-platform.md §3 and design-system.css `@theme`). Any px value not in the map
// causes the script to abort loudly rather than silently drop/approximate a size, since an
// approximation would change rendering (--font-scale: 1 must reproduce prior pixels exactly).
//
// Can be deleted after the one-time migration; kept for now in case new hard-coded sizes
// need re-running through it during Phase 0 development.

import { readFileSync, writeFileSync } from "node:fs";
import { globSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");

// Exact px -> token class name. Every value found in the codebase must have an entry here;
// the script throws if it meets a px value that isn't mapped.
const PX_TO_TOKEN = {
  10: "text-caption",
  11: "text-tiny",
  12: "text-label",
  13: "text-body",
  14: "text-ui",
  15: "text-lead",
  17: "text-heading-sm",
  18: "text-heading",
  26: "text-stat",
};

const dryRun = process.argv.includes("--dry");

const files = globSync("src/**/*.{vue,ts}", { cwd: root }).map((f) => path.join(root, f));

const pattern = /text-\[(\d+)px\]/g;

let filesChanged = 0;
let totalReplacements = 0;
const unmapped = new Set();

for (const file of files) {
  const original = readFileSync(file, "utf8");
  let replacements = 0;

  const updated = original.replace(pattern, (match, pxStr) => {
    const px = Number(pxStr);
    const token = PX_TO_TOKEN[px];
    if (!token) {
      unmapped.add(px);
      return match; // leave untouched; reported below
    }
    replacements += 1;
    return token;
  });

  if (replacements > 0) {
    filesChanged += 1;
    totalReplacements += replacements;
    if (!dryRun) {
      writeFileSync(file, updated, "utf8");
    }
    console.log(`${replacements.toString().padStart(3)}  ${path.relative(root, file)}`);
  }
}

if (unmapped.size > 0) {
  console.error(`\nUnmapped px values found (no token defined): ${[...unmapped].join(", ")}`);
  process.exitCode = 1;
}

console.log(`\n${dryRun ? "[dry-run] " : ""}${filesChanged} files, ${totalReplacements} replacements`);
