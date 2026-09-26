# 19 — Shared shadcn date picker (`AppDatePicker`), replacing native `<input type="date">`

**Status: done** (2026-09-26). All three phases implemented, `build`/`check`/`verify:mocks`/`memory`
green, e2e suite run repeatedly with one real fix applied (a gallery label collision — see Phase B).
Manual light/dark/RTL/keyboard verification was not possible this session (no interactive browser
tool available) — do it before fully trusting the UI, per the note on each phase file.

Triggered by user feedback: single-date fields across the app render the OS's native date-picker
popup instead of a themed one, which breaks "same soul" (rule 5/13/14 in CLAUDE.md — every control
should look and feel consistent, in every theme, light and dark). Since `AppInput` and a handful of
bare `<input type="date">` elements are the seam, this is one new shared component plus a mechanical
migration, not a UX redesign — `DateRangeFilter`'s inline-range picker (already using native inputs
by *documented* decision, doc 16 Phase C) stays out of scope; only single-date fields move.

## Why not just wire up the existing `Calendar` primitive per page

`Calendar.vue` (`src/modules/core/components/shadcn/calendar/`) already exists and is used by
`RangeCalendar`/`DateRangeFilter`'s sibling. But it speaks `@internationalized/date`'s `DateValue`,
not the `YYYY-MM-DD` strings every form, Zod validator, and mock service in this app uses today.
Every call site would otherwise duplicate the `DateValue ↔ string` conversion. `AppDatePicker` does
that conversion once, at its own boundary, and keeps the string-model contract every existing
`v-model="someDateString"` already relies on — no page-level changes to types or validators.

## Decisions

| # | Decision | Why |
|---|---|---|
| 1 | New component `core/components/ui/AppDatePicker.vue`, built on shadcn `Popover` + `Calendar`. | Matches rule 1 ("shadcn first"): primitives already exist, just not wired into a single-date field. |
| 2 | Model type stays `string \| undefined` in `YYYY-MM-DD` (the same shape `toDateKey()`/services already use). | Zero type changes in callers; `formatDate`/`dateKeyToIso` and every mock service keep working unmodified. |
| 3 | `AppInput` grows no new variant. Instead, every `AppInput type="date"` call site is replaced with `AppDatePicker`, and `AppInput`'s `'date'` type is removed from its `type` union once the last caller is migrated. | Two ways to render a date field long-term would violate rule 2 ("never duplicate") and rule 4 (keep a stable, single, intentional API) — cleaner to retire the option than keep both alive. |
| 4 | The text field stays typeable (not calendar-only). Typed digits are parsed and re-validated against the same `YYYY-MM-DD` shape; the calendar (opened via a trailing icon button) is the alternate input path. | CLAUDE.md doesn't ask for calendar-only; native `<input type="date">` was already typeable, so removing that would be a regression, not just a re-skin. |
| 5 | `DateRangeFilter.vue` is explicitly **not** touched. | Its own file header already documents why (range values, no closed/trigger state to hang a popover off) — re-opening that is a UX change, not this plan's scope. |
| 6 | Line-item date fields (`PurchaseReceivePage` batch expiry, `StockAdjustmentFormPage` line expiry) migrate too, in Phase C, using a compact variant (no visible label, small footprint for a table cell). | Same rule 6 violation (raw `<input>` in a page) and same inconsistency the user reported; skipping them would leave half the app un-migrated. |

## Phases

| Phase | File | What | Size | Status |
|---|---|---|---|---|
| A | [phase-a-component.md](phase-a-component.md) | Build `AppDatePicker`, add to `/dev/ui` gallery + `docs/design_system.md` | S–M | done |
| B | [phase-b-migrate-appinput.md](phase-b-migrate-appinput.md) | Migrate all ~25 `AppInput type="date"` call sites; retire the `'date'` type from `AppInput` | M | done |
| C | [phase-c-migrate-raw-inputs.md](phase-c-migrate-raw-inputs.md) | Migrate the remaining raw `<input type="date">` sites (`InvoiceFormPage`, `JournalDetailPage`, line-item expiry fields) | S–M | done |

**Why this order.** A builds and proves the component in isolation (gallery, both themes, RTL,
keyboard) before anything depends on it. B is the bulk of the value (most call sites) and is
mechanical once A is done. C mops up the remaining bespoke spots, including the smaller table-cell
variant.

**Agent instructions** (same rules as [15](../../../docs/v2/15-action-plan.md) and
[17](../../../docs/v2/17-ui-system-rtl-themes.md)):
- UI-only against the mock backend. No service/type changes — this plan only touches presentation.
- Tick boxes as you go; set each phase's Status to `done` and add a status note at the top of that
  phase file when finished.
- One commit per phase.
- Run the **whole** e2e suite at the end of each phase, not just flows touching the pages you edited
  — date fields appear in invoice, purchase, payment, voucher, journal, expense, and setup flows.
- When all three phases are done and green, move this folder to `plans/completed/` and update
  `docs/v2/README.md` if it indexes this plan.

**Definition of done (every phase):** `bun run build`, `bun run check`, `bun run verify:mocks`,
`bun run memory` (Phase A adds a new shared component — structural), the full e2e suite with no
console errors, light + dark at 1280/1920 px, RTL, and by keyboard (calendar must open/navigate/close
with keyboard only — reka-ui's `Popover`/`Calendar` give this for free, verify it isn't broken by
custom trigger markup).

## Risks

| Risk | Mitigation |
|---|---|
| Typed-text parsing accepts a value the calendar would never produce (e.g. `2026-13-45`) | Reuse the same `YYYY-MM-DD` regex + `Date` validity check already implied by `dateKeyToIso`; invalid typed text is rejected on blur, not silently coerced |
| `@internationalized/date` `DateValue` timezone handling drifts from `toDateKey()`'s local-time convention | Convert via calendar day/month/year fields directly (no `Date` object round-trip inside the picker itself), mirroring how `Calendar.vue` already avoids timezone math |
| Popover z-index/overflow inside dialogs or sticky save bars (invoice/purchase forms use full-height scrolling layouts) | Verify in Phase A's gallery example nested inside a `Dialog`, and again in Phase B/C's real pages, not just standalone |
| Migration touches money-adjacent pages (invoice, purchase, payment dates) | These are presentation-only date *fields*, not posting logic — no math changes; still run `bun run verify:mocks` after each phase per the definition of done |
