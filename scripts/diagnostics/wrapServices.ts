/**
 * One-time codemod (18.B2): wraps every top-level `export (async )?function` in a `*Service.ts`
 * file with `wrap()` (from `defineService.ts`), so each call is timed and its failures logged,
 * with the exact same public API (same function names, same signatures) — pages that
 * `import { x } from '...Service'` need no changes.
 *
 * Transform, per function:
 *
 *   export function foo(a: string): Bar { ... }
 *   ->
 *   export const foo = wrap('mod.foo', function foo(a: string): Bar { ... });
 *
 * A *named* function expression (`function foo(...)`, not anonymous) is used deliberately: any
 * call to `foo(...)` elsewhere in the same file — including recursive/sibling calls between
 * service functions, e.g. `checkDuplicates` calling `findDuplicates` — keeps resolving through the
 * function expression's own name binding, so no call site anywhere in the file needs to change.
 * This is why a brace-counting scan is used to find each function's exact body span, rather than a
 * single-line regex substitution: the `export function` keywords need replacing at the start AND
 * the matching closing brace needs `);` appended, with nothing in between touched.
 *
 *   bun run scripts/diagnostics/wrapServices.ts [--dry]
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { relative } from 'node:path';
import { Glob } from 'bun';

const DRY = process.argv.includes('--dry');

// The diagnostics module's own services are the logging plumbing itself — wrapping them would be
// self-referential (wrap() calling into logService calling into wrap()).
const FILES = [...new Glob('src/modules/*/services/*.ts').scanSync({ cwd: process.cwd() })]
  .map((f) => f.replaceAll('\\', '/'))
  .filter((f) => !f.startsWith('src/modules/diagnostics/'))
  .sort();

function moduleNameOf(file: string): string {
  const m = file.match(/^src\/modules\/([^/]+)\//);
  return m ? m[1] : 'core';
}

const EXPORT_FN_START = /^export\s+(async\s+)?function\s+(\w+)\s*(<[^(]*>)?\s*\(/gm;

/** From the `(` right after a function's name/generics, scans forward to find: the matching `)`
 * that closes the parameter list (handling nested parens in default values/type generics), then
 * the return-type annotation up to the opening `{`, then the matching `}` that closes the body
 * (handling nested braces/strings/template-literals/comments/regex so a `}` inside a string
 * literal isn't mistaken for the function's own closing brace). Returns the body's end index
 * (the position right after the closing `}`), or -1 if the scan runs off the end of the file. */
function findFunctionEnd(src: string, parenOpenIdx: number): number {
  let i = parenOpenIdx;
  let depth = 0;
  // 1) parameter list
  for (; i < src.length; i++) {
    if (src[i] === '(') depth++;
    else if (src[i] === ')') {
      depth--;
      if (depth === 0) {
        i++;
        break;
      }
    }
  }
  // 2) return type / whitespace up to the opening brace. A return type can itself contain braces
  // (`Promise<X & { a: string }>`) or angle brackets (`Promise<Map<string, X>>`) — track both so
  // the body's own `{` (only reachable at angle-bracket depth 0) isn't confused with one nested
  // inside the return-type annotation.
  let angleDepth = 0;
  for (; i < src.length; i++) {
    const c = src[i];
    if (c === '<') angleDepth++;
    else if (c === '>') angleDepth = Math.max(0, angleDepth - 1);
    else if (c === '{' && angleDepth === 0) break;
  }
  if (src[i] !== '{') return -1;
  // 3) body, brace-counting with string/template/comment awareness
  depth = 0;
  for (; i < src.length; i++) {
    const c = src[i];
    if (c === '{') depth++;
    else if (c === '}') {
      depth--;
      if (depth === 0) return i + 1;
    } else if (c === '"' || c === "'" || c === '`') {
      const quote = c;
      i++;
      while (i < src.length && src[i] !== quote) {
        if (src[i] === '\\') i++;
        i++;
      }
    } else if (c === '/' && src[i + 1] === '/') {
      while (i < src.length && src[i] !== '\n') i++;
    } else if (c === '/' && src[i + 1] === '*') {
      i += 2;
      while (i < src.length && !(src[i] === '*' && src[i + 1] === '/')) i++;
      i++;
    }
  }
  return -1;
}

// Files that intentionally have no service functions to wrap (types/pure helpers) — skipped
// rather than producing an empty `defineService({})`.
let changed = 0;
let skipped = 0;

for (const file of FILES) {
  const src = readFileSync(file, 'utf8');
  if (src.includes("from '@/modules/diagnostics/services/defineService'")) {
    skipped++;
    continue; // already wrapped
  }

  const moduleName = moduleNameOf(file);

  // Collect every top-level `export function` match with its full span, back to front, so
  // replacing later matches first never invalidates earlier matches' indices.
  const matches: { start: number; nameStart: number; name: string; parenIdx: number; asyncKw: string }[] = [];
  for (const m of src.matchAll(EXPORT_FN_START)) {
    const asyncKw = m[1] ? 'async ' : '';
    const name = m[2];
    const nameStart = m.index! + m[0].indexOf(name);
    const parenIdx = m.index! + m[0].length - 1; // the '(' the regex ends on
    matches.push({ start: m.index!, nameStart, name, parenIdx, asyncKw });
  }

  if (!matches.length) {
    skipped++;
    continue;
  }

  let out = src;
  for (let k = matches.length - 1; k >= 0; k--) {
    const { start, name, parenIdx, asyncKw } = matches[k];
    const bodyEnd = findFunctionEnd(out, parenIdx);
    if (bodyEnd === -1) {
      throw new Error(`${file}: could not find the end of function '${name}' — aborting this file untouched`);
    }
    const fnText = out.slice(start, bodyEnd); // "export async function name(...) { ... }"
    const withoutExport = fnText.replace(/^export\s+/, '');
    const wrapped = `export const ${name} = wrap('${moduleName}.${name}', ${withoutExport});`;
    out = out.slice(0, start) + wrapped + out.slice(bodyEnd);
  }

  const importLine = `import { wrap } from '@/modules/diagnostics/services/defineService';`;
  const importLines = [...out.matchAll(/^import[^\n]*;\s*$/gm)];
  if (importLines.length) {
    const last = importLines[importLines.length - 1];
    const insertAt = last.index! + last[0].length;
    out = out.slice(0, insertAt) + '\n' + importLine + '\n' + out.slice(insertAt);
  } else {
    out = importLine + '\n\n' + out;
  }

  if (DRY) {
    console.log(`would wrap ${relative(process.cwd(), file)}: ${matches.map((m) => m.name).join(', ')}`);
  } else {
    writeFileSync(file, out);
    console.log(`wrapped ${relative(process.cwd(), file)}: ${matches.length} function(s)`);
  }
  changed++;
}

console.log(`\n${changed} file(s) ${DRY ? 'would be' : ''} wrapped, ${skipped} skipped.`);
