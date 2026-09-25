# 16 — Equal: close fix, rebrand, shadcn-vue UI kit & sidebar-07 layout

**Status: planned** (2026-09-25). Nothing below is started yet.

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
- [ ] Add `brand.ts` and replace every hard-coded "نظام المحاسبة ونقاط البيع": `index.html` title,
      `router/index.ts` title fallback, `LoginPage.vue`, `WelcomePage.vue`, the sidebar subtitle,
      the xlsx `workbook.creator` in `exportXlsx.ts` and `importXlsx.ts`.
- [ ] `tauri.conf.json`: `productName` → `Equal`, window `title` → `ايكوال المحاسبي`. This changes the
      exe/installer name, which is acceptable at 0.1.0; note it in the release notes.
- [ ] Brand assets: copy to `src/assets/brand/` (`logo-light-bg.webp`, `logo-dark-bg.webp`,
      `mark.webp`). Add a `BrandLogo.vue` component with `variant="lockup" | "mark"` that switches
      to the right file for the current theme.
- [ ] **Vectorize `fav.webp` → `src/assets/brand/mark.svg`.** 192 px enlarged to the 1024 px an app
      icon needs is visibly blurry, so trace the mark into a vector instead. A feasibility check
      has been run (2026-09-25), with only temporary scratch output:
      - Tool: `potracer` (pure-Python potrace, `pip install potracer`) + `resvg-py` for rendering.
        `vtracer` **segfaults** on the local Python 3.14 and must not be used.
      - Pipeline: composite onto white → grayscale → Lanczos ×4 (768 px) → threshold → potrace
        (`turdsize=3, alphamax=1.0, opticurve=True, opttolerance=0.2`) → one `<path
        fill="currentColor" fill-rule="evenodd">` in a `0 0 768 768` viewBox (~50–65 KB,
        120–180 curves).
      - Gotchas: potracer traces the **False** pixels, so pass the *inverted* ink mask (the first
        try came out as a filled green square); and a dark threshold (`< 170`) wipes out the
        face hatching.
      - To do: pick the threshold by eye from 190 / 215 / 235 (zoomed crops of the face at 1024 px);
        compare against a plain Lanczos enlargement; choose whichever reads better at 16, 32, 256
        and 1024 px. The small sizes may simply use the raster instead.
      - `currentColor` lets one SVG serve both themes: green on light, white on dark or on the green tile.
      - Optional hardening: run the result through `svgo` to trim the path precision.
- [ ] App icons from `mark.svg`: 1024×1024 PNG with a white mark at ~80% on a `#10886C` rounded tile
      (radius ≈ 22%, matching the lockup tile) → `bunx tauri icon` regenerates `src-tauri/icons/*`.
      Also `public/favicon.svg` (the vector itself) plus `favicon.ico` as a fallback.
      Add the tracing script as `scripts/brand/trace-mark.py` so the icon can be rebuilt reproducibly.
- [ ] Login + welcome pages: full lockup above the form. Sidebar: the mark (see Phase D).
- [ ] Add the `equal` accent preset to `design-system.css` (light and dark shades, checked for
      ≥4.5:1 contrast of white text on the color). Default `data-accent` → `equal` in
      `useAppearance.ts` for new installs. Existing installs keep the accent they picked.
- [ ] README title and intro updated.
- [ ] PDFs: company letterheads keep the *customer's* store logo (no change). Only the metadata
      `creator`/`producer` in `pdf/render.rs` → "Equal Accounting".
- [ ] Gate: build + screenshots of login, welcome and home in light and dark; installer
      (`bun run tauri build`) shows the new name and icon.

---

## Phase C — shadcn-vue foundation, and the `App*` kit on top of it

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
- [ ] `components.json` + `cn` util + deps + `tw-animate-css` import; `bunx shadcn-vue@latest init` pointed at our aliases.
- [ ] Token bridge + text-size mapping + `--radius` in `design-system.css`; `ConfigProvider dir="rtl"` in `App.vue`.
- [ ] `bun run check` (text-token guard) passes on generated code. Extend the script's ignore list only if a generated file has a real need.
- [ ] Add the base components: `button input textarea label field native-select combobox command
      popover dialog alert-dialog switch toggle-group tabs badge card skeleton empty input-group
      table tooltip calendar range-calendar separator dropdown-menu avatar collapsible breadcrumb
      sheet sidebar sonner kbd`.
- [ ] Re-implement each `App*` from the map on its shadcn base, one group per commit:
      (1) Button/Badge/Card/Skeleton/Empty, (2) Input/Textarea/NativeSelect/Switch/SearchInput,
      (3) Modal/ConfirmDialog/Combobox/SegmentedControl/DateRange, (4) DataTable styling.
- [ ] A dev-only `/dev/ui` page that shows every component in every state (light/dark, RTL,
      disabled, loading, error), to review before swapping.
- [ ] Update [design_system.md](../design_system.md): "Build with shadcn-vue components; only add a
      custom component when no shadcn component fits, and keep it in `modules/core/components/ui`".
- [ ] Gate: all e2e flows green (fix selectors only where the DOM truly changed, preferring
      `get_by_role`), plus screenshots of the ten busiest screens (home, POS, invoice form,
      invoices list, journal entry, trial balance, product form, party page, settings, reports hub).

---

## Phase D — App shell from `sidebar-07` ("a sidebar that collapses to icons")

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
- [ ] Add the `sidebar-07` block (`bunx shadcn-vue@latest add sidebar-07`) into a scratch folder, then
      adapt it into `modules/core/components/layout/` (`AppSidebar`, `NavMain`, `NavQuickActions`,
      `NavUser`, `BrandBranchSwitcher`). Delete its sample data and `pages/sidebar/index.vue`.
- [ ] Add a group `icon` to `NAVIGATION`; give `NavItem` the active-route logic (`exact` handling as today).
- [ ] Rewrite `DefaultLayout.vue` as `SidebarProvider` → `AppSidebar` + `SidebarInset`
      (header + `<main>` with the same `max-w-[1400px]` content container and `ErrorBoundary`).
- [ ] Remove the old `AppSidebar` collapse button and the top-bar `BranchSwitcher` / `UserMenu` (moved into the sidebar).
- [ ] RTL check: rail on the correct edge, submenu indent line on the start side, chevrons mirrored,
      tooltips opening toward the content.
- [ ] Keyboard: Tab order sidebar → header → content; arrow keys inside menus; Ctrl+B; the
      "skip to content" link still works.
- [ ] Update e2e selectors that relied on `header button` for the user menu or branch switcher (→ `get_by_role`).
- [ ] Gate: screenshots expanded and collapsed, light and dark, 1280 and 1920 px, for each role
      (cashier, storekeeper and accountant see different menus).

---

## Phase E — Replace the remaining hand-built chrome

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
