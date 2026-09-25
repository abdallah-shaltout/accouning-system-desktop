// Guard against a new/edited accent preset silently dropping below the 4.5:1 white/dark-on-accent
// contrast target (docs/v2/17-ui-system-rtl-themes.md Phase E). Reads the same hex values the app
// itself uses — parsed straight out of design-system.css's `:root[data-accent="…"]` blocks — so this
// can never drift from what's actually shipped (no separate hand-maintained color list to go stale).

import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const cssPath = path.join(root, "src/assets/styles/design-system.css");
const css = readFileSync(cssPath, "utf8");

function hexToRgb(hex: string): [number, number, number] {
  const n = hex.replace("#", "");
  return [parseInt(n.slice(0, 2), 16), parseInt(n.slice(2, 4), 16), parseInt(n.slice(4, 6), 16)];
}

// WCAG relative luminance + contrast ratio (standard formulas, per-channel sRGB gamma correction).
function relativeLuminance([r, g, b]: [number, number, number]): number {
  const [R, G, B] = [r, g, b].map((c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * R + 0.7152 * G + 0.0722 * B;
}

function contrastRatio(hexA: string, hexB: string): number {
  const lA = relativeLuminance(hexToRgb(hexA));
  const lB = relativeLuminance(hexToRgb(hexB));
  const [lighter, darker] = lA > lB ? [lA, lB] : [lB, lA];
  return (lighter + 0.05) / (darker + 0.05);
}

// Pulls every `:root[data-accent="X"] { ... }` / `:root.dark[data-accent="X"] { ... }` block's
// --color-primary and --color-on-primary — the actual text-on-accent pairing used by buttons/badges.
const BLOCK_PATTERN = /:root(\.dark)?\[data-accent="([\w-]+)"\]\s*{([^}]*)}/g;

interface Finding {
  accent: string;
  mode: "light" | "dark";
  ratio: number;
}

const findings: Finding[] = [];
let match: RegExpExecArray | null;
while ((match = BLOCK_PATTERN.exec(css))) {
  const [, isDark, accent, body] = match;
  const primary = /--color-primary:\s*(#[0-9a-fA-F]{6})/.exec(body)?.[1];
  const onPrimary = /--color-on-primary:\s*(#[0-9a-fA-F]{6})/.exec(body)?.[1];
  if (!primary || !onPrimary) continue;
  findings.push({ accent, mode: isDark ? "dark" : "light", ratio: contrastRatio(primary, onPrimary) });
}

const MIN_RATIO = 4.5;
const failures = findings.filter((f) => f.ratio < MIN_RATIO);

for (const f of findings) {
  const status = f.ratio >= MIN_RATIO ? "OK  " : "FAIL";
  console.log(`  ${status}  ${f.accent} (${f.mode}): ${f.ratio.toFixed(2)}:1`);
}

if (failures.length > 0) {
  console.error(`\ncheck-contrast: ${failures.length} accent/mode combination(s) below ${MIN_RATIO}:1 white/dark-on-accent contrast.`);
  process.exit(1);
}
console.log(`\ncheck-contrast: all ${findings.length} accent/mode combinations clear ${MIN_RATIO}:1.`);
