# 18.E — Address picker (`eg.json`, `sa.json`)

**Status note (2026-09-26):** implementation done and verified (`bun run build`, `bun run check`,
`bun run verify:mocks` 98/0/0, `bun run memory`, the new `address_picker.py` e2e flow, all green — see
run log below). **NOT fully done — two data items are placeholders, blocking real shipping:**
- `eg.json` is a hand-authored placeholder (not the real Tech-Labs export — no network access this
  session). MIT-licensed, not blocked, just needs a real `bun run geo:build` run once online (script
  is written and ready — see `scripts/geo/build.ts`). Logged in `TODO.md`.
- `sa.json` is a small hand-authored seed, explicitly NOT the real homaily dataset — the Saudi data
  license decision below is still open and must be resolved by the user before real Saudi geo data is
  bundled. Logged in `TODO.md` and in this folder's `README.md`.

Both are clearly marked in their own `source`/`license` fields — see E1 below.

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

- [x] Built by `scripts/geo/build.ts` (`bun run geo:build`) with pinned source commits. The PHPMyAdmin export wrapper in
  the Egypt files is unwrapped. Names are trimmed and deduped, and sorted with `Intl.Collator('ar')`. The output is
  committed; the script only exists to refresh it. **Not run live this session (no network access) — `eg.json`/
  `sa.json` are hand-authored placeholders/seeds instead; see the status note above and `TODO.md`.**
- [x] Loaded **lazily** (`import()` per country), so neither file is in the main bundle. Verified in the `bun run build`
  output: `eg-*.js` and `sa-*.js` are separate chunks.
- [x] Add `data` as a recognized layer in `scripts/memory/config.ts` — layers are derived dynamically from folder names
  (`scripts/memory/analyze/classify.ts`'s `locate()`), so the actual fix was adding `json` to the scanner's recognized
  `extensions` (`scripts/memory/config.ts` + `types.ts`'s `Lang` union) so `core/data/geo/*.json` gets scanned/counted
  at all — confirmed `data 2` now appears in `core`'s layer breakdown in `AGENT_MEMORY.md`.

### E2. Service, type and component

- [x] `core/services/geoService.ts` (the seam; a real backend can serve it later): `getRegions(c)`, `getCities(c, regionId)`, `getDistricts(c, cityId)`, `searchPlaces(c, q)` (plus `getLabels(c)`, needed so `AddressFields` shows each country's own region/city/district terms instead of hard-coding Egypt's).
- [x] One `Address` type in `core/types` (`core/types/address.ts`; `parties/types`' `NationalAddress` is kept but marked `@deprecated`, since existing seeded/legacy party records still use it — see the migration note below). It stores **ids and names** (the name is a snapshot, so printed documents never change if the dataset is refreshed): `country, regionId, regionName, cityId, cityName, districtId?, districtName?` + per-country fields:
  - **EG:** street, building no, floor, apartment, landmark (علامة مميزة), postal code (optional). This mirrors the ETA address shape, so e-invoicing later needs no migration.
  - **SA:** street, building no (4 digits), additional no (4), postal code (5), unit no, short address (`AAAA9999`).
- [x] `addressSchema(country)` Zod validators in `core/helpers/validation.ts`, driven by `profile.address`.
- [x] **`AddressFields`** block (`core/components/blocks`): cascading `AppCombobox`es (region → city → district). Changing a parent clears its children (fixed a real bug during e2e testing — the cascading clear needed to react to the id itself, not only the combobox's `@select` event, so clearing via the "X" button also cascades — see `TODO.md`). Arabic-tolerant search (أ/إ/آ→ا, ة→ه, ى→ي, strip "ال"/"حي " — added `normalizePlaceName`/`matchesPlaceSearch` to `search.ts`, since the existing `normalizeArabic` only did the letter-unification half). "غير موجود في القائمة؟ اكتب يدوياً" free-text fallback per level. Keyboard-only usable (built on `AppCombobox`'s reka-ui listbox). Rule 3: added to `/dev/ui` (`DevUiPage.vue`'s "العنوان (AddressFields)" card) + `docs/design_system.md`.
- [x] `formatAddress(addr)` helper for print (`core/helpers/format.ts`) — wired into `pdfService.ts`'s `company`/`party` payload blocks (via a new `resolvePartyAddressLine()` helper in `parties/helpers/partyAddress.ts` for the party side, which prefers `structuredAddress` → legacy `nationalAddress` → the old plain string, so nothing regresses for existing seeded data).
- [x] Replaced every hand-typed address: `StepCompany` (setup wizard), `GeneralSettingsPage`, `PartyFormPage` (extracted its address section — shrank the page from 551 back to 524 lines by moving the migration helper into `parties/helpers/partyAddress.ts`), `BranchesSettingsPage` + `StepBranches` (branches' `address` string → `Address`). `PartyFormModal` has no address field at all (it's the quick-add name+phone+type-only modal — confirmed nothing to migrate there).
- [x] Migration: existing plain-string/legacy `NationalAddress` fields map to the new type's `*Name`-only fields with no id (`migrateLegacyAddress()` in `parties/helpers/partyAddress.ts`); nothing is lost — verified `verify:mocks` stays 98/0/0 with the existing seeded party/branch/settings fixtures untouched.
- [x] e2e `address_picker.py`: EG القاهرة → a city by keyboard; SA منطقة الرياض → الرياض → a district; free-text fallback; a party form's "يظهر في الفاتورة هكذا" preview shows the formatted address correctly (used instead of a reload-based print check — this session's dev server wasn't persisting IndexedDB across reload, an environment issue unrelated to this phase, logged in `TODO.md`). All checks green, no console errors.

