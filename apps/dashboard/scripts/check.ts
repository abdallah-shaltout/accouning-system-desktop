/**
 * Structure guards for `apps/dashboard` (CLAUDE.md "Definition of done": `bun run check`).
 * Pragmatic regex-based checks, not a full AST linter — modeled in spirit on the desktop app's
 * `scripts/check-routes.js` (same "route-ok:" escape-hatch convention), but this project's own logic:
 *
 *   1. Cross-module `pages/`/`components/` imports — a module may only import another module's
 *      `services/`, `schemas/` or `types/` (docs/03-architecture.md "Module rules").
 *   2. Path-string navigation — `router.push('/...')`, `:to="'/...'"`, `{ path: '/...' }` instead of a
 *      named route object (CLAUDE.md "Navigation": "Never a path string").
 *   3. Raw hex colors / `text-[Npx]` / `rounded-[...]` arbitrary Tailwind values in `.vue` files
 *      (CLAUDE.md "UI": "No hex colors, no text-[Npx], no rounded-[...]").
 *
 * A finding can be allow-listed with `/* check-ok: <reason> *\/` on the same line.
 */
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { globSync } from "node:fs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");

let hits = 0;
const findings: string[] = [];

function report(relFile: string, line: number, message: string): void {
  hits += 1;
  findings.push(`${relFile}:${line}: ${message}`);
}

function lineOf(content: string, index: number): number {
  let line = 1;
  for (let i = 0; i < index; i++) if (content.charCodeAt(i) === 10) line++;
  return line;
}

/** Blank out comments (preserving line breaks) so prose mentioning paths/colors isn't flagged. */
function stripComments(content: string): string {
  return content
    .replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, " "))
    .replace(/\/\/[^\n]*/g, (m) => " ".repeat(m.length));
}

function isCheckOk(original: string, index: number): boolean {
  const lineEnd = original.indexOf("\n", index);
  const line = original.slice(original.lastIndexOf("\n", index) + 1, lineEnd === -1 ? undefined : lineEnd);
  return line.includes("check-ok:") || line.includes("route-ok:");
}

const moduleFiles = globSync("src/modules/**/*.{vue,ts}", { cwd: root }).map((f) => f.replaceAll("\\", "/"));

// ─── 1. Cross-module pages/components imports ──────────────────────────────────────────────────
const MODULE_IMPORT_RE = /from\s+["'](?:@\/modules|\.\.?\/(?:\.\.\/)*modules)\/([^/"']+)\/(pages|components)\//g;

for (const relFile of moduleFiles) {
  const currentModuleMatch = /^src\/modules\/([^/]+)\//.exec(relFile);
  const currentModule = currentModuleMatch?.[1];
  const original = readFileSync(path.join(root, relFile), "utf8");
  const content = stripComments(original);

  MODULE_IMPORT_RE.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = MODULE_IMPORT_RE.exec(content))) {
    const importedModule = m[1];
    const importedLayer = m[2];
    if (importedModule === currentModule) continue; // same-module pages/components import is fine
    if (isCheckOk(original, m.index)) continue;
    report(
      relFile,
      lineOf(content, m.index),
      `cross-module import of "${importedModule}/${importedLayer}" — a module may only import ` +
        `another module's services/, schemas/ or types/ (docs/03-architecture.md)`,
    );
  }
}

// ─── 2. Path-string navigation ──────────────────────────────────────────────────────────────────
// Any `router.push/replace('/...')`, `:to="'/...'"`, `to="/..."`, or `{ path: '/...' }` — named route
// objects only (CLAUDE.md rule 25 equivalent for this project).
const PUSH_REPLACE_STRING_RE = /\.(?:push|replace)\(\s*(["'`])(\/[^"'`]*)\1/g;
const TO_ATTR_STRING_RE = /:to=["'](["'`])(\/[^"'`]*)\1["']|\bto=["'](\/[^"']*)["']/g;
const PATH_OBJECT_RE = /\{\s*path:\s*["'`]/g;

const allSrcFiles = globSync("src/**/*.{vue,ts}", { cwd: root })
  .map((f) => f.replaceAll("\\", "/"))
  .filter((f) => !f.endsWith(".d.ts"));

for (const relFile of allSrcFiles) {
  const original = readFileSync(path.join(root, relFile), "utf8");
  const content = stripComments(original);

  PUSH_REPLACE_STRING_RE.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = PUSH_REPLACE_STRING_RE.exec(content))) {
    if (isCheckOk(original, m.index)) continue;
    report(relFile, lineOf(content, m.index), `path-string navigation: ${m[2]} — use a named route object`);
  }

  TO_ATTR_STRING_RE.lastIndex = 0;
  while ((m = TO_ATTR_STRING_RE.exec(content))) {
    if (isCheckOk(original, m.index)) continue;
    const target = m[2] ?? m[3];
    report(relFile, lineOf(content, m.index), `path-string ":to"/"to": ${target} — use a named route object`);
  }

  // Route-record files declare their own `path:` legitimately — skip those.
  if (!/^src\/modules\/[^/]+\/routes(\.ts|\/[^/]+\.ts)$/.test(relFile)) {
    PATH_OBJECT_RE.lastIndex = 0;
    while ((m = PATH_OBJECT_RE.exec(content))) {
      if (isCheckOk(original, m.index)) continue;
      report(relFile, lineOf(content, m.index), `{ path: ... } navigation target — use { name: ... }`);
    }
  }
}

// ─── 3. Raw hex colors / arbitrary text-size / rounded values in .vue files ────────────────────
const HEX_COLOR_RE = /#[0-9a-fA-F]{3,8}\b/g;
const TEXT_PX_RE = /\btext-\[[^\]]+\]/g;
const ROUNDED_ARBITRARY_RE = /\brounded(?:-[a-z]+)?-\[[^\]]+\]/g;

const vueFiles = globSync("src/**/*.vue", { cwd: root }).map((f) => f.replaceAll("\\", "/"));

for (const relFile of vueFiles) {
  const original = readFileSync(path.join(root, relFile), "utf8");
  const content = stripComments(original);

  for (const [re, label] of [
    [HEX_COLOR_RE, "raw hex color"],
    [TEXT_PX_RE, "arbitrary text-[...] size"],
    [ROUNDED_ARBITRARY_RE, "arbitrary rounded-[...] value"],
  ] as const) {
    re.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = re.exec(content))) {
      if (isCheckOk(original, m.index)) continue;
      report(relFile, lineOf(content, m.index), `${label}: ${m[0]} — use a design token`);
    }
  }
}

for (const finding of findings.sort()) console.error(finding);

if (hits > 0) {
  console.error(
    `\nFound ${hits} guard violation(s). See docs/03-architecture.md (module boundaries), ` +
      `CLAUDE.md "Navigation" (named routes) and "UI" (tokens only). Mark a deliberate exception with ` +
      `"check-ok: <reason>" on the same line.`,
  );
  process.exit(1);
}

console.log("check: no module-boundary, path-string navigation or raw-token violations found.");
