/**
 * 07-purchases §8(b) P-P4 (other-supplier part): a landed-cost line billed by a different supplier
 * (a shipping company) posts its own `Cr payable` line tagged with that supplier.
 *
 * KNOWN MOCK BUG (lane L2 report, 2026-09-29 — not fixed here, report-only): that AP amount has no
 * payable document behind it (no PO/bill for the shipper), so the shipper's `supplierBalance()` is
 * 77.77 while Σ open documents is 0 → invariant `supplier-allocation` breaks, and the amount can
 * never be settled through a payment allocation. This case fails on the mock until that is decided
 * and fixed (07-purchases Q-U4 area; needs an ACC- issue per P4-7).
 */
import { defineCase } from '../../case';
import * as purchaseService from '../../../../src/modules/purchases/services/purchaseService';
import * as partyService from '../../../../src/modules/parties/services/partyService';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'purchases/p-p4-other-supplier-landed',
  source: '03-domains/07-purchases.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: '**.phones', emptyArrayOnly: true, reason: '05-parties D-6: phones is always an array in Rust; the mock leaves it absent on a party saved without phones (absent ≡ [])' },
  ],
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const sup = s.baseId('sup-4');
    const shipper = await s.step('shipper', () =>
      partyService.saveSupplier({ type: 'company', name: 'شركة الشحن السريع', vatNumber: '300123123100003', active: true }),
    );
    const po = await s.step('save-and-confirm', () =>
      purchaseService.savePurchaseOrder({
        supplierId: sup,
        date,
        confirm: true,
        lines: [
          { productId: s.baseId('prd-23'), qty: 7, costPrice: 33.33 },
          { productId: s.baseId('prd-24'), qty: 5, costPrice: 47.1 },
        ],
        landedCosts: [
          { label: 'شحن المورد', amount: 20, spreadBy: 'value' },
          { label: 'تخليص جمركي', amount: 77.77, spreadBy: 'qty', supplierId: shipper.id },
        ],
      }),
    );
    await s.step('journal', () => accountingService.getJournalEntriesForSource('purchaseOrder', po.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('shipper-balance', () => partyService.getSupplier(shipper.id));
    await s.step('shipper-open-documents', () => paymentService.getOpenDocuments('supplier', shipper.id));
    await s.step('shipper-statement', () => partyService.getSupplierStatement(shipper.id));
  },
});
