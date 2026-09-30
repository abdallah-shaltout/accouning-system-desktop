/**
 * `bun run parity` — the mock ↔ Rust parity runner (plan 21 Part 04, phase B, B-8; decisions P4-2..P4-6).
 *
 *   bun run parity [--lane L1..L5] [--case <glob>[,<glob>]] [--mock-only] [--host <exe>] [--db <name>] [--bundles] [--ingest]
 *
 * - Each case (`scripts/parity/cases/<domain>/<id>.ts`, API in `case.ts`) runs a **mock pass** then a
 *   **rust pass** (through `parity_host`, protocol in `host-protocol.ts`), and the two result trees
 *   (steps, books, invariants) are diffed (`diff.ts`) and filtered by the allowlist (`allow.ts`).
 * - `--mock-only` runs the mock pass twice and diffs run 1 against run 2 — a determinism check, with
 *   invariants after every step. This is what the B2 lanes use before the host exists.
 * - `--bundles` replays every `scripts/verify/cases/*.json` on both sides (`bundles.ts`). Alone it
 *   runs only the bundles; with `--lane`/`--case` it runs both.
 * - `--host` defaults to the newest `.diagnostics/parity/bin/parity_host-*.exe`; `--db` defaults to
 *   `equal_parity_<lane>` (`equal_parity` without a lane). The host needs `EQUAL_TEST_DATABASE_URL`.
 * - Output: one summary line per case, and `.diagnostics/parity/<ts>/<domain>/<id>.json` with every
 *   diff (path, kind, mock, rust, step, allowedBy). Exit code 1 on any unexplained diff, aborted
 *   case, broken invariant or harness error; 2 on a usage/setup error.
 */
// Must run before any other import: `bases.ts`/`case.ts`/service modules read `localStorage`/
// `indexedDB` at call time, and some (`templateService.ts`) even swallow the ReferenceError and
// silently misbehave instead of crashing (see `polyfills.ts`'s header comment).
import { installPolyfills } from './polyfills';
installPolyfills();

import { mkdirSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, sep } from 'node:path';
import { setLatencyMode } from '../../src/mocks/utils';
import { GLOBAL_ALLOW, applyAllow, validateAllow } from './allow';
import { loadBase } from './bases';
import { bundleFiles, runBundle } from './bundles';
import { laneOf, type Lane, type ParityCase } from './case';
import { realNow } from './clock';
import { diffTrees, IdMap, idsWithoutCreatedAt, type Diff } from './diff';
import { ingest, type CaseFindingInput } from './ingest';
import { runPass, type PassResult } from './pass';
import { importAllServices } from './services';
import { HostTransport, newestHostExe } from './transport';

const ROOT = join(import.meta.dirname, '../..');
const CASES_DIR = join(import.meta.dirname, 'cases');

// --- CLI -----------------------------------------------------------------------------------------

interface Cli {
  lane: Lane | null;
  caseGlobs: string[];
  mockOnly: boolean;
  host: string | null;
  db: string | null;
  bundles: boolean;
  ingest: boolean;
}

function usage(msg: string): never {
  console.error(`parity: ${msg}`);
  console.error('usage: bun run parity [--lane L1..L5] [--case <glob>] [--mock-only] [--host <exe>] [--db <name>] [--bundles] [--ingest]');
  process.exit(2);
}

function parseCli(argv: string[]): Cli {
  const cli: Cli = { lane: null, caseGlobs: [], mockOnly: false, host: null, db: null, bundles: false, ingest: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const val = () => argv[++i] ?? usage(`${a} needs a value`);
    if (a === '--lane') {
      const l = val().toUpperCase();
      if (!/^L[1-5]$/.test(l)) usage(`--lane must be L1..L5, got "${l}"`);
      cli.lane = l as Lane;
    } else if (a === '--case') cli.caseGlobs.push(...val().split(',').map((s) => s.trim()).filter(Boolean));
    else if (a === '--mock-only') cli.mockOnly = true;
    else if (a === '--host') cli.host = val();
    else if (a === '--db') cli.db = val();
    else if (a === '--bundles') cli.bundles = true;
    else if (a === '--ingest') cli.ingest = true;
    else usage(`unknown argument "${a}"`);
  }
  return cli;
}

function globToRegExp(glob: string): RegExp {
  let re = '';
  for (let i = 0; i < glob.length; i++) {
    const ch = glob[i];
    if (ch === '*' && glob[i + 1] === '*') {
      re += '.*';
      i++;
    } else if (ch === '*') re += '[^/]*';
    else if (ch === '?') re += '[^/]';
    else re += ch.replace(/[.+^${}()|[\]\\]/g, '\\$&');
  }
  return new RegExp(`^${re}$`);
}

// --- case discovery ------------------------------------------------------------------------------

function caseFiles(dir: string): string[] {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return [];
  }
  const out: string[] = [];
  for (const e of entries) {
    const p = join(dir, e.name);
    if (e.isDirectory()) out.push(...caseFiles(p));
    else if (e.name.endsWith('.ts')) out.push(p);
  }
  return out.sort();
}

async function loadCases(cli: Cli): Promise<{ cases: ParityCase[]; errors: string[] }> {
  const errors: string[] = [];
  const cases: ParityCase[] = [];
  const globs = cli.caseGlobs.map(globToRegExp);
  for (const file of caseFiles(CASES_DIR)) {
    const expected = relative(CASES_DIR, file).split(sep).join('/').replace(/\.ts$/, '');
    let c: ParityCase | undefined;
    try {
      c = ((await import(file)) as { default?: ParityCase }).default;
    } catch (e) {
      errors.push(`${expected}: failed to load — ${e instanceof Error ? e.message : String(e)}`);
      continue;
    }
    if (!c || typeof c.run !== 'function') {
      errors.push(`${expected}: no default export from defineCase()`);
      continue;
    }
    if (c.name !== expected) {
      errors.push(`${expected}: name "${c.name}" must equal its path under cases/ ("${expected}")`);
      continue;
    }
    if (globs.length && !globs.some((g) => g.test(c.name))) continue;
    const lane = laneOf(c);
    if (cli.lane && lane !== null && lane !== cli.lane) continue;
    errors.push(...validateAllow(c.allow ?? [], c.name));
    cases.push(c);
  }
  return { cases, errors };
}

// --- one case ------------------------------------------------------------------------------------

interface CaseOutcome {
  c: ParityCase;
  ok: boolean;
  problems: string[];
  diffs: Diff[];
  unexplained: Diff[];
  a: PassResult;
  b: PassResult | null;
  baseMs: number;
}

function tree(p: PassResult): unknown {
  return { steps: p.steps, books: p.books, invariants: p.invariants };
}

const noCreatedAtByBase = new Map<string, Set<string>>();
function importedIdsWithoutCreatedAt(key: string, data: unknown): Set<string> {
  let set = noCreatedAtByBase.get(key);
  if (!set) noCreatedAtByBase.set(key, (set = idsWithoutCreatedAt(data)));
  return set;
}

async function runCase(c: ParityCase, transport: HostTransport | null, mockOnly: boolean): Promise<CaseOutcome> {
  const t0 = performance.now();
  const base = loadBase(c.base, c.name);
  const baseMs = performance.now() - t0;
  const labels = mockOnly ? (['run1', 'run2'] as const) : (['mock', 'rust'] as const);
  const a = await runPass(c, base, 'mock', null);
  const b = a.harnessError ? null : await runPass(c, base, mockOnly ? 'mock' : 'rust', mockOnly ? null : transport);
  const problems: string[] = [];
  for (const [label, p] of [[labels[0], a], [labels[1], b]] as const) {
    if (!p) continue;
    if (p.harnessError) problems.push(`${label}: harness error — ${p.harnessError}`);
    if (p.aborted) problems.push(`${label}: aborted — ${p.aborted}`);
    for (const br of p.invariantBreaks) problems.push(`${label}: invariant broken after step "${br.step}": ${br.failures.join('; ')}`);
  }
  let diffs: Diff[] = [];
  if (b && !a.harnessError && !b.harnessError) {
    // 00-import D-8 / P4-13: only base-snapshot rows that had no createdAt may show Rust's import stamp.
    const importedWithoutCreatedAt = !mockOnly && base.snapshot ? importedIdsWithoutCreatedAt(base.key, base.snapshot.data) : undefined;
    diffs = diffTrees(tree(a), tree(b), new IdMap(b.idPairs), { epsilon: c.epsilon, unordered: c.unordered, importedWithoutCreatedAt });
    applyAllow(diffs, [...GLOBAL_ALLOW, ...(c.allow ?? [])]);
  }
  const unexplained = diffs.filter((d) => !d.allowedBy);
  return { c, ok: problems.length === 0 && unexplained.length === 0, problems, diffs, unexplained, a, b, baseMs };
}

function writeCaseOutput(outDir: string, o: CaseOutcome, mode: string): void {
  const file = join(outDir, `${o.c.name}.json`);
  mkdirSync(dirname(file), { recursive: true });
  const pass = (p: PassResult | null) =>
    p && {
      side: p.side,
      aborted: p.aborted,
      harnessError: p.harnessError,
      invariantBreaks: p.invariantBreaks,
      resetCounts: p.resetCounts,
      timings: p.timings,
      steps: p.steps,
      books: p.books,
      invariants: p.invariants,
    };
  writeFileSync(
    file,
    JSON.stringify(
      {
        case: o.c.name,
        source: o.c.source,
        base: typeof o.c.base === 'function' ? 'custom' : o.c.base,
        lane: laneOf(o.c),
        mode,
        ok: o.ok,
        problems: o.problems,
        unexplained: o.unexplained.length,
        diffs: o.diffs,
        passes: { a: pass(o.a), b: pass(o.b) },
      },
      null,
      2,
    ),
  );
}

const ms = (n: number) => `${Math.round(n)}ms`;
const avg = (xs: number[]) => (xs.length ? xs.reduce((s, x) => s + x, 0) / xs.length : 0);
const short = (v: unknown) => {
  const s = JSON.stringify(v);
  return s === undefined ? 'absent' : s.length > 120 ? `${s.slice(0, 117)}...` : s;
};

// --- main ----------------------------------------------------------------------------------------

/** The mock's `mutate()` schedules a debounced IndexedDB save that can only fail headless (Bun has no
 * `indexedDB`); `persist.ts` logs that as best-effort. Drop exactly that line so a long run's output
 * stays readable — every other console.error passes through. */
function silenceHeadlessPersistNoise(): void {
  const original = console.error.bind(console);
  console.error = (...args: unknown[]) => {
    if (typeof args[0] === 'string' && args[0].startsWith('[mocks/persist]')) return;
    original(...args);
  };
}

async function main(): Promise<number> {
  const cli = parseCli(process.argv.slice(2));
  setLatencyMode('off');
  silenceHeadlessPersistNoise();
  const runCasesToo = !cli.bundles || cli.caseGlobs.length > 0 || cli.lane !== null;
  const mode = cli.mockOnly ? 'mock-only' : 'mock-vs-rust';

  const { cases, errors } = runCasesToo ? await loadCases(cli) : { cases: [], errors: [] };
  errors.push(...validateAllow(GLOBAL_ALLOW, 'allow.ts GLOBAL_ALLOW'));
  if (errors.length) {
    for (const e of errors) console.error(`parity: ${e}`);
    return 2;
  }
  if (runCasesToo && cases.length === 0 && !cli.bundles) {
    console.error('parity: no case matched (cases live in scripts/parity/cases/<domain>/<id>.ts)');
    return 2;
  }

  const stamp = realNow().toISOString().replace(/[:.]/g, '-');
  const outDir = join(ROOT, '.diagnostics/parity', stamp);
  mkdirSync(outDir, { recursive: true });

  let transport: HostTransport | null = null;
  if (!cli.mockOnly) {
    const exe = cli.host ?? newestHostExe();
    if (!exe) usage('no parity_host build in .diagnostics/parity/bin/ — pass --host <exe>, or use --mock-only until the manager\'s host build round (B-4)');
    if (!process.env.EQUAL_TEST_DATABASE_URL) usage('EQUAL_TEST_DATABASE_URL is not set (bun run db:dev → mysql://root:equal-dev@127.0.0.1:3499)');
    const db = cli.db ?? (cli.lane ? `equal_parity_${cli.lane.toLowerCase()}` : 'equal_parity');
    transport = new HostTransport(exe, db, join(outDir, 'host.stderr.log'));
    console.log(`parity: host ${relative(ROOT, exe)} · db ${db}`);
  }
  console.log(`parity: ${mode}${cli.lane ? ` · lane ${cli.lane}` : ''} · ${cases.length} case(s)${cli.bundles ? ' + bundles' : ''} · output ${relative(ROOT, outDir)}`);

  let failed = 0;
  const findings: CaseFindingInput[] = [];
  const invA: number[] = [];
  const invB: number[] = [];
  const resets: number[] = [];
  try {
    for (const c of cases) {
      const o = await runCase(c, transport, cli.mockOnly);
      writeCaseOutput(outDir, o, mode);
      invA.push(...o.a.timings.invariantMs);
      if (o.b) invB.push(...o.b.timings.invariantMs);
      if (o.b?.side === 'rust') resets.push(o.b.timings.resetMs);
      const allowed = o.diffs.length - o.unexplained.length;
      const time = `${ms(o.a.timings.runMs)}${o.b ? ` / ${ms(o.b.timings.runMs)}` : ''}`;
      console.log(`${o.ok ? 'PASS' : 'FAIL'}  ${c.name}  steps=${Object.keys(o.a.steps).length} unexplained=${o.unexplained.length} allowed=${allowed}  ${time}`);
      if (!o.ok) {
        failed++;
        for (const p of o.problems) console.log(`      ${p}`);
        for (const d of o.unexplained.slice(0, 8)) console.log(`      diff ${d.kind} at ${d.path}: ${short(d.mock)} ≠ ${short(d.rust)}`);
        if (o.unexplained.length > 8) console.log(`      … ${o.unexplained.length - 8} more in the output file`);
      }
      findings.push({
        caseName: c.name,
        source: c.source,
        unexplained: o.unexplained,
        invariantBreaks: [...o.a.invariantBreaks, ...(o.b?.invariantBreaks ?? [])].flatMap((b) => b.failures),
      });
    }

    if (cli.bundles) {
      await importAllServices();
      const files = bundleFiles();
      for (const file of files) {
        const r = await runBundle(file, transport);
        const name = relative(ROOT, file).split(sep).join('/');
        writeFileSync(join(outDir, `bundle-${name.split('/').pop()}`), JSON.stringify(r, null, 2));
        const ok = r.problems.length === 0;
        console.log(`${ok ? 'PASS' : 'FAIL'}  bundle ${name}  actions=${r.mock.steps.length}`);
        if (!ok) {
          failed++;
          for (const p of r.problems.slice(0, 8)) console.log(`      ${p}`);
        }
      }
    }
  } finally {
    await transport?.close();
  }

  const timing = [`mock invariants avg ${ms(avg(invA))}`];
  if (!cli.mockOnly) timing.push(`rust __invariants avg ${ms(avg(invB))}`, `__reset avg ${ms(avg(resets))}`);
  console.log(`parity: ${failed === 0 ? 'green' : `${failed} failing`} · ${timing.join(' · ')}`);

  if (cli.ingest) {
    const n = ingest(findings.filter((f) => f.unexplained.length || f.invariantBreaks.length), outDir);
    console.log(`parity: --ingest wrote ${n} finding(s) through scripts/diagnostics/run.ts --ingest`);
  }
  return failed === 0 ? 0 : 1;
}

process.exit(await main());
