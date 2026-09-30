/**
 * 12-accounting §8(b): `getJournalEntry`'s joined fields (§3.1, §3.4) — `createdByName`,
 * `sourceLabel` and `sourceLink` for each source kind (invoice, refund → its invoice, purchase
 * order, payment → `payments?highlight=`, stock adjustment, expense → none, manual → none),
 * `reversedById/Number` (after a reversal in the story), and `related` (other posted entries of the
 * same source — empty for every seeded document). Plus `getJournalEntriesForSource`.
 *
 * A non-empty `related` needs two posted entries on one source; the only path that makes one is a
 * payment allocated later to an FX document (its realized-FX entry reuses the payment source).
 * That path is lane L3's `payments/payment-allocate-later-fx`; on the mock it currently breaks the
 * `one-active-entry`, `allocations-within-total` and `fx-conversion` invariants (reported by lane
 * L4, 2026-09-29), so it is not repeated here.
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import type { JournalSourceKind } from '../../../../src/modules/accounting/types';

export default defineCase({
  name: 'accounting/journal-detail-related',
  source: '03-domains/12-accounting.md §8(b)',
  base: 'demo-sa',
  user: 'accountant',
  async run(s) {
    // One seeded entry of each source kind, the latest of each.
    const kinds: JournalSourceKind[] = ['invoice', 'refund', 'purchaseOrder', 'purchaseReturn', 'payment', 'stockAdjustment', 'expense', 'voucher', 'shift'];
    for (const kind of kinds) {
      const rows = await s.step(`list-${kind}`, async () => (await accountingService.getJournalEntries({ sourceKind: kind })).slice(0, 1));
      if (rows[0]) {
        await s.step(`detail-${kind}`, () => accountingService.getJournalEntry(rows[0].id));
        await s.step(`for-source-${kind}`, () => accountingService.getJournalEntriesForSource(kind, rows[0].sourceRef!.id));
      }
    }
    const manual = await s.step('list-manual', async () => (await accountingService.getJournalEntries({ type: 'MANUAL' })).slice(0, 1));
    await s.step('detail-manual', () => accountingService.getJournalEntry(manual[0].id));
    await s.step('for-source-unknown', () => accountingService.getJournalEntriesForSource('invoice', 'inv-missing'));
    await s.expectError('detail-unknown', () => accountingService.getJournalEntry('je-missing'));
    const reversal = await s.step('reverse-manual', () => accountingService.reverseJournalEntry(manual[0].id, '2026-06-30', 'تصحيح'));
    await s.step('detail-reversed-original', () => accountingService.getJournalEntry(manual[0].id));
    await s.step('detail-reversal', () => accountingService.getJournalEntry(reversal.id));
  },
});
