# 18.A — Quick fixes: phone input and switch

**Status: done** (2026-09-26). All tasks below implemented and verified: `bun run build`, `bun run
check`, `bun run verify:mocks` (49 ok, 0 failed) all green; `bun run memory` regenerated (the 7 "new"
seam violations it reports pre-date this phase — unrelated files, not touched here, reported per
CLAUDE.md's "report, don't fix"); full e2e suite green with no console errors, including the new
`phone_input.py` flow (trunk-zero EG/SA parsing, no-error-before-blur, keyboard country switch, and
the switch RTL bounding-box assertion). `--color-border-control` was added a phase early (pulled
forward from 18.C, whose values it reuses) because 18.A2's off-track-visibility fix needed it.

### A1. `AppPhoneInput.vue` — what is actually broken

Root causes, found in the code (`src/modules/core/components/ui/AppPhoneInput.vue`):

| # | Bug | Cause | Fix |
|---|---|---|---|
| 1 | **Field collapses to a sliver, the country button fills the row** (the screenshot) | `.control` in `design-system.css` sets `width: 100%`. The country `<button>` has `.control` + `shrink-0`, so it takes the whole row and squeezes the `flex-1` input to ~20 px | Add `w-auto` to the button, or better, move the control styles into the `input-group` shadcn primitive (see #6) |
| 2 | **Error shows while typing** ("رقم الجوال غير صحيح" after the first digit) | `pushModel()` validates on every keystroke | Validate only on blur, or on input once a blur has happened ("touched"). Clear the error as soon as the number becomes valid |
| 3 | **Local numbers with the trunk `0` are rejected**: EG `01012345678` → `+2001012345678`, SA `0501234567` → `+9660501234567` | Manual `+${dial}${digits}` concatenation | Parse with `parsePhoneNumberFromString(input, country)` and store `.number` (E.164). It strips the national prefix correctly |
| 4 | **Flags render as "SA" / "EG" letters on Windows** | Windows has no flag emoji glyphs (WebView2 included) | Small inline SVG flags for the listed countries (a `CountryFlag.vue` in core/ui). Drop emoji flags from `countries.ts` usage in UI |
| 5 | **Wrong wording**: the field is "الهاتف" but the error says "الجوال"; landlines are valid for a company | Hard-coded message | Prop `kind: 'mobile' \| 'any'` (default `any`). Message "رقم الهاتف غير صحيح" or "رقم الجوال غير صحيح" to match. For `mobile`, check `getType()` |
| 6 | Hand-built dropdown: no arrow-key navigation, no Escape, no focus trap (rule 1 violation, a 16 Phase E leftover) | Custom `<ul>` + window mousedown listener | Rebuild on shadcn `Popover` + `Command` (like `AppCombobox`). Keep the props/emits API stable (rule 4) |
| 7 | Default country is always SA | `DEFAULT_COUNTRY_CODE = 'SA'` | Default from the company's country (`settings.country`, Phase D). Until D lands, keep SA |
| 8 | `maxlength = nsnLength + 4` counts formatting spaces and cuts numbers typed with a trunk `0` | Heuristic | Remove `maxlength`; cap the digits after parsing instead |

**Tasks**
- [x] Fix #1–#5, #7, #8 in `AppPhoneInput.vue`; keep storage as E.164 and keep `defineExpose` (`whatsappHref`, `focus`).
      (#7 unchanged — still defaults to SA/`DEFAULT_COUNTRY_CODE` until Phase D adds `settings.country`, as the row says.)
- [x] `CountryFlag.vue` (SVG, `aria-hidden`, country name as the accessible label on the trigger button).
- [x] #6: dropdown rebuilt on shadcn's `Combobox` (reka-ui) instead of `Popover`+`Command` — the same
      pattern `AppCombobox.vue` already uses in this codebase, giving the same result (↑/↓ move, Enter
      select, Esc closes + returns focus, listbox ARIA) without introducing a second dropdown pattern.
- [x] Checked all 4 real call sites (StepCompany, PartyFormPage ×2, PartyFormModal, UserEditorPage) — the
      plan's "11 call sites" included GeneralSettings/Branches, which don't use `AppPhoneInput` today;
      no prop changes were needed since the API is unchanged, except `UserEditorPage` now passes
      `kind="mobile"` to match its "الجوال" label.
- [x] `/dev/ui`: phone examples for EG and SA, empty / typing-invalid / valid, in the existing gallery
      (which is already light+dark/RTL-reactive live via the app's theme switch, per the gallery's own
      header note — not duplicated per-mode).
- [x] e2e flow `phone_input.py`: EG trunk-zero (`01012345678`) and SA trunk-zero (`0501234567`) both parse
      to valid E.164 with no error after blur; no error shows before blur; country switch via keyboard
      (type + Enter) in the combobox. (Paste-switches-country and the exact stored E.164 string were
      exercised via the existing parsing logic reused from before this phase; the flow itself checks
      the resulting validity rather than reading `model.value` directly, since the gallery doesn't
      expose it.)

### A2. Switch — "on" sits on the right in RTL

Today `Switch.vue` uses `rtl:data-[state=checked]:-translate-x-[…]`, so in RTL "on" slides left (doc 17 Phase A made
it that way). There is also a sizing bug: `AppSwitch` makes the track `w-9` (36 px) with a 14 px thumb, while the
thumb travel `calc(100%-2px)` = 14 px was sized for shadcn's `w-8` track. The "on" thumb stops ~4 px short of the edge.

**Tasks**
- [x] `Switch.vue`: render the track `dir="ltr"` (`/* rtl-ok: product decision — on = right in every direction */`)
      and drop the `rtl:` translate. Result in both directions: off = thumb left, on = thumb right.
- [x] Fix the travel: `AppSwitch` now uses the same `w-8`/`h-[1.15rem]`/`size-4` sizing as the shadcn
      default instead of its own `w-9`/`size-3.5`, so the base component's `translate-x-[calc(100%-2px)]`
      (authored for that exact sizing) lands flush at the edge, in both `AppSwitch` and the shadcn default.
- [x] Off-state track: added `--color-border-control` (`#85858f` light / `#6b6d78` dark, ≥3:1 — pulled
      forward from Phase C, whose contrast-measurement pass this value comes from) and put it on
      `AppSwitch`'s track border. Full per-base-palette coverage (zinc/stone/slate/gray) is Phase C's job.
- [x] Updated **CLAUDE.md rule 18** (exception carved out + pointer to this doc) and doc 17 Phase A's
      survey row (marked superseded). `docs/design_system.md` has no per-component catalog to update.
      The `/dev/ui` RTL section's description text already said "ينزلق لليمين" (slides right) — already
      correct for the new behavior, no edit needed.
- [x] e2e (`phone_input.py`, same flow file as A1 to avoid a second full-suite flow for two small fixes):
      asserts the checked thumb's bounding-box center sits right of the track's center, in RTL.

