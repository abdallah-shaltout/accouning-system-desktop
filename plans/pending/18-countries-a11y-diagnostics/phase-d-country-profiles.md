# 18.D — Country profiles: Egypt first, then Saudi

**Why this is an accounting fix, not only UI.** Today an Egyptian company is set up with **15% VAT**
(`seedEmptyCompany` seeds 15% and `applyCountryTax` ignores the country, `void input.vatRegistered`). Amounts are
written in words in riyals (`tafqit.ts`), invoices use Saudi ZATCA titles ("فاتورة ضريبية مبسطة") and QR, and
`BASE_CURRENCY = 'SAR'` plus `|| 'SAR'` fallbacks are scattered around. The wizard also asks for the company's
address, VAT number (`^3\d{13}3$`) and phone **before** it asks for the country.

### D1. One owner file: `src/modules/core/helpers/countryProfiles.ts`

```ts
interface CountryProfile {
  code: 'EG' | 'SA';
  nameAr: string;
  currency: { code: 'EGP' | 'SAR'; symbolAr: 'ج.م' | 'ر.س'; minorAr: 'قرش' | 'هللة'; words: TafqitCurrency };
  vat: { standardRate: 14 | 15; pricesIncludeTaxDefault: boolean; label: string };
  taxId: { label: string; pattern: RegExp; hint: string };     // EG: 'رقم التسجيل الضريبي', 9 digits · SA: 'الرقم الضريبي', 15 digits 3…3
  commercialRegister: { label: string; pattern?: RegExp };
  phone: { defaultCountry: 'EG' | 'SA' };
  address: AddressSchema;                                        // Phase E
  eInvoice: 'none' | 'zatca-phase1';                             // EG ETA later (decision 4)
  invoiceTitles: { b2b: string; b2c: string };
}
```

Everything country-specific reads from here. Nothing else hard-codes a country, rate, currency or label.

**Tasks**
- [ ] Create `countryProfiles.ts` (EG, SA). Store `settings.country` and default it to **EG** (decision 2).
- [ ] **Wizard order:** move "الدولة والعملة والضريبة" before "بيانات المنشأة" (swap in `WIZARD_STEPS`; progress is keyed by `key`, so saved progress survives, but verify it). Country options: EG, SA only.
- [ ] `applyCountryTax` applies the profile: tax rates and names (14% / 15%), `pricesIncludeTax` default, currency, `settings.country`. Allowed only before the first posting (same lock as the base currency).
- [ ] `StepCompany`: labels, patterns and hints for the tax id and CR from the profile; phone default from the profile.
- [ ] Remove SAR hard-codes: `BASE_CURRENCY`, `|| 'SAR'` fallbacks, `useSettingsStore` default, and the currency option lists in `PartyFormPage` / `GeneralSettingsPage` (one list in core).
- [ ] `MoneyText`: symbol per currency (`RiyalIcon` only for SAR, "ج.م" for EGP). `tafqit.ts` takes the currency (جنيه/قرش vs ريال/هللة, with the same dual/plural rules).
- [ ] Invoices (A4, thermal, Typst payload): titles from the profile; ZATCA QR only when `eInvoice === 'zatca-phase1'`. `VatReportPage` and `TaxesSettingsPage` copy from the profile.
- [ ] **Accounting safety:** `seedEmptyCompany(country)`; `verify:mocks` runs every invariant against **both** an SA and an EG company; golden totals for a 14% invoice (line discount → invoice discount → VAT order unchanged). Read [02](../../../docs/v2/02-accounting-review.md) first.
- [ ] e2e: `setup_wizard_eg.py` (EG → 14%, EGP, Egyptian address, invoice prints "ج.م" and جنيه in words, no ZATCA QR) + keep the SA flow.

