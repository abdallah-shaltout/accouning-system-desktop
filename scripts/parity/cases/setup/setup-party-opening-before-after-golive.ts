/**
 * 02-setup §8(b): a party's opening balance from the party form (§3.7) — before the go-live date
 * the counter account is opening-balance equity (3900); after it, capital directly (with the
 * "بعد تاريخ البدء" note); customer debit, supplier credit, and the mirrored sides; a zero amount
 * writes nothing (`undefined`). Read back through the entries, the party balances and statements.
 *
 * Known red on the mock (reported by lane L4, 2026-09-29): `customer-allocation` /
 * `supplier-allocation` (§4.6) compare only documents and ignore opening balances, and
 * `opening-balance-equity` (§4.9) is checked on a finished company, so every party opening trips
 * them. The postings themselves follow docs/v2/05 §4.
 */
import { defineCase } from '../../case';
import * as setupService from '../../../../src/modules/setup/services/setupService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'setup/setup-party-opening-before-after-golive',
  source: '03-domains/02-setup.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const customer = s.baseId('cus-4');
    const supplier = s.baseId('sup-2');
    await s.step('set-go-live', () => setupService.saveOnboardingProgress({ goLiveDate: '2026-04-15' }));
    await s.step('zero', async () => ({ entryId: (await setupService.postPartyOpening({ partyKind: 'customer', partyId: customer, amount: 0, side: 'debit', asOfDate: '2026-04-10' })) ?? null }));
    const cBefore = await s.step('customer-before-golive', () =>
      setupService.postPartyOpening({ partyKind: 'customer', partyId: customer, amount: 2750.25, side: 'debit', asOfDate: '2026-04-10' }),
    );
    const cAfter = await s.step('customer-after-golive-credit', () =>
      setupService.postPartyOpening({ partyKind: 'customer', partyId: customer, amount: 300, side: 'credit', asOfDate: '2026-05-01' }),
    );
    const sBefore = await s.step('supplier-before-golive', () =>
      setupService.postPartyOpening({ partyKind: 'supplier', partyId: supplier, amount: 5400, side: 'credit', asOfDate: '2026-04-15' }),
    );
    const sAfter = await s.step('supplier-after-golive-debit', () =>
      setupService.postPartyOpening({ partyKind: 'supplier', partyId: supplier, amount: 125.5, side: 'debit', asOfDate: '2026-06-01' }),
    );
    for (const [name, id] of [['c-before', cBefore], ['c-after', cAfter], ['s-before', sBefore], ['s-after', sAfter]] as const) {
      await s.step(`entry-${name}`, () => accountingService.getJournalEntry(id!));
    }
    await s.step('equity-net', () => setupService.getOpeningBalanceEquityNet());
    await s.step('customer', () => partyService.getCustomer(customer));
    await s.step('customer-statement', () => partyService.getCustomerStatement(customer));
    await s.step('supplier', () => partyService.getSupplier(supplier));
    await s.step('supplier-statement', () => partyService.getSupplierStatement(supplier));
  },
});
