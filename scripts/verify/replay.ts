/**
 * Headless replay of a repro bundle (18.F4): `bun run verify:replay <bundle.json>`.
 *
 * A bundle is exported from `/dev/diagnostics`'s "المحاسبة" tab ("تصدير حالة لإعادة الإنتاج",
 * `accountingDebugService.ts`'s `exportReproBundle`) — its `startSnapshot` is a full `MockDb` clone
 * taken when recording started, and `actions` is every wrapped service call made since, in order
 * (`src/modules/diagnostics/services/defineService.ts`'s `wrap()` records each one via
 * `actionJournal.ts` while recording is on).
 *
 * This script restores that snapshot into the mock backend's real `db` (in place — `db` is a
 * `const` object, so every field is overwritten rather than the reference replaced, exactly like
 * `src/mocks/persist.ts`'s snapshot loader does), imports every `modules/*\/services/*.ts` file so
 * `defineService.ts`'s registry is fully populated, then replays each action by looking its
 * `source` up in that registry and calling it with its recorded args — printing the invariants
 * after every step and stopping at the first one that breaks (so a bug report becomes a permanent,
 * runnable regression the moment someone can point at the exact step it broke on).
 *
 * Minimized bundles (irrelevant leading actions trimmed, PII scrubbed) move to
 * `scripts/verify/cases/*.json` as permanent regression cases — see `runCases()` below, called by
 * `bun run verify:replay` with no argument.
 */
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { db } from '../../src/mocks/db';
import { runAllInvariants } from '../../src/mocks/backend/invariants';
import { serviceRegistry } from '../../src/modules/diagnostics/services/defineService';
import type { ActionJournalEntry, ReproBundle } from '../../src/modules/diagnostics/services/actionJournal';

// Import every service module so wrap() has registered its exports before we look any of them up
// by name. import.meta.glob isn't available outside Vite, so this walks the same directories a
// codemod would — see scripts/diagnostics/wrapServices.ts for the equivalent write-side walk.
async function importAllServices(): Promise<void> {
  const modulesDir = join(import.meta.dirname, '../../src/modules');
  for (const moduleName of readdirSync(modulesDir, { withFileTypes: true })) {
    if (!moduleName.isDirectory()) continue;
    const servicesDir = join(modulesDir, moduleName.name, 'services');
    let files: string[];
    try {
      files = readdirSync(servicesDir).filter((f) => f.endsWith('.ts'));
    } catch {
      continue; // module has no services/ dir
    }
    for (const file of files) {
      await import(join(servicesDir, file));
    }
  }
}

function restoreSnapshot(snapshot: unknown): void {
  if (!snapshot || typeof snapshot !== 'object') throw new Error('bundle.startSnapshot is missing or not an object');
  Object.assign(db, snapshot);
}

interface StepResult {
  seq: number;
  source: string;
  ok: boolean;
  error?: string;
  invariantFailures: string[];
}

async function replayBundle(bundle: ReproBundle): Promise<{ steps: StepResult[]; firstBreak?: StepResult }> {
  restoreSnapshot(bundle.startSnapshot);
  const registry = serviceRegistry();
  const steps: StepResult[] = [];
  let firstBreak: StepResult | undefined;

  for (const action of bundle.actions as ActionJournalEntry[]) {
    const fn = registry.get(action.source);
    let ok = true;
    let error: string | undefined;
    if (!fn) {
      ok = false;
      error = `unknown service "${action.source}" — not found in the registry (renamed or moved since the bundle was recorded?)`;
    } else {
      try {
        const result = fn(...action.args);
        if (result instanceof Promise) await result;
      } catch (e) {
        ok = false;
        error = e instanceof Error ? e.message : String(e);
      }
    }

    const invariantFailures = runAllInvariants(db)
      .filter((r) => !r.passed)
      .map((r) => `${r.key}: ${r.message}`);

    const step: StepResult = { seq: action.seq, source: action.source, ok, error, invariantFailures };
    steps.push(step);

    if (!firstBreak && (!ok || invariantFailures.length > 0)) firstBreak = step;
  }

  return { steps, firstBreak };
}

async function runOne(path: string): Promise<boolean> {
  const bundle = JSON.parse(readFileSync(path, 'utf-8')) as ReproBundle;
  console.log(`--- replaying ${path} (${bundle.actions.length} action(s), recorded ${bundle.startedAt} -> ${bundle.endedAt}) ---`);
  const { steps, firstBreak } = await replayBundle(bundle);

  for (const s of steps) {
    const icon = s.ok && s.invariantFailures.length === 0 ? 'OK  ' : 'FAIL';
    console.log(`  ${icon}  [${s.seq}] ${s.source}${s.error ? ` — ${s.error}` : ''}`);
    for (const f of s.invariantFailures) console.log(`         invariant broken: ${f}`);
  }

  if (firstBreak) {
    console.log(`\nFIRST BREAK at step ${firstBreak.seq} (${firstBreak.source}).`);
    return false;
  }
  console.log(`\nall ${steps.length} step(s) replayed clean, invariants held throughout.`);
  return true;
}

/** No-argument mode: replays every minimized case in `scripts/verify/cases/` — the permanent
 * regression suite an `ACC-` issue's `regression_test` points at (18.F5). */
async function runCases(): Promise<boolean> {
  const casesDir = join(import.meta.dirname, 'cases');
  let files: string[];
  try {
    files = readdirSync(casesDir).filter((f) => f.endsWith('.json'));
  } catch {
    console.log('scripts/verify/cases/ does not exist yet — no regression cases to replay.');
    return true;
  }
  if (!files.length) {
    console.log('scripts/verify/cases/ is empty — no regression cases to replay.');
    return true;
  }
  let allOk = true;
  for (const file of files) {
    const ok = await runOne(join(casesDir, file));
    if (!ok) allOk = false;
    console.log('');
  }
  return allOk;
}

const arg = process.argv[2];
await importAllServices();
const ok = arg ? await runOne(arg) : await runCases();
process.exit(ok ? 0 : 1);
