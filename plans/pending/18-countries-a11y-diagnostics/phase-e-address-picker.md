# 18.E — Address picker (`eg.json`, `sa.json`)

### E1. Data

| File | Source | License | Levels | Size target |
|---|---|---|---|---|
| `src/modules/core/data/geo/eg.json` | Tech-Labs/egypt-governorates-and-cities-db (`governorates.json` 27 rows: `governorate_name_ar/en`; `cities.json` ~396 rows: `governorate_id`, `city_name_ar/en`) | MIT | محافظة → مدينة / مركز / حي (قسم) | < 60 KB |
| `src/modules/core/data/geo/sa.json` | homaily (`regions_lite`, `cities_lite`, `districts_lite`: `name_ar/en`, `region_id`, `city_id`) | **GPL-2.0, see open decision** | منطقة → مدينة → حي | < 450 KB |

One normalized shape for both, so the UI does not branch per country:

```json
{ "country": "EG", "source": "…", "sourceCommit": "…", "license": "MIT", "generatedAt": "…",
  "labels": { "region": "المحافظة", "city": "المدينة / المركز", "district": null },
  "regions": [ { "id": "1", "ar": "القاهرة", "en": "Cairo",
      "cities": [ { "id": "1", "ar": "15 مايو", "en": "15 May", "districts": [] } ] } ] }
```

- Built by `scripts/geo/build.ts` (`bun run geo:build`) with pinned source commits. The PHPMyAdmin export wrapper in
  the Egypt files is unwrapped. Names are trimmed and deduped, and sorted with `Intl.Collator('ar')`. The output is
  committed; the script only exists to refresh it.
- Loaded **lazily** (`import()` per country), so neither file is in the main bundle.
- Add `data` as a recognized layer in `scripts/memory/config.ts`.

### E2. Service, type and component

- [ ] `core/services/geoService.ts` (the seam; a real backend can serve it later): `getRegions(c)`, `getCities(c, regionId)`, `getDistricts(c, cityId)`, `searchPlaces(c, q)`.
- [ ] One `Address` type in `core/types` (replaces `parties/types` `NationalAddress`). It stores **ids and names** (the name is a snapshot, so printed documents never change if the dataset is refreshed): `country, regionId, regionName, cityId, cityName, districtId?, districtName?` + per-country fields:
  - **EG:** street, building no, floor, apartment, landmark (علامة مميزة), postal code (optional). This mirrors the ETA address shape, so e-invoicing later needs no migration.
  - **SA:** street, building no (4 digits), additional no (4), postal code (5), unit no, short address (`AAAA9999`).
- [ ] `addressSchema(country)` Zod validators in `core/helpers/validation.ts`, driven by `profile.address`.
- [ ] **`AddressFields`** block (`core/components/blocks`): cascading `AppCombobox`es (region → city → district). Changing a parent clears its children. Arabic-tolerant search (أ/إ/آ→ا, ة→ه, ى→ي, strip "ال"/"حي "; check `search.ts`). "غير موجود في القائمة؟ اكتب يدوياً" free-text fallback. Keyboard-only usable. Rule 3: `/dev/ui` + `design_system.md` entries.
- [ ] `formatAddress(addr)` helper for print (A4, thermal, Typst payload, party statement).
- [ ] Replace every hand-typed address: `StepCompany`, `GeneralSettingsPage`, `PartyFormPage` (extract its address section, which also shrinks the 537-line page), `PartyFormModal`, branches (`address` string → `Address`).
- [ ] Migration: existing string fields map to `*Name` with no id; nothing is lost.
- [ ] e2e `address_picker.py`: EG القاهرة → a city by keyboard; SA منطقة الرياض → الرياض → a district; free-text fallback; printed invoice shows the formatted address.

