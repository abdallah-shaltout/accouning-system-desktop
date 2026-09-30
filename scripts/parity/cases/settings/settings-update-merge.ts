/**
 * L1 platform lane (phase-b2-parity-cases.md). Exercises `settingsService.updateSettings`: a plain
 * field patch, a nested `printer` merge, and the two validation messages (01-settings.md §3
 * `update_settings` steps 1-2).
 */
import { defineCase } from '../../case';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';

export default defineCase({
  name: 'settings/settings-update-merge',
  source: '03-domains/01-settings.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('before', () => settingsService.getSettings());
    await s.step('update-store-name', () => settingsService.updateSettings({ storeName: 'متجر الاختبار' }));
    // Nested printer merge: only `a4PrinterName` changes, `thermal`/`labelPrinterName` (if any) survive.
    await s.step('update-printer', () => settingsService.updateSettings({ printer: { a4PrinterName: 'HP LaserJet' } }));
    await s.step('after', () => settingsService.getSettings());
    await s.expectError('blank-store-name', () => settingsService.updateSettings({ storeName: '   ' }));
    // EG-style invalid VAT number against the current (SA) profile — country unchanged.
    await s.expectError('bad-vat-number', () => settingsService.updateSettings({ vatNumber: '123' }));
  },
});
