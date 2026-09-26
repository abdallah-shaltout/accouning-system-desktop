/**
 * Multi-currency (docs/v2/10-branches-currencies-cost-centers.md §2). Rate convention throughout
 * this file and every caller: `rate` = base currency per 1 unit of the foreign currency (matches
 * `Account.currency` balances, `JournalLine.rate`, `ExchangeRate.rate`). The rate-entry FORM may
 * accept the inverse ("1 SAR = 0.2667 USD") — that inversion happens in the UI layer, never here.
 */
import type { Currency, ExchangeRate, ExchangeRateInput } from '@/modules/settings/types';
import { db } from '../db';
import { mutate } from '../persist';
import { ApiError, localDateKey, round2, uid } from '../utils';
import { DEFAULT_COUNTRY, countryProfile } from '@/modules/core/helpers/countryProfiles';

export function baseCurrency(): string {
  return db.settings.currency || countryProfile(DEFAULT_COUNTRY).currency.code;
}

export function isBaseCurrency(code: string | undefined): boolean {
  return !code || code === baseCurrency();
}

export function currencyByCode(code: string): Currency | undefined {
  return db.currencies.find((c) => c.code === code);
}

export function activeCurrencies(): Currency[] {
  return db.currencies.filter((c) => c.active);
}

export function createCurrency(input: Currency, userId: string): Currency {
  void userId;
  if (input.code === baseCurrency()) throw new ApiError('هذه هي العملة الأساسية بالفعل');
  if (db.currencies.some((c) => c.code === input.code)) throw new ApiError('هذه العملة مضافة بالفعل');
  const currency: Currency = { ...input, code: input.code.toUpperCase() };
  mutate(() => db.currencies.push(currency));
  return currency;
}

export function updateCurrency(code: string, patch: Partial<Currency>): Currency {
  const currency = currencyByCode(code);
  if (!currency) throw new ApiError('العملة غير موجودة', 'NOT_FOUND');
  mutate(() => Object.assign(currency, patch));
  return currency;
}

/**
 * The base currency is locked after the first posting (docs/v2/10 §2, README decision context:
 * Phase 1/5 already established this concept for onboarding — this confirms/wires the lock here).
 */
export function isBaseCurrencyLocked(): boolean {
  return db.journalEntries.length > 0;
}

export function setBaseCurrency(code: string): void {
  if (isBaseCurrencyLocked()) throw new ApiError('لا يمكن تغيير العملة الأساسية بعد بدء الترحيل', 'FORBIDDEN');
  mutate(() => (db.settings.currency = code.toUpperCase()));
}

/** Saves a rate-table row. `input.rate` is base-per-unit; `input.inverseRate` (unit-per-base) is inverted first when that's what the form captured. */
export function saveExchangeRate(input: ExchangeRateInput): ExchangeRate {
  const currency = currencyByCode(input.currency);
  if (!currency) throw new ApiError('العملة غير مفعّلة', 'NOT_FOUND');
  const rate = input.rate ?? (input.inverseRate ? round2(1 / input.inverseRate) : undefined);
  if (!rate || rate <= 0) throw new ApiError('أدخل سعر الصرف');
  const row: ExchangeRate = { id: uid('fx'), currency: input.currency.toUpperCase(), date: localDateKey(input.date), rate };
  mutate(() => {
    // One rate per (currency, date) — a later save for the same day overwrites it.
    db.exchangeRates = db.exchangeRates.filter((r) => !(r.currency === row.currency && r.date === row.date));
    db.exchangeRates.push(row);
  });
  return row;
}

/** The latest rate on or before `asOf` (defaults to today) — the default a document proposes. */
export function latestRate(currency: string, asOf?: string): number | undefined {
  const c = currencyByCode(currency);
  if (c?.fixed && c.fixedRate) return c.fixedRate;
  const cutoff = asOf ? localDateKey(asOf) : localDateKey(new Date());
  const candidates = db.exchangeRates.filter((r) => r.currency === currency && r.date <= cutoff).sort((a, b) => b.date.localeCompare(a.date));
  return candidates[0]?.rate;
}

export function requireRate(currency: string, asOf?: string): number {
  const rate = latestRate(currency, asOf);
  if (!rate) throw new ApiError(`لا يوجد سعر صرف لعملة ${currency}`, 'VALIDATION');
  return rate;
}

/**
 * Converts a set of already-rounded FC line amounts to base currency, applying the exact rounding
 * rule from docs/v2/10 §2: base per line = round2(fc × rate); any cent difference between Σ lines
 * and round2(fcTotal × rate) goes to the LARGEST line (never a separate account). Returns the
 * per-line base amounts (same order/length as `fcAmounts`), summing to exactly `round2(fcTotal × rate)`.
 */
export function convertLinesToBase(fcAmounts: number[], rate: number): number[] {
  if (!fcAmounts.length) return [];
  const perLine = fcAmounts.map((fc) => round2(fc * rate));
  const fcTotal = round2(fcAmounts.reduce((a, b) => a + b, 0));
  const target = round2(fcTotal * rate);
  const sumPerLine = round2(perLine.reduce((a, b) => a + b, 0));
  let diff = round2(target - sumPerLine);
  if (diff !== 0) {
    // Largest line absorbs the rounding difference, per the doc's exact rule.
    let largestIdx = 0;
    for (let i = 1; i < perLine.length; i++) if (Math.abs(fcAmounts[i]) > Math.abs(fcAmounts[largestIdx])) largestIdx = i;
    perLine[largestIdx] = round2(perLine[largestIdx] + diff);
    diff = 0;
  }
  return perLine;
}

/** Single-amount convenience (document/payment level, not a line array). */
export function toBase(fc: number, rate: number): number {
  return round2(fc * rate);
}
