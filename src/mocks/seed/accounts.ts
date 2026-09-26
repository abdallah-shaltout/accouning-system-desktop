import { db } from '../db';
import { accountsFixture, buildAccounts, fiscalYearFixture } from '../fixtures/accounts';
import { postJournal } from '../backend/core';
import { clone } from '../utils';
import type { CountryCode } from '@/modules/core/helpers/countryProfiles';

/**
 * Accounts area: chart of accounts (v2 standard template + country add-on — docs/v2/03-chart-of-
 * accounts.md §4) and fiscal years. Other seed areas (catalog, people, settings) don't depend on
 * this running first, but the "daily operations" history does — it posts against these accounts.
 *
 * v2 doc 18.D: `country` defaults to SA — unchanged for the pre-existing demo seed
 * (`seedDatabase()`, `bun run dev`'s "explore with demo data") and every golden number it produces.
 * `country: 'EG'` builds the same standard template *without* the SA-only zakat add-on (there's no
 * EG add-on yet — the standard template already fits) — used by `verify:mocks`'s EG invariant run.
 */
export function seedAccounts(now: Date, country: CountryCode = 'SA'): void {
  db.accounts = country === 'SA' ? clone(accountsFixture) : clone(buildAccounts({ template: 'standard', country }));
  db.fiscalYears = fiscalYearFixture(now);
}

/** Posts the opening capital journal. Split out since it needs a `date`/`createdBy`, unlike the rest of this area. */
export function postOpeningCapital(date: string, createdBy: string): void {
  postJournal({
    date,
    description: 'رأس المال الافتتاحي',
    type: 'MANUAL',
    lines: [
      { role: 'cash', debit: 20000 },
      { role: 'bank', debit: 180000 },
      { role: 'capital', credit: 200000 },
    ],
    createdBy,
  });
}
