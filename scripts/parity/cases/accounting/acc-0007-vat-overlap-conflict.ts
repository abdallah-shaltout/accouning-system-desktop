/**
 * ACC-0007 verification (P4-7, 12b D-A6 / quirk Q5): a VAT settlement whose period overlaps a live
 * settlement is refused with CONFLICT, and a voided (reversed) settlement frees its period.
 *
 * Base: the SA demo plus three settlement entries posted through the mock's own
 * `postVatSettlement` — April (then voided with a mirror + `reversed`, the shape `ledger::reverse`
 * produces), May (live) and 2026-06-10…06-20 (live). The case only submits periods that must be
 * refused: every *successful* settlement moves `vatOutput`/`vatInput`, which the mock's §4.5
 * invariant (Σ document VAT, settlements not excluded) counts as a new break — reported by lane L4,
 * 2026-09-29. Freeing is proven by a refusal: 04-15…05-15 overlaps the voided April settlement and
 * the live May one, and the message names May — April (posted first) would be named if it still
 * counted.
 *
 * Both overlap shapes of the issue file are refused: (1) the new period starts inside an earlier
 * settled period (05-15…06-05 vs May), (2) the earlier settlement is dated (its `to`) outside the
 * new period (06-01…06-12 vs 06-10…06-20), plus the exact same period.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import { db, session, type MockDb } from '../../../../src/mocks/db';
import { seedDatabase } from '../../../../src/mocks/seed';
import { clone, resetIdCounters } from '../../../../src/mocks/utils';
import { postVatSettlement } from '../../../../src/mocks/backend/journal';
import { postJournal } from '../../../../src/mocks/backend/core';
import { pinClock } from '../../clock';
import { SEED_NOW } from '../../bases';

function baseWithSettlements(): MockDb {
  // Same seed call as the `demo-sa` base, under the SA zone and fresh id counters so the ids do
  // not depend on which base this process built first. `bases.ts` unpins after this returns.
  pinClock(SEED_NOW, 'Asia/Riyadh', 'base:acc-0007');
  resetIdCounters();
  seedDatabase(new Date(SEED_NOW), 'SA');
  db.settings.country ??= 'SA';
  session.userId = 'usr-3';
  const april = postVatSettlement('2026-04-01', '2026-04-30', 'usr-3');
  const mirror = postJournal({
    date: '2026-04-30',
    description: `عكس القيد ${april.number} — ${april.description}`,
    type: 'VAT_SETTLEMENT',
    lines: april.lines.map((l) => ({ accountId: l.accountId, description: l.description, debit: l.credit, credit: l.debit })),
    createdBy: 'usr-3',
  });
  mirror.reversalOfId = april.id;
  mirror.reversalReason = 'إلغاء تسوية أبريل';
  april.reversed = true;
  april.reversalReason = 'إلغاء تسوية أبريل';
  postVatSettlement('2026-05-01', '2026-05-31', 'usr-3');
  postVatSettlement('2026-06-10', '2026-06-20', 'usr-3');
  session.userId = '';
  return clone(db);
}

export default defineCase({
  name: 'accounting/acc-0007-vat-overlap-conflict',
  source: 'docs/diagnostics/issues/ACC-0007-vat-double-settlement.md; 03-domains/12b-period-close.md §7 Q5',
  base: baseWithSettlements,
  user: 'accountant',
  async run(s) {
    await s.step('settlements', () => accountingService.getJournalEntries({ type: 'VAT_SETTLEMENT' }));
    await s.expectError('overlap-starts-inside', () => accountingService.submitVatSettlement('2026-05-15', '2026-06-05'));
    await s.expectError('overlap-earlier-dated-outside', () => accountingService.submitVatSettlement('2026-06-01', '2026-06-12'));
    await s.expectError('same-period', () => accountingService.submitVatSettlement('2026-05-01', '2026-05-31'));
    await s.expectError('inside-period', () => accountingService.submitVatSettlement('2026-06-12', '2026-06-15'));
    await s.expectError('voided-april-is-free', () => accountingService.submitVatSettlement('2026-04-15', '2026-05-15'));
    await s.login('admin');
    // The admin's closed-period override does not bypass the overlap guard.
    await s.expectError('overlap-admin', () => accountingService.submitVatSettlement('2026-05-31', '2026-06-30'));
    await s.step('totals-may', () => accountingService.getVatPeriodTotals('2026-05-01', '2026-05-31'));
  },
});
