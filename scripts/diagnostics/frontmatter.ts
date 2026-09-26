/**
 * A minimal YAML-frontmatter reader/writer for `docs/diagnostics/issues/*.md` (18.B7). The schema
 * is flat key: value pairs only (see `phase-b-diagnostics.md`'s example) — no lists, no nesting —
 * so a hand-rolled parser avoids pulling in a YAML dependency for one small, fixed shape.
 */
export interface IssueFrontmatter {
  id: string;
  kind: 'bug' | 'perf' | 'debug' | 'accounting';
  status: 'open' | 'investigating' | 'fixed' | 'verified' | 'wontfix';
  area: string;
  fingerprint?: string;
  first_seen: string;
  last_seen: string;
  occurrences: number;
  debug_namespace?: string;
  regression_test?: string;
  fixed_in?: string;
}

export interface ParsedIssue {
  frontmatter: IssueFrontmatter;
  body: string;
  filePath: string;
}

const FRONTMATTER_RE = /^---\n([\s\S]*?)\n---\n?([\s\S]*)$/;

function parseValue(raw: string): string | number {
  const trimmed = raw.trim();
  if (trimmed === '') return '';
  if (/^-?\d+$/.test(trimmed)) return Number(trimmed);
  // Strip a single layer of matching quotes, if present.
  if ((trimmed.startsWith('"') && trimmed.endsWith('"')) || (trimmed.startsWith("'") && trimmed.endsWith("'"))) {
    return trimmed.slice(1, -1);
  }
  return trimmed;
}

export function parseIssueFile(filePath: string, contents: string): ParsedIssue {
  const match = FRONTMATTER_RE.exec(contents);
  if (!match) throw new Error(`${filePath}: missing YAML frontmatter`);
  const [, yamlBlock, body] = match;
  const fm: Record<string, string | number> = {};
  for (const line of yamlBlock.split('\n')) {
    if (!line.trim() || line.trim().startsWith('#')) continue;
    const idx = line.indexOf(':');
    if (idx === -1) continue;
    const key = line.slice(0, idx).trim();
    const value = parseValue(line.slice(idx + 1));
    fm[key] = value;
  }
  return { frontmatter: fm as unknown as IssueFrontmatter, body: body.trim(), filePath };
}

const FIELD_ORDER: (keyof IssueFrontmatter)[] = [
  'id', 'kind', 'status', 'area', 'fingerprint', 'first_seen', 'last_seen', 'occurrences', 'debug_namespace', 'regression_test', 'fixed_in',
];

export function serializeIssueFile(fm: IssueFrontmatter, body: string): string {
  const lines = ['---'];
  for (const key of FIELD_ORDER) {
    const value = fm[key];
    if (value === undefined || value === '') continue;
    lines.push(`${key}: ${value}`);
  }
  lines.push('---', '', body.trim(), '');
  return lines.join('\n');
}
