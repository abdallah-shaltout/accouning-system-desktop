# 05 — UI rules

These carry over the desktop app's design rules (`docs/design_system.md` and the UI rules in the root `CLAUDE.md`),
adapted for the web.

## Tokens

- Start from the desktop `src/assets/styles/design-system.css` token values, **copied** into
  `src/shared/styles/tokens.css` (this is a separate project, so it isn't imported). Bridge them to shadcn with
  Tailwind v4 `@theme inline`.
- Colors, radius and fonts come only from tokens. No hex, no `text-[Npx]`, no `rounded-[…]`, no font weight ≥ 700.
- Hairline borders, not heavy shadows. One primary-color action per view.
- Light and dark must both look right.

## RTL

- `<html dir="rtl" lang="ar">`, plus reka `ConfigProvider dir="rtl"`.
- Logical utilities only: `ms/me`, `ps/pe`, `start/end`, `border-s/e`, `rounded-s/e`, `text-start/end`.
- Icons carry meaning: back points →, forward/next points ←. Numeric and physical directions aren't mirrored.
- Phone numbers, codes (the 6-char activation code, invoice numbers), amounts and versions go in `dir="ltr"` / the
  `num` class inside RTL text.

## Forms

- `FormField` + App controls, Zod schemas from the module's `schemas/`, vee-validate.
- Errors appear under the field in Arabic. Server `fieldErrors` map to the same fields.
- A long form (the plan version editor) has a sticky save bar, an unsaved-changes guard and Ctrl+S.

## Tables

- `DataTable` with server pagination, sort and search mapped to the backend `ApiFeatures` query. It has an empty
  state, a loading skeleton, and row actions in a dropdown.

## Copy

- Arabic, short and calm. It speaks to a shop owner, not an accountant.
- The upgrade language matches the desktop: «رقّي», «جرّبها», «الأنسب», «شهرين هدية», «أقل من 10 جنيه في اليوم».
- The shared messages (empty, loading, error, unsaved, confirm delete, session expired) live in `shared/config/copy.ts`.
- Error toasts use the backend `code` → Arabic map in `shared/api/errors.ts`.

## Portal on phones

- Portal pages work at 360 px: single column, large touch targets, and receipt upload from the camera
  (`accept="image/*,application/pdf" capture`).
