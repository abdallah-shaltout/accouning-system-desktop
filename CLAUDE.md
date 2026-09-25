# CLAUDE.md — Equal (ايكوال المحاسبي)

Arabic-only, RTL-first desktop accounting app: **Tauri v2 + Vue 3 (`<script setup>`) + Pinia + Vue Router
(hash) + Tailwind v4 + shadcn-vue (reka-ui) + Zod**. These rules are **mandatory** for every change —
they exist so new work never breaks the UI foundation or the accounting core.

## Goal

One product that feels **small, calm and consistent** to a non-technical shop owner or cashier:
every page looks and behaves the same ("same soul"), works perfectly in RTL, light and dark, by
keyboard, and never produces wrong accounting numbers.

## Read before working

| Doc | Why |
|---|---|
| `docs/design_system.md` | Tokens, components, do/don't — the visual source of truth |
| `docs/v2/17-ui-system-rtl-themes.md` | **Current plan**: RTL, motion, save dialog, sidebar, themes, shared UI system |
| `docs/v2/15-action-plan.md` | Definition of done (the gates below come from here) |
| `docs/v2/02-accounting-review.md` | Posting rules and invariants — read before touching any money logic |
| `docs/v2/README.md` | Index of every v2 doc and the decisions already made |

When you finish a planned task, tick its box in the doc and add a status note at the top of the phase.

## Workflow

- Work directly on `master`, in place. **No git worktrees.**
- **UI-only against the mock backend** (`src/mocks/`). There is no real database.
- **Seam rule:** pages and components call only `modules/*/services/*`. Never import `src/mocks/*`
  from a page, component or store — a real backend must be able to replace the mock without
  touching screens. **Known legacy violations** (don't add more; move them behind a service when you
  touch the file — tracked in doc 17 Phase F): `WelcomePage`, `SetupWizardPage`, `DevMenu`
  (seed/persist/latency), `VoucherDetailPage`, `ExpenseDetailPage`, `AccountantHome`,
  `StorekeeperHome` (direct `db` reads), `PurchaseFormPage` (`computePurchaseTotals`), `PartyFormPage`
  (`uid`), `PosPage` (`events.on`), `AttachmentField`/`AttachmentViewer`/`ProductImageGallery`
  (attachment store).
- Decide on the recommended option yourself; ask only about scope-changing or irreversible choices.
- Small, logically scoped commits. Commit only when asked or when a planned phase says so.

## Architecture

- Feature code lives in `src/modules/<module>/{pages,components,services,controllers,helpers,routes,types}`.
- Shared code lives in `src/modules/core/`:
  - `components/shadcn/*` — shadcn-vue primitives, owned in-repo (`bunx shadcn-vue@latest add <name>`).
  - `components/ui/*` — `App*` wrappers and project components (`AppButton`, `DataTable`, `PageHeader`,
    `MoneyText`, `StatusBadge`…), built on the shadcn primitives.
  - `components/blocks/*` and `components/layouts/*` — shared page blocks and page layouts
    (**being built in doc 17 Phase F**; see "Building pages" below).
  - `components/layout/*` — the app shell (sidebar, header).
- Product name comes only from `core/helpers/brand.ts` (`APP_NAME_AR`, `APP_NAME_EN`, `APP_SHORT`).
  Never hard-code it. Never change the Tauri `identifier` or the backup `APP_NAME`.

## UI rules

### Components
1. **shadcn first.** Use an existing `App*`/block/layout if one fits, else a shadcn primitive. Build a
   custom component only when neither fits, and put it in `core/components/ui` (shared) or
   `modules/<m>/components` (feature-only) — never inline a reusable piece inside a page.
2. **Never duplicate.** If you are about to copy markup or logic from another page (a table, a form
   section, a line-items grid, a filter row, a totals card), extract or reuse the shared version instead.
3. Every new shared component is added to the dev gallery (`/dev/ui`, `core/pages/DevUiPage.vue`)
   with light, dark and RTL examples, and to `docs/design_system.md`, in the same commit.
4. Keep an `App*` wrapper's props/emits API stable when rebuilding it — pages must not need edits.

### Building pages (target system — doc 17 Phase F)
5. A page = **one layout + blocks**: `ListPage`, `FormPage`, `DetailPage`, `SettingsPage`, `ReportShell`.
   Until a layout exists, follow the closest existing page's structure and `PageHeader`.
6. **No raw `<table>`** in `modules/*/pages` — use `DataTable` (print pages / Typst templates excepted).
7. **No bare `<input>`/`<select>`/`<label>`** in pages — use `FormField` + an `App*` control.
8. Forms use Zod validation, a sticky save/cancel bar, an unsaved-changes guard, and Ctrl+S to save.
9. Line-item documents (invoice, purchase, journal, adjustment, count, transfer) share **one**
   line-items editor; totals/VAT math stays in the existing helpers.
10. Money, dates, numbers and statuses are formatted only via `MoneyText`, DataTable column types,
    `format.ts` helpers and `StatusBadge`. Numbers stay LTR inside RTL text (`num` class).
11. Shared Arabic copy (empty, error, loading, unsaved changes, confirm delete) lives in the blocks,
    not repeated in pages.
12. Page files stay under ~250 lines; move page-specific pieces into `modules/<m>/components/`.

### Design tokens
13. Colors, radius and fonts come **only** from tokens in `src/assets/styles/design-system.css`
    (bridged to shadcn via `@theme inline`). No hex colors, no `rounded-[…]`, no `text-[Npx]`
    (`bun run check` enforces the text sizes).
14. Theme settings (mode, accent, base color, radius, font, density) are per-device settings in
    `core/controllers/useAppearance.ts` / `useTheme.ts`. Everything must look right in **every**
    theme preset, light **and** dark.
15. One primary-color action per view. Hairline borders, not heavy shadows. No font weight ≥ 700.

### RTL — real RTL, not "swap left and right"
16. Use logical utilities only: `ms/me`, `ps/pe`, `start/end`, `border-s/e`, `rounded-s/e`,
    `text-start/end`. Physical `left/right/ml/mr/pl/pr/text-left/text-right` are allowed only when
    driven by an explicit physical `side` prop (Floating UI sides, Sheet/Sidebar `side`), or for
    centering (`left-1/2 -translate-x-1/2`) — mark such lines `/* rtl-ok: reason */`.
17. Icons carry **meaning**: back points →, forward/next/open points ←, and sequence chevrons,
    breadcrumbs, submenu arrows and pagination are mirrored. Physical/numeric directions (trends,
    sort asc/desc, from→to between LTR amounts, play, undo/redo circles) are **not** mirrored.
18. Things that slide, fill or move follow reading direction: "next" content enters from the left,
    progress fills from the right, a switch's "on" thumb sits on the left. Arrow-key navigation is
    mirrored (reka-ui does it via `ConfigProvider dir="rtl"`; custom handlers must too).
19. Phone numbers, IBANs, codes stay `dir="ltr"` inside RTL layouts.

### Motion
20. **Animations always run at full power.** Do not add reduced-motion checks, `motion-reduce:`
    variants, `prefers-reduced-motion` media queries or a "reduce motion" setting.

### Desktop behavior
21. Any file the user exports (Excel, PDF, backup, JSON) goes through the **native Save dialog**
    (Tauri `plugin-dialog` `save()` + `plugin-fs`), never a silent `<a download>`
    (browser fallback only for dev/e2e). One shared save helper — see doc 17 Phase C.
22. The POS keeps its full-screen chrome-free layout. App chrome has the `no-print` class.

### Navigation
23. The sidebar stays small: collapsible groups (only the active one open), brand/branch at the top,
    user at the bottom. New pages go **into an existing group** in `core/helpers/navigation.ts`
    with an `area` for role filtering — do not add new top-level groups without a reason.
24. Every new page gets `meta.title` (and `meta.section`) for the breadcrumb, and is findable from
    the command palette.

## Accounting safety

- Never change posting rules, VAT math (tax-inclusive by default, discount order: line → invoice →
  VAT), weighted-average cost or the system-role account resolution without reading
  `docs/v2/02-accounting-review.md` and keeping `bun run verify:mocks` fully green.

## Definition of done (every change that touches UI or logic)

```bash
bun run build                 # vue-tsc + vite build
bun run check                 # text-token guard today; RTL / UI-rule / contrast guards join it (doc 17)
bun run verify:mocks          # accounting invariants — must be all OK, 0 failed
bun run dev                   # (in the background) e2e needs the app on http://localhost:1420
python scripts/e2e/run.py     # the WHOLE suite, not only the flows you touched (--only <flow> for one)
bun run stop                  # free ports 1420/1421 when done
```

- All e2e flows green with **no console errors**. Run the full suite at the end of every phase —
  partial runs have missed real regressions before.
- Check light + dark, at 1280 px and 1920 px, in RTL, and by keyboard.
- New tables and entities: Excel export + command-palette search.
- Tauri/Rust changes: `cargo build --manifest-path src-tauri/Cargo.toml` and a real `bun run desktop` check.
- e2e selectors: prefer `get_by_role` / accessible names over tag or class selectors; use
  `safe_print()` (not `print()`) in flow files — the Windows console can't encode Arabic.

## Don't

- Don't import mocks from UI code, hard-code the app name, or use hex colors / pixel text sizes.
- Don't copy-paste a table, form, filter bar or line grid into a new page — reuse the shared one.
- Don't use physical left/right utilities or mirror icons blindly.
- Don't add reduced-motion code or silent downloads.
- Don't skip hooks (`--no-verify`) or leave the suite red.
