/** Analyzer — per-domain inventory: layers, service API (the seam) and routes. */
import type { Config } from '../config';
import type { ModuleSummary, ParsedRepo } from '../types';
import { locate, stem } from './classify';

const apiNames = (repo: ParsedRepo, file: string) =>
  (repo.exports.get(file) ?? []).filter((s) => s.kind === 'function' || s.kind === 'const').map((s) => s.name);

export function analyzeModules(repo: ParsedRepo, config: Config): ModuleSummary[] {
  const byName = new Map<string, ModuleSummary>();
  for (const file of repo.files) {
    const loc = locate(file.path, config);
    if (loc.area !== 'module') continue;
    const mod = byName.get(loc.module!) ?? {
      name: loc.module!, files: 0, lines: 0, layers: {}, services: [], routes: [], hasPaletteCommands: false,
    };
    mod.files++;
    mod.lines += file.lines;
    mod.layers[loc.layer!] = (mod.layers[loc.layer!] ?? 0) + 1;
    if (loc.layer === 'services' && file.lang === 'ts') mod.services.push({ file: stem(file.path), api: apiNames(repo, file.path) });
    if (loc.layer === 'commands') mod.hasPaletteCommands = true;
    byName.set(mod.name, mod);
  }
  for (const route of repo.routes) byName.get(locate(route.file, config).module!)?.routes.push(route);
  return [...byName.values()].sort((a, b) => a.name.localeCompare(b.name));
}
