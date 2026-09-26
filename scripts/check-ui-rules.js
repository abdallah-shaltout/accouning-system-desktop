// UI-rules guard (docs/v2/17-ui-system-rtl-themes.md Phase F, F-0 — "Building pages" rules,
// docs/design_system.md → "Building pages"). Flags pages that don't yet assemble from the shared
// layout/blocks system: raw `<table>`, bare `<input>`/`<select>`/`<label>`, and direct shadcn
// structural-primitive imports inside `modules/*/pages/*.vue`, plus page files over 300 lines.
//
// **Warning mode only (F-0):** this always exits 0 and only logs findings. Doc 17's F-1..F-6 migrate
// the 100+ existing pages to `ListPage`/`FormPage`/`DetailPage`/`SettingsPage` + the new blocks in
// batches; F-6 is what flips this script to error mode (non-zero exit) once every page has moved and
// the findings below are at zero. Until then a nonzero count here is expected and not a regression —
// don't add *new* raw markup to a page you're touching, but don't chase the existing count to zero
// either; that's the migration's job, not this guard's.
//
// Allow-listed (kept on raw markup by design, per doc 17 "Out of scope for F"): print pages (their
// own printable HTML/Typst-adjacent markup), the POS screen (full-screen custom layout), and the
// template designer (a freeform canvas, not a form).

import { readFileSync, globSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");

const MAX_PAGE_LINES = 300;

// Substring match against the page's repo-relative path — covers every print page
// (`*PrintPage.vue`), POS (`PosPage.vue`, `ShiftReportPage.vue`'s Z-report print block reuses print
// styling) and the template designer (a freeform canvas, not a form).
const ALLOW_LIST_SUBSTRINGS = [
  "PrintPage.vue",
  "modules/invoices/pages/PosPage.vue",
  "modules/settings/pages/TemplateDesignerPage.vue",
];

function isAllowListed(relFile) {
  return ALLOW_LIST_SUBSTRINGS.some((s) => relFile.includes(s));
}

// Raw structural markup a page should get from a block instead (FormField for input/select/label,
// DataTable for table). Matched on the opening tag only, so `<input` also catches self-closing and
// attribute-bearing forms; a `<label` inside a shadcn/blocks component itself isn't scanned (only
// modules/*/pages/*.vue is).
const RAW_TAG_PATTERNS = [
  { tag: "<table", rule: "raw <table> — use DataTable" },
  { tag: "<input", rule: "bare <input> — use FormField + an App* control" },
  { tag: "<select", rule: "bare <select> — use FormField + AppSelect" },
  { tag: "<label", rule: "bare <label> — use FormField" },
];

// Structural shadcn imports a page shouldn't need once it's built from a layout + blocks (Building
// pages rule 1: "a page ... does not import shadcn primitives for structure"). Non-structural
// primitives (button, badge, tooltip, dropdown-menu, dialog, sheet, kbd, avatar...) are fine in a
// page and not flagged here.
const STRUCTURAL_SHADCN_PATTERN = /from\s*['"]@\/modules\/core\/components\/shadcn\/(table|field|tabs)['"]/g;

function countLines(content) {
  return content.split("\n").length;
}

const files = globSync("src/modules/*/pages/*.vue", { cwd: root }).map((f) => f.replaceAll("\\", "/"));

let findings = 0;
const byRule = new Map();

function report(relFile, line, message) {
  findings += 1;
  byRule.set(message, (byRule.get(message) ?? 0) + 1);
  console.log(`${relFile}:${line}: ${message}`);
}

for (const relFile of files) {
  if (isAllowListed(relFile)) continue;
  const file = path.join(root, relFile);
  const content = readFileSync(file, "utf8");
  const lines = content.split("\n");

  lines.forEach((line, i) => {
    for (const { tag, rule } of RAW_TAG_PATTERNS) {
      if (line.includes(tag)) report(relFile, i + 1, rule);
    }
  });

  let m;
  STRUCTURAL_SHADCN_PATTERN.lastIndex = 0;
  while ((m = STRUCTURAL_SHADCN_PATTERN.exec(content))) {
    report(relFile, countLines(content.slice(0, m.index)), `structural shadcn import (${m[1]}) — assemble from a layout + blocks instead`);
  }

  const lineCount = countLines(content);
  if (lineCount > MAX_PAGE_LINES) {
    report(relFile, 1, `page is ${lineCount} lines (> ${MAX_PAGE_LINES}) — move page-specific pieces into modules/<m>/components/`);
  }
}

console.log(
  `\ncheck-ui-rules: ${findings} finding(s) across ${files.length} page file(s) (warning mode — doc 17 Phase F-0; does not fail the build).`
);
if (findings > 0) {
  console.log("By rule:");
  for (const [rule, count] of byRule) console.log(`  ${count}\t${rule}`);
}

// F-0: warning mode only. F-6 flips this to `process.exit(findings > 0 ? 1 : 0)` once every page
// has migrated to the shared layouts/blocks and this count is zero.
process.exit(0);
