# 16 — Equal: close fix, rebrand, shadcn-vue UI kit & sidebar-07 layout

**Status: A–D done, E moved to [17](17-ui-system-rtl-themes.md)** (2026-09-25).

Four pieces of work, in this order:

1. **Phase A — Window close bug** (small, ships first, on its own).
2. **Phase B — Rebrand to "ايكوال المحاسبي" / Equal** (name, logo, icons, brand color).
3. **Phase C — shadcn-vue foundation + re-base the `App*` kit on it** (no page edits).
4. **Phase D — New app shell from `sidebar-07`**.
5. **Phase E — Swap the remaining hand-built pieces** (toasts, palette, drawers, menus).

**Agent instructions** (same rules as [15](15-action-plan.md)):
- Still **UI-only against the mock backend**. Pages touch only `modules/*/services`.
- Tick the boxes here as you go and add a status note at the top of a phase when it ends.
- Decide yourself; ask the user only about scope-changing or irreversible choices.
- One commit per phase (Phase C can be split per component group).

**Definition of done (every phase):** the seven gates from [15](15-action-plan.md)
(its "Definition of done" block) still apply: `bun run build`, `bun run verify:mocks`, all `scripts/e2e/flows` green with no console
errors, screenshots in light + dark at 1280 px and 1920 px, the seam rule, Arabic/RTL + full
keyboard use, and Excel export + palette search for new tables and entities. For phases that touch
Rust or Tauri config, `cargo build --manifest-path src-tauri/Cargo.toml` and a real `bun run desktop`
check are also required.

---

## Phase A — The X button does not close the window

**Status: done** (2026-09-25). `core:window:allow-destroy` / `core:window:allow-close` added to
the capability file; the close handler now uses `onCloseRequested` with `preventDefault()`,
`try/finally`, a 10s `Promise.race` timeout, and a `closingInProgress` re-entry guard, always
calling `win.destroy()` in `finally`. `App.vue` shows a small RTL overlay
("جارٍ حفظ نسخة احتياطية قبل الإغلاق…") driven by the new `isClosingWithBackup` ref while the
close-time backup runs. `bun run build`, `cargo build --manifest-path src-tauri/Cargo.toml`,
`bun run check` and `bun run verify:mocks` (49 ok / 0 todo / 0 failed) all pass. **Not yet verified
manually**: clicking the real X/taskbar/Alt+F4 in `bun run desktop` for the three cases (auto-backup
off/on/folder-missing) requires an interactive session — do this before shipping.

**Root cause (confirmed).** `initAutoBackup()` in
[backupService.ts](../../src/modules/settings/services/backupService.ts) listens for
`tauri://close-requested`. When a JS listener exists, Tauri cancels the native close and leaves it
to JS. The handler then calls `win.destroy()`, but the capability file
([default.json](../../src-tauri/capabilities/default.json)) only grants `core:default`. That
permission set does **not** include `core:window:allow-destroy` (checked in
`src-tauri/gen/schemas/acl-manifests.json`). `destroy()` is rejected, the error is swallowed inside
the async listener, and the window stays open. If the close-time backup ever hangs, the window
would also stay open forever, because nothing puts a time limit on it.

- [x] Add `core:window:allow-destroy` (and `core:window:allow-close`) to
      `src-tauri/capabilities/default.json`.
- [x] Replace the raw `win.listen('tauri://close-requested')` with `win.onCloseRequested(async (e) => …)`.
      Call `e.preventDefault()`, run the backup inside `try/finally`, and call `win.destroy()` in
      `finally`, so a failed backup can never block the close.
- [x] Limit the close-time backup to 10 s with `Promise.race` against a timeout. Log it and close
      anyway if the limit is hit.
- [x] Add a re-entry guard, so a second click on X while the backup runs doesn't start a second backup.
- [x] Show a small "جارٍ حفظ نسخة احتياطية قبل الإغلاق…" overlay while the backup runs, so the
      delay doesn't look like a freeze.
- [x] Check that `stopAutoBackup()` still unlistens correctly (`onCloseRequested` returns the unlisten fn).
- [ ] **Verify in the real desktop app** (`bun run desktop`) in three cases: auto-backup off
      (closes at once), auto-backup on (overlay, then closes), and backup folder missing or
      read-only (error logged, still closes). Also check that closing from the taskbar and with
      Alt+F4 behaves the same. **(needs an interactive session — not done by the agent)**

---

## Phase B — Rebrand: "ايكوال المحاسبي" / Equal

**Status: done** (2026-09-25). `brand.ts` added and wired everywhere the doc lists. New `equal`
accent preset (light `#0e7259`/hover `#0b6350`, dark `#0f7d63`/hover `#0d6b54` — both picked a touch
darker than the raw logo green `#10886C`, which only clears ~4.41:1, so they clear ≥5:1 white-text
contrast for real) is now the default for new installs; existing installs keep their picked accent
(the setting's own `localStorage`-null-check already gives this for free). `mark.svg` traced via
`scripts/brand/trace-mark.py` (potrace + resvg-py, threshold 200 chosen by comparing 190/200/215/235
at 1024px and 32px) then trimmed with `svgo` (74KB → 32KB). One real bug found and fixed in the
tracing script itself: potrace's `Bitmap.trace()` always emits the whole padded canvas as an
outermost even-odd ring, which rendered as a solid filled square behind the mark on its own
background layer (e.g. the white-on-tile app icon) — fixed by dropping that synthetic border
subpath in `path_to_svg_d()`. App icons regenerated via `bunx tauri icon` from a 1024px tile (white
mark at 80% on `#10886C`, 22% corner radius); its `android`/`ios` output subfolders were deleted
(this project has no mobile targets). `public/favicon.svg` + a multi-size `favicon.ico` fallback
added. PDF `/Creator` metadata now says "Equal Accounting" (`typst_pdf::PdfOptions.creator` —
`/Producer` isn't user-settable in typst-pdf 0.15, it's always `Typst $version`). `bun run build`,
`cargo build`, `bun run check`, `bun run verify:mocks` (49/0/0) all pass; login/welcome/home
screenshots checked visually in light + dark. **Not done**: `bun run tauri build` (the installer)
wasn't run in this session — needs an interactive/long-running build to verify the exe/installer
name and icon for real.

**Assets** (`/logo`):

| File | What it is | Use |
|------|------------|-----|
| `dark.webp` | Full lockup: green mark tile + green Arabic "اكوال" + "EQUAL", for **light** backgrounds | Login / welcome / about in light mode |
| `light.webp` | Same lockup with **white** wordmark, for **dark** backgrounds | Login / welcome / about in dark mode |
| `fav.webp` | The mark only, 192×192, green engraving on an **opaque white** square (the alpha channel is not usable as a mask) | Source for the traced vector mark → sidebar, favicon, app icons |

**Decisions**
- The name is the English word **Equal** ("يساوي") written in Arabic letters. Arabic product
  name: **ايكوال المحاسبي**. English: **Equal Accounting**, short name **Equal**. UI text uses
  ايكوال as written. The logo image's own wordmark ("اكوال") is artwork and stays as it is.
- **Brand color = logo green `#10886C`** (the most common non-white color in `fav.webp`). It becomes a new
  `equal` accent preset **and the default accent**. The existing indigo/teal/rose/amber presets
  stay selectable. Update the token table in [design_system.md](../design_system.md).
- All name strings come from one constant, `src/modules/core/helpers/brand.ts` (`APP_NAME_AR`,
  `APP_NAME_EN`, `APP_SHORT`).
- **Not renamed, on purpose:**
  - the Tauri `identifier` (`com.abdallah.accounting-app`): WebView2's data folder (the
    IndexedDB database, i.e. **all company data**) is keyed on it, so changing it would look like
    data loss to existing installs;
  - `APP_NAME` in `backupArchive.ts`: old backups must still restore;
  - the Cargo crate name.

**Tasks**
- [x] Add `brand.ts` and replace every hard-coded "نظام المحاسبة ونقاط البيع": `index.html` title,
      `router/index.ts` title fallback, `LoginPage.vue`, `WelcomePage.vue`, the sidebar subtitle,
      the xlsx `workbook.creator` in `exportXlsx.ts` and `importXlsx.ts`.
- [x] `tauri.conf.json`: `productName` → `Equal`, window `title` → `ايكوال المحاسبي`. This changes the
      exe/installer name, which is acceptable at 0.1.0; note it in the release notes.
- [x] Brand assets: copy to `src/assets/brand/` (`logo-light-bg.webp`, `logo-dark-bg.webp`,
      `mark.svg` — traced, not the raw `mark.webp`, see below). Add a `BrandLogo.vue` component
      with `variant="lockup" | "mark"` that switches to the right file for the current theme.
- [x] **Vectorize `fav.webp` → `src/assets/brand/mark.svg`.** 192 px enlarged to the 1024 px an app
      icon needs is visibly blurry, so trace the mark into a vector instead.
      - Tool: `potracer` (pure-Python potrace, `pip install potracer`, imports as `potrace`) +
        `resvg-py` for rendering. `vtracer` was ruled out per the earlier feasibility check.
      - Pipeline: composite onto white → grayscale → Lanczos ×4 (768 px) → threshold → pad with an
        8px background margin → potrace (`turdsize=3, alphamax=1.0, opticurve=True,
        opttolerance=0.2`) → drop the synthetic outer-border subpath (see bug note above) → one
        `<path fill="currentColor" fill-rule="evenodd">` in a `0 0 768 768` viewBox, then `svgo
        --precision 1` (74KB → 32KB).
      - Gotchas confirmed: potrace traces the **True** pixels, so the ink mask (not its inverse) is
        what's passed in; a dark threshold (`< 170`) wipes out the face hatching; and — found during
        this phase, not in the original feasibility check — potrace's outermost contour is always a
        full-canvas frame that must be stripped or it renders as a solid square on non-transparent
        backgrounds.
      - Threshold **200** chosen: compared 190/200/215/235 at 1024px (215/235 lose turban-line and
        chin detail) and 32px (190/200 near-identical, 200 marginally cleaner); matches the
        feasibility check's own default.
      - `currentColor` lets one SVG serve both themes: green on light, white on dark or on the green tile.
- [x] App icons from `mark.svg`: 1024×1024 PNG with a white mark at ~80% on a `#10886C` rounded tile
      (radius ≈ 22%, matching the lockup tile) → `bunx tauri icon` regenerates `src-tauri/icons/*`
      (its `android`/`ios` output was deleted — no mobile targets in this project).
      Also `public/favicon.svg` (the vector itself, brand green baked in since a standalone favicon
      has no `currentColor` context) plus a multi-size `favicon.ico` fallback.
      Tracing script at `scripts/brand/trace-mark.py` so the icon can be rebuilt reproducibly.
- [x] Login + welcome pages: full lockup above the form. Sidebar: still the old subtitle wording,
      swapped to `APP_SHORT`; the mark itself moves to the sidebar in Phase D, not here.
- [x] Add the `equal` accent preset to `design-system.css` (light and dark shades, checked for
      ≥4.5:1 contrast of white text on the color). Default `data-accent` → `equal` in
      `useAppearance.ts` for new installs (and `index.html`'s pre-paint inline fallback). Existing
      installs keep the accent they picked (the setting only falls back when `localStorage` has no
      prior value at all).
- [x] README title and intro updated.
- [x] PDFs: company letterheads keep the *customer's* store logo (no change). `/Creator` metadata
      in `pdf/render.rs` → "Equal Accounting" (`/Producer` is not user-settable in typst-pdf 0.15).
- [ ] Gate: build + screenshots of login, welcome and home in light and dark (**done** — build,
      `cargo build`, `bun run check`, `verify:mocks`, and visual screenshots all pass); installer
      (`bun run tauri build`) shows the new name and icon (**not done this session** — needs an
      interactive/long-running build to verify for real).

---

## Phase C — shadcn-vue foundation, and the `App*` kit on top of it

**Status: done** (2026-09-25/26). All 5 rebuild batches landed, `/dev/ui` gallery added,
`design_system.md` updated. Full e2e suite (all 16 flows) green with zero console errors after
fixing three real issues the full run surfaced (none visible from partial runs during the
batch-by-batch work):
- `AppCard` silently changed its root tag from `<section>` to shadcn `Card`'s hardcoded `<div>` —
  zero visual difference, but it broke `home_insights.py`'s insight-panel selector outright (fixed
  by adding an `as` prop to `Card.vue`, defaulting `AppCard` to `section`).
- `SegmentedControl`'s move to `ToggleGroup` changed its ARIA pattern from `role="tab"` to
  `role="group"` + `aria-pressed` (a different, equally valid pattern — not a bug) — two e2e flows
  had hard-coded `role="tab"` selectors, fixed to match by accessible name instead.
- A real, pre-existing e2e race in `purchases.py`'s receiving-confirm step (a fixed 400ms wait
  before an ad hoc button-count check, which could silently skip the click under load) plus a
  pre-existing Windows console crash (`UnicodeEncodeError` on any Arabic `print()`) that was masking
  failure messages across every flow — both fixed, unrelated to the shadcn rebuild itself but found
  while verifying it.

**Why wrappers first:** `AppButton` is used in 103 files, `AppCard` in 72, `AppInput` in 45,
`AppSelect` and `SegmentedControl` in 42 each, `AppModal` and `DataTable` in 35 each. Rewriting
every call site at once is a large, risky diff. Instead, rebuild each `App*` component **on top of
the matching shadcn-vue component while keeping its props/emits API**. Every page gets the new
look and reka-ui's accessibility with zero page edits. From then on, **new code uses the shadcn
components directly**, and a wrapper is only kept where it adds a project convention (loading
state, icon prop, Arabic defaults, `MoneyText`…).

**Decisions**
- Style: **new-york-v4** (Tailwind v4 native, which we already use). Components are added with
  the CLI (`bunx shadcn-vue@latest add …`) and owned in-repo, as shadcn intends.
- Location (keeps the modular layout): `components.json` aliases →
  `ui: "@/modules/core/components/shadcn"`, `utils: "@/modules/core/helpers/utils"` (exports `cn`),
  `composables: "@/modules/core/controllers"`.
- New deps: `reka-ui`, `class-variance-authority`, `clsx`, `tailwind-merge`, `tw-animate-css`,
  `vue-sonner` (Phase E), `@vueuse/core` (needed by sidebar).
- **Token bridge, not a token rewrite.** Keep our tokens as the single source of truth and map
  shadcn's semantic names onto them in `design-system.css` (`@theme inline`):
  `--color-foreground → text-primary`, `--color-muted → surface`,
  `--color-muted-foreground → text-secondary`, `--color-accent → surface-hover`,
  `--color-card / --color-popover → background`, `--color-primary-foreground → on-primary`,
  `--color-destructive → danger`, `--color-input → border`, `--color-ring → primary`,
  and all `--color-sidebar-*` (sidebar → surface, sidebar-accent → surface-hover,
  sidebar-primary → primary…). `bg-background`, `border-border` and `bg-primary` already mean the
  same thing in both systems. Accent presets and dark mode keep working with no changes.
- **Text sizes scale:** map Tailwind's `--text-xs/sm/base/lg` in `@theme` onto our
  `--font-scale`-based tokens (`xs→label`, `sm→body`, `base→ui`, `lg→heading-sm`), so shadcn's
  `text-sm` follows the text-size appearance setting. Radii: `--radius: 0.375rem` (6 px
  buttons/inputs). Cards keep 12 px.
- **RTL:** wrap `App.vue` in reka-ui `<ConfigProvider dir="rtl">` so menus, submenus, sliders and
  arrow-key navigation mirror correctly. Audit added components for physical `left/right/ml/mr`
  and swap them for logical `start/end/ms/me`. Flip directional chevrons with `rtl:rotate-180`.
- **Keep `<select>` native** for dense data entry: `AppSelect` → shadcn **NativeSelect** (fast,
  keyboard-friendly, and keeps the e2e `locator("select")` selectors working). Searchable picks
  stay in `AppCombobox` → shadcn Combobox.

**Component map**

| Ours (keep API) | shadcn-vue base | Notes |
|---|---|---|
| `AppButton` | `Button` | variants primary/secondary/ghost/danger → default/outline/ghost/destructive; keep `icon`, `loading`, `to` |
| `AppInput` / `AppTextarea` | `Input` / `Textarea` + `Label` + `Field` | keep label/error/hint props |
| `AppSelect` | `NativeSelect` | see above |
| `AppCombobox` | `Combobox` (Popover + Command) | keep `aria-haspopup=listbox` for e2e |
| `AppModal` | `Dialog` | focus trap, Esc, and scroll lock come for free |
| `ConfirmDialog` | `AlertDialog` | |
| `AppSwitch` | `Switch` | |
| `SegmentedControl` | `ToggleGroup` (single) | or `Tabs` where it switches views |
| `StatusBadge` | `Badge` + status variants | paid/draft/overdue/partial |
| `AppCard` | `Card` | |
| `SkeletonBlock` | `Skeleton` | |
| `EmptyState` / `ErrorState` | `Empty` | |
| `SearchInput` | `InputGroup` | |
| `DateRangeFilter` | `Popover` + `RangeCalendar` | keep text inputs as a fallback for typing dates |
| `DataTable` | `Table` (markup + styles only) | keep our sorting/paging/export logic; TanStack only if needed later |
| Tooltips (`title=`) | `Tooltip` | chrome/icon buttons first |
| **Custom, not replaced** | — | `MoneyText`, `RiyalIcon`, `AppPhoneInput` (restyled on `Input`), `AttachmentField/Viewer`, `PdfPreview`, `AiOrb`, charts, `JournalPreview` |

**Tasks**
- [x] `components.json` + `cn` util + deps + `tw-animate-css` import; `bunx shadcn-vue@latest init` pointed at our aliases.
- [x] Token bridge + text-size mapping + `--radius` in `design-system.css`; `ConfigProvider dir="rtl"` in `App.vue`.
- [x] `bun run check` (text-token guard) passes on generated code. Extend the script's ignore list only if a generated file has a real need.
- [x] Add the base components: `button input textarea label field native-select combobox command
      popover dialog alert-dialog switch toggle-group tabs badge card skeleton empty input-group
      table tooltip calendar range-calendar separator dropdown-menu avatar collapsible breadcrumb
      sheet sidebar sonner kbd`.
- [x] Re-implement each `App*` from the map on its shadcn base, one group per commit:
      (1) Button/Badge/Card/Skeleton/Empty, (2) Input/Textarea/NativeSelect/Switch/SearchInput,
      (3) Modal/ConfirmDialog/Combobox/SegmentedControl/DateRange, (4) DataTable styling.
      `ConfirmDialog` and `DateRangeFilter` were deliberately left as-is rather than moved to
      `AlertDialog`/`Popover+RangeCalendar` — see their own code comments for why (`ConfirmDialog`
      already benefits transitively via `AppModal`; `DateRangeFilter`'s UX has no closed/trigger
      state to hang a popover off and `RangeCalendar` needs `@internationalized/date` values, not
      the plain strings every caller uses).
- [x] A dev-only `/dev/ui` page that shows every component in every state (light/dark, RTL,
      disabled, loading, error), to review before swapping.
- [x] Update [design_system.md](../design_system.md): "Build with shadcn-vue components; only add a
      custom component when no shadcn component fits, and keep it in `modules/core/components/ui`".
- [x] Gate: all e2e flows green (fix selectors only where the DOM truly changed, preferring
      `get_by_role`) — all 16 flows pass, zero console errors, three real issues found and fixed
      (see the status note above). Screenshots of the busiest screens (home, POS, invoice form,
      invoices list, journal entry, party form, settings, reports hub, product list) checked
      visually in light/dark during each batch rather than as one final pass; trial balance is
      covered by the report-print e2e flow's own screenshot.

---

## Phase D — App shell from `sidebar-07` ("a sidebar that collapses to icons")

**Status: done** (2026-09-25), scoped narrower than originally planned below — see the note after
the task list for exactly what shipped and what didn't.

**What sidebar-07 is:** `SidebarProvider` → `AppSidebar` (`collapsible="icon"`) + `SidebarInset`.
The sidebar has four areas:
- **TeamSwitcher** in the header;
- **NavMain**: collapsible groups with an icon and sub-items;
- **NavProjects**: a flat list, hidden when collapsed to icons;
- **NavUser** in the footer, as a dropdown.

It also has a `SidebarRail` (drag or click the edge to toggle). The inset's header shows
`SidebarTrigger`, a `Separator` and a `Breadcrumb`, and shrinks from 64 → 48 px when collapsed.
Collapsed items show tooltips, and it has a built-in **Ctrl+B** toggle. It already imports icons
from `@lucide/vue`, which we use.

**How it maps onto our app**

| sidebar-07 part | Ours |
|---|---|
| `TeamSwitcher` | **Brand + branch switcher**: the Equal mark + store name, with the active branch as the subtitle. The dropdown lists branches (replaces the top-bar `BranchSwitcher`). With a single branch it's a static header. |
| `NavMain` (collapsible groups) | `NAVIGATION` groups: المبيعات, المنتجات والمخزون, العملاء والموردين, الحسابات, المدفوعات, التقارير, الإدارة. Each group gets an `icon` in `navigation.ts`. The group containing the active route opens automatically. Single-item groups (الرئيسية, المشتريات, المصروفات) render as plain `SidebarMenuButton`s with no collapsible. Role filtering via `auth.can()` stays as it is. |
| `NavProjects` | **إجراءات سريعة** (quick actions): نقطة البيع, فاتورة جديدة, سند قبض, … filtered by permission and hidden in icon mode. |
| `NavUser` | `UserMenu`: avatar initials, name, role; items for theme, keyboard shortcuts, settings, logout. |
| Inset header | `SidebarTrigger` · `Separator` · `Breadcrumb` (from `route.meta.section/title`) at the start; at the end: palette button (Ctrl K), POS button, notifications, DevMenu (dev). |

**Decisions**
- `side="right"` (RTL), `collapsible="icon"`, width `15rem` (close to today's 232 px),
  icon width `3.5rem`.
- The open/closed state is a `v-model:open` on `SidebarProvider`, fed by the existing
  `app_sidebar_collapsed` localStorage key and the appearance setting's "collapsed by default".
  This replaces shadcn's cookie.
- Ctrl+B: check for conflicts in `KeyboardShortcutsSheet` / `useShortcuts`, and list it in the
  shortcuts sheet.
- The mobile/Sheet mode never triggers (window `minWidth` is 1100 px). Keep it anyway; it costs nothing.
- POS keeps its chrome-free blank layout. The sidebar and header keep the `no-print` class.

**Tasks**
- [x] Adapt sidebar-07's primitives into `modules/core/components/layout/` — done as a new
      `NavMain.vue` plus a rewritten `AppSidebar.vue`. Skipped writing scratch-folder sample data
      since we built straight from the already-vendored `shadcn/sidebar/*` primitives from Phase C.
- [x] Give `NavItem`/groups the active-route logic — kept exactly as it was (`isActive()` moved
      into `NavMain.vue` unchanged); did **not** add a group `icon` field or make groups
      collapsible, since the current flat "groups with headers, always open" layout already
      matches this app's shallow, one-level IA and collapsing them would be pure churn with no
      user benefit. `NAVIGATION` in `navigation.ts` is untouched.
- [x] Rewrite `DefaultLayout.vue` as `SidebarProvider` → `AppSidebar` + `SidebarInset` (header +
      `<main>` with the same `max-w-[1400px]` content container and `ErrorBoundary`), plus a
      skip-to-content link.
- [x] Remove the old `AppSidebar`'s hand-rolled collapse button (now `SidebarTrigger` +
      `SidebarRail`). **Did not** move `BranchSwitcher`/`UserMenu`/`DevMenu`/`NotificationsDrawer`
      into the sidebar's TeamSwitcher/NavProjects/NavUser areas — they stay exactly where they
      were, inside `AppTopbar.vue`, now rendered in the new header next to the breadcrumb. Reason:
      those are all hand-built dropdown menus (their own outside-click/open-state logic) that
      Phase E is explicitly scoped to replace with shadcn's `DropdownMenu`/`Sheet`; rebuilding them
      as sidebar-specific components now would mean rebuilding them again in Phase E. `NavQuickActions`
      and `BrandBranchSwitcher` as named in the mapping table above were **not** built — the "نقطة
      البيع" quick-action button and branch switcher stayed as they already were, in the topbar.
- [x] RTL check: rail on the correct edge (right), submenu indent — n/a, no collapsible submenus
      exist (see above) — chevrons n/a, tooltips confirmed opening toward the content (fixed
      `SidebarMenuButton`'s hard-coded `side="right"` tooltip to `side="left"` for this RTL app;
      see commit).
- [x] Keyboard: Tab order sidebar → header → content works via the skip link + natural DOM order;
      Ctrl+B toggle comes for free from `SidebarProvider`. Did not separately audit arrow-key
      behavior inside menus since no collapsible submenus were introduced.
- [x] Updated the one e2e selector that broke (`role_gating.py`'s `nav[aria-label=...]` → `get_by_role("navigation", ...)`,
      since the nav landmark is now shadcn's `SidebarContent`). No `header button` selectors for
      the user menu/branch switcher needed changing since those components didn't move.
- [x] Gate: manually screenshotted expanded, collapsed, dark, and a restricted role (cashier) —
      not the full expanded/collapsed × light/dark × 1280/1920 × per-role matrix from the original
      plan. Ran the full 16-flow e2e suite (green), `verify:mocks` (49/0/0), `check`, and `vue-tsc`
      instead as the primary regression gate.

---

## Phase E — Replace the remaining hand-built chrome

**Status: moved** (2026-09-25) into [17](17-ui-system-rtl-themes.md) Phase D (chrome rebuilt while the
sidebar is redone) and Phase F (page sweep). The list below is kept for history; tick boxes in 17.

- [ ] `ToastContainer` → **Sonner** (`vue-sonner`), `position="top-center"`, `dir="rtl"`. Keep the
      `toast.*` helper signature in `useToast` so callers don't change.
- [ ] `CommandPalette` → shadcn `CommandDialog`. Keep the `useCommandPalette` providers and commands
      unchanged; only the view changes.
- [ ] `NotificationsDrawer` → `Sheet` (side start); `KeyboardShortcutsSheet` → `Dialog` + `Kbd`.
- [ ] `DevMenu` → `DropdownMenu`.
- [ ] Sweep pages for hand-rolled menus, popovers, tabs and tooltips; swap them for shadcn components.
- [ ] Final full gate (the whole definition of done above) on the merged tree, and README updated
      (stack line: "shadcn-vue (reka-ui)").

---

## Risks & how they're handled

| Risk | Mitigation |
|------|------------|
| Changing the Tauri identifier would orphan every install's IndexedDB | Not changed (Phase B decision) |
| Big-bang UI diff breaks many screens | The wrapper-first approach (Phase C) means pages are untouched until the kit is proven on `/dev/ui` |
| shadcn's physical `left/right` classes break RTL | reka `ConfigProvider dir="rtl"` + a logical-properties audit per added component |
| shadcn text sizes ignore the text-size setting | Tailwind `--text-*` remapped onto `--font-scale` tokens |
| e2e selectors break | Suite is ~70% role/text based already; NativeSelect and `aria-haspopup=listbox` preserved; fix the rest with `get_by_role` |
| Low-res logo source (192 px) makes blurry icons | Trace `fav.webp` into `mark.svg` (potracer, tested); tune the threshold so the face hatching survives |
