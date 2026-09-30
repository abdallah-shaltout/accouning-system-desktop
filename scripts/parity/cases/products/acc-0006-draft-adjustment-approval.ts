/**
 * ACC-0006 verification (plan 21 Part 04 L2-0, P4-7; issue docs/diagnostics/issues/ACC-0006-draft-
 * adjustment-approval.md): completing a DRAFT stock adjustment whose value is at or above
 * `inventoryApprovalThreshold` is refused without an approver — the same FORBIDDEN message as a
 * direct adjustment — and posts once a manager approves (the manager's PIN is verified first, which
 * is what creates the Rust-side G-P3 grant, 06b D-I2).
 */
import { defineCase } from '../../case';
import * as authService from '../../../../src/modules/users/services/authService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';

export default defineCase({
  name: 'products/acc-0006-draft-adjustment-approval',
  source: 'docs/diagnostics/issues/ACC-0006-draft-adjustment-approval.md · 03-domains/06b-inventory.md §8(b) P-I5',
  base: 'demo-sa',
  async run(s) {
    await s.step('set-threshold', () => settingsService.updateSettings({ inventoryApprovalThreshold: 1000 }));
    const p = await s.step('product', () =>
      productService.createProduct({ name: 'صنف اختبار الاعتماد', sku: 'APR-001', type: 'product', costPrice: 10, price: 20, active: true }),
    );
    const input = {
      type: 'STOCK_IN' as const,
      date: '2026-06-30T09:00:00.000Z',
      reason: 'found' as const,
      note: 'فائض',
      lines: [{ productId: p.id, qtyChange: 200 }],
    };
    // The direct path refuses at 2000 ≥ 1000 without an approver.
    await s.expectError('direct-over-threshold', () => inventoryService.createStockAdjustment(input));
    // A draft posts nothing, so it is not checked…
    const draft = await s.step('save-draft', () => inventoryService.createStockAdjustment(input, true));
    // …but completing it posts, so it is (the ACC-0006 fix).
    await s.expectError('complete-without-approver', () => inventoryService.completeAdjustment(draft.id));
    await s.step('still-draft', () => inventoryService.getStockAdjustment(draft.id));
    await s.step('product-unchanged', () => productService.getProduct(p.id));

    const manager = await s.step('verify-manager-pin', () => authService.verifyManagerPin('manager', 'manager123'));
    await s.step('complete-with-approver', () => inventoryService.completeAdjustment(draft.id, manager.id));
    await s.step('completed', () => inventoryService.getStockAdjustment(draft.id));
    await s.step('journal', () => accountingService.getJournalEntriesForSource('stockAdjustment', draft.id).then((ls) => Promise.all(ls.map((l) => accountingService.getJournalEntry(l.id)))));
    await s.step('product-after', () => productService.getProduct(p.id));
    await s.expectError('complete-twice', () => inventoryService.completeAdjustment(draft.id, manager.id));
  },
});
