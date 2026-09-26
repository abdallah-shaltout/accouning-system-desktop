/**
 * v2 phase 5 (docs/v2/05-onboarding.md): the setup wizard's own service layer. Thin wrappers
 * around `src/mocks/backend/setup.ts` / `opening.ts`, matching every other module's services shape
 * (pages never import `src/mocks/backend` directly).
 */
import { ApiError, clone, db, delay, session, uid } from '@/mocks';
import * as setupBackend from '@/mocks/backend/setup';
import {
  closeOpeningBalanceEquity,
  hasFirstUsePosted,
  openingBalanceEquityNet,
  postOpeningEntry,
  postOpeningStockForBranch,
  postPartyOpeningBalance,
  reversePartyOpeningBalance,
  type OpeningEntryInput,
  type OpeningStockLine,
  type PartyOpeningBalanceInput,
} from '@/mocks/backend/opening';
import { createCurrency, setBaseCurrency } from '@/mocks/backend/currency';
import { mutate } from '@/mocks/persist';
import type { Account, FiscalYear } from '@/modules/accounting/types';
import type { AccountTemplate } from '@/mocks/fixtures/accounts';
import type { Branch } from '@/modules/settings/types';
import type { WizardBranchInput, WizardPaymentMethodInput } from '@/mocks/backend/setup';
import { countryProfile, type CountryCode } from '@/modules/core/helpers/countryProfiles';

import { wrap } from '@/modules/diagnostics/services/defineService';

// --- Progress ----------------------------------------------------------------------------------

export interface OnboardingProgress {
  businessType?: string;
  goLiveDate?: string;
  completedStep?: number;
  skipped: string[];
  done: string[];
  finishedAt?: string;
}

export const getOnboardingProgress = wrap('setup.getOnboardingProgress', async function getOnboardingProgress(): Promise<OnboardingProgress> {
  await delay(60);
  const o = db.settings.onboarding;
  return clone({ businessType: o?.businessType, goLiveDate: o?.goLiveDate, completedStep: o?.completedStep, skipped: o?.skipped ?? [], done: o?.done ?? [], finishedAt: o?.finishedAt });
});

export const saveOnboardingProgress = wrap('setup.saveOnboardingProgress', async function saveOnboardingProgress(patch: Partial<OnboardingProgress>): Promise<void> {
  await delay(40);
  mutate(() => (db.settings.onboarding = { ...db.settings.onboarding, ...clone(patch) }));
});

export const markStepDone = wrap('setup.markStepDone', async function markStepDone(key: string): Promise<void> {
  await delay(20);
  mutate(() => {
    const done = new Set(db.settings.onboarding?.done ?? []);
    done.add(key);
    db.settings.onboarding = { ...db.settings.onboarding, done: [...done] };
  });
});

export const markStepSkipped = wrap('setup.markStepSkipped', async function markStepSkipped(key: string): Promise<void> {
  await delay(20);
  mutate(() => {
    const skipped = new Set(db.settings.onboarding?.skipped ?? []);
    skipped.add(key);
    db.settings.onboarding = { ...db.settings.onboarding, skipped: [...skipped] };
  });
});

// --- Step: business type -----------------------------------------------------------------------

export const applyBusinessTypeDefaults = wrap('setup.applyBusinessTypeDefaults', async function applyBusinessTypeDefaults(businessType: string): Promise<void> {
  await delay(40);
  setupBackend.applyBusinessTypeUnitDefaults(businessType);
});

// --- Step: country/currency/tax --------------------------------------------------------------

export const isBaseCurrencyLocked = wrap('setup.isBaseCurrencyLocked', async function isBaseCurrencyLocked(): Promise<boolean> {
  await delay(30);
  return db.journalEntries.length > 0;
});

export const applyCountryTax = wrap('setup.applyCountryTax', async function applyCountryTax(input: {
  country: CountryCode;
  currency: string;
  vatRegistered: boolean;
  pricesIncludeTax: boolean;
  extraCurrencies: { code: string; rate: number }[];
}): Promise<void> {
  await delay();
  if (await isBaseCurrencyLocked()) throw new ApiError('لا يمكن تغيير الدولة أو العملة الأساسية بعد أول ترحيل', 'FORBIDDEN');

  const profile = countryProfile(input.country);
  if (db.settings.currency !== input.currency) setBaseCurrency(input.currency);

  // v2 doc 18.D: applies the *full* profile, not only currency — tax rate/name, prices-include-VAT
  // default and `settings.country` (today Egypt was wrongly seeded at 15% because this only ever
  // touched `pricesIncludeTax`; the VAT rate/name/country now all come from the chosen profile).
  mutate(() => {
    db.settings.pricesIncludeTax = input.pricesIncludeTax;
    db.settings.country = profile.code;
    db.taxes = db.taxes.map((t) => {
      if (t.id === 'tax-vat-out') return { ...t, rate: profile.vat.standardRate, name: `${profile.vat.label} (مبيعات)` };
      if (t.id === 'tax-vat-in') return { ...t, rate: profile.vat.standardRate, name: `${profile.vat.label} (مشتريات)` };
      return t;
    });
  });

  for (const c of input.extraCurrencies) {
    if (!c.code || db.currencies.some((x) => x.code === c.code)) continue;
    createCurrency({ code: c.code, nameAr: c.code, symbol: c.code, decimals: 2, active: true, fixed: false, fixedRate: c.rate }, session.userId);
  }
  if (input.extraCurrencies.length) mutate(() => (db.settings.features = { ...db.settings.features, currencies: true }));
  void input.vatRegistered; // captured for the review screen; no extra posting needed (tax rows are updated above, not posted).
});

// --- Step: fiscal year ---------------------------------------------------------------------

export const applyFiscalYear = wrap('setup.applyFiscalYear', async function applyFiscalYear(startMonth: number, startDay: number, goLiveDate: string): Promise<FiscalYear> {
  await delay();
  const fy = setupBackend.setFiscalYear(startMonth, startDay, goLiveDate);
  mutate(() => (db.settings.onboarding = { ...db.settings.onboarding, goLiveDate }));
  return clone(fy);
});

// --- Step: branches --------------------------------------------------------------------------

export const applyBranches = wrap('setup.applyBranches', async function applyBranches(branches: WizardBranchInput[]): Promise<Branch[]> {
  await delay();
  setupBackend.applyBranches(branches, session.userId);
  return clone(db.branches);
});

// --- Step: chart of accounts -----------------------------------------------------------------

export const previewCoaTemplate = wrap('setup.previewCoaTemplate', function previewCoaTemplate(template: AccountTemplate, country: CountryCode = 'EG', businessType?: string): Account[] {
  return setupBackend.previewCoaTemplate(template, country, businessType);
});

export const applyCoaTemplate = wrap('setup.applyCoaTemplate', async function applyCoaTemplate(template: AccountTemplate, country: CountryCode = 'EG', businessType?: string): Promise<Account[]> {
  await delay();
  const accounts = setupBackend.applyCoaTemplate(template, country, businessType);
  mutate(() => (db.settings.onboarding = { ...db.settings.onboarding, coaTemplate: template }));
  return clone(accounts);
});

// --- Step: payment methods --------------------------------------------------------------------

export const applyPaymentMethods = wrap('setup.applyPaymentMethods', async function applyPaymentMethods(methods: WizardPaymentMethodInput[]): Promise<void> {
  await delay();
  setupBackend.applyPaymentMethods(methods);
});

// --- Step 8: opening balances -----------------------------------------------------------------

export const getOpeningBalanceEquityNet = wrap('setup.getOpeningBalanceEquityNet', async function getOpeningBalanceEquityNet(): Promise<number> {
  await delay(30);
  return openingBalanceEquityNet();
});

export const isFirstUsePosted = wrap('setup.isFirstUsePosted', async function isFirstUsePosted(): Promise<boolean> {
  await delay(20);
  return hasFirstUsePosted();
});

export const postOpeningBalances = wrap('setup.postOpeningBalances', async function postOpeningBalances(
  input: Omit<OpeningEntryInput, 'createdBy'>,
  closeTarget: 'capital' | 'ownerCurrent',
): Promise<{ openingEntryId: string; closingEntryId?: string }> {
  await delay(200);
  const openingEntry = postOpeningEntry({ ...input, createdBy: session.userId });
  const closingEntry = closeOpeningBalanceEquity(input.date, closeTarget, session.userId);
  mutate(
    () =>
      (db.settings.onboarding = {
        ...db.settings.onboarding,
        openingEntryId: openingEntry.id,
        closingEntryId: closingEntry?.id,
      }),
  );
  return { openingEntryId: openingEntry.id, closingEntryId: closingEntry?.id };
});

export const postOpeningStock = wrap('setup.postOpeningStock', async function postOpeningStock(branchId: string, date: string, lines: OpeningStockLine[]): Promise<void> {
  await delay(150);
  postOpeningStockForBranch(branchId, date, lines, session.userId);
});

/** Re-closes 3900 after the opening entry or opening stock changed (idempotent — no-op if already zero). */
export const recloseOpeningBalanceEquity = wrap('setup.recloseOpeningBalanceEquity', async function recloseOpeningBalanceEquity(date: string, target: 'capital' | 'ownerCurrent' = 'capital'): Promise<void> {
  await delay(80);
  closeOpeningBalanceEquity(date, target, session.userId);
});

// --- Party opening balance (docs/v2/05 §4, party-form stub) -----------------------------------

export const postPartyOpening = wrap('setup.postPartyOpening', async function postPartyOpening(input: Omit<PartyOpeningBalanceInput, 'createdBy'>): Promise<string | undefined> {
  await delay(120);
  const entry = postPartyOpeningBalance({ ...input, createdBy: session.userId });
  return entry?.id;
});

export const reversePartyOpening = wrap('setup.reversePartyOpening', async function reversePartyOpening(entryId: string): Promise<void> {
  await delay(120);
  reversePartyOpeningBalance(entryId, session.userId);
});

// --- Finish ------------------------------------------------------------------------------------

export const finishOnboarding = wrap('setup.finishOnboarding', async function finishOnboarding(): Promise<void> {
  await delay(60);
  mutate(() => (db.settings.onboarding = { ...db.settings.onboarding, finishedAt: new Date().toISOString() }));
});

export { uid };
