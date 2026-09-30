/**
 * L1 platform lane. Payment methods: create, reorder (with an unknown id mixed in, ignored), the two
 * validation messages, delete refusals (`canDelete=false`, in-use), then a clean delete.
 * Every input carries `active` (required by `PaymentMethodInput`; Wave 2 fix — without it Rust's
 * args decode failed as INTERNAL and the mock stored a row with no `active` key, i.e. inactive), and
 * the seeded ids go through `s.baseId` so Rust sees its imported UUIDs.
 */
import { defineCase } from '../../case';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';

export default defineCase({
  name: 'settings/settings-payment-methods-order',
  source: '03-domains/01-settings.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('list-before', () => settingsService.getPaymentMethods());

    const created = await s.step('create', () =>
      settingsService.savePaymentMethod({
        name: 'محفظة تجريبية',
        type: 'wallet',
        accountRole: 'walletClearing',
        feePct: 1.5,
        showInPos: true,
        showInPayments: true,
        sortOrder: 99,
        active: true,
      }),
    );

    await s.step('update', () => settingsService.savePaymentMethod({ ...created, name: 'محفظة تجريبية معدّلة' }, created.id));

    const before = await s.step('list-before-reorder', () => settingsService.getPaymentMethods());
    const ids = before.map((m) => m.id).reverse();
    await s.step('reorder', () => settingsService.reorderPaymentMethods([...ids, 'pm-does-not-exist']));
    await s.step('list-after-reorder', () => settingsService.getPaymentMethods());

    await s.expectError('blank-name', () =>
      settingsService.savePaymentMethod({ name: '  ', type: 'cash', accountRole: 'cash', feePct: 0, showInPos: true, showInPayments: true, sortOrder: 1, active: true }),
    );
    await s.expectError('bad-fee', () =>
      settingsService.savePaymentMethod({ name: 'طريقة', type: 'card', accountRole: 'cardClearing', feePct: 250, showInPos: true, showInPayments: true, sortOrder: 1, active: true }),
    );
    await s.expectError('update-missing', () =>
      settingsService.savePaymentMethod({ name: 'طريقة', type: 'card', accountRole: 'cardClearing', feePct: 1, showInPos: true, showInPayments: true, sortOrder: 1, active: true }, 'pm-does-not-exist'),
    );

    // `pm-cash` is the seeded, non-deletable default; `pm-credit` is used by seeded credit sales.
    await s.expectError('delete-cannot-delete', () => settingsService.deletePaymentMethod(s.baseId('pm-cash')));
    await s.expectError('delete-in-use', () => settingsService.deletePaymentMethod(s.baseId('pm-credit')));

    await s.step('delete-disposable', () => settingsService.deletePaymentMethod(created.id));
    await s.step('list-final', () => settingsService.getPaymentMethods());
  },
});
