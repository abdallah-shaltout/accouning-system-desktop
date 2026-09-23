import { db } from '../db';
import { accountGroupsFixture, accountsFixture, fiscalYearFixture } from '../fixtures/accounts';
import { postJournal } from '../backend/core';
import { clone } from '../utils';

/**
 * Accounts area: chart of accounts, account groups, fiscal years, and the opening capital entry.
 * Other seed areas (catalog, people, settings) don't depend on this running first, but the
 * "daily operations" history does — it posts against these accounts.
 */
export function seedAccounts(now: Date): void {
  db.accountGroups = clone(accountGroupsFixture);
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
      { code: '1110', debit: 20000 },
      { code: '1120', debit: 180000 },
      { code: '3100', credit: 200000 },
    ],
    createdBy,
  });
}
