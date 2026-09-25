// Guard against physical-side Tailwind utilities and raw directional icon imports creeping back
// into src/ — see docs/v2/17-ui-system-rtl-themes.md Phase A. Physical classes (ml-/mr-/pl-/pr-/
// left-/right-/border-l-/border-r-/rounded-l-/rounded-r-/text-left/text-right) break layout under
// RTL because they don't flip with `dir`; use the logical Tailwind utilities instead (ms-/me-/
// ps-/pe-/start-/end-/border-s-/border-e-/rounded-s-/rounded-e-/text-start/text-end). Likewise,
// importing ChevronLeft/ChevronRight/ArrowLeft/ArrowRight from @lucide/vue directly hard-codes a
// screen direction; use `dirIcon` + `DirIcon.vue` (modules/core/helpers/dirIcon.ts) for anything
// meaning back/forward/prev/next/open, so it mirrors under RTL.
//
// A finding can be allow-listed by adding `/* rtl-ok: <reason> */` right before the offending
// class inside the string (or anywhere on the line for an icon-import finding) — for cases driven
// by an explicit physical `side` prop (Floating UI sides, Sheet's side="left|right", Sidebar's side
// branches), a centered dialog's `left-[50%]`, or a genuinely physical/numeric icon (trend arrows,
// transfer from→to, sort direction, undo/redo).

import { readFileSync, globSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");

// Class-bearing spans to scan: class="..." / :class="..." attributes (single-line, since Vue
// template attributes can't span a `"..."` across lines) and cn(...) call bodies, which commonly
// span multiple lines in this codebase's shadcn components — so that span is matched non-greedily
// across the whole file rather than line by line.
const CLASS_ATTR_PATTERN = /:?class(?:List)?\s*=\s*"([^"]*)"/g;
const PHYSICAL_UTILITY_PATTERN = /\b(?:ml|mr|pl|pr)-(?!auto\b)[\w[\]./%-]+|\bborder-[lr]\b|\bborder-[lr]-[\w[\]./-]+|\brounded-[lr]\b|\brounded-[lr]-[\w-]+|\btext-left\b|\btext-right\b|(?<![\w-])(?:left|right)-[\w[\]./%-]+/g;

// Extracts the full parenthesized body of every `cn(...)` call, matching nested parens so a
// multi-line class list (this codebase's shadcn components) is captured as one span.
function findCnCallBodies(content) {
  const spans = [];
  const callRe = /\bcn\(/g;
  let m;
  while ((m = callRe.exec(content))) {
    const start = m.index + m[0].length;
    let depth = 1;
    let i = start;
    while (i < content.length && depth > 0) {
      if (content[i] === "(") depth++;
      else if (content[i] === ")") depth--;
      i++;
    }
    if (depth === 0) spans.push({ text: content.slice(start, i - 1), start });
  }
  return spans;
}
const ICON_IMPORT_PATTERN = /import\s*\{[^}]*\b(ChevronLeft|ChevronRight|ArrowLeft|ArrowRight)\b[^}]*\}\s*from\s*["']@lucide\/vue["']/g;

// shadcn primitives that legitimately branch on an explicit physical `side` prop (Floating UI
// sides, Sheet's side="left|right", Sidebar's side branches) — Phase A left these alone by design.
const SIDE_PROP_EXEMPT = new Set([
  "src/modules/core/components/shadcn/sidebar/SidebarRail.vue",
  "src/modules/core/components/shadcn/sidebar/Sidebar.vue",
  "src/modules/core/components/shadcn/sheet/SheetContent.vue",
]);

// shadcn primitives (and the sidebar's own accordion chevron, which follows the exact same
// disclosure-triangle pattern as DropdownMenuSubTrigger below) that render the base library's own
// directional icon and mirror it themselves with `rtl:rotate-180`/`rtl:-scale-x-100` (see the
// component) rather than going through DirIcon — pages import from dirIcon.ts instead, but these
// are the legitimate direct-import sites.
const ICON_PRIMITIVE_EXEMPT = new Set([
  "src/modules/core/components/shadcn/range-calendar/RangeCalendarNextButton.vue",
  "src/modules/core/components/shadcn/range-calendar/RangeCalendarPrevButton.vue",
  "src/modules/core/components/shadcn/calendar/CalendarNextButton.vue",
  "src/modules/core/components/shadcn/calendar/CalendarPrevButton.vue",
  "src/modules/core/components/shadcn/breadcrumb/BreadcrumbSeparator.vue",
  "src/modules/core/components/shadcn/dropdown-menu/DropdownMenuSubTrigger.vue",
  "src/modules/core/components/layout/NavMain.vue",
  "src/modules/core/helpers/dirIcon.ts",
]);

function lineOf(content, index) {
  let line = 1;
  for (let i = 0; i < index; i++) if (content.charCodeAt(i) === 10) line++;
  return line;
}

// Finds physical-utility hits inside a class-bearing span, honoring an `rtl-ok:` comment that sits
// anywhere within that same span (not just the exact line) since cn(...) bodies are multi-line.
function findPhysicalClasses(content) {
  const found = [];
  const spans = [];

  let m;
  CLASS_ATTR_PATTERN.lastIndex = 0;
  while ((m = CLASS_ATTR_PATTERN.exec(content))) {
    spans.push({ text: m[1], start: m.index });
  }
  spans.push(...findCnCallBodies(content));

  for (const span of spans) {
    if (span.text.includes("rtl-ok:")) continue;
    const utilityMatches = span.text.match(PHYSICAL_UTILITY_PATTERN);
    if (utilityMatches) {
      for (const u of utilityMatches) found.push({ text: u, line: lineOf(content, span.start) });
    }
  }
  return found;
}

const files = globSync("src/**/*.{vue,ts,tsx,js}", { cwd: root }).map((f) => f.replaceAll("\\", "/"));

let hits = 0;

for (const relFile of files) {
  const file = path.join(root, relFile);
  const content = readFileSync(file, "utf8");

  const sideExempt = SIDE_PROP_EXEMPT.has(relFile);
  const iconExempt = ICON_PRIMITIVE_EXEMPT.has(relFile);

  if (!sideExempt) {
    for (const { text, line } of findPhysicalClasses(content)) {
      hits += 1;
      console.error(`${relFile}:${line}: physical class: ${text}`);
    }
  }

  if (!iconExempt) {
    const lines = content.split("\n");
    lines.forEach((line, i) => {
      if (line.includes("rtl-ok:")) return;
      const iconMatches = line.match(ICON_IMPORT_PATTERN);
      if (iconMatches) {
        hits += iconMatches.length;
        console.error(`${relFile}:${i + 1}: direct directional icon import — use dirIcon.ts / DirIcon.vue instead`);
      }
    });
  }
}

if (hits > 0) {
  console.error(
    `\nFound ${hits} RTL guard violation(s). Use logical Tailwind utilities (ms-/me-/ps-/pe-/start-/end-/border-s-/border-e-/rounded-s-/rounded-e-/text-start/text-end) and dirIcon.ts/DirIcon.vue, or mark a legitimate physical case with an "rtl-ok: <reason>" comment in the same class string / cn(...) call.`
  );
  process.exit(1);
}

console.log("check-rtl: no physical-side classes or raw directional icon imports found.");
