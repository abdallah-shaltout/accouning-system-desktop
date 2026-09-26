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
import { seedDatabase } from '../../src/mocks/seed';
import type { CountryCode } from '../../src/modules/core/helpers/countryProfiles';
import * as accounts from './accounts';
import * as inventory from './inventory';
import * as parties from './parties';
import * as sales from './sales';
import * as branches from './branches';
import type { Result } from './shared';

const areas: Record<string, { run: () => Result[] }> = { accounts, inventory, parties, sales, branches };

const only = process.argv[2];
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
      if (r.status === 'fail') failCount++;
      else if (r.status === 'todo') todoCount++;
      else okCount++;
    }
    console.log('');
  }
}

console.log(`${okCount} ok, ${todoCount} todo, ${failCount} failed`);
process.exit(failCount > 0 ? 1 : 0);
