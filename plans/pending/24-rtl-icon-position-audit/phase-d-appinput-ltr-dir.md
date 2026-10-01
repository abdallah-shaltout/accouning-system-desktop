# Phase D — `AppInput`'s `ltr` prop: icon position vs text direction disagreement (user-reported)

## Status: done

## What was wrong

Screenshot: the password field's eye-toggle button rendered on the **left**, and the password dots
were pushed against it on the left edge instead of being right-aligned with clear space before the
icon.

Root cause in [`AppInput.vue`](../../../src/modules/core/components/ui/AppInput.vue) (before fix):

- The outer `<div class="relative flex items-center">` (old line 55) had no `dir` override, so it
  always inherited the app's `dir="rtl"`.
- The suffix icon span (old line 79, `absolute end-2.5`) is positioned relative to that outer div —
  under inherited `dir="rtl"`, `end` resolves to the **left**.
- The inner `<Input>` (old line 75) got `:dir="ltr || type === 'number' ? 'ltr' : undefined"` directly
  — so for an `ltr`-flagged field, the input itself switched to `dir="ltr"`, making its own `pe-10`
  (padding-end) and `text-end` resolve to the **right**.
- Icon position and text alignment were computed against two different `dir` contexts for the same
  visual field — the icon sat left, the padding/alignment reserved space on the right. The password
  dots ended up crammed on the left with no clearance, overlapping the icon's visual territory.

`AppPasswordInput.vue` always passes `ltr` (rule 19: passwords are a code-like LTR value) and always
supplies a `suffix` slot (the eye toggle), so every password field in the app hit this exact
combination — this wasn't an edge case.

## Fix

- [x] Moved `:dir="ltr || type === 'number' ? 'ltr' : undefined"` from the inner `<Input>` to the
      **outer** wrapper div
      ([`AppInput.vue:55`](../../../src/modules/core/components/ui/AppInput.vue#L55)). Confirmed
      `Input.vue` ([`shadcn/input/Input.vue`](../../../src/modules/core/components/shadcn/input/Input.vue))
      is a plain passthrough with no `dir` of its own, so it still inherits `dir="ltr"` correctly from
      its new parent — no visual change to the text itself, only to the icon positioning.
- [x] Removed the now-redundant `:dir` from the inner `<Input>` (same effective value, now inherited).
- [x] Effect: prefix/suffix icon position (`start-2.5`/`end-2.5`) and the input's own
      `ps-8`/`pe-10`/`text-end` now resolve against the same `dir`, for every `ltr`-flagged field with
      an icon — not just passwords. Every `AppPasswordInput` call site (login, user editor, danger-zone
      confirm dialog, etc.) is fixed without touching any call site, since they all go through this one
      shared component.
- [x] `bun run check` and `bun run build` show no new errors referencing `AppInput.vue`.

## Follow-up — explicit `text-left` for password dots

After the `dir` fix above, the user asked for the password field's text to be explicitly `text-left`
(not `text-end`, which under the field's `dir="ltr"` already renders as right-aligned — a deliberate
override, not a bug fix). Implemented in
[`AppPasswordInput.vue`](../../../src/modules/core/components/ui/AppPasswordInput.vue): added a
`passwordInputClass` computed using `cn('app-password-input text-left', props.inputClass)` instead of
the old inline template-literal string, so the required `/* rtl-ok: ... */` escape comment
(`scripts/check-rtl.js`, rule 16) can sit inside the real `cn(...)` call where the guard script reads
it — the comment is now a genuine source comment, never interpolated into the rendered DOM `class`
attribute (an earlier attempt that embedded the comment inside the template-literal class string itself
was wrong: it would have leaked `/* ... */` text into the actual `class=""` HTML attribute at runtime).
`bun run check` and `bun run build` confirmed clean after this change.

## Deferred (not done this pass)

- [ ] Manual visual check of a real password field (Login page, or Settings → danger zone's confirm
      dialog) at 1280/1920px, light+dark — not done this session (no `bun run desktop` run). Same
      deferral as Phases B and C, per [[feedback_e2e-at-end]] memory.
- [ ] Check whether any **other** `ltr`-flagged `AppInput` with a `prefix` (not just `suffix`) exists
      and visually confirm it too — `grep` found 29 files referencing `ltr`/`AppPasswordInput` but most
      don't combine `ltr` with a prefix/suffix icon; worth a quick visual pass during the real app check
      above rather than reading all 29 now.

## Gate

- [x] `bun run build` / `bun run check` green, no errors in `AppInput.vue`.
- [ ] Visual confirmation pending a real app run (see Deferred above) — plan stays in `pending/` until
      that happens.
