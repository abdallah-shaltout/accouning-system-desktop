# Phase B — `AppButton` `iconPosition` prop, fix call sites, gallery, docs

## Status: pending (do not start until Phase A's audit list is final)

## Tasks

### `AppButton.vue`

- [ ] Add prop `iconPosition?: 'start' | 'end'` with `withDefaults` default `'start'`
      ([AppButton.vue:17-33](../../../src/modules/core/components/ui/AppButton.vue#L17-L33)).
- [ ] Reorder the template so the icon renders before or after `<slot />` based on `iconPosition`,
      keeping the `loading` spinner's existing leading position untouched (a spinner replacing a leading
      icon stays leading regardless of `iconPosition` — don't change loading-state visuals).
      ([AppButton.vue:73-81](../../../src/modules/core/components/ui/AppButton.vue#L73-L81))
- [ ] Keep `:class="iconRtlFlip && 'rtl:-scale-x-100'"` exactly as-is — position and mirroring are
      orthogonal (`iconPosition` controls DOM order / visual side, `iconRtlFlip` controls which way the
      chevron points); both are needed together for a correct "next" button.
- [ ] Verify no visual regression for every existing call site (default `'start'` unchanged) — this is
      an additive prop, not a behavior change for anyone who doesn't pass it.

### Call sites (from Phase A's final list — update this if Phase A found more than the known case)

- [ ] `SetupWizardPage.vue:209` — add `icon-position="end"` to the "التالي"/"ابدأ العمل" button.
      Leave `icon-rtl-flip="!isLast"` and the `Check` icon for the last step untouched (a trailing
      checkmark on the final "ابدأ العمل" step is also correct trailing position, so `icon-position="end"`
      applies to both branches of `isLast`).

### Dev gallery (`DevUiPage.vue`) — UI rule 3

- [ ] Next to the existing labeled "back" example
      ([DevUiPage.vue:423-426](../../../src/modules/core/pages/DevUiPage.vue#L423-L426)), add a labeled
      "next" example: `<AppButton size="sm" variant="primary" :icon="dirIcon.next" icon-rtl-flip icon-position="end">التالي</AppButton>`
      with a caption matching the existing one's style ("زر تالي — يشير لليسار في RTL، الأيقونة بعد النص").
- [ ] Confirm it renders correctly light/dark (toggle theme in dev) and RTL (app is RTL-only, already
      covered).

### `docs/design_system.md` — UI rule 3

- [ ] Find the existing `dirIcon`/icon-mirroring documentation section (search for "dirIcon" or
      "RTL" in the doc) and add a short entry next to it: `AppButton`'s `iconPosition` prop
      (`'start' | 'end'`, default `'start'`), with the rule — back/prev = start (leading), next/forward/open
      = end (trailing) — and one code example matching the gallery addition above.

## Gate (definition of done for the whole plan)

- [ ] `bun run build` green.
- [ ] `bun run check` green.
- [ ] Manual check: Setup Wizard footer at 1280px and 1920px, light + dark, keyboard Tab focus ring
      visible and not clipped on the "التالي" button with the icon now trailing.
- [ ] Dev gallery new example renders correctly in both themes.
- [ ] Move this whole plan folder to `plans/completed/24-rtl-icon-position-audit/`, set every status
      to `done`, run `bun run memory` if it tracks component prop signatures.
