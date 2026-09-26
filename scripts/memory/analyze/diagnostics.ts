/**
 * Analyzer — 18.G "Open diagnostics" section. Reads every `docs/diagnostics/issues/*.md` file's
 * YAML frontmatter (already scanned as ordinary `.md` files under `docs/`, since `docs` is one of
 * `config.scanRoots`) and summarizes the ledger's *open* issues by kind and area, so an agent sees
 * known failures before starting work on a module — without re-deriving parse logic: this reuses
 * the exact same frontmatter reader `scripts/diagnostics/run.ts` (18.B7) already has.
 */
import path from 'node:path';
import { parseIssueFile, type IssueFrontmatter } from '../../diagnostics/frontmatter';
import type { Config } from '../config';
import type { ParsedRepo } from '../types';

export interface OpenDiagnosticsSummary {
  /** One row per open/investigating issue, newest-first-ish (as they appear in the ledger). */
  issues: { id: string; kind: IssueFrontmatter['kind']; area: string; status: IssueFrontmatter['status']; occurrences: number; lastSeen: string; debugNamespace?: string; file: string }[];
  byKind: Record<string, number>;
  byArea: { area: string; count: number }[];
  /** Debug namespaces worth turning on while working in an area with open `debug`-kind issues. */
  debugNamespaces: string[];
}

export function analyzeOpenDiagnostics(repo: ParsedRepo, config: Config): OpenDiagnosticsSummary {
  const prefix = config.paths.diagnosticsIssues + '/';
  const issueFiles = repo.files.filter((f) => f.lang === 'md' && f.path.startsWith(prefix));

  const issues: OpenDiagnosticsSummary['issues'] = [];
  for (const f of issueFiles) {
    let parsed: ReturnType<typeof parseIssueFile>;
    try {
      parsed = parseIssueFile(f.path, f.content);
    } catch {
      continue; // a malformed issue file doesn't break the whole memory build
    }
    const fm = parsed.frontmatter;
    if (fm.status !== 'open' && fm.status !== 'investigating') continue;
    issues.push({
      id: fm.id,
      kind: fm.kind,
      area: fm.area,
      status: fm.status,
      occurrences: Number(fm.occurrences) || 0,
      lastSeen: fm.last_seen,
      debugNamespace: fm.debug_namespace,
      file: path.posix.join(config.paths.diagnosticsIssues, path.posix.basename(f.path)),
    });
  }
  issues.sort((a, b) => (a.lastSeen === b.lastSeen ? a.id.localeCompare(b.id) : b.lastSeen.localeCompare(a.lastSeen)));

  const byKind: Record<string, number> = {};
  for (const i of issues) byKind[i.kind] = (byKind[i.kind] ?? 0) + 1;

  const areaCounts = new Map<string, number>();
  for (const i of issues) areaCounts.set(i.area, (areaCounts.get(i.area) ?? 0) + 1);
  const byArea = [...areaCounts].map(([area, count]) => ({ area, count })).sort((a, b) => b.count - a.count || a.area.localeCompare(b.area));

  const debugNamespaces = [...new Set(issues.map((i) => i.debugNamespace).filter((x): x is string => !!x))].sort();

  return { issues, byKind, byArea, debugNamespaces };
}
