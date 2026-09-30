/**
 * 02-setup §8(b): reversing a party opening balance (§3.7) — the mirror entry (OPENING, dated the
 * UTC day of "now", quirk Q-6; description "عكس: …"), the original flagged `reversed`, the party
 * balance back to where it was; refusals: unknown entry (NOT_FOUND) and an entry that is not a
 * party opening (decision D-10: a manual entry, and the demo's opening-close entry that has no
 * party line). Not exercised: a second reversal (quirk Q-7: the mock reverses again, Rust refuses)
 * and the allocated-opening refusal (no payment service allocates to an `opening` target).
 *
 * Known red on the mock for the same reason as `setup/setup-party-opening-before-after-golive`
 * (§4.6 party-allocation invariants ignore opening balances; reported by lane L4, 2026-09-29).
 * The opening is posted after go-live (to capital) so 3900 stays zero.
 */
import { defineCase } from '../../case';
import * as setupService from '../../../../src/modules/setup/services/setupService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'setup/setup-party-opening-reverse',
  source: '03-domains/02-setup.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const customer = s.baseId('cus-5');
    await s.step('set-go-live', () => setupService.saveOnboardingProgress({ goLiveDate: '2026-04-15' }));
    await s.step('customer-before', () => partyService.getCustomer(customer));
    const entryId = await s.step('post', () => setupService.postPartyOpening({ partyKind: 'customer', partyId: customer, amount: 999.99, side: 'debit', asOfDate: '2026-05-20' }));
    await s.step('customer-with-opening', () => partyService.getCustomer(customer));
    await s.expectError('unknown', () => setupService.reversePartyOpening('je-missing'));
    const manual = await s.step('a-manual-entry', async () => (await accountingService.getJournalEntries({ type: 'MANUAL' })).slice(0, 1));
    await s.expectError('not-opening-manual', () => setupService.reversePartyOpening(manual[0].id));
    const close = await s.step('opening-close-entry', async () => (await accountingService.getJournalEntries({ type: 'CLOSING' })).slice(0, 1));
    await s.expectError('not-party-opening', () => setupService.reversePartyOpening(close[0].id));
    await s.step('reverse', () => setupService.reversePartyOpening(entryId!));
    await s.step('original', () => accountingService.getJournalEntry(entryId!));
    await s.step('opening-entries', () => accountingService.getJournalEntries({ type: 'OPENING', from: '2026-05-01' }));
    await s.step('customer-after', () => partyService.getCustomer(customer));
  },
});
