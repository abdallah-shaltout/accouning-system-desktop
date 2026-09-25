/** One renderer per AGENT_MEMORY.md section. Each is pure: (data) → markdown string. */
import type { Config } from '../config';
import type { Analysis, ParsedRepo } from '../types';
import { code, list, section, table } from './md';

type Ctx = { repo: ParsedRepo; analysis: Analysis; config: Config };

export function header({ analysis }: Ctx): string {
  const { files, lines, byLang } = analysis.totals;
  const langs = Object.entries(byLang).sort().map(([l, n]) => `${l} ${n}`).join(', ');
  return [
    '# AGENT_MEMORY',
    '> **Generated** by `bun run memory` (scripts/memory). Do not edit by hand — re-run after structural changes\n' +
      '> (new module, service, route, Rust command, mock file, or moved folders). `bun run memory:check` fails when stale.',
    `Indexed: **${files} files / ${lines.toLocaleString('en-US')} lines** (${langs}).`,
    '**Lookup order:** Where-to-find → Domain map → Service API → Routes → IPC → Mock map. Only grep when this file has no answer.',
  ].join('\n\n');
}

export function stack({ repo }: Ctx): string {
  const m = repo.manifests;
  const scripts = Object.entries(m.scripts).map(([k, v]) => [code(`bun run ${k}`), code(v)]);
  return section(
    'Stack & commands',
    table(['Layer', 'Facts'], [
      ['Desktop shell', `Tauri v2 — product ${code(m.tauri.productName ?? '?')}, identifier ${code(m.tauri.identifier ?? '?')} (never change)`],
      ['Rust crates', list(m.cargoDeps, 40)],
      ['Rust extra binaries', list(m.cargoBins)],
      ['Frontend deps', list(m.deps, 40)],
      ['Dev deps', list(m.devDeps)],
    ]),
    table(['Command', 'Runs'], scripts),
  );
}

export function architecture({ analysis, config }: Ctx): string {
  const count = (layer: string) => analysis.modules.reduce((n, m) => n + (m.layers[layer] ?? 0), 0);
  return section(
    'Architecture (layers & data flow)',
    '```text\n' +
      [
        `pages (${count('pages')}) / components (${count('components')}) / controllers (${count('controllers')})   src/modules/<domain>/…`,
        `        │  may call ONLY ▼                  (seam rule — see Boundary report)`,
        `services (${count('services')})   src/modules/<domain>/services/*   ← swap point for a real backend`,
        `   │                                   │`,
        `   ▼                                   ▼`,
        `mock backend  ${config.paths.mocks}/     Tauri IPC invoke('<cmd>') → ${config.paths.rustEntry}`,
        `(db, persist, events, seed,           (PDF/Typst render, thermal printing)`,
        ` backend/* = accounting engine)       + plugins fs / dialog / opener / sql`,
      ].join('\n') +
      '\n```',
    'Module anatomy: `pages` (routed screens) · `components` (feature-only UI) · `controllers` (composables / Pinia stores) · ' +
      '`services` (async API, the seam) · `helpers` (pure logic) · `routes` (route records + meta) · `types` · `validators` (Zod) · ' +
      '`commands.ts` (command-palette registration).',
  );
}

export function domainMap({ analysis }: Ctx): string {
  const rows = analysis.modules.map((m) => [
    `**${m.name}**`,
    `${m.files} / ${m.lines}`,
    Object.entries(m.layers).sort().map(([l, n]) => `${l} ${n}`).join(', '),
    m.routes.length,
    m.hasPaletteCommands ? 'yes' : '',
  ]);
  return section('Domain map', table(['Module', 'Files / lines', 'Layers (file count)', 'Routes', 'Palette'], rows));
}

export function serviceApi({ analysis }: Ctx): string {
  const rows = analysis.modules.flatMap((m) => m.services.map((s) => [m.name, code(s.file), list(s.api, 40)]));
  return section('Service API (the seam — pages call only these)', table(['Module', 'Service', 'Exports'], rows));
}

export function routes({ analysis }: Ctx): string {
  const rows = analysis.modules.flatMap((m) =>
    m.routes.map((r) => [m.name, code(r.path), r.name ?? '', r.title ?? '', r.area ?? '', r.component ?? '']),
  );
  return section('Routes', table(['Module', 'Path', 'Name', 'Title', 'Area', 'Page'], rows));
}

export function dependencies({ analysis }: Ctx): string {
  const byFrom = new Map<string, string[]>();
  for (const e of analysis.moduleEdges) byFrom.set(e.from, [...(byFrom.get(e.from) ?? []), `${e.to} (${e.count})`]);
  const fanIn = new Map<string, number>();
  for (const e of analysis.moduleEdges) fanIn.set(e.to, (fanIn.get(e.to) ?? 0) + 1);
  return section(
    'Module dependencies (who imports whom)',
    'Counts are import statements. `app` = router / main.ts / App.vue; `mocks` = src/mocks.',
    table(
      ['From', 'Imports from', 'Imported by (# modules)'],
      [...byFrom].sort().map(([from, tos]) => [`**${from}**`, tos.join(', '), fanIn.get(from) ?? 0]),
    ),
    '**Most-used npm packages** (files importing): ' + list(analysis.packages.map((p) => `${p.pkg} (${p.files})`), 25),
  );
}

export function ipc({ repo, analysis }: Ctx): string {
  const invokedBy = (name: string) => [...new Set(repo.ipc.filter((c) => c.command === name).map((c) => c.file))];
  const registered = new Set(repo.rust.registered.map((r) => r.split('::').pop()));
  const rows = repo.rust.commands.map((c) => [
    code(c.name), code(c.qualified), code(c.file), registered.has(c.name) ? 'yes' : '**no**', list(invokedBy(c.name), 5),
  ]);
  const mods = repo.rust.mods.map((m) => `${m.file.replace(/^src-tauri\/src\//, '')} → ${m.public ? 'pub ' : ''}mod ${m.name}`);
  const i = analysis.boundaries.ipc;
  return section(
    'Rust ↔ Vue IPC contract',
    table(['Command', 'Rust path', 'Defined in', 'Registered', 'Invoked from'], rows),
    `**Plugins:** ${list(repo.rust.plugins)}. Frontend plugin use: ${list(
      analysis.packages.filter((p) => p.pkg.startsWith('@tauri-apps/')).map((p) => `${p.pkg} (${p.files})`),
    )}`,
    `**Rust module tree:** ${list(mods, 60)}`,
    `**Contract gaps:** invoked-but-unregistered ${list(i.invokedNotRegistered)} · registered-but-never-invoked ${list(i.registeredNotInvoked)} · ` +
      `defined-but-unregistered ${list(i.definedNotRegistered)} · registered-but-undefined ${list(i.registeredNotDefined)}`,
  );
}

export function mocks({ analysis }: Ctx): string {
  const rows = analysis.mocks.map((m) => [code(m.file), list(m.api, 25), list(m.importers, 20, (s) => s)]);
  return section(
    'Mock backend map (src/mocks — accounting engine)',
    'Read `docs/v2/02-accounting-review.md` before touching posting, VAT, cost or account resolution; keep `bun run verify:mocks` green.',
    table(['File', 'Exported functions', 'Used by'], rows),
  );
}

export function kit({ analysis }: Ctx): string {
  const rows = Object.entries(analysis.kit).map(([group, names]) => [group, list(names, 120)]);
  return section('Shared UI kit (reuse before building — src/modules/core)', table(['Group', 'Names'], rows));
}

export function boundaries({ analysis, config }: Ctx): string {
  const b = analysis.boundaries;
  const seamRows = b.seam.map((v) => [code(v.file), list(v.targets, 10), v.known ? 'known legacy' : '**NEW**']);
  return section(
    'Boundary report',
    `### Seam violations — value imports of \`src/mocks\` outside \`services/\` (${b.seam.filter((v) => !v.known).length} new, ${b.seam.filter((v) => v.known).length} known)`,
    table(['File', 'Mock targets', 'Status'], seamRows),
    `### Pages over ${config.maxPageLines} lines (CLAUDE.md rule 12)`,
    table(['Page', 'Lines'], b.oversizedPages.map((p) => [code(p.file), p.lines])),
    '### Cross-module page imports',
    table(['From', 'Imports page'], b.crossModulePageImports.map((p) => [code(p.from), code(p.target)])),
  );
}

export function landmarks({ analysis }: Ctx): string {
  const rows = analysis.landmarks.map((l) => [l.label, l.exists ? code(l.path) : `~~${l.path}~~ (missing)`]);
  return section('Where to find X', table(['Concern', 'Path'], rows));
}

export function docs({ repo }: Ctx): string {
  return section('Docs index', table(['Doc', 'Title'], repo.docs.map((d) => [code(d.path), d.title])));
}
