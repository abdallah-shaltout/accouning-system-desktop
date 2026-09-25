/** Analyzer — who imports whom: module→module edges, npm package usage, mock-backend consumers. */
import type { Config } from '../config';
import type { Analysis, ModuleEdge, ParsedRepo } from '../types';
import { nodeOf } from './classify';

export function analyzeModuleEdges(repo: ParsedRepo, config: Config): ModuleEdge[] {
  const counts = new Map<string, number>();
  for (const edge of repo.imports) {
    if (!edge.target) continue;
    const from = nodeOf(edge.from, config);
    const to = nodeOf(edge.target, config);
    if (from === to) continue;
    const key = `${from}\u0000${to}`;
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  return [...counts]
    .map(([key, count]) => {
      const [from, to] = key.split('\u0000');
      return { from, to, count };
    })
    .sort((a, b) => a.from.localeCompare(b.from) || b.count - a.count || a.to.localeCompare(b.to));
}

export function analyzePackages(repo: ParsedRepo): Analysis['packages'] {
  const files = new Map<string, Set<string>>();
  for (const edge of repo.imports) {
    if (edge.pkg) files.set(edge.pkg, (files.get(edge.pkg) ?? new Set()).add(edge.from));
  }
  return [...files]
    .map(([pkg, set]) => ({ pkg, files: set.size }))
    .sort((a, b) => b.files - a.files || a.pkg.localeCompare(b.pkg));
}

export function analyzeMocks(repo: ParsedRepo, config: Config): Analysis['mocks'] {
  const prefix = config.paths.mocks + '/';
  const importers = new Map<string, Set<string>>();
  for (const edge of repo.imports) {
    if (!edge.target?.startsWith(prefix) || edge.from.startsWith(prefix)) continue;
    const node = nodeOf(edge.from, config);
    const who = node === 'app' ? edge.from.replace(/^src\//, '').replace(/\.(ts|vue)$/, '') : node;
    importers.set(edge.target, (importers.get(edge.target) ?? new Set()).add(who));
  }
  return repo.files
    .filter((f) => f.path.startsWith(prefix) && f.lang === 'ts')
    .map((f) => ({
      file: f.path.slice(prefix.length),
      api: (repo.exports.get(f.path) ?? []).filter((s) => s.kind === 'function').map((s) => s.name),
      importers: [...(importers.get(f.path) ?? [])].sort(),
    }));
}
