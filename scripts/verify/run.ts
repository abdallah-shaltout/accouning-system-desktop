/**
 * Runs every `scripts/verify/<area>.ts` invariant check against a freshly seeded mock DB.
 *
 *   bun run scripts/verify/run.ts            # every area (this is `bun run verify:mocks`)
 *   bun run scripts/verify/run.ts accounts    # one area only
 *
 * Each area's `run()` returns `Result`s: 'ok' (printed), 'fail' (printed, non-zero exit) or
 * 'todo' (printed, doesn't fail — invariants whose posting rules land in a later phase; see
 * docs/v2/02-accounting-review.md §4 and docs/v2/15-action-plan.md Phase 0).
 */
import { seedDatabase } from '../../src/mocks/seed';
import * as accounts from './accounts';
import * as inventory from './inventory';
import * as parties from './parties';
import * as sales from './sales';
import type { Result } from './shared';

const areas: Record<string, { run: () => Result[] }> = { accounts, inventory, parties, sales };

const only = process.argv[2];
const names = only ? [only] : Object.keys(areas);
for (const n of names) {
  if (!areas[n]) {
    console.error(`unknown area "${n}" — known areas: ${Object.keys(areas).join(', ')}`);
    process.exit(1);
  }
}

console.log('seeding demo database…');
seedDatabase();
console.log('');

let failCount = 0;
let todoCount = 0;
let okCount = 0;

for (const name of names) {
  console.log(`--- ${name} ---`);
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

console.log(`${okCount} ok, ${todoCount} todo, ${failCount} failed`);
process.exit(failCount > 0 ? 1 : 0);
