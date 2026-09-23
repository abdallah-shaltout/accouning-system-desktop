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

// --- Branches ------------------------------------------------------------------------------

export async function getBranches(): Promise<Branch[]> {
  await delay(100);
  return clone(backend.listBranches());
}

export async function createBranch(input: BranchInput): Promise<Branch> {
  await delay();
  return clone(backend.createBranch(input, session.userId));
}

export async function updateBranch(id: string, input: Partial<BranchInput>): Promise<Branch> {
  await delay();
  return clone(backend.updateBranch(id, input, session.userId));
}

export async function deactivateBranch(id: string): Promise<Branch> {
  await delay();
  return clone(backend.deactivateBranch(id, session.userId));
}

export async function reactivateBranch(id: string): Promise<Branch> {
  await delay();
  return clone(backend.reactivateBranch(id, session.userId));
}

// --- Cost centers ----------------------------------------------------------------------------

export async function getCostCenters(): Promise<CostCenter[]> {
  await delay(100);
  return clone(backend.listCostCenters());
}

export async function createCostCenter(input: CostCenterInput): Promise<CostCenter> {
  await delay();
  return clone(backend.createCostCenter(input, session.userId));
}

export async function updateCostCenter(id: string, input: Partial<CostCenterInput>): Promise<CostCenter> {
  await delay();
  return clone(backend.updateCostCenter(id, input, session.userId));
}

export async function deleteCostCenter(id: string): Promise<void> {
  await delay();
  backend.deleteCostCenter(id);
}

// --- Currencies ------------------------------------------------------------------------------

export async function getCurrencies(): Promise<Currency[]> {
  await delay(100);
  return clone(db.currencies);
}

export async function getExchangeRates(currency?: string): Promise<ExchangeRate[]> {
  await delay(100);
  return clone(currency ? db.exchangeRates.filter((r) => r.currency === currency) : db.exchangeRates);
}

export async function createCurrency(input: Currency): Promise<Currency> {
  await delay();
  return clone(currencyBackend.createCurrency(input, session.userId));
}

export async function updateCurrency(code: string, patch: Partial<Currency>): Promise<Currency> {
  await delay();
  return clone(currencyBackend.updateCurrency(code, patch));
}

export async function saveExchangeRate(input: ExchangeRateInput): Promise<ExchangeRate> {
  await delay();
  return clone(currencyBackend.saveExchangeRate(input));
}

export async function isBaseCurrencyLocked(): Promise<boolean> {
  await delay(50);
  return currencyBackend.isBaseCurrencyLocked();
}

export async function setBaseCurrency(code: string): Promise<void> {
  await delay();
  currencyBackend.setBaseCurrency(code);
}

// --- Revaluation wizard ------------------------------------------------------------------------

export async function getRevaluationPreview(rates: Record<string, number>) {
  await delay(150);
  return clone(revalBackend.openFcBalances(rates));
}

export async function getDefaultRevaluationRates(): Promise<Record<string, number>> {
  await delay(50);
  return clone(revalBackend.defaultRevaluationRates());
}

export async function postRevaluation(date: string, rates: Record<string, number>) {
  await delay(200);
  return revalBackend.postRevaluation(date, rates, session.userId);
}
