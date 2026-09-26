import { db } from '../db';
import { paymentMethodsFixture, settingsFixture, taxesFixture } from '../fixtures/settings';
import { clone } from '../utils';
import { countryProfile, type CountryCode } from '@/modules/core/helpers/countryProfiles';

/**
 * Settings area: store settings, taxes and payment methods.
 *
 * v2 doc 18.D: `country` defaults to SA — unchanged for the pre-existing demo seed and its golden
 * numbers. `country: 'EG'` re-derives the tax rate/name, currency and store settings from the EG
 * profile (14% VAT, EGP) instead of the SA fixture — used by `verify:mocks`'s EG invariant run.
 */
export function seedSettings(country: CountryCode = 'SA'): void {
  db.paymentMethods = clone(paymentMethodsFixture);
  if (country === 'SA') {
    db.taxes = clone(taxesFixture);
    db.settings = clone(settingsFixture);
    return;
  }

  const profile = countryProfile(country);
  db.taxes = clone(taxesFixture).map((t) => {
    if (t.id === 'tax-vat-out') return { ...t, rate: profile.vat.standardRate, name: `${profile.vat.label} (مبيعات)` };
    if (t.id === 'tax-vat-in') return { ...t, rate: profile.vat.standardRate, name: `${profile.vat.label} (مشتريات)` };
    return t;
  });
  db.settings = {
    ...clone(settingsFixture),
    currency: profile.currency.code,
    country: profile.code,
    pricesIncludeTax: profile.vat.pricesIncludeTaxDefault,
    vatNumber: undefined,
    commercialRegister: undefined,
    address: undefined,
    phone: undefined,
  };
}
