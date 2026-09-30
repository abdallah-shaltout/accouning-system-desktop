/**
 * L1 platform lane. `saveTax`/`deleteTax` (01-settings.md §3 "Taxes"): create, update, the three
 * validation messages in order, default-switch behaviour, delete refusals (default / in-use), then
 * a clean delete.
 */
import { defineCase } from '../../case';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';

export default defineCase({
  name: 'settings/settings-tax-crud',
  source: '03-domains/01-settings.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('list-before', () => settingsService.getTaxes());

    const created = await s.step('create', () =>
      settingsService.saveTax({
        name: 'ضريبة تجريبية',
        rate: 5,
        type: 'OUTPUT',
        isDefault: false,
        active: true,
        category: 'S',
        direction: 'sales',
        accountRole: 'vatOutput',
      }),
    );

    await s.step('update', () => settingsService.saveTax({ ...created, name: 'ضريبة تجريبية معدّلة', rate: 7 }, created.id));

    // Making it the new default OUTPUT tax must clear the seeded default (`tax-vat-out`).
    await s.step('make-default', () => settingsService.saveTax({ ...created, name: 'ضريبة تجريبية معدّلة', rate: 7, isDefault: true }, created.id));
    await s.step('list-after-default-switch', () => settingsService.getTaxes());

    await s.expectError('blank-name', () =>
      settingsService.saveTax({ name: '   ', rate: 5, type: 'OUTPUT', isDefault: false, active: true, category: 'S', direction: 'sales', accountRole: 'vatOutput' }),
    );
    await s.expectError('bad-rate', () =>
      settingsService.saveTax({ name: 'ض', rate: 150, type: 'OUTPUT', isDefault: false, active: true, category: 'S', direction: 'sales', accountRole: 'vatOutput' }),
    );
    await s.expectError('exempt-no-reason', () =>
      settingsService.saveTax({ name: 'ض معفاة', rate: 0, type: 'OUTPUT', isDefault: false, active: true, category: 'E', direction: 'sales', accountRole: 'vatOutput' }),
    );
    await s.expectError('update-missing', () =>
      settingsService.saveTax({ name: 'غير موجودة', rate: 5, type: 'OUTPUT', isDefault: false, active: true, category: 'S', direction: 'sales', accountRole: 'vatOutput' }, 'tax-does-not-exist'),
    );

    // Delete refusals: current default, then a tax used by a posted invoice line.
    await s.expectError('delete-default', () => settingsService.deleteTax(created.id));
    await s.expectError('delete-in-use', () => settingsService.deleteTax(s.baseId('tax-vat-out')));

    // A clean delete of a never-used, non-default tax.
    const disposable = await s.step('create-disposable', () =>
      settingsService.saveTax({ name: 'ضريبة للحذف', rate: 2, type: 'INPUT', isDefault: false, active: true, category: 'S', direction: 'purchase', accountRole: 'vatInput' }),
    );
    await s.step('delete-disposable', () => settingsService.deleteTax(disposable.id));
    await s.step('list-final', () => settingsService.getTaxes());
  },
});
