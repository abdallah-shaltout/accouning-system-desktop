/** Analyzer — architecture rule checks: mock seam, Rust IPC contract, page size, cross-module page reach-ins. */
import type { Config } from '../config';
import type { Analysis, ParsedRepo, SeamViolation } from '../types';
import { locate, stem } from './classify';

function seamViolations(repo: ParsedRepo, config: Config): SeamViolation[] {
  const byFile = new Map<string, Set<string>>();
  for (const edge of repo.imports) {
    if (edge.typeOnly || !edge.target?.startsWith(config.seam.forbiddenTarget)) continue;
    const loc = locate(edge.from, config);
    const allowed = loc.area === 'mocks' || config.seam.compositionRoots.includes(edge.from) || (loc.area === 'module' && config.seam.allowedLayers.includes(loc.layer!));
    if (!allowed) byFile.set(edge.from, (byFile.get(edge.from) ?? new Set()).add(edge.target.slice(config.paths.mocks.length + 1)));
  }
  return [...byFile]
    .map(([file, targets]) => ({ file, targets: [...targets].sort(), known: config.seam.knownLegacy.includes(stem(file)) }))
    .sort((a, b) => Number(a.known) - Number(b.known) || a.file.localeCompare(b.file));
}

function ipcContract(repo: ParsedRepo): Analysis['boundaries']['ipc'] {
  const last = (q: string) => q.split('::').pop()!;
  const registered = new Set(repo.rust.registered.map(last));
  const defined = new Set(repo.rust.commands.map((c) => c.name));
  const invoked = new Set(repo.ipc.map((c) => c.command));
  const minus = (a: Set<string>, b: Set<string>) => [...a].filter((x) => !b.has(x)).sort();
  return {
    registeredNotDefined: minus(registered, defined),
    definedNotRegistered: minus(defined, registered),
    registeredNotInvoked: minus(registered, invoked),
    invokedNotRegistered: minus(invoked, registered),
  };
}

export function analyzeBoundaries(repo: ParsedRepo, config: Config): Analysis['boundaries'] {
  const oversizedPages = repo.files
    .filter((f) => f.lang === 'vue' && locate(f.path, config).layer === 'pages' && f.lines > config.maxPageLines)
    .map((f) => ({ file: f.path, lines: f.lines }))
    .sort((a, b) => b.lines - a.lines);

  // Modules talk through services/components/helpers; reaching into another module's pages is a smell
  // (route files lazy-load their own pages, so same-module imports are excluded).
  const crossModulePageImports = repo.imports
    .filter((e) => e.target)
    .map((e) => ({ e, from: locate(e.from, config), to: locate(e.target!, config) }))
    .filter(({ from, to }) => to.area === 'module' && to.layer === 'pages' && from.module !== to.module)
    .map(({ e }) => ({ from: e.from, target: e.target! }))
    .sort((a, b) => a.from.localeCompare(b.from));

  return { seam: seamViolations(repo, config), ipc: ipcContract(repo), oversizedPages, crossModulePageImports };
}
