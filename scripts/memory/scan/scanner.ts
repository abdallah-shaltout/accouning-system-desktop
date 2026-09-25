/** Stage 1 — walk the configured roots and load every indexable source file. */
import fs from 'node:fs';
import path from 'node:path';
import type { Config } from '../config';
import type { SourceFile } from '../types';

export function toPosix(p: string): string {
  return p.split(path.sep).join('/');
}

function isIgnored(rel: string, name: string, config: Config): boolean {
  return config.ignoreDirs.includes(name) || config.ignorePrefixes.some((p) => rel.startsWith(p));
}

function walk(dirAbs: string, config: Config, out: SourceFile[]): void {
  for (const entry of fs.readdirSync(dirAbs, { withFileTypes: true })) {
    const abs = path.join(dirAbs, entry.name);
    const rel = toPosix(path.relative(config.root, abs));
    if (isIgnored(rel, entry.name, config)) continue;
    if (entry.isDirectory()) {
      walk(abs, config, out);
      continue;
    }
    const ext = entry.name.slice(entry.name.lastIndexOf('.') + 1);
    const lang = config.extensions[ext];
    if (!entry.isFile() || !lang) continue;
    const content = fs.readFileSync(abs, 'utf8');
    out.push({ path: rel, lang, lines: content.split('\n').length, content });
  }
}

export function scan(config: Config): SourceFile[] {
  const files: SourceFile[] = [];
  for (const root of config.scanRoots) {
    const abs = path.join(config.root, root);
    if (fs.existsSync(abs)) walk(abs, config, files);
  }
  return files.sort((a, b) => a.path.localeCompare(b.path));
}

/** Directories (not files) that exist at a repo-relative path — used for landmarks and kit listing. */
export function listDir(config: Config, rel: string): fs.Dirent[] {
  const abs = path.join(config.root, rel);
  return fs.existsSync(abs) ? fs.readdirSync(abs, { withFileTypes: true }) : [];
}

export function exists(config: Config, rel: string): boolean {
  return fs.existsSync(path.join(config.root, rel));
}

export function readJson<T>(config: Config, rel: string): T | null {
  const abs = path.join(config.root, rel);
  return fs.existsSync(abs) ? (JSON.parse(fs.readFileSync(abs, 'utf8')) as T) : null;
}

export function readText(config: Config, rel: string): string {
  const abs = path.join(config.root, rel);
  return fs.existsSync(abs) ? fs.readFileSync(abs, 'utf8') : '';
}
