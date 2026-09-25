# 17 — Real RTL, full motion, themes, a simpler sidebar, and one shared UI system

**Status: planned** (2026-09-25). Nothing below is started yet.

This file replaces **Phase E of [16](16-equal-rebrand-and-ui-kit.md)**. Everything that phase listed
(toasts, command palette, notifications drawer, shortcuts sheet, dev menu, branch switcher, user
menu) is folded into Phases D and F below.

Six phases, in this order. Each one leaves the app working, so you can stop after any of them:

| Phase | What | Size |
|---|---|---|
| A | **Real RTL** in every shadcn component: logical sides, mirrored icons, switch thumb, sliding animations | M |
| B | **Full motion**: remove every reduced-motion path | S |
| C | **Native "Save as…"** for every Excel / JSON / backup file | S |
| D | **Simple sidebar** (the sidebar-07 screenshot): collapsible groups, brand + branch at the top, user at the bottom | M |
| E | **Themes**: base color, accent, radius, font… with presets and a live preview, like shadcn's theme page | M |
| F | **One UI system**: shared layouts, form, table and line-item building blocks; every page migrated onto them; a guard script so new pages can't drift | L |

**Agent instructions** (same rules as [15](15-action-plan.md) and [16](16-equal-rebrand-and-ui-kit.md)):
- Still **UI-only against the mock backend**. Pages touch only `modules/*/services`.
- Tick the boxes here as you go and add a status note at the top of a phase when it ends.
- Decide yourself; ask the user only about scope-changing or irreversible choices.
- One commit per phase (Phase F: one commit per building block, then one per migration batch).
- Run the **whole** e2e suite at the end of each phase, not just the flows you touched (see 16 Phase C:
  two regressions only showed up in the full run).

**Definition of done (every phase):** the seven gates from [15](15-action-plan.md) — `bun run build`,
`bun run verify:mocks`, all `scripts/e2e/flows` green with no console errors, screenshots in light +
dark at 1280 px and 1920 px, the seam rule, Arabic/RTL + full keyboard use, and Excel export + palette
search for new tables and entities. Phase C touches Tauri capabilities, so it also needs
`cargo build --manifest-path src-tauri/Cargo.toml` and a real `bun run desktop` check.

---

## Phase A — Real RTL, not "swap left and right"

**The problem.** shadcn-vue is written LTR-first. 16 Phase D fixed the sidebar and dropdown menu
only. A survey of `modules/core/components/shadcn/*` on 2026-09-25 still finds:

| Component | Problem | Fix |
|---|---|---|
| `switch/Switch.vue` | Thumb moves with `translate-x-[calc(100%-2px)]` → in RTL "on" slides the **wrong way** | `rtl:-translate-x-[calc(100%-2px)]` (or `data-[state=checked]:translate-x-[…]` + `rtl:` negation); checked thumb must sit at the **start** edge's opposite, i.e. left in RTL |
| `breadcrumb/BreadcrumbSeparator.vue` | `ChevronRight` points away from the next crumb | `ChevronLeft` in RTL (`rtl:rotate-180`) |
| `calendar/*PrevButton`, `*NextButton`, `range-calendar/*` | Prev = `ChevronLeft`, Next = `ChevronRight`, placed with `left-1`/`right-1` | Mirror icons, `start-1`/`end-1`; month grid already follows `dir` via reka-ui — verify |
| `calendar/Calendar.vue` | `pl-2`, `pr-6`, `right-1` on the month/year selects | `ps-2`, `pe-6`, `end-1` |
| `input-group/*` | `pl-2`/`pr-2`/`pl-3`/`pr-3`, `ml-[-0.45rem]`, `mr-[-0.45rem]` on addons | `ps`/`pe`/`ms`/`me`; inline-start addon must sit on the **right** in RTL |
| `native-select/NativeSelect.vue` | Chevron at `right-3.5`, `pr-9` | `end-3.5`, `pe-9` |
| `toggle-group/ToggleGroupItem.vue` | `first:rounded-l-md last:rounded-r-md border-l` | `first:rounded-s-md last:rounded-e-md border-s` (this is **SegmentedControl** — the rounded ends are currently on the wrong items) |
| `range-calendar/RangeCalendarCell.vue` | Range start/end use `rounded-l`/`rounded-r` | `rounded-s`/`rounded-e` |
| `dialog/DialogHeader.vue`, `alert-dialog/AlertDialogHeader.vue` | `sm:text-left` | `sm:text-start` |
| `dialog/DialogScrollContent.vue`, `sheet/SheetContent.vue` | Close button at `right-4` | `end-4` (close ✕ belongs top-**left** in RTL) |
| `command/CommandShortcut.vue`, `field/FieldError.vue` | `ml-auto`, `ml-4` | `ms-auto`, `ms-4` |
| `sidebar/SidebarInset.vue` | `ml-0` / `ml-2` for the inset variant | `ms-0` / `ms-2` |

**Not bugs — leave alone** (they are driven by an explicit physical `side` prop, so they are
already correct): popover/tooltip/dropdown/combobox `data-[side=left]:slide-in-from-right-2`
(Floating UI sides are physical), `Sheet`'s `side="left|right"` classes, `Sidebar.vue`'s
`side === 'left' ? …` branches, and dialogs centered with `left-[50%] translate-x-[-50%]`.

**Directional meaning, not just position.** RTL is also about which way *things point and move*:
- **Back / next.** "Back" in Arabic points **right** (→), "next/forward" points **left** (←). 30 files
  in `src/` use `ChevronLeft/Right` or `ArrowLeft/Right` (PageHeader's back button, DataTable
  pagination, PdfPreview paging, SetupWizard steps, PosPage, KpiCard trends, row "open" chevrons…).
  Each must be checked for *meaning*, not flipped blindly:
  - navigation/sequence icons (back, next, pagination, wizard steps, "open detail" row chevrons,
    breadcrumbs, submenu chevrons) → mirror;
  - icons that show a **real physical or numeric direction** (trend up/down, sort asc/desc, an
    undo/redo "circular" arrow, a clock, a media-play ▶, a transfer "from → to" arrow between two
    LTR amounts) → do **not** mirror.
- **Slides and swipes.** Anything that slides in from "the next page" (wizard steps, sheet drawers,
  carousel-like transitions, toasts) must come from the **left** in RTL.
- **Progress & sliders.** Progress bars and any future slider fill from the **right**.
- **Numbers stay LTR** inside RTL text (already handled by the `num` class / `MoneyText`); phone
  and IBAN inputs stay `dir="ltr"` with `text-end` alignment.
- **Keyboard.** In menus and toggle groups, ArrowRight moves to the **previous** item in RTL —
  reka-ui does this when it gets `dir="rtl"` from `ConfigProvider`; verify every custom keyboard
  handler (`useHotkeys` users, POS grid, DataTable row navigation) does the same.

**One helper, used everywhere.** Add `modules/core/components/ui/DirIcon.vue` —
`<DirIcon :icon="ChevronRight" />` renders the icon with `rtl:-scale-x-100` — and a
`dirIcon.ts` map (`back`, `forward`, `open`, `prev`, `next`) so pages say `icons.back` instead of
picking a chevron. Pages stop importing `ChevronLeft/Right`/`ArrowLeft/Right` directly.

**Guard.** Extend `scripts/check-text-tokens.js` (or add `scripts/check-rtl.js`, wired into
`bun run check`) to fail on physical classes (`\b(ml|mr|pl|pr|left|right|border-l|border-r|rounded-l|rounded-r|text-left|text-right)-`)
and on direct `ChevronLeft|ChevronRight|ArrowLeft|ArrowRight` imports in `src/**`, with an
allow-list comment (`/* rtl-ok: <reason> */`) for the legitimate cases above.

**Tasks**
- [ ] Fix every row of the table above.
- [ ] Add `DirIcon.vue` + `dirIcon.ts`; migrate the 30 files, deciding per icon (mirror vs keep) and
      noting any "keep" with an `rtl-ok` comment.
- [ ] Audit slide directions: `SheetContent`, wizard step transitions, toasts, `SetupWizardPage`,
      any `<Transition>` using `translate-x`.
- [ ] Audit custom keyboard handlers for Arrow keys under RTL.
- [ ] Add the RTL guard to `bun run check`; get it to zero findings.
- [ ] Add an **RTL section** to `/dev/ui` showing: switch off/on, toggle group, breadcrumb, calendar
      + range, input group with start/end addons, native select, sheet from both sides, dialog close
      button, pagination, back button.
- [ ] Update `docs/design_system.md` "RTL-first" with the mirror / don't-mirror rules above.
- [ ] Gate: screenshot of the `/dev/ui` RTL section light + dark; full e2e suite.

---

## Phase B — Full motion, always

**Why animations look dead today.** `design-system.css` (lines ~252–265) sets every animation and
transition to ~0 ms whenever **either** the in-app "تقليل الحركة" toggle is on **or** Windows reports
`prefers-reduced-motion: reduce` — which it does whenever *Settings → Accessibility → Visual effects →
Animation effects* is off, a common setting on office PCs. So on many machines motion is disabled
with no way to turn it back on from inside the app.

**Decision (user request):** motion always runs at full power. Remove the feature entirely.

**Tasks**
- [ ] Delete the `@media (prefers-reduced-motion: reduce)` block and the `:root.motion-reduce …` rule
      in `src/assets/styles/design-system.css`.
- [ ] Delete from `useAppearance.ts`: `prefersReducedMotionMedia`, `applyMotion`, `reduceMotionSetting`,
      `reduceMotion`, `setReduceMotion`, and their lines in `initAppearance()`. Remove the stale
      `app_reduce_motion` localStorage key once on boot.
- [ ] Delete the "تقليل الحركة" switch from `AppearanceSettingsPage.vue`.
- [ ] `AiOrb.vue`: remove the `matchMedia('(prefers-reduced-motion…')` check (line ~334) and its
      `@media` block (line ~715).
- [ ] Grep for `motion-reduce:` / `motion-safe:` Tailwind variants and remove them (none found on
      2026-09-25; re-check).
- [ ] Check shadcn animations actually run: dialog/sheet/popover/dropdown enter+exit, sidebar
      collapse, accordion/collapsible height, toast slide. Fix any component missing `tw-animate-css`
      classes.
- [ ] Update docs 14 §3 (appearance settings list) to drop the motion setting.
- [ ] Gate: with Windows "Animation effects" **off**, open a dialog and collapse the sidebar in
      `bun run desktop` — both animate.

---

## Phase C — Every exported file asks where to save

**The problem.** PDFs already open the native Save dialog (`pdfService.ts` → `@tauri-apps/plugin-dialog`
`save()` + `plugin-fs` `writeFile()`). Everything else uses a browser `<a download>`, which WebView2
silently drops into *Downloads* with no prompt:

| Call site | File |
|---|---|
| `core/helpers/exportXlsx.ts:78` | every DataTable / list "تصدير Excel" |
| `reports/helpers/export.ts:60` | report Excel export |
| `core/components/import/importXlsx.ts:46, :134` | import template, import error file |
| `settings/services/backupService.ts:177` | backup archive |
| `templates/pages/TemplateDesignerPage.vue:159` | template JSON export |

**Tasks**
- [ ] Add `core/services/saveFile.ts`: `saveFile(bytes | Blob, { suggestedName, filters })` →
      Tauri `save()` + `writeFile()` in the desktop app; falls back to `<a download>` in a plain
      browser (dev / e2e). Returns the chosen path, or `null` if the user cancels.
- [ ] Remember the **last folder** per file kind (Excel / PDF / backup) in localStorage and pass it
      as `defaultPath`.
- [ ] After saving, toast `تم الحفظ` with the file name and an **"فتح المجلد"** action
      (`@tauri-apps/plugin-opener` `revealItemInDir`). Cancel = no toast, no error.
- [ ] Move `pdfService.ts`'s save onto the same helper so there is one code path.
- [ ] Replace all six call sites above.
- [ ] Check `src-tauri/capabilities/*.json` grants `dialog:allow-save`, `fs:allow-write-file` (scoped to
      user folders) and `opener:allow-reveal-item-in-dir`.
- [ ] e2e: the browser fallback keeps `page.expect_download()` flows (reports_v2, labels_templates)
      green; add a unit check that `saveFile` picks the Tauri path when `window.__TAURI_INTERNALS__` exists.
- [ ] Gate: `cargo build`, then in `bun run desktop` export a list to Excel → Save dialog appears
      with an Arabic default name → file opens in Excel.

---

## Phase D — A small, calm sidebar (sidebar-07, done fully)

**What the user wants** (screenshot, 2026-09-25): the sidebar should feel *small and easy*, not
a wall of 30 links. sidebar-07 does this with:
- a **brand/team switcher** at the top (logo + name + subtitle, `⌃⌄` chevron);
- **collapsible groups** — each shows as *one* row (icon + name + chevron); only the group you are
  in is open, with a thin indent line for its items;
- a short flat list (**Projects** in the demo);
- the **user** at the bottom (avatar + name + email, opens a menu);
- collapses to an icon rail; header shows `SidebarTrigger` · `Separator` · `Breadcrumb` only.

16 Phase D shipped the shell but deliberately kept flat, always-open groups and left the branch
switcher / user menu in the top bar. This phase does the rest.

**New information architecture** (≈ 9 rows closed instead of ~33 links):

| Row | Icon | Opens to |
|---|---|---|
| الرئيسية | House | (single link) |
| المبيعات | ShoppingCart | نقطة البيع · الورديات · الفواتير · فاتورة جديدة · عروض الأسعار |
| المخزون | Package | المنتجات · التصنيفات والوحدات · قوائم الأسعار · التسويات · الجرد · الحركة · الصلاحية · التحويلات |
| العملاء والموردين | Users | العملاء · الموردين |
| المشتريات والمصروفات | ShoppingBag | أوامر الشراء · المصروفات (two tiny groups merged) |
| الحسابات | BookOpen | دليل الحسابات · القيود · القوالب المتكررة · تسوية الضريبة · السنة المالية |
| المدفوعات | HandCoins | السندات · تسوية البطاقات · السندات العامة |
| التقارير | ChartColumn | التقارير · التحليلات |
| الإدارة | Settings | طلبات الاعتماد · المستخدمين · الإعدادات |

Below the groups, a small **إجراءات سريعة** list (the "Projects" slot): بيع جديد, فاتورة جديدة, سند قبض —
filtered by permission, hidden in icon mode.

**Decisions**
- Only the group containing the current route is open on load; opening another group closes the
  previous one (accordion). Open state is not persisted — fewer surprises for non-technical users.
- A group whose role filter leaves **one** item renders as a plain link (a cashier never sees a
  one-item accordion).
- In icon mode a group shows its icon; clicking it opens a **flyout** (DropdownMenu, side toward the
  content) listing its items — no need to expand the sidebar.
- **Header (top of sidebar):** `BrandBranchSwitcher` — logo, store name, active branch as subtitle;
  dropdown lists branches + "كل الفروع". Single-branch company → static, no chevron.
- **Footer:** `NavUser` — initials avatar, name, role; menu: المظهر، الوضع الداكن، اختصارات لوحة المفاتيح،
  الإعدادات، (dev: تبديل المستخدم)، تسجيل الخروج. Replaces `UserMenu.vue`.
- **Top bar** keeps only: trigger · breadcrumb (start) and search (Ctrl K) · notifications · POS button
  (end). `DevMenu` moves into `NavUser`'s dev section. Theme toggle moves into `NavUser`.
- Hand-built dropdowns are rebuilt on shadcn while moving (this is 16 Phase E's leftover):
  `BranchSwitcher` → `DropdownMenu`, `UserMenu` → `DropdownMenu`, `DevMenu` → `DropdownMenu` section,
  `NotificationsDrawer` → `Sheet`, `KeyboardShortcutsSheet` → `Dialog` + `Kbd`, `CommandPalette` →
  `CommandDialog`, `ToastContainer` → `Sonner` (keep `useToast()`'s API).

**Tasks**
- [ ] `navigation.ts`: give `NavGroup` an `icon` and the new grouping above; keep `NavItem`'s `area`
      filtering and `exact` logic.
- [ ] `NavMain.vue`: `Collapsible` per group (accordion), single-item → plain link, icon-mode flyout,
      `rtl:rotate-180` on the group chevron, sub-items via `SidebarMenuSub` (indent line on the start side).
- [ ] `NavQuickActions.vue`, `BrandBranchSwitcher.vue`, `NavUser.vue`.
- [ ] Slim `AppTopbar.vue` to search · notifications · POS; delete `BranchSwitcher.vue`, `UserMenu.vue`,
      `DevMenu.vue` once their content lives in the new components.
- [ ] Rebuild `NotificationsDrawer`, `KeyboardShortcutsSheet`, `CommandPalette`, `ToastContainer` on
      shadcn (keep their composable APIs so callers don't change).
- [ ] Add Ctrl+B to `GLOBAL_SHORTCUTS` so it shows in the shortcuts sheet.
- [ ] Update e2e selectors: `branches_currencies.py` uses `header button` for the branch switcher →
      `get_by_role("button", name=…)` inside the sidebar; user-switch / logout flows → `NavUser`.
- [ ] Gate: screenshots expanded + collapsed (with a flyout open), light + dark, 1280 + 1920, for
      admin, cashier, storekeeper, accountant — each must show only its own groups.

---

## Phase E — Themes like shadcn

**Today:** one fixed neutral palette + 5 accent presets (`data-accent`) + a fixed `--radius: 0.375rem`.
shadcn's own theme page lets you pick a **base color**, an **accent**, a **radius**, and see it live.

**Model** — a theme is a small object, stored per device like the other appearance settings
(`makeSetting` in `useAppearance.ts`):

```ts
interface ThemeConfig {
  base: 'neutral' | 'zinc' | 'stone' | 'slate' | 'gray';   // background/surface/border/text greys
  accent: AccentPreset;                                    // existing: equal, indigo, teal, rose, amber (+ blue, violet, orange)
  radius: 0 | 0.25 | 0.375 | 0.5 | 0.75 | 1;               // rem → --radius (sm/md/lg/xl derive from it)
  mode: 'light' | 'dark' | 'system';                        // existing useTheme
  font: FontFamily;                                         // existing
  density: Density;                                         // existing
}
```

- Each **base** defines the full light + dark neutral set (`--color-background`, `surface`,
  `surface-hover`, `border`, `text-primary`, `text-secondary`) under `:root[data-base="…"]` /
  `:root.dark[data-base="…"]`. The shadcn bridge (`@theme inline`) already maps shadcn tokens onto
  ours, so every shadcn component follows automatically.
- **Radius** sets `--radius` on `<html>`; audit components that hard-code `rounded-md`/`rounded-lg`
  outside the token scale (`rounded-[…]`) and move them to `rounded-(--radius-*)`-based utilities.
- **Presets**: a few named full themes (e.g. *إيكوال* = neutral + equal + 0.375, *كلاسيكي* = slate +
  indigo + 0.5, *ناعم* = stone + teal + 0.75, *حاد* = zinc + equal + 0) — one click sets all fields.
- **Contrast check** kept from 16 Phase B: every accent × base × mode combination must clear 4.5:1 for
  white-on-accent; a small script (`scripts/check-contrast.ts`) computes it from the CSS so new presets
  can't regress.

**Tasks**
- [ ] Define the 5 base palettes (light + dark) in `design-system.css`; add 3 accents (blue, violet, orange).
- [ ] `ThemeConfig` + `applyTheme()` in `useAppearance.ts` (sets `data-base`, `data-accent`, `--radius`);
      migrate existing keys without losing current users' choices.
- [ ] Settings → المظهر: a **theme customizer** — preset cards, base swatches, accent swatches, radius
      segmented control, mode, and a **live preview panel** (a card with buttons, input, switch, badge,
      table rows, a dialog trigger) that updates instantly. "إعادة التعيين" returns to the إيكوال preset.
- [ ] Remove hard-coded radius values found by the audit.
- [ ] `scripts/check-contrast.ts` wired into `bun run check`.
- [ ] Printed documents (Typst PDFs, receipts) are **not** themed — they keep the brand look. Note it in 12.
- [ ] Gate: screenshots of 3 presets × light/dark on the dashboard and an invoice form.

---

## Phase F — One shared UI system

**The problem** (survey of 109 `*Page.vue` files, 2026-09-25):
- 39 pages build their own `<table>` markup instead of `DataTable`; 36 use `DataTable`.
- Forms have no shared structure: labels, hints, errors, sections, the save/cancel bar, the
  "unsaved changes" guard and Ctrl+S are rewritten per page. 9 pages exceed 400 lines
  (`PosPage` 860, `JournalEntryFormPage` 591, `PartyFormPage` 536, `ProductFormPage` 481,
  `InvoiceFormPage` 454, `StockAdjustmentFormPage` 454, `TemplateDesignerPage` 452,
  `JournalListPage` 475, `PartyDetailPage` 406).
- The same **line-items grid** (product/account picker, qty, unit, price, discount, tax, total, add/remove
  rows, keyboard entry) exists separately in invoice, purchase, journal, stock adjustment, stock count
  and transfer forms.

**Goal:** a page should be *assembled* from a small set of layouts and blocks, so every new page
automatically looks and behaves like the rest ("same soul"), and a fix in one block fixes every page.

### F1. The building blocks

Layer 1 — **primitives** (exist): `modules/core/components/shadcn/*` + `ui/App*`.

Layer 2 — **blocks** (new, in `modules/core/components/blocks/`):

| Block | Replaces | Contract |
|---|---|---|
| `FormField` | hand-written label + input + hint + error | `label`, `hint`, `error`, `required`, `name`; wraps shadcn `Field`; slot for any control; RTL + `aria-describedby` wired |
| `FormSection` | ad-hoc `<AppCard>` + heading per form part | `title`, `description`, optional collapsible; 1/2/3-column responsive grid |
| `FormActions` | per-page save/cancel rows | sticky bottom bar; primary + secondary + danger slots; shows "تغييرات غير محفوظة" |
| `useForm()` | per-page `ref` + manual validation | Zod schema → values, errors, `dirty`, `submit()`, `reset()`; route-leave guard when dirty; Ctrl+S = submit |
| `FilterBar` | per-page search + selects + date range rows | `SearchInput` + declared filters + `DateRangeFilter` + saved views + "مسح" ; syncs to the URL query |
| `DataTable` (extend) | 39 raw tables | column defs with `type: 'money' \| 'date' \| 'number' \| 'status' \| 'party' \| 'actions'` → formatting + alignment automatic; totals footer; row click → detail; selection + bulk actions; empty/loading/error states; Excel export via Phase C |
| `LineItemsEditor` | 6 copies of the line grid | generic over a line type; column config; keyboard (Enter = next cell, Ctrl+Enter = new line, Del = remove); totals emitted; used by invoice, purchase, journal, adjustment, count, transfer |
| `TotalsPanel` | per-form totals cards | rows (label, amount, emphasis); tafqit line optional |
| `DetailHeader` | per-page detail tops | title, number, `StatusBadge`, meta chips, action buttons (print, edit, more ⋯) |
| `StatCards` | per-page KPI rows | array of `{ label, value, trend, to }` |

Layer 3 — **page layouts** (new, in `modules/core/components/layouts/`):

| Layout | Slots | Used by |
|---|---|---|
| `ListPage` | header (title, primary action) · `FilterBar` · `DataTable` · pagination | invoices, products, customers, suppliers, POs, expenses, payments, journal, stock movements… |
| `FormPage` | header · `FormSection`s · aside (totals / help) · `FormActions` | product, party, invoice, purchase, journal entry, expense, adjustment… |
| `DetailPage` | `DetailHeader` · optional `StatCards` · tabs (details / lines / payments / history / attachments) · aside | invoice, purchase, party, product, journal entry… |
| `SettingsPage` | nav list · section content | all settings pages |
| `ReportPage` | already exists as `ReportShell` — align it with `FilterBar` | reports |

### F2. The rules (written into `docs/design_system.md` → new "Building pages" section)

1. A page is **one layout + blocks**. It does not import shadcn primitives for structure.
2. No raw `<table>` in `modules/*/pages` — use `DataTable` (print pages and Typst templates excepted).
3. No bare `<input>`/`<select>`/`<label>` in pages — use `FormField` + an `App*` control.
4. Money, dates, numbers and statuses are formatted **only** by column types / `MoneyText` / `StatusBadge`.
5. Arabic copy for shared states (empty, error, loading, unsaved changes, confirm delete) lives in the
   blocks, not in pages.
6. Page files stay under ~250 lines; page-specific pieces go to `modules/<m>/components/`.
7. New block? Add it to `/dev/ui` and to the design-system doc in the same commit.

**Guard:** `scripts/check-ui-rules.js` in `bun run check` — flags `<table`, `<input`, `<select`, `<label`
and shadcn structural imports inside `modules/*/pages/*.vue` (allow-list: print pages, POS, template
designer), and page files over 300 lines (warning).

### F3. Migration batches (one commit each, full e2e after each)

- [ ] **F-0** Build all blocks and layouts, add them to `/dev/ui` (with RTL + dark examples), write
      the "Building pages" doc section and the guard script (warning mode).
- [ ] **F-1 Lists:** invoices, quotations, products, customers, suppliers, POs, expenses, payments,
      vouchers, journal list, stock movements/adjustments/counts/transfers, users → `ListPage`.
- [ ] **F-2 Line-item forms:** invoice, purchase, journal entry, stock adjustment, stock count,
      transfer → `FormPage` + `LineItemsEditor` + `TotalsPanel`. Highest risk — accounting e2e
      flows (`desk_invoice`, `purchases`, `accountant_journal`, `refund_payment`) must stay green and
      `verify:mocks` 49/0/0.
- [ ] **F-3 Simple forms:** product, party, expense, payment, category/unit, price list, user,
      branch, currency → `FormPage` + `useForm`.
- [ ] **F-4 Detail pages:** invoice, purchase, party, product, journal entry, shift → `DetailPage`.
- [ ] **F-5 Settings pages** → `SettingsPage`.
- [ ] **F-6** Guard script to **error** mode; zero findings; update 15's definition of done to
      include `bun run check` passing the UI rules.

**Out of scope for F:** POS (`PosPage`) keeps its own full-screen layout, but reuses `LineItemsEditor`'s
math helpers and `TotalsPanel`; the template designer stays custom.

---

## Risks

| Risk | Handling |
|---|---|
| Mirroring an icon whose direction is *physical* (trend arrow, transfer from→to) | Per-icon decision in Phase A, documented with `rtl-ok` comments; `/dev/ui` shows both kinds |
| Collapsible groups hide pages users used to see at a glance | Command palette (Ctrl K) finds every page; quick actions list; active group always open |
| `LineItemsEditor` changes accounting math | It only renders + emits; totals/VAT math stays in the existing `helpers` (tax-inclusive, discount order) and is covered by `verify:mocks` + e2e |
| Theme presets break contrast | `check-contrast.ts` in `bun run check` |
| Big migration churn | Layout-first (F-0), then batches with the full suite after each; pages keep their services untouched |
