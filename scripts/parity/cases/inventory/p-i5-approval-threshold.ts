/**
 * 06b-inventory §8(b) P-I5: `inventoryApprovalThreshold` — a STOCK_IN or LOSS whose value is at or
 * above it needs `approvedBy` (FORBIDDEN, value/threshold in the message); with a manager whose PIN
 * was just verified (the Rust-side G-P3 grant, D-I2) it posts and stores the approver; below the
 * threshold nothing is needed; drafts are not checked when saved.
 */
import { defineCase } from '../../case';
import * as authService from '../../../../src/modules/users/services/authService';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';

export default defineCase({
  name: 'inventory/p-i5-approval-threshold',
  source: '03-domains/06b-inventory.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    await s.step('set-threshold', () => settingsService.updateSettings({ inventoryApprovalThreshold: 500 }));
    // 30 × 18.0644 = 541.93 ≥ 500
    const gift = { type: 'STOCK_IN' as const, date, reason: 'gift' as const, lines: [{ productId: s.baseId('prd-14'), qtyChange: 30 }] };
    await s.expectError('stock-in-no-approver', () => inventoryService.createStockAdjustment(gift));
    // 3 × 185.4433 = 556.33 ≥ 500
    const loss = { type: 'LOSS' as const, date, lines: [{ productId: s.baseId('prd-6'), qtyChange: 3 }] };
    await s.expectError('loss-no-approver', () => inventoryService.createStockAdjustment(loss));
    // 27 × 18.0644 = 487.74 < 500 → no approver needed.
    await s.step('under-threshold', () =>
      inventoryService.createStockAdjustment({ type: 'STOCK_IN', date, reason: 'gift', lines: [{ productId: s.baseId('prd-14'), qtyChange: 27 }] }),
    );
    await s.step('draft-not-checked', () => inventoryService.createStockAdjustment(gift, true));

    await s.expectError('pin-wrong-password', () => authService.verifyManagerPin('manager', 'nope'));
    await s.expectError('pin-not-a-manager', () => authService.verifyManagerPin('cashier', 'cashier123'));
    const manager = await s.step('pin-manager', () => authService.verifyManagerPin('manager', 'manager123'));
    const approved = await s.step('stock-in-approved', () => inventoryService.createStockAdjustment({ ...gift, approvedBy: manager.id }));
    await s.step('stock-in-approved-detail', () => inventoryService.getStockAdjustment(approved.id));
    await s.step('loss-approved', () => inventoryService.createStockAdjustment({ ...loss, approvedBy: manager.id }));
    await s.step('list', () => inventoryService.getStockAdjustments({ from: '2026-06-30', to: '2026-06-30' }));
  },
});
