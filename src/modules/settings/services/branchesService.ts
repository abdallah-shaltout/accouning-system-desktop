/**
 * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md): thin service wrappers around
 * `src/mocks/backend/branches.ts` / `currency.ts` / `revaluation.ts`, matching every other
 * module's services file shape — pages never import `src/mocks/backend` directly.
 */
import { clone, db, delay, session } from '@/mocks';
import * as backend from '@/mocks/backend/branches';
import * as currencyBackend from '@/mocks/backend/currency';
import * as revalBackend from '@/mocks/backend/revaluation';
import type { Branch, BranchInput, CostCenter, CostCenterInput, Currency, ExchangeRate, ExchangeRateInput } from '../types';

import { backendCall, usesRust } from '@/modules/core/services/backend';
import { wrap } from '@/modules/diagnostics/services/defineService';

// --- Branches ------------------------------------------------------------------------------

export const getBranches = wrap('settings.getBranches', async function getBranches(): Promise<Branch[]> {
  if (usesRust('settings')) return backendCall('settings_get_branches');
  await delay(100);
  return clone(backend.listBranches());
});

export const createBranch = wrap('settings.createBranch', async function createBranch(input: BranchInput): Promise<Branch> {
  if (usesRust('settings')) return backendCall('settings_create_branch', { input });
  await delay();
  return clone(backend.createBranch(input, session.userId));
});

export const updateBranch = wrap('settings.updateBranch', async function updateBranch(id: string, input: Partial<BranchInput>): Promise<Branch> {
  if (usesRust('settings')) return backendCall('settings_update_branch', { id, input });
  await delay();
  return clone(backend.updateBranch(id, input, session.userId));
});

export const deactivateBranch = wrap('settings.deactivateBranch', async function deactivateBranch(id: string): Promise<Branch> {
  if (usesRust('settings')) return backendCall('settings_deactivate_branch', { id });
  await delay();
  return clone(backend.deactivateBranch(id, session.userId));
});

export const reactivateBranch = wrap('settings.reactivateBranch', async function reactivateBranch(id: string): Promise<Branch> {
  if (usesRust('settings')) return backendCall('settings_reactivate_branch', { id });
  await delay();
  return clone(backend.reactivateBranch(id, session.userId));
});

// --- Cost centers ----------------------------------------------------------------------------

export const getCostCenters = wrap('settings.getCostCenters', async function getCostCenters(): Promise<CostCenter[]> {
  if (usesRust('settings')) return backendCall('settings_get_cost_centers');
  await delay(100);
  return clone(backend.listCostCenters());
});

export const createCostCenter = wrap('settings.createCostCenter', async function createCostCenter(input: CostCenterInput): Promise<CostCenter> {
  if (usesRust('settings')) return backendCall('settings_create_cost_center', { input });
  await delay();
  return clone(backend.createCostCenter(input, session.userId));
});

export const updateCostCenter = wrap('settings.updateCostCenter', async function updateCostCenter(id: string, input: Partial<CostCenterInput>): Promise<CostCenter> {
  if (usesRust('settings')) return backendCall('settings_update_cost_center', { id, input });
  await delay();
  return clone(backend.updateCostCenter(id, input, session.userId));
});

export const deleteCostCenter = wrap('settings.deleteCostCenter', async function deleteCostCenter(id: string): Promise<void> {
  if (usesRust('settings')) {
    await backendCall('settings_delete_cost_center', { id });
    return;
  }
  await delay();
  backend.deleteCostCenter(id, session.userId);
});

// --- Currencies ------------------------------------------------------------------------------

export const getCurrencies = wrap('settings.getCurrencies', async function getCurrencies(): Promise<Currency[]> {
  if (usesRust('settings')) return backendCall('settings_get_currencies');
  await delay(100);
  return clone(db.currencies);
});

export const getExchangeRates = wrap('settings.getExchangeRates', async function getExchangeRates(currency?: string): Promise<ExchangeRate[]> {
  if (usesRust('settings')) return backendCall('settings_get_exchange_rates', { currency });
  await delay(100);
  return clone(currency ? db.exchangeRates.filter((r) => r.currency === currency) : db.exchangeRates);
});

export const createCurrency = wrap('settings.createCurrency', async function createCurrency(input: Currency): Promise<Currency> {
  if (usesRust('settings')) return backendCall('settings_create_currency', { input });
  await delay();
  return clone(currencyBackend.createCurrency(input, session.userId));
});

export const updateCurrency = wrap('settings.updateCurrency', async function updateCurrency(code: string, patch: Partial<Currency>): Promise<Currency> {
  if (usesRust('settings')) return backendCall('settings_update_currency', { code, patch });
  await delay();
  return clone(currencyBackend.updateCurrency(code, patch));
});

export const saveExchangeRate = wrap('settings.saveExchangeRate', async function saveExchangeRate(input: ExchangeRateInput): Promise<ExchangeRate> {
  if (usesRust('settings')) return backendCall('settings_save_exchange_rate', { input });
  await delay();
  return clone(currencyBackend.saveExchangeRate(input));
});

export const isBaseCurrencyLocked = wrap('settings.isBaseCurrencyLocked', async function isBaseCurrencyLocked(): Promise<boolean> {
  if (usesRust('settings')) return backendCall('settings_is_base_currency_locked');
  await delay(50);
  return currencyBackend.isBaseCurrencyLocked();
});

export const setBaseCurrency = wrap('settings.setBaseCurrency', async function setBaseCurrency(code: string): Promise<void> {
  if (usesRust('settings')) {
    await backendCall('settings_set_base_currency', { code });
    return;
  }
  await delay();
  currencyBackend.setBaseCurrency(code, session.userId);
});

// --- Revaluation wizard ------------------------------------------------------------------------

export const getRevaluationPreview = wrap('settings.getRevaluationPreview', async function getRevaluationPreview(rates: Record<string, number>) {
  if (usesRust('settings')) return backendCall('settings_get_revaluation_preview', { rates });
  await delay(150);
  return clone(revalBackend.openFcBalances(rates));
});

export const getDefaultRevaluationRates = wrap('settings.getDefaultRevaluationRates', async function getDefaultRevaluationRates(): Promise<Record<string, number>> {
  if (usesRust('settings')) return backendCall('settings_get_default_revaluation_rates');
  await delay(50);
  return clone(revalBackend.defaultRevaluationRates());
});

export const postRevaluation = wrap('settings.postRevaluation', async function postRevaluation(date: string, rates: Record<string, number>) {
  if (usesRust('settings')) return backendCall('settings_post_revaluation', { date, rates });
  await delay(200);
  return revalBackend.postRevaluation(date, rates, session.userId);
});
