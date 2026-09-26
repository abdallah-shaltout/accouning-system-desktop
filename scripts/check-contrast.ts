// Guards every color pairing the design system promises against WCAG 2.x contrast minimums, reading
// the same hex values the app itself uses — parsed straight out of design-system.css's `:root`,
// `:root.dark`, `[data-base="…"]` and `[data-accent="…"]` blocks — so this can never drift from
// what's actually shipped (no separate hand-maintained color list to go stale). Wired into `bun run
// check` (18.C; this also closes doc 17 Phase E's task, which only checked the accent/on-primary pair).
//
// Checks, per WCAG 2.x:
//   - text/background pairs (secondary text, danger/success/warning as text) — 4.5:1 (1.4.3)
//   - control boundaries (border-control on background/surface) — 3:1 (1.4.11)
//   - focus ring boundary (border-control, the same token every focus ring is built on) — 3:1 (1.4.11)
//   - on-primary text (white/dark text on the accent color) — 4.5:1 (1.4.3)

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

function hex(body: string, varName: string): string | undefined {
  const re = new RegExp(`${varName}:\\s*(#[0-9a-fA-F]{6})`);
  return re.exec(body)?.[1];
}

// --- Parse base token values -------------------------------------------------------------------
// The light neutral defaults live in `@theme { ... }` (Tailwind's token block); the dark neutral
// overrides live in the unqualified `:root.dark { ... }` block (not the accent/base-qualified ones).
const THEME_BLOCK = /@theme\s*{([\s\S]*?)\n}/.exec(css)?.[1] ?? "";
const ROOT_DARK = /:root\.dark\s*{([^}]*)}/.exec(css)?.[1] ?? "";

interface BaseTokens {
  background: string;
  surface: string;
  surfaceHover: string;
  textSecondary: string;
  danger: string;
  success: string;
  warning: string;
  borderControl: string;
}

function readTokens(body: string, fallback: Partial<BaseTokens> = {}): BaseTokens {
  return {
    background: hex(body, "--color-background") ?? fallback.background!,
    surface: hex(body, "--color-surface") ?? fallback.surface!,
    surfaceHover: hex(body, "--color-surface-hover") ?? fallback.surfaceHover!,
    textSecondary: hex(body, "--color-text-secondary") ?? fallback.textSecondary!,
    danger: hex(body, "--color-danger") ?? fallback.danger!,
    success: hex(body, "--color-success") ?? fallback.success!,
    warning: hex(body, "--color-warning") ?? fallback.warning!,
    borderControl: hex(body, "--color-border-control") ?? fallback.borderControl!,
  };
}

const neutralLight = readTokens(THEME_BLOCK);
const neutralDark = readTokens(ROOT_DARK, neutralLight);

// `[data-base="X"]` blocks only override a subset of tokens (background/surface/text/border), so
// each base falls back to the shared neutral value (from :root / :root.dark) for whatever it
// doesn't redeclare — exactly how the app resolves CSS custom properties at runtime.
const BASE_BLOCK_PATTERN = /:root(\.dark)?\[data-base="([\w-]+)"\]\s*{([^}]*)}/g;
const bases: { name: string; mode: "light" | "dark"; tokens: BaseTokens }[] = [
  { name: "neutral", mode: "light", tokens: neutralLight },
  { name: "neutral", mode: "dark", tokens: neutralDark },
];
let baseMatch: RegExpExecArray | null;
while ((baseMatch = BASE_BLOCK_PATTERN.exec(css))) {
  const [, isDark, name, body] = baseMatch;
  const mode = isDark ? "dark" : "light";
  const fallback = mode === "dark" ? neutralDark : neutralLight;
  bases.push({ name, mode, tokens: readTokens(body, fallback) });
}

// --- Run checks ---------------------------------------------------------------------------------
const TEXT_MIN = 4.5;
const CONTROL_MIN = 3.0;

interface Finding {
  label: string;
  ratio: number;
  min: number;
}
const findings: Finding[] = [];

for (const { name, mode, tokens } of bases) {
  const surfaces: [string, string][] = [
    ["background", tokens.background],
    ["surface", tokens.surface],
    ["surface-hover", tokens.surfaceHover],
  ];
  for (const [surfaceName, surfaceHex] of surfaces) {
    findings.push({ label: `${name}/${mode}: secondary text on ${surfaceName}`, ratio: contrastRatio(tokens.textSecondary, surfaceHex), min: TEXT_MIN });
    findings.push({ label: `${name}/${mode}: danger text on ${surfaceName}`, ratio: contrastRatio(tokens.danger, surfaceHex), min: TEXT_MIN });
    findings.push({ label: `${name}/${mode}: success text on ${surfaceName}`, ratio: contrastRatio(tokens.success, surfaceHex), min: TEXT_MIN });
    findings.push({ label: `${name}/${mode}: warning text on ${surfaceName}`, ratio: contrastRatio(tokens.warning, surfaceHex), min: TEXT_MIN });
  }
  // Control boundary + focus ring both resolve to --color-border-control (design-system.css:
  // --color-input, --color-ring, .control's border/focus). Checked against background and surface,
  // since controls sit on both (plain pages vs. cards/wizard rail).
  findings.push({ label: `${name}/${mode}: control border on background`, ratio: contrastRatio(tokens.borderControl, tokens.background), min: CONTROL_MIN });
  findings.push({ label: `${name}/${mode}: control border on surface`, ratio: contrastRatio(tokens.borderControl, tokens.surface), min: CONTROL_MIN });
}

// Accent/on-primary — every `:root[data-accent="X"]` / `:root.dark[data-accent="X"]` block's
// --color-primary vs --color-on-primary (the text-on-accent pairing used by buttons/badges).
const ACCENT_BLOCK_PATTERN = /:root(\.dark)?\[data-accent="([\w-]+)"\]\s*{([^}]*)}/g;
let accentMatch: RegExpExecArray | null;
while ((accentMatch = ACCENT_BLOCK_PATTERN.exec(css))) {
  const [, isDark, accent, body] = accentMatch;
  const primary = hex(body, "--color-primary");
  const onPrimary = hex(body, "--color-on-primary");
  if (!primary || !onPrimary) continue;
  findings.push({ label: `accent ${accent}/${isDark ? "dark" : "light"}: on-primary text`, ratio: contrastRatio(primary, onPrimary), min: TEXT_MIN });
}

const failures = findings.filter((f) => f.ratio < f.min);

for (const f of findings) {
  const status = f.ratio >= f.min ? "OK  " : "FAIL";
  console.log(`  ${status}  ${f.label}: ${f.ratio.toFixed(2)}:1 (needs ${f.min}:1)`);
}

if (failures.length > 0) {
  console.error(`\ncheck-contrast: ${failures.length} of ${findings.length} pairing(s) below their WCAG minimum.`);
  process.exit(1);
}
console.log(`\ncheck-contrast: all ${findings.length} pairings clear their WCAG minimum.`);
