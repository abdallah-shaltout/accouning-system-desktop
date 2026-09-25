/** Parser — import graph and exported API for .ts / .vue files. Regex-based: no compiler needed. */
import path from 'node:path';
import type { Config } from '../config';
import type { ExportSymbol, ImportEdge, SourceFile } from '../types';

const STATIC_IMPORT = /^\s*import\s+(type\s+)?([^;'"]*?)\s+from\s+['"]([^'"]+)['"]/gm;
const RE_EXPORT = /^\s*export\s+(type\s+)?(\*[^;'"]*?|\{[^}]*\})\s+from\s+['"]([^'"]+)['"]/gm;
const SIDE_EFFECT = /^\s*import\s+['"]([^'"]+)['"]/gm;
const DYNAMIC = /\bimport\(\s*['"]([^'"]+)['"]\s*\)/g;

/** The code an SFC contributes to the graph: its <script> blocks only. */
export function scriptOf(file: SourceFile): string {
  if (file.lang !== 'vue') return file.content;
  return [...file.content.matchAll(/<script[^>]*>([\s\S]*?)<\/script>/g)].map((m) => m[1]).join('\n');
}

/** `{ type A, type B }` is type-only too. */
function allSpecifiersTyped(clause: string): boolean {
  const braces = clause.match(/\{([^}]*)\}/);
  if (!braces || /^[^{]*\w/.test(clause.split('{')[0].replace(/,\s*$/, ''))) return false;
  const names = braces[1].split(',').map((s) => s.trim()).filter(Boolean);
  return names.length > 0 && names.every((n) => n.startsWith('type '));
}

function packageName(spec: string): string {
  const parts = spec.split('/');
  return spec.startsWith('@') ? parts.slice(0, 2).join('/') : parts[0];
}

function resolve(from: string, spec: string, known: Set<string>, config: Config): Pick<ImportEdge, 'target' | 'pkg'> {
  let base: string | null = null;
  for (const [alias, dir] of Object.entries(config.aliases)) {
    if (spec.startsWith(alias)) base = dir + spec.slice(alias.length);
  }
  if (base === null && spec.startsWith('.')) base = path.posix.normalize(path.posix.join(path.posix.dirname(from), spec));
  if (base === null) return { target: null, pkg: packageName(spec) };
  for (const suffix of config.resolveSuffixes) {
    if (known.has(base + suffix)) return { target: base + suffix, pkg: null };
  }
  const tsTwin = base.replace(/\.js$/, '.ts');
  return { target: known.has(tsTwin) ? tsTwin : null, pkg: null };
}

export function parseImports(files: SourceFile[], config: Config): ImportEdge[] {
  const code = files.filter((f) => f.lang === 'ts' || f.lang === 'vue');
  const known = new Set(code.map((f) => f.path));
  const edges: ImportEdge[] = [];
  for (const file of code) {
    const src = scriptOf(file);
    const add = (spec: string, typeOnly: boolean, dynamic: boolean) =>
      edges.push({ from: file.path, spec, typeOnly, dynamic, ...resolve(file.path, spec, known, config) });
    for (const m of src.matchAll(STATIC_IMPORT)) add(m[3], !!m[1] || allSpecifiersTyped(m[2]), false);
    for (const m of src.matchAll(RE_EXPORT)) add(m[3], !!m[1], false);
    for (const m of src.matchAll(SIDE_EFFECT)) add(m[1], false, false);
    for (const m of src.matchAll(DYNAMIC)) add(m[1], false, true);
  }
  return edges;
}

const EXPORT_PATTERNS: [RegExp, ExportSymbol['kind']][] = [
  [/^export\s+(?:async\s+)?function\s*\*?\s*(\w+)/gm, 'function'],
  [/^export\s+(?:const|let)\s+(\w+)/gm, 'const'],
  [/^export\s+(?:abstract\s+)?class\s+(\w+)/gm, 'class'],
  [/^export\s+(?:type|interface|enum|const\s+enum)\s+(\w+)/gm, 'type'],
];

export function parseExports(files: SourceFile[]): Map<string, ExportSymbol[]> {
  const map = new Map<string, ExportSymbol[]>();
  for (const file of files.filter((f) => f.lang === 'ts')) {
    const symbols: ExportSymbol[] = [];
    for (const [re, kind] of EXPORT_PATTERNS) {
      for (const m of file.content.matchAll(re)) symbols.push({ name: m[1], kind });
    }
    for (const m of file.content.matchAll(/^export\s*\{([^}]*)\}\s*;?\s*$/gm)) {
      for (const part of m[1].split(',')) {
        const name = part.trim().split(/\s+as\s+/).pop()?.replace(/^type\s+/, '');
        if (name) symbols.push({ name, kind: 'const' });
      }
    }
    if (/^export\s+default\b/m.test(file.content)) symbols.push({ name: 'default', kind: 'default' });
    if (symbols.length) map.set(file.path, symbols);
  }
  return map;
}
