# 18.D — Country profiles: Egypt first, then Saudi

**Status (2026-09-26): implementation done, `verify:mocks` green for both SA and EG (98 ok, 0 todo,
0 failed), `bun run build`/`bun run check` green. The e2e flow is written
(`scripts/e2e/flows/setup_wizard_eg.py`) but NOT run against a live browser** — the shared dev
server was in active use by another session when this phase reached that gate; see `TODO.md`'s
"e2e not run against the shared dev server" entry for exactly what to run and why. Don't move this
phase to `plans/completed/` until someone runs `python scripts/e2e/flows/onboarding.py` and `python
scripts/e2e/flows/setup_wizard_eg.py` (or the whole suite) and confirms both pass with no console
errors.

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
- [x] Create `countryProfiles.ts` (EG, SA). Store `settings.country` and default it to **EG** (decision 2).
- [x] **Wizard order:** move "الدولة والعملة والضريبة" before "بيانات المنشأة" (swap in `WIZARD_STEPS`; progress is keyed by `key`, so saved progress survives, but verify it). Country options: EG, SA only.
- [x] `applyCountryTax` applies the profile: tax rates and names (14% / 15%), `pricesIncludeTax` default, currency, `settings.country`. Allowed only before the first posting (same lock as the base currency).
- [x] `StepCompany`: labels, patterns and hints for the tax id and CR from the profile; phone default from the profile.
- [x] Remove SAR hard-codes: `BASE_CURRENCY`, `|| 'SAR'` fallbacks, `useSettingsStore` default, and the currency option lists in `PartyFormPage` / `GeneralSettingsPage` (one list in core).
- [x] `MoneyText`: symbol per currency (`RiyalIcon` only for SAR, "ج.م" for EGP). `tafqit.ts` takes the currency (جنيه/قرش vs ريال/هللة, with the same dual/plural rules).
- [x] Invoices (A4, thermal, Typst payload): titles from the profile; ZATCA QR only when `eInvoice === 'zatca-phase1'`. `VatReportPage` and `TaxesSettingsPage` copy from the profile. (`VatReportPage` was already rate-agnostic — reads from `db.taxes`, nothing to change; `TaxesSettingsPage`'s "new tax" default rate now follows the profile.)
- [x] **Accounting safety:** `seedEmptyCompany(country)`; `verify:mocks` runs every invariant against **both** an SA and an EG company; golden totals for a 14% invoice (line discount → invoice discount → VAT order unchanged). Read [02](../../../docs/v2/02-accounting-review.md) first. **Result: 98 ok, 0 todo, 0 failed** (49 SA + 49 EG). Found and fixed a real bug along the way: `seedDatabase()`/`seedEmptyCompany()` never reset the shared `db` object, so seeding twice in one process (needed to test both countries) posted the second seed on top of the first's leftovers — added `resetDb()` in `src/mocks/db.ts`.
- [x] e2e: `setup_wizard_eg.py` written (EG → 14%, EGP, invoice prints "ج.م" and جنيه in words, no ZATCA QR) + `onboarding.py` updated for the new step order and to explicitly exercise the SA path. **Not yet run against a live browser** — see the status note at the top of this file and `TODO.md`.

