/**
 * v2 doc 18.D (plans/pending/18-countries-a11y-diagnostics/phase-d-country-profiles.md): the one
 * owner file for everything that varies by country — tax rate/name, currency, tax-id/CR labels and
 * patterns, phone default, invoice titles and e-invoicing scheme. Nothing else in the app should
 * hard-code a country, VAT rate, currency code or ZATCA-specific label — read it from here.
 *
 * Today only Egypt (EG, default) and Saudi Arabia (SA) are supported — the wizard's country picker
 * (`StepCountryTax.vue`) only offers these two. Adding a country later means adding one profile here
 * plus (if it needs its own chart-of-accounts add-on) a `xxAddonRows()` in
 * `src/mocks/fixtures/accounts.ts` — never a new `if (country === ...)` branch scattered elsewhere.
 */
import type { TafqitCurrencyWords } from './tafqit';
// (type-only import — no runtime dependency, so `tafqit.ts` importing this module back stays acyclic.)

export type CountryCode = 'EG' | 'SA';
export type CurrencyCode = 'EGP' | 'SAR';

/**
 * Address shape every country profile carries a slot for (doc 18.D task list: "CountryProfile has a
 * typed address slot per the doc so Phase E can build on it"). Phase E (the address picker) builds
 * `AddressFields`/geo JSON on top of this — this phase only defines the shape and the two fields the
 * wizard's `StepCompany.vue` already collects (city/district/street/buildingNo/postalCode), mapped
 * generically so a future country's different address shape (e.g. Egypt's governorate) still fits.
 */
export interface AddressSchema {
  /** Ordered field keys this country's address form should show, e.g. ['city', 'district', 'street', 'buildingNo', 'postalCode']. */
  fields: ('city' | 'district' | 'street' | 'buildingNo' | 'postalCode' | 'governorate')[];
  labels: Partial<Record<'city' | 'district' | 'street' | 'buildingNo' | 'postalCode' | 'governorate', string>>;
}

export interface CountryProfile {
  code: CountryCode;
  nameAr: string;
  currency: {
    code: CurrencyCode;
    /** Arabic currency symbol shown next to amounts (`MoneyText`) — "ج.م" for EGP, the `RiyalIcon` glyph for SAR. */
    symbolAr: string;
    /** Arabic name of the minor unit — قرش (EG) / هللة (SA) — used by `tafqit.ts`. */
    minorAr: string;
    /** تفقيط word set for `tafqit.ts` (major/minor unit words with full singular/dual/plural agreement). */
    words: TafqitCurrencyWords;
  };
  vat: {
    standardRate: 14 | 15;
    /** Whether shelf/entered prices include VAT by default (`StoreSettings.pricesIncludeTax`). */
    pricesIncludeTaxDefault: boolean;
    /** Arabic label for the standard-rate VAT tax record's name, e.g. "ضريبة القيمة المضافة 14% (مبيعات)". */
    label: string;
  };
  taxId: {
    label: string;
    pattern: RegExp;
    hint: string;
  };
  commercialRegister: {
    label: string;
    pattern?: RegExp;
  };
  phone: {
    defaultCountry: CountryCode;
  };
  /** Phase E builds the actual address-field component on top of this shape — not implemented here. */
  address: AddressSchema;
  /** 'zatca-phase1': render the ZATCA QR + simplified/standard tax-invoice titles. 'none': plain invoice, no QR. */
  eInvoice: 'none' | 'zatca-phase1';
  invoiceTitles: { b2b: string; b2c: string };
}

const SA_ADDRESS: AddressSchema = {
  fields: ['city', 'district', 'street', 'buildingNo', 'postalCode'],
  labels: { city: 'المدينة', district: 'الحي', street: 'الشارع', buildingNo: 'رقم المبنى', postalCode: 'الرمز البريدي' },
};

const EG_ADDRESS: AddressSchema = {
  fields: ['governorate', 'city', 'district', 'street', 'buildingNo'],
  labels: { governorate: 'المحافظة', city: 'المدينة', district: 'الحي', street: 'الشارع', buildingNo: 'رقم المبنى' },
};

export const COUNTRY_PROFILES: Record<CountryCode, CountryProfile> = {
  EG: {
    code: 'EG',
    nameAr: 'مصر',
    currency: {
      code: 'EGP',
      symbolAr: 'ج.م',
      minorAr: 'قرش',
      words: {
        majorNoun: 'جنيه',
        major: { one: 'جنيه واحد', two: 'جنيهان', few: 'جنيهات', many: 'جنيهاً' },
        majorFeminine: false,
        minor: { one: 'قرش واحد', two: 'قرشان', few: 'قروش', many: 'قرشاً' },
        minorFeminine: false,
        nationality: 'مصرياً',
      },
    },
    vat: {
      standardRate: 14,
      pricesIncludeTaxDefault: true,
      label: 'ضريبة القيمة المضافة 14%',
    },
    taxId: {
      label: 'رقم التسجيل الضريبي',
      pattern: /^\d{9}$/,
      hint: '9 أرقام',
    },
    commercialRegister: {
      label: 'رقم السجل التجاري',
    },
    phone: { defaultCountry: 'EG' },
    address: EG_ADDRESS,
    eInvoice: 'none',
    invoiceTitles: { b2b: 'فاتورة ضريبية', b2c: 'فاتورة مبيعات' },
  },
  SA: {
    code: 'SA',
    nameAr: 'السعودية',
    currency: {
      code: 'SAR',
      symbolAr: 'ر.س',
      minorAr: 'هللة',
      words: {
        majorNoun: 'ريال',
        major: { one: 'ريال واحد', two: 'ريالان', few: 'ريالات', many: 'ريالاً' },
        majorFeminine: false,
        minor: { one: 'هللة واحدة', two: 'هللتان', few: 'هللات', many: 'هللة' },
        minorFeminine: true,
        nationality: 'سعودياً',
      },
    },
    vat: {
      standardRate: 15,
      pricesIncludeTaxDefault: true,
      label: 'ضريبة القيمة المضافة 15%',
    },
    taxId: {
      label: 'الرقم الضريبي',
      pattern: /^3\d{13}3$/,
      hint: '15 رقماً يبدأ وينتهي بالرقم 3',
    },
    commercialRegister: {
      label: 'رقم السجل التجاري',
    },
    phone: { defaultCountry: 'SA' },
    address: SA_ADDRESS,
    eInvoice: 'zatca-phase1',
    invoiceTitles: { b2b: 'فاتورة ضريبية', b2c: 'فاتورة ضريبية مبسطة' },
  },
};

export const DEFAULT_COUNTRY: CountryCode = 'EG';

export function countryProfile(code: string | undefined | null): CountryProfile {
  return COUNTRY_PROFILES[(code as CountryCode) in COUNTRY_PROFILES ? (code as CountryCode) : DEFAULT_COUNTRY];
}

/** Profile lookup by currency code — used where only the currency is known (e.g. `MoneyText`, `tafqit`). */
export function profileByCurrency(currency: string | undefined | null): CountryProfile {
  const found = Object.values(COUNTRY_PROFILES).find((p) => p.currency.code === currency);
  return found ?? COUNTRY_PROFILES[DEFAULT_COUNTRY];
}

export const COUNTRY_OPTIONS: { value: CountryCode; label: string }[] = [
  { value: 'EG', label: '🇪🇬 مصر' },
  { value: 'SA', label: '🇸🇦 السعودية' },
];
