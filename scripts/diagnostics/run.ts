/**
 * The "failed to fix" issue ledger (18.B7) — same generated-index pattern as `AGENT_MEMORY.md`:
 *
 *   bun run diag                      # rebuild docs/diagnostics/ISSUES.md, stub any new fingerprint
 *   bun run diag:check                # exit 1 if ISSUES.md is stale (CI / pre-commit)
 *   bun run scripts/diagnostics/run.ts --ingest <findings.json>
 *                                      # 18.G: create/refresh issues from e2e/perf/verify:mocks/bundle-size
 *                                      # findings (scripts/e2e/run.py writes this file), then rebuild the index
 *
 * Reads every `docs/diagnostics/issues/*.md` file's frontmatter (see `frontmatter.ts` — the schema
 * is `plans/pending/18-countries-a11y-diagnostics/phase-b-diagnostics.md`'s B7 section) and renders
 * `docs/diagnostics/ISSUES.md`. Never hand-edit ISSUES.md — edit the per-issue files, or let a new
 * fingerprint get stubbed automatically from `.diagnostics/logs/error.jsonl` (the dev-server
 * `/__diag` middleware's local, gitignored output — see `vite.config.ts`), or from `--ingest`.
 */
import fs from 'node:fs';
import path from 'node:path';
import { parseIssueFile, serializeIssueFile, type IssueFrontmatter } from './frontmatter';

const ROOT = path.resolve(import.meta.dirname, '../..');
const ISSUES_DIR = path.join(ROOT, 'docs/diagnostics/issues');
const OUTPUT = path.join(ROOT, 'docs/diagnostics/ISSUES.md');
const ERROR_LOG = path.join(ROOT, '.diagnostics/logs/error.jsonl');

const KIND_LABEL: Record<IssueFrontmatter['kind'], string> = { bug: 'خلل', perf: 'أداء', debug: 'تتبع', accounting: 'محاسبة' };
const STATUS_LABEL: Record<IssueFrontmatter['status'], string> = { open: 'مفتوح', investigating: 'قيد الفحص', fixed: 'أُصلح', verified: 'مُتحقَّق', wontfix: 'لن يُصلح' };

function listIssueFiles(): string[] {
  if (!fs.existsSync(ISSUES_DIR)) return [];
  return fs
    .readdirSync(ISSUES_DIR)
    .filter((f) => f.endsWith('.md'))
    .map((f) => path.join(ISSUES_DIR, f));
}

function readIssues() {
  return listIssueFiles().map((filePath) => parseIssueFile(filePath, fs.readFileSync(filePath, 'utf8')));
}

/** `wrap()` (`modules/diagnostics/services/defineService.ts`) logs every thrown service error to
 * the `error` channel, including expected user-facing validation failures (`ApiError` — "اختر
 * عميلاً أولاً", not a bug). Auto-stubbing those would flood the ledger with fake "open bugs" on
 * every normal validation message, so they're excluded here; a real `ApiError` worth tracking
 * (one that reveals an actual gap) should be turned into an issue by hand. */
const EXCLUDED_ERROR_NAMES = new Set(['ApiError']);

/** Reads today's local error log (dev-server only, gitignored) and returns fingerprints that have
 * no matching issue file yet — `bun run diag` stubs one `BUG-NNNN-<fingerprint>.md` per new one so
 * nothing silently falls off the ledger. No-op when the log doesn't exist (CI, a fresh checkout). */
function findNewFingerprints(known: Set<string>): { fingerprint: string; name: string; message: string; source: string }[] {
  if (!fs.existsSync(ERROR_LOG)) return [];
  const lines = fs.readFileSync(ERROR_LOG, 'utf8').split('\n').filter(Boolean);
  const seen = new Map<string, { fingerprint: string; name: string; message: string; source: string }>();
  for (const line of lines) {
    try {
      const entry = JSON.parse(line);
      const fp = entry?.err?.fingerprint;
      if (!fp || known.has(fp) || seen.has(fp) || EXCLUDED_ERROR_NAMES.has(entry.err.name)) continue;
      seen.set(fp, { fingerprint: fp, name: entry.err.name, message: entry.err.message, source: entry.source });
    } catch {
      /* a malformed line just doesn't contribute a stub */
    }
  }
  return [...seen.values()];
}

function nextId(kind: IssueFrontmatter['kind'], existing: IssueFrontmatter[]): string {
  const prefix = { bug: 'BUG', perf: 'PERF', debug: 'DBG', accounting: 'ACC' }[kind];
  const used = existing.filter((f) => f.id.startsWith(prefix + '-')).map((f) => Number(f.id.split('-')[1]) || 0);
  const next = (used.length ? Math.max(...used) : 0) + 1;
  return `${prefix}-${String(next).padStart(4, '0')}`;
}

function slug(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 40) || 'issue';
}

/**
 * 18.G: one finding fed in from outside this file — an e2e flow failure/console error, a perf
 * regression against `docs/diagnostics/perf-baseline.json`, a `verify:mocks` invariant failure, or
 * a bundle-size regression. `scripts/e2e/run.py` (and `verify:mocks`'s wrapper) writes an array of
 * these to a JSON file and calls `--ingest <file>`, so every "something went wrong" path in the dev
 * loop creates or refreshes a ledger issue through this one mechanism — the same one B7's automatic
 * fingerprint stubbing already used, extended to take findings from a fixed shape instead of only
 * parsing `.diagnostics/logs/error.jsonl`.
 */
export interface IngestFinding {
  kind: IssueFrontmatter['kind'];
  /** Stable dedupe key — an error fingerprint, `perf:<flow>:<metric>`, `acc:<invariant>:<case>`, or `bundle-size`. */
  fingerprint: string;
  area: string;
  title: string;
  body: string;
  debug_namespace?: string;
  /** The structured error's `name` (e.g. `ApiError`), when this finding came from the `error`
   * channel — lets `ingest()` apply the same `EXCLUDED_ERROR_NAMES` filter B7's fingerprint
   * auto-stubbing already uses, so an e2e flow hitting an expected user-facing validation message
   * (not a bug) doesn't flood the ledger just because a different code path (scripts/e2e/run.py)
   * fed it in instead of the dev-server log scrape. */
  error_name?: string;
}

/** Creates a new issue file for a fingerprint never seen before, or bumps `occurrences`/`last_seen`
 * on the existing one (re-opening it if it had been marked `fixed`/`wontfix` — a regression that
 * reappears is not "closed" any more). Returns 'created' | 'refreshed' | 'unchanged' (the last one
 * only when the issue is already `verified`, which a mechanical rerun must never silently reopen —
 * an issue that regressed after being verified needs a human to look, so it's left alone here and
 * the caller should treat that case as still worth surfacing in run output). */
function upsertIssue(existing: IssueFrontmatter[], finding: IngestFinding, today: string): 'created' | 'refreshed' | 'unchanged' {
  const current = existing.find((f) => f.fingerprint === finding.fingerprint);
  if (current) {
    if (current.status === 'verified') return 'unchanged';
    current.last_seen = today;
    current.occurrences = (current.occurrences || 0) + 1;
    if (current.status === 'fixed' || current.status === 'wontfix') current.status = 'open';
    const filePath = path.join(ISSUES_DIR, fileFor(current.id)!);
    const { body } = parseIssueFile(filePath, fs.readFileSync(filePath, 'utf8'));
    fs.writeFileSync(filePath, serializeIssueFile(current, body));
    return 'refreshed';
  }

  const id = nextId(finding.kind, existing);
  const frontmatter: IssueFrontmatter = {
    id,
    kind: finding.kind,
    status: 'open',
    area: finding.area,
    fingerprint: finding.fingerprint,
    first_seen: today,
    last_seen: today,
    occurrences: 1,
    debug_namespace: finding.debug_namespace,
  };
  existing.push(frontmatter);
  const body = [`## ${finding.title}`, '', finding.body].join('\n');
  fs.mkdirSync(ISSUES_DIR, { recursive: true });
  fs.writeFileSync(path.join(ISSUES_DIR, `${id}-${slug(finding.title)}.md`), serializeIssueFile(frontmatter, body));
  return 'created';
}

function fileFor(id: string): string | undefined {
  return fs.existsSync(ISSUES_DIR) ? fs.readdirSync(ISSUES_DIR).find((f) => f.startsWith(id + '-')) : undefined;
}

function ingest(filePath: string): void {
  const raw: IngestFinding[] = JSON.parse(fs.readFileSync(filePath, 'utf8'));
  const excluded = raw.filter((f) => f.error_name && EXCLUDED_ERROR_NAMES.has(f.error_name));
  const findings = raw.filter((f) => !(f.error_name && EXCLUDED_ERROR_NAMES.has(f.error_name)));
  const existing = readIssues().map((i) => i.frontmatter);
  const today = new Date().toISOString().slice(0, 10);
  const counts = { created: 0, refreshed: 0, unchanged: 0 };
  for (const f of findings) counts[upsertIssue(existing, f, today)]++;
  fs.writeFileSync(OUTPUT, renderIndex(existing));
  console.log(
    `ingested ${findings.length} finding(s): ${counts.created} created, ${counts.refreshed} refreshed, ` +
      `${counts.unchanged} unchanged (already verified)` +
      (excluded.length ? `, ${excluded.length} skipped (expected validation error, not a bug)` : '') +
      ` — docs/diagnostics/ISSUES.md updated.`,
  );
}

function stubNewIssues(existing: IssueFrontmatter[]): number {
  const known = new Set(existing.map((f) => f.fingerprint).filter(Boolean) as string[]);
  const fresh = findNewFingerprints(known);
  const today = new Date().toISOString().slice(0, 10);
  for (const f of fresh) {
    const id = nextId('bug', existing);
    const frontmatter: IssueFrontmatter = {
      id,
      kind: 'bug',
      status: 'open',
      area: f.source.split('.')[0] || 'unknown',
      fingerprint: f.fingerprint,
      first_seen: today,
      last_seen: today,
      occurrences: 1,
    };
    existing.push(frontmatter);
    const body = [
      `## ${f.name}: ${f.message}`,
      '',
      `المصدر: \`${f.source}\``,
      '',
      '### خطوات إعادة الإنتاج',
      '',
      '(لم تُوثَّق بعد — أُنشئ هذا الملف تلقائياً من رمز بصمة خطأ جديد.)',
      '',
      '### ما جُرِّب ولم ينجح',
      '',
      '(لا شيء بعد.)',
      '',
      '### الملفات ذات الصلة',
      '',
      `- \`${f.source.replace(/\./g, '/')}\` (تخمين من اسم المصدر — تحقق منه)`,
    ].join('\n');
    fs.mkdirSync(ISSUES_DIR, { recursive: true });
    fs.writeFileSync(path.join(ISSUES_DIR, `${id}-${slug(f.message)}.md`), serializeIssueFile(frontmatter, body));
  }
  return fresh.length;
}

function renderIndex(issues: IssueFrontmatter[]): string {
  const sorted = [...issues].sort((a, b) => (a.status === b.status ? a.id.localeCompare(b.id) : a.status.localeCompare(b.status)));
  const open = sorted.filter((i) => i.status === 'open' || i.status === 'investigating');
  const closed = sorted.filter((i) => i.status === 'fixed' || i.status === 'verified' || i.status === 'wontfix');

  const row = (i: IssueFrontmatter) => {
    const file = fs.readdirSync(ISSUES_DIR).find((f) => f.startsWith(i.id + '-'));
    const link = file ? `[${i.id}](issues/${file})` : i.id;
    return `| ${link} | ${KIND_LABEL[i.kind]} | ${STATUS_LABEL[i.status]} | ${i.area} | ${i.occurrences} | ${i.last_seen} |`;
  };

  const header = '| المعرف | النوع | الحالة | المنطقة | التكرار | آخر ظهور |\n|---|---|---|---|---|---|';

  return [
    '# سجل المشاكل (Issues) — ملف مُولَّد',
    '',
    '**لا تُعدِّل هذا الملف يدوياً** — عدّل ملفات `issues/*.md` أو شغّل `bun run diag` (مثل `AGENT_MEMORY.md`).',
    '',
    `آخر تحديث: ${new Date().toISOString()}`,
    '',
    `## مفتوح / قيد الفحص (${open.length})`,
    '',
    open.length ? [header, ...open.map(row)].join('\n') : '_لا شيء._',
    '',
    `## مُغلق (${closed.length})`,
    '',
    closed.length ? [header, ...closed.map(row)].join('\n') : '_لا شيء._',
    '',
  ].join('\n');
}

function main() {
  const ingestIdx = process.argv.indexOf('--ingest');
  if (ingestIdx !== -1) {
    const filePath = process.argv[ingestIdx + 1];
    if (!filePath) {
      console.error('--ingest requires a findings JSON file path');
      process.exit(1);
    }
    ingest(filePath);
    return;
  }

  const checkOnly = process.argv.includes('--check');
  const before = readIssues().map((i) => i.frontmatter);
  const beforeCount = before.length;

  const stubbed = checkOnly ? 0 : stubNewIssues(before);
  const markdown = renderIndex(before);

  const currentOnDisk = fs.existsSync(OUTPUT) ? fs.readFileSync(OUTPUT, 'utf8').replace(/\r\n/g, '\n') : '';
  // The "آخر تحديث" timestamp always differs, so compare everything else for staleness instead.
  const stripTimestamp = (s: string) => s.replace(/آخر تحديث: .*/g, '');
  const stale = stripTimestamp(currentOnDisk) !== stripTimestamp(markdown) || stubbed > 0;

  if (checkOnly) {
    if (stale) {
      console.error('docs/diagnostics/ISSUES.md is stale — run `bun run diag`.');
      process.exit(1);
    }
    console.log(`docs/diagnostics/ISSUES.md is up to date (${beforeCount} issue(s)).`);
    return;
  }

  fs.mkdirSync(path.dirname(OUTPUT), { recursive: true });
  fs.writeFileSync(OUTPUT, markdown);
  console.log(`docs/diagnostics/ISSUES.md: ${before.length} issue(s), ${stubbed} new stub(s) — written.`);
}

main();
