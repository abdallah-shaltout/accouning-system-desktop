/**
 * Runs every `scripts/verify/<area>.ts` invariant check against a freshly seeded mock DB.
 *
 *   bun run scripts/verify/run.ts            # every area, SA then EG (this is `bun run verify:mocks`)
 *   bun run scripts/verify/run.ts accounts    # one area only, SA then EG
 *
 * Each area's `run()` returns `Result`s: 'ok' (printed), 'fail' (printed, non-zero exit) or
 * 'todo' (printed, doesn't fail — invariants whose posting rules land in a later phase; see
 * docs/v2/02-accounting-review.md §4 and docs/v2/15-action-plan.md Phase 0).
 *
 * v2 doc 18.D: runs the *entire* suite twice — once against the SA demo seed (15% VAT, SAR) and once
 * against an EG seed (14% VAT, EGP) — so every accounting invariant (balanced entries, trial balance,
 * VAT control, FX, …) is proven to hold for both, not only the Saudi numbers this script always
 * exercised before. Same replay (`seedHistory`/`seedPurchases8`/…), same posting rules — only the
 * seeded rate/currency differ, since a wrong rate/currency should still trip every one of these
 * checks the same way (they compare GL balances to recomputed sums, not to hard-coded expected
 * totals — except the branches area's one pinned FX example, which is currency-agnostic by numbers).
 */
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { seedDatabase } from '../../src/mocks/seed';
import type { CountryCode } from '../../src/modules/core/helpers/countryProfiles';
import * as accounts from './accounts';
import * as inventory from './inventory';
import * as parties from './parties';
import * as sales from './sales';
import * as branches from './branches';
import type { Result } from './shared';

const areas: Record<string, { run: () => Result[] }> = { accounts, inventory, parties, sales, branches };

/** 18.G: every `fail` result becomes an `ACC-` ledger issue, through the same
 * `scripts/diagnostics/run.ts --ingest` upsert mechanism e2e/perf findings use (see
 * plans/pending/18-countries-a11y-diagnostics/phase-g-dev-loop.md) — reused, not duplicated, per
 * Phase F's own note that this was the one piece it left unwired. */
function reportFailuresToLedger(failures: { area: string; country: string; message: string }[]): void {
  if (failures.length === 0) return;
  const findings = failures.map((f) => ({
    kind: 'accounting' as const,
    fingerprint: `acc:${f.area}:${f.message.slice(0, 120)}`,
    area: f.area,
    title: `verify:mocks (${f.country}) — ${f.message}`,
    body: [
      `المصدر: \`scripts/verify/${f.area}.ts\` (seed: ${f.country})`,
      '',
      '### الفحص الفاشل',
      '',
      '```',
      f.message,
      '```',
      '',
      'راجع `docs/v2/02-accounting-review.md` §4 قبل التعديل. لا يصبح هذا العنصر `verified` إلا بعد',
      'إضافة حالة انحدار في `scripts/verify/cases/*.json` (CLAUDE.md "Accounting safety").',
    ].join('\n'),
  }));
  const tmp = path.join(os.tmpdir(), `verify-mocks-findings-${Date.now()}.json`);
  fs.writeFileSync(tmp, JSON.stringify(findings));
  try {
    execFileSync('bun', ['run', 'scripts/diagnostics/run.ts', '--ingest', tmp], { stdio: 'inherit' });
  } finally {
    fs.unlinkSync(tmp);
  }
}

const only = process.argv.slice(2).find((a) => !a.startsWith('--'));
const names = only ? [only] : Object.keys(areas);
for (const n of names) {
  if (!areas[n]) {
    console.error(`unknown area "${n}" — known areas: ${Object.keys(areas).join(', ')}`);
    process.exit(1);
  }
}

let failCount = 0;
let todoCount = 0;
let okCount = 0;
const failures: { area: string; country: string; message: string }[] = [];

const COUNTRIES: CountryCode[] = ['SA', 'EG'];
for (const country of COUNTRIES) {
  console.log(`seeding demo database (${country})…`);
  seedDatabase(new Date(), country);
  console.log('');

  for (const name of names) {
    console.log(`--- ${name} (${country}) ---`);
    const results = areas[name].run();
    for (const r of results) {
      const icon = r.status === 'ok' ? 'OK  ' : r.status === 'fail' ? 'FAIL' : 'TODO';
      console.log(`  ${icon}  ${r.message}`);
      if (r.status === 'fail') {
        failCount++;
        failures.push({ area: name, country, message: r.message });
      } else if (r.status === 'todo') todoCount++;
      else okCount++;
    }
    console.log('');
  }
}

console.log(`${okCount} ok, ${todoCount} todo, ${failCount} failed`);
if (!process.argv.includes('--no-ledger')) reportFailuresToLedger(failures);
process.exit(failCount > 0 ? 1 : 0);
