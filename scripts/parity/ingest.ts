/**
 * `bun run parity --ingest` (plan 21 Part 04, B-12; used by phase E's gate run only, never in lane
 * loops): turns every unexplained diff into a ledger finding and hands the file to
 * `bun run scripts/diagnostics/run.ts --ingest` — the one 18.G mechanism, never a second writer.
 * A GL, party, stock or invariant diff is `accounting` (→ `ACC-`), anything else `bug` (→ `BUG-`),
 * with `area` = the case's domain.
 */
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import type { Diff } from './diff';

export interface CaseFindingInput {
  caseName: string;
  source: string;
  unexplained: Diff[];
  invariantBreaks: string[];
}

interface IngestFinding {
  kind: 'bug' | 'accounting';
  fingerprint: string;
  area: string;
  title: string;
  body: string;
}

const ACCOUNTING_PATH = /^(books\.(trialBalance|customers|suppliers|inventory)|invariants)(?:$|[.[])/;

function lines(diffs: Diff[]): string[] {
  return diffs.slice(0, 25).map((d) => `- \`${d.path}\` (${d.kind}): mock=\`${JSON.stringify(d.mock)?.slice(0, 160)}\` rust=\`${JSON.stringify(d.rust)?.slice(0, 160)}\``);
}

export function buildFindings(inputs: CaseFindingInput[]): IngestFinding[] {
  const out: IngestFinding[] = [];
  for (const c of inputs) {
    const area = c.caseName.split('/')[0];
    const acc = c.unexplained.filter((d) => ACCOUNTING_PATH.test(d.path));
    const other = c.unexplained.filter((d) => !ACCOUNTING_PATH.test(d.path));
    if (acc.length || c.invariantBreaks.length) {
      out.push({
        kind: 'accounting',
        fingerprint: `parity:acc:${c.caseName}`,
        area,
        title: `parity ${c.caseName} — books/invariants differ between mock and Rust`,
        body: [
          `المصدر: \`scripts/parity/cases/${c.caseName}.ts\` (${c.source})`,
          '',
          ...lines(acc),
          ...c.invariantBreaks.map((b) => `- invariant: ${b}`),
          '',
          'أعد التشغيل: `bun run parity --case "' + c.caseName + '"`. اقرأ `docs/v2/02-accounting-review.md` قبل أي إصلاح.',
        ].join('\n'),
      });
    }
    if (other.length) {
      out.push({
        kind: 'bug',
        fingerprint: `parity:bug:${c.caseName}`,
        area,
        title: `parity ${c.caseName} — ${other.length} result diff(s) between mock and Rust`,
        body: [`المصدر: \`scripts/parity/cases/${c.caseName}.ts\` (${c.source})`, '', ...lines(other), '', 'أعد التشغيل: `bun run parity --case "' + c.caseName + '"`.'].join('\n'),
      });
    }
  }
  return out;
}

/** Writes the findings next to the run's output and calls the diagnostics ingester. */
export function ingest(inputs: CaseFindingInput[], outDir: string): number {
  const findings = buildFindings(inputs);
  if (!findings.length) return 0;
  const file = join(outDir, 'findings.json');
  writeFileSync(file, JSON.stringify(findings, null, 2));
  execFileSync('bun', ['run', 'scripts/diagnostics/run.ts', '--ingest', file], { stdio: 'inherit', cwd: join(import.meta.dirname, '../..') });
  return findings.length;
}
