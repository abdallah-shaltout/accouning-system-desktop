/**
 * L1 platform lane. Currency + exchange-rate CRUD (01-settings.md §3 "Currency"): create, the two
 * refusal messages, update, rate save + same-day resave (moves to the end of the list), the
 * base-currency lock (already tripped in `demo-sa` — journal entries exist). Currency CRUD lives in
 * `branchesService.ts` (thin wrapper over `mocks/backend/currency.ts`), not `settingsService.ts`.
 * `demo-sa` already seeds `USD` with two days of rate history (v2 phase 9, `branches9.ts`), so this
 * case uses `EUR` for the fresh-currency assertions and layers new dates onto `USD`'s existing rates.
 */
import { defineCase } from '../../case';
import * as branchesService from '../../../../src/modules/settings/services/branchesService';

export default defineCase({
  name: 'settings/settings-currency-rates',
  source: '03-domains/01-settings.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('list-before', () => branchesService.getCurrencies());
    await s.step('locked-before', () => branchesService.isBaseCurrencyLocked());

    const eur = await s.step('create-eur', () => branchesService.createCurrency({ code: 'eur', nameAr: 'يورو', symbol: '€', decimals: 2, active: true }));

    await s.expectError('create-base-currency', () => branchesService.createCurrency({ code: 'SAR', nameAr: 'ريال', symbol: 'ر.س', decimals: 2, active: true }));
    await s.expectError('create-duplicate', () => branchesService.createCurrency({ code: 'USD', nameAr: 'دولار', symbol: '$', decimals: 2, active: true }));

    await s.step('update', () => branchesService.updateCurrency(eur.code, { active: false }));
    await s.expectError('update-missing', () => branchesService.updateCurrency('ZZZ', { active: true }));

    await s.step('save-rate', () => branchesService.saveExchangeRate({ currency: 'EUR', date: '2026-06-15', rate: 4.05 }));
    // Same-day resave: replaces (moves to the end of the array) rather than adding a second row.
    await s.step('resave-rate-same-day', () => branchesService.saveExchangeRate({ currency: 'EUR', date: '2026-06-15', rate: 4.06 }));
    await s.step('save-rate-inverse', () => branchesService.saveExchangeRate({ currency: 'EUR', date: '2026-06-16', inverseRate: 0.25 }));
    await s.step('list-rates-eur', () => branchesService.getExchangeRates('EUR'));
    await s.step('list-rates-usd', () => branchesService.getExchangeRates('USD'));
    await s.step('list-rates-all', () => branchesService.getExchangeRates());

    await s.expectError('rate-unknown-currency', () => branchesService.saveExchangeRate({ currency: 'ZZZ', date: '2026-06-16', rate: 1 }));
    await s.expectError('rate-zero', () => branchesService.saveExchangeRate({ currency: 'EUR', date: '2026-06-17', rate: 0 }));

    // The base currency is already locked in `demo-sa` (posted journal entries exist).
    await s.expectError('set-base-currency-locked', () => branchesService.setBaseCurrency('EUR'));

    await s.step('list-final', () => branchesService.getCurrencies());
  },
});
