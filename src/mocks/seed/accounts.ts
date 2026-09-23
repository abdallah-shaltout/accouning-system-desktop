import { db } from '../db';
import { accountsFixture, fiscalYearFixture } from '../fixtures/accounts';
import { postJournal } from '../backend/core';
import { clone } from '../utils';

/**
 * Accounts area: chart of accounts (v2 standard template + SA add-on — docs/v2/03-chart-of-
 * accounts.md §4) and fiscal years. Other seed areas (catalog, people, settings) don't depend on
 * this running first, but the "daily operations" history does — it posts against these accounts.
 */
export function seedAccounts(now: Date): void {
  db.accounts = clone(accountsFixture);
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
