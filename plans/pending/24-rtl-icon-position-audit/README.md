# 24 — RTL icon position: leading vs trailing in `AppButton`

## Status: pending

## The bug (confirmed, not guessed)

Screenshot: Setup Wizard footer, "التالي" (Next) button — the chevron sits on the **visual right**
of the label, with nothing to its right and the Arabic text to its left. "السابق" (Back) in the same
screenshot is correct (chevron on the right, pointing right).

Root cause, read directly from the code:

- [`dirIcon.ts`](../../../src/modules/core/helpers/dirIcon.ts) is correct: `next`/`forward`/`open` point
  left in RTL, `back`/`prev` point right in RTL — this already matches real RTL convention (iOS,
  Windows Arabic, Gmail عربي): reading starts at the right and advances left, so "next" is a
  reading-**end** action and "back" is a reading-**start** action.
- [`AppButton.vue:74-81`](../../../src/modules/core/components/ui/AppButton.vue#L74-L81) renders the
  `icon` prop **before** `<slot />` unconditionally. In a `flex` row under `dir="rtl"`, DOM-first renders
  visually rightmost. So the icon is always pinned to the reading-**start** side — correct for
  `back`/`prev` (which belong at the start) but wrong for `next`/`forward`/`open` (which belong at the
  end, i.e. after the label, on the visual left).
- `AppButton` has no prop distinguishing a leading icon from a trailing icon. It has never needed one
  until a labeled "next" button existed.
- The only call site with both an icon **and** a visible label using a "next"-direction `dirIcon` is
  [`SetupWizardPage.vue:209`](../../../src/modules/setup/pages/SetupWizardPage.vue#L209)
  (`:icon="dirIcon.next"` on the "التالي" button). Every other `dirIcon.next/forward/open` use is either
  icon-only with no label (`PdfPreview.vue` page nav, `DataTable.vue` pager — unaffected, nothing to be
  on the wrong side of) or a bare `DirIcon` with custom surrounding markup the page already controls
  (`JournalListPage.vue`, `ChartOfAccountsPage.vue`, `KpiCard.vue`, `SetupChecklistCard.vue`,
  `DevUiPage.vue` gallery rows, `DataTable.vue` expand chevron) — those are not `AppButton` and are out
  of scope for this fix, but are included below as an audit pass to make sure none of them has the same
  DOM-order mistake in their own template.
- `docs/design_system.md` and the dev gallery ([`DevUiPage.vue:410-426`](../../../src/modules/core/pages/DevUiPage.vue#L410-L426))
  only ever demo a **back** button (`icon-rtl-flip`, leading icon) — never a labeled **next** button —
  which is why this shipped without being caught visually.

## Decision (per CLAUDE.md "decide, don't ask" — this is an implementation detail, not a scope change)

**Next/forward/open points left in RTL and sits after the label (trailing). Back/prev points right in
RTL and sits before the label (leading).** This is the existing `dirIcon.ts` semantics; the only change
is giving `AppButton` a way to place the icon on the correct side instead of always leading.

`AppButton` gets a new prop: **`iconPosition?: 'start' | 'end'`, default `'start'`** (keeps every existing
call site's rendered output byte-for-byte identical — rule "keep an App* wrapper's props/emits API
stable" is satisfied because the default is unchanged and no existing call passes a new prop). Call
sites using a `next`/`forward`/`open` `dirIcon` with a visible label pass `icon-position="end"`.

Rejected alternative: a second `trailingIcon` prop. More API surface for the same job; `iconPosition`
reads clearly at call sites (`icon-position="end"` next to `:icon="dirIcon.next"`) and matches the
logical `start`/`end` vocabulary rule 16 already mandates elsewhere in this file.

## Definition of done

- [ ] `AppButton.vue` renders the icon on the correct side per `iconPosition`, default unchanged.
- [ ] `SetupWizardPage.vue`'s "التالي" button passes `icon-position="end"` and visually matches: label
      first (rightmost in RTL), chevron last (leftmost), pointing left.
- [ ] Audit pass (see Phase A) over every `dirIcon.next/forward/open`/`dirIcon.back/prev` call site
      confirms no other `AppButton` or bare-markup button has the icon on the wrong side of a label.
- [ ] Dev gallery (`/dev/ui` → `DevUiPage.vue`) gets a labeled "next" example next to the existing
      labeled "back" example, light/dark/RTL, per UI rule 3.
- [ ] `docs/design_system.md` documents the leading/trailing icon rule next to the existing
      `dirIcon`/RTL icon guidance, per UI rule 3.
- [ ] `bun run build`, `bun run check` green.
- [ ] Visual check at 1280px and 1920px, light + dark, RTL, keyboard focus ring not clipped by the
      reordered icon — per "Definition of done" in CLAAUDE.md (full e2e run deferred — see
      [[feedback_e2e-at-end]] memory: e2e/desktop checks batch into one later testing plan, not every
      small UI phase).
- [ ] `bun run memory` run if any new exported prop changes `AGENT_MEMORY.md`'s service/component
      surface listing (prop addition to an existing shared component — check if memory tracks this
      level of detail; if not, skip).

## Phases

| File | What | Size | Status |
| --- | --- | --- | --- |
| [phase-a-audit.md](phase-a-audit.md) | Confirm every `dirIcon` call site's icon position is correct; list exact fixes needed | ~10 files read, 0-2 files changed | done |
| [phase-b-appbutton-fix.md](phase-b-appbutton-fix.md) | Add `iconPosition` prop to `AppButton`, fix `SetupWizardPage`, update gallery + docs | 3 files changed | done |
| [phase-c-appdatepicker-fix.md](phase-c-appdatepicker-fix.md) | Move `AppDatePicker`'s calendar icon from `end` to `start`, matching `SearchInput`'s prefix-icon convention (user-reported, screenshot) | 1 file changed | done |
| [phase-d-appinput-ltr-dir.md](phase-d-appinput-ltr-dir.md) | Fix `AppInput`'s `ltr` prop so prefix/suffix icon position and text padding/alignment resolve against the same `dir` (user-reported, password field eye icon) | 1 file changed | done |

## Addendum — `AppDatePicker` (found during implementation, same bug class)

User reported a second instance via screenshot: the Setup Wizard's "تاريخ بدء العمل بالنظام" field had
its calendar icon sitting on the far **left**, visually detached from the date text. Read
[`AppDatePicker.vue`](../../../src/modules/core/components/ui/AppDatePicker.vue): the icon was
positioned at the input's logical **end** (`end-1.5` + `pe-9`), while every sibling input component
(`SearchInput.vue`'s magnifying glass, `AppInput.vue`'s `prefix` slot) places its type-identifying icon
at the logical **start** (`start-2.5`/`ps-8`). This isn't an RTL-mirroring bug (the date text correctly
stays `dir="ltr"`/`text-end` per rule 19) — it's an inconsistency with the established prefix-icon
convention in this UI kit: a calendar icon identifies the field type, same role as a search icon, and
should lead the field like it does everywhere else. Fixed by swapping `end-1.5`/`pe-9` to
`start-1.5`/`ps-9` ([`AppDatePicker.vue:113-136`](../../../src/modules/core/components/ui/AppDatePicker.vue#L113-L136)).
Used in 24+ pages — single shared component, fixed once. Audited `AppSelect.vue` (native `<select>`,
no custom icon), `AppCombobox.vue` (no absolute-positioned icons, trailing chevron is in natural flex
order already), `AppTextarea.vue` (no icon) — all clean, no further fixes needed.

## Addendum 2 — `AppInput`'s `ltr` prop + prefix/suffix icon (user-reported, password field)

Second user screenshot: the password field's eye-toggle sat on the **left**, and the password dots were
jammed against it on the left instead of being right-aligned with room for the icon. Root cause, read
from [`AppInput.vue`](../../../src/modules/core/components/ui/AppInput.vue): when `ltr` is set (every
`AppPasswordInput` sets it — rule 19, passwords are a code-like LTR value), only the inner `<Input>`
got `dir="ltr"` (old line 75); the **outer** `<div class="relative flex items-center">` that positions
the prefix/suffix icons via `absolute start-2.5`/`end-2.5` kept inheriting the app's `dir="rtl"`. So the
icon's `end` resolved against RTL (→ left) while the input's own `pe-10`/`text-end` resolved against
its own `dir="ltr"` (→ right) — two different coordinate systems disagreeing about which side is "end",
for the exact same field. Fix: move `:dir="ltr || type === 'number' ? 'ltr' : undefined"` onto the
**outer** wrapper div instead of the inner `<Input>` (the shadcn `Input.vue` is a plain passthrough
with no `dir` handling of its own, so it still inherits `dir="ltr"` correctly from its new parent).
Now the icon position and the text padding/alignment always agree on which side is "end", for every
`ltr`-flagged `AppInput`/`AppPasswordInput`/`AppCombobox`-adjacent field with a prefix or suffix.
Every `AppPasswordInput` in the app (login, user editor, danger-zone confirm, etc.) goes through this
one component, so the fix applies everywhere without touching call sites.

## Out of scope (later note, not a task here)

- Any *other* "same soul" RTL inconsistency not about icon leading/trailing position (e.g. motion
  direction, mirrored vs non-mirrored icons per rule 17) — this plan is scoped to the one confirmed bug
  class: label+icon buttons putting the icon on the wrong side.
- Rebuilding `DataTable`'s pager buttons to use `AppButton` instead of bare `<button>` — they're
  icon-only today so the bug doesn't apply; not touching working code outside the bug's blast radius.
