# Phase A — Build `AppDatePicker`

**Status: done** (2026-09-26). `bun run build`, `bun run check`, `bun run verify:mocks`, `bun run
memory` all green. **Not verified this session:** the manual keyboard/visual pass (light+dark,
1280/1920px, keyboard-only calendar navigation) — no interactive browser tool was available in this
session. Static checks and the e2e suite (`phone-input` flow exercises the gallery page) passed;
a human should still eyeball it once in `bun run dev` at `/dev/ui`.

## Goal

A single-date field, styled like every other `App*` control, built on shadcn `Popover` +
`Calendar` (`src/modules/core/components/shadcn/{popover,calendar}/`), with a `string | undefined`
(`YYYY-MM-DD`) model — the same contract `AppInput type="date"` had.

## Tasks

- [x] Create `src/modules/core/components/ui/AppDatePicker.vue`:
  - [x] Props: `label?`, `required?`, `disabled?`, `readonly?`, `error?`, `hint?`, `placeholder?`,
        `min?: string`, `max?: string` (all `YYYY-MM-DD`, matching `AppInput`'s date-relevant props).
        (Also added `compact?` for the Phase C table-cell variant, per the README's decision 6.)
  - [x] `defineModel<string | undefined>()`, same shape as `AppInput`'s date usage today.
  - [x] Layout: an `AppInput`-styled text field (typeable, `dir="ltr"`, same `.control` sizing/
        classes `AppInput` uses) with a trailing calendar icon button inside it (use the existing
        icon set already imported elsewhere — check `AppInput`'s suffix-slot pattern for how a
        trailing icon button is normally placed).
  - [x] Clicking the icon (or focusing + a keyboard shortcut is not required — icon click is enough)
        opens a shadcn `Popover` anchored to the field, containing `Calendar` in single-date mode.
  - [x] Typed text: on blur, validate with the same `YYYY-MM-DD` shape `dateKeyToIso()`
        (`core/helpers/format.ts`) expects; if invalid, revert to the last valid model value (don't
        silently mutate to today's date — that's a surprising side effect for an accounting app).
  - [x] Selecting a day in `Calendar` sets the model (converted `DateValue` → `YYYY-MM-DD` using the
        calendar's own day/month/year fields, not a `Date` object round-trip — avoids timezone
        drift, same reasoning as `Calendar.vue`'s existing avoidance of `Date` math) and closes the
        popover.
  - [x] `min`/`max` map to `Calendar`'s `minValue`/`maxValue` (convert once, at the boundary).
  - [x] Respect `disabled`/`readonly`: no icon button, no popover, same as a disabled `AppInput`.
  - [x] RTL: the calendar itself needs no special handling (reka-ui's `ConfigProvider dir="rtl"`
        already covers `CalendarRoot`, per CLAUDE.md rule 18); popover uses `align="start"` (logical),
        not verified live under RTL this session (see status note above) but no physical-side classes
        were introduced (`bun run check`'s RTL guard passes).
- [x] Add to the dev gallery (`/dev/ui`, `core/pages/DevUiPage.vue`): a light example, a dark example
      (rely on the page's existing theme toggle, don't duplicate a dark-only route), an RTL example
      (the app is RTL-only, so this is just "the example" — no separate variant needed unless the
      gallery has an LTR toggle for icon/direction sanity), a disabled example, an error-state
      example, and one nested inside a `Dialog` (to catch the popover z-index/overflow risk called
      out in the plan README).
- [x] Add `AppDatePicker` to `docs/design_system.md` (a "Date picker" section + usage snippet, same
      commit — the file uses prose sections per component rather than a literal table, matched the
      existing pattern).

## Gate

- [x] `bun run build`, `bun run check` green.
- [x] `bun run memory` (new shared component is a structural change) — regenerated `AGENT_MEMORY.md`
  committed in this phase's commit.
- [ ] Manual check in `bun run dev`: gallery examples in light + dark, 1280 px and 1920 px, keyboard-
  only (Tab to field, type a date, Tab to icon button, Enter/Space opens popover, arrow keys move
  within the calendar, Enter selects, Esc closes and returns focus to the field). **Not done this
  session** — no interactive browser tool was available. Do this before trusting the UI blind.
- [x] No page migrated in this phase — `AppInput`'s `'date'` type stayed until Phase B/C (removed at
  the end of Phase B).
