# CLAUDE.md — Equal (ايكوال المحاسبي)

Arabic-only, RTL-first desktop accounting app: **Tauri v2 + Vue 3 (`<script setup>`) + Pinia + Vue Router
(hash) + Tailwind v4 + shadcn-vue (reka-ui) + Zod**. These rules are **mandatory** for every change —
they exist so new work never breaks the UI foundation or the accounting core.

## Goal

One product that feels **small, calm and consistent** to a non-technical shop owner or cashier:
every page looks and behaves the same ("same soul"), works perfectly in RTL, light and dark, by
keyboard, and never produces wrong accounting numbers.

## Agent memory — `AGENT_MEMORY.md` (read first)

`AGENT_MEMORY.md` is a generated map of this repo: domain modules and their layers, every service
function (the seam), every route, module→module imports, the Rust↔Vue IPC contract, the mock-backend
API and who uses it, the shared UI kit, boundary violations, and a "where to find X" index. It is built
by `bun run memory` (pipeline in `scripts/memory/`: scan → parse → analyze → render; all settings in
`scripts/memory/config.ts`).

### Protocol
1. **Read `AGENT_MEMORY.md` before any task, plan, or broad search.** Resolve "where is X / who calls
   Y / does Z exist" from it first. Grep or open files only for what it does not answer, or to read
   the exact code you will change.
2. Before building a component, service, helper or route, check the Service API, Shared UI kit and
   Routes sections, then **reuse or extend** what exists (UI rules 1–2 below).
3. Treat the Boundary report as a to-do list. Don't add a new seam violation, oversized page or
   cross-module page import. When you touch a file that already has one, fix it.
4. The memory is generated. **Never hand-edit it** — change the code or `scripts/memory/config.ts`.

### Architecture rules (all new code)
- **Domain first.** Code belongs to exactly one domain module (`src/modules/<domain>/`) in the layer
  that matches its job: `pages` → `components` → `controllers` → `services` → `helpers`/`types`/
  `validators`. No new top-level folders, and no "utils" dumping grounds for domain logic.
- **Modules talk through public surfaces:** another module's `services`, `types`, `helpers` or shared
  `core` components. Never its `pages`, and never the mock backend (the seam rule).
- **Centralize configuration.** Constants, labels, navigation, brand, theme tokens, route meta and
  tool settings live in their single owning file (see "Where to find X"). Do not re-declare them
  locally or hard-code the values.
- **Scale by adding, not by branching.** New behavior goes into a new module entry (a route record,
  a service function, a palette command, a Rust command registered in `lib.rs`), not into growing
  `if`/`switch` chains in shared code. Keep functions small and single-purpose, and keep pages under
  ~250 lines.
- **Rust ↔ Vue:** a new `#[tauri::command]` gets registered in `generate_handler!` and called only
  from a `core`/module **service**, never directly from a page. It must show up in the IPC table with
  no contract gaps.

### Keeping the memory current
- After a **structural change**, run `bun run memory` and commit `AGENT_MEMORY.md` with that change.
  Structural changes are: a new/renamed/moved module, service, route, page, shared component, mock
  file or Rust command, or a new domain. The run takes under a second. `bun run memory:check` fails
  when the file is stale.
- If you find the memory stale and can't regenerate it (for example, a read-only task), **tell the
  user to run `bun run memory`** and say what looked out of date.

## Read before working

| Doc | Why |
|---|---|
| `AGENT_MEMORY.md` | Generated repo map — read first (see above) |
| `docs/design_system.md` | Tokens, components, do/don't — the visual source of truth |
| `plans/pending/` | **Active plans** — one folder per plan (see "Plans" below) |
| `docs/v2/17-ui-system-rtl-themes.md` | Earlier plan (pre-`plans/`): RTL, motion, save dialog, sidebar, themes, shared UI system |
| `docs/v2/15-action-plan.md` | Definition of done (the gates below come from here) |
| `docs/v2/02-accounting-review.md` | Posting rules and invariants — read before touching any money logic |
| `docs/v2/README.md` | Index of every v2 doc and the decisions already made |

When you finish a planned task, tick its box in the plan file and add a status note at the top of the phase.

## Plans

Every implementation plan lives in `plans/`, never loose in `docs/` or the repo root.

### Layout and lifecycle
- `plans/pending/` holds plans that are not fully done. `plans/completed/` holds plans that are implemented
  and verified. `plans/` is a live index, not an archive.
- **One folder per plan**, even if it is a single file: `plans/pending/<NN>-<kebab-name>/`. `NN` continues
  the doc numbering (the first one is `18`). The folder has a `README.md` with the overview, the decisions,
  the definition of done, and a phase table (`File · What · Size · Status`) that links every file.
- A plan with phases or sub-specs gets **one file per phase** (`phase-a-<name>.md`, …) inside its folder, not
  one huge file. A small plan can be just `README.md`.
- Links to docs are relative from the plan folder (`../../../docs/v2/02-accounting-review.md`), so they keep
  working when the folder moves.
- **Move on completion, in the same turn.** When the last task of a plan is done and its definition of done is
  green (build, check, verify:mocks, full e2e), move the **whole folder** with
  `git mv plans/pending/<plan> plans/completed/<plan>`, set every status to `done`, fix any link that pointed at the
  old path (for example `docs/v2/README.md`), and run `bun run memory`. This is part of finishing the work, not a
  later cleanup. If something could not be verified (for example no real `bun run desktop` check this session),
  leave the plan in `pending/`, write exactly what is missing at the top of its README, and tell the user.
  Never leave its status unclear.

### Plan quality (a plan is ready only when all of these hold)
- **Production-ready:** no placeholder logic and no "TODO: figure out later".
- **Checklist-driven:** every phase has `- [ ]` tasks and a gate, so implementation is not improvised.
- **Grounded in this repo:** it names the real files, services, routes and tokens (from `AGENT_MEMORY.md`), and
  follows the architecture and UI rules in this file. No generic solutions bolted on.
- **Not overkill:** it solves what was asked; extra ideas go in a short "later" note, not in the tasks.
- **No conflicts:** checked against existing features, routes, models and the other pending plans. If it reverses
  an earlier decision or a rule in this file, it says so and updates that rule as a task.
- **Traceable:** every step has a stated reason (a bug found in code, a measurement, a user decision). Nothing is
  guessed; open questions are listed as decisions for the user.

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
    progress fills from the right. **Exception (product decision, doc 18.A2):** a switch's "on"
    thumb always sits on the **right**, in every direction — unlike iOS/Material, which mirror it
    in RTL. `Switch.vue` renders its track `dir="ltr"` for this reason (`/* rtl-ok: … */`). Arrow-key
    navigation is mirrored (reka-ui does it via `ConfigProvider dir="rtl"`; custom handlers must too).
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

## Diagnostics (18.B — read before touching errors, performance or the audit trail)

One logger, five channels (`error`, `perf`, `debug`, `audit`, `accounting`), one `LogEntry` JSON
schema — `src/modules/diagnostics/services/logService.ts`'s `log.error/perf/debug/audit/accounting()`.
Never `console.error`/`console.log` for anything a user could hit or a developer might need to
correlate later; call `log.*` instead so it lands in the right channel with a fingerprint and a
correlation id.

- **How to log.** `log.error(source, msg, err?, data?)` for anything that broke (always on).
  `log.perf(source, msg, data)` for a duration/budget note (always on, cheap). `log.debug(namespace,
  msg, data)` for a trace that should stay silent unless a developer turned that namespace on via
  `localStorage['equal.debug']` or the `/dev/diagnostics` → التتبع tab. Business audit
  (`logAudit`/`logActivity` in `mocks/backend/core.ts`) is separate from these — it is business data
  (who did what to which entity), lives in `db.audit`, ships in every backup, and is never a
  diagnostic log (see plan doc 18 decision 3). A new backend write that changes a business entity
  should call `logAudit` (or the `logActivity` adapter) at the same point it always did, not `log.*`.
- **Where to look.** `/dev/diagnostics` (dev builds, linked from the sidebar user menu's dev section
  and the command palette) has tabs for Errors (grouped by fingerprint), Performance (p50/p95 +
  budget breaches), Debug (namespace toggles + live tail), Audit and Accounting (stub — Phase F).
  `Ctrl+Shift+D` opens a docked overlay with the last 50 entries across every channel without
  leaving the page. In dev, `.diagnostics/logs/<channel>.jsonl` (gitignored) holds today's entries on
  disk — read it directly instead of reproducing a bug through the UI when a log line will do.
- **Opening a ledger issue.** `docs/diagnostics/ISSUES.md` is a generated index (like
  `AGENT_MEMORY.md`) over `docs/diagnostics/issues/*.md` — **never hand-edit `ISSUES.md`**. To open
  an issue: add a `docs/diagnostics/issues/<KIND>-NNNN-<slug>.md` file with the YAML frontmatter
  schema in `scripts/diagnostics/frontmatter.ts` (`id`, `kind`, `status`, `area`, `fingerprint?`,
  `first_seen`, `last_seen`, `occurrences`, `debug_namespace?`, `regression_test?`, `fixed_in?`) and a
  body with repro steps, what was tried and failed, and the related files, then run `bun run diag` to
  rebuild the index. `bun run diag` also auto-stubs a new `BUG-NNNN` file for any error fingerprint
  seen in the local dev log that has no issue file yet, so nothing silently falls off the ledger.
- **Closing a ledger issue.** Set `status: fixed` and `fixed_in: <commit>` once the fix lands; it only
  becomes `verified` once a `regression_test` (a `scripts/verify/cases/*.json` case) exists and
  passes. Run `bun run diag` afterward so `ISSUES.md` picks up the change; `bun run diag:check`
  (wired into the definition-of-done gates) fails the run if it's stale.
- **User-facing.** An error toast always ends with a short code (`رمز الخطأ: E-7F3A` — the first 4 hex
  characters of the error's fingerprint). Settings → حول / الدعم → "تصدير ملف التشخيص" exports a zip
  of every channel's logs + app version + OS + redacted settings (and, opt-in only, a DB snapshot)
  through the shared `saveFile` helper — never build a second export path for this.

## Definition of done (every change that touches UI or logic)

```bash
bun run build                 # vue-tsc + vite build
bun run check                 # text-token guard today; RTL / UI-rule / contrast guards join it (doc 17)
bun run verify:mocks          # accounting invariants — must be all OK, 0 failed
bun run memory                # regenerate AGENT_MEMORY.md (structural changes); memory:check verifies
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
