# Phase C — `AppDatePicker` icon position (user-reported addendum)

## Status: done

## What was wrong

[`AppDatePicker.vue`](../../../src/modules/core/components/ui/AppDatePicker.vue) placed its
`CalendarDays` trigger button at the input's logical **end** (`end-1.5`, with the text input reserving
space via `pe-9`). Every other type-identifying prefix icon in the UI kit — `SearchInput.vue`'s
`Search` icon, `AppInput.vue`'s `prefix` slot — sits at the logical **start** instead. The date text
itself is correctly `dir="ltr"`/`text-end` (rule 19, numbers/dates stay LTR), so this wasn't a mirroring
bug; it was `AppDatePicker` not matching its sibling components' icon-position convention.

## Fix

- [x] `AppDatePicker.vue`: `pe-9` → `ps-9` on the text input, `end-1.5` → `start-1.5` on the
      `PopoverTrigger` button. No change to `dir="ltr"`/`text-end` on the text itself.
- [x] Audited `AppSelect.vue`, `AppCombobox.vue`, `AppTextarea.vue` — no icon-position issues found,
      no changes needed.
- [x] `bun run check` and `bun run build` show no new errors referencing `AppDatePicker.vue`.

## Deferred (not done this pass)

- [ ] Dev gallery (`/dev/ui`) and `docs/design_system.md` don't yet have an explicit before/after note
      for this specific component — the general leading/trailing icon rule added in Phase B covers the
      principle, but a dedicated `AppDatePicker` gallery row showing the icon on the correct side would
      make this concrete. Small, can be folded into Phase B's gallery work or done standalone later.
- [ ] Manual visual check at 1280/1920px, light+dark, of a real date field (Setup Wizard step 2,
      "تاريخ البدء") — not done this session (no `bun run desktop` run); flagging per
      [[feedback_e2e-at-end]] memory, same as Phase B's deferred e2e check.

## Gate

- [x] `bun run build` / `bun run check` green, no errors in `AppDatePicker.vue`.
- [ ] Visual confirmation pending a real app run (see Deferred above) — plan stays in `pending/` until
      that happens, per CLAUDE.md's "move on completion" rule (verification gate not fully green yet).
