// Guard against path-string navigation creeping back into src/ — see plan 20 (named route objects).
// A path string given to router.push/replace, RouterLink/AppButton/KpiCard `:to`, PageHeader
// `:back`, a route-record `redirect`, or built via a helper/ternary, is not checked by anything: a
// typo or a renamed route only fails at run time as a silent not-found page. Use a named route
// object instead: `{ name, params?, query?, hash? }` (CLAUDE.md rule 25).
//
// A finding can be allow-listed by adding `/* route-ok: <reason> */` on the same line — for a
// genuine URL round-trip such as the login `?redirect=` fullPath. An empty reason still counts as
// a finding.

import { readFileSync, globSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");

const ROUTE_FILE_GLOB = "src/modules/*/routes{.ts,/*.ts}";

/** First path segment of every route record — the list this guard flags literals against. Grows
 *  by itself as routes are added, so it never needs manual upkeep. */
function collectRouteSegments() {
  const segments = new Set();
  const files = globSync(ROUTE_FILE_GLOB, { cwd: root });
  const PATH_RE = /path:\s*["']([^"']*)["']/g;
  for (const relFile of files) {
    const content = readFileSync(path.join(root, relFile), "utf8");
    let m;
    while ((m = PATH_RE.exec(content))) {
      const p = m[1];
      const seg = p.replace(/^\//, "").split("/")[0];
      if (seg && !seg.startsWith(":")) segments.add(seg);
    }
  }
  return segments;
}

function lineOf(content, index) {
  let line = 1;
  for (let i = 0; i < index; i++) if (content.charCodeAt(i) === 10) line++;
  return line;
}

function isRouteOk(content, index) {
  const lineEnd = content.indexOf("\n", index);
  const line = content.slice(content.lastIndexOf("\n", index) + 1, lineEnd === -1 ? undefined : lineEnd);
  return line.includes("route-ok:");
}

const routeSegments = collectRouteSegments();
const segmentPattern = [...routeSegments].map((s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|");

// Rule 1: any string literal (quote or template) whose text starts with `/<segment>` where
// <segment> is a real route's first path segment. Catches ternaries, helpers, config arrays and
// template attribute values alike, since it matches the literal text itself rather than a call-site
// shape (`return d.existingId.startsWith('sup') ? \`/suppliers/${id}\` : ...` is still caught).
const PATH_LITERAL_RE = new RegExp(`([\`'"])(\\/(?:${segmentPattern})(?:\\/[^\`'"]*)?)\\1`, "g");

// Rule 2: home / bare-path targets and `{ path: ... }` used as a navigation target.
const HOME_TARGET_RE = /\bto=["']\/["']|:to=["']'\/'["']|\.(?:push|replace)\(\s*["']\/["']\s*\)|\breturn\s*["']\/["']/g;
const PATH_OBJECT_RE = /\{\s*path:\s*["']/g;

const ROUTE_RECORD_FILE = /^src\/modules\/[^/]+\/routes(\.ts|\/[^/]+\.ts)$/;

const files = globSync("src/modules/**/*.{vue,ts}", { cwd: root })
  .map((f) => f.replaceAll("\\", "/"))
  .filter((f) => !f.endsWith(".gen.d.ts"));

let hits = 0;
const byFile = new Map();

function report(relFile, line, message) {
  hits += 1;
  if (!byFile.has(relFile)) byFile.set(relFile, []);
  byFile.get(relFile).push(`${relFile}:${line}: ${message}`);
}

for (const relFile of files) {
  const content = readFileSync(path.join(root, relFile), "utf8");
  const isRouteRecordFile = ROUTE_RECORD_FILE.test(relFile);

  if (!isRouteRecordFile) {
    PATH_LITERAL_RE.lastIndex = 0;
    let m;
    while ((m = PATH_LITERAL_RE.exec(content))) {
      if (isRouteOk(content, m.index)) continue;
      report(relFile, lineOf(content, m.index), `path-string navigation target: ${m[2]}`);
    }

    HOME_TARGET_RE.lastIndex = 0;
    while ((m = HOME_TARGET_RE.exec(content))) {
      if (isRouteOk(content, m.index)) continue;
      report(relFile, lineOf(content, m.index), `bare "/" navigation target — use { name: 'home' }`);
    }

    PATH_OBJECT_RE.lastIndex = 0;
    while ((m = PATH_OBJECT_RE.exec(content))) {
      if (isRouteOk(content, m.index)) continue;
      report(relFile, lineOf(content, m.index), `{ path: ... } navigation target — use { name: ... }`);
    }
  } else {
    // Route-record files: only a `redirect:` value is checked, never the record's own `path:`.
    const redirectRe = /redirect:\s*["']([^"']*)["']/g;
    let m;
    while ((m = redirectRe.exec(content))) {
      if (isRouteOk(content, m.index)) continue;
      report(relFile, lineOf(content, m.index), `redirect path string: ${m[1]} — use { name: ... }`);
    }
  }
}

for (const relFile of [...byFile.keys()].sort()) {
  for (const line of byFile.get(relFile)) console.error(line);
}

if (hits > 0) {
  console.error(
    `\nFound ${hits} route-object guard violation(s). Navigate with a named route object ` +
      `({ name, params?, query?, hash? }), never a path string or { path }. Mark a genuine URL ` +
      `round-trip with "route-ok: <reason>" on the same line.`,
  );
  process.exit(1);
}

console.log("check-routes: no path-string navigation targets found.");
