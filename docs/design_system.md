# Design System — "Linear-style" Light/Dark

Source of truth for tokens: `src/assets/styles/design-system.css` (already scaffolded — this doc explains the intent behind it and fills in component-level guidance, adapted from `references/vat-invoice-app/design.md`, a Linear-inspired reference).

## Principles

- **Midnight precision instrument**: near-black/near-white surfaces, hairline borders instead of heavy shadows, one accent color used sparingly for the single primary action per view.
- **Compact density**: tight paddings (8–12px), small radii (6px buttons/inputs, 12px cards), Inter-family type at 400–510 weight — never bold (700+).
- **RTL-first**: Arabic is the primary UI language. Layout mirrors for RTL; numbers and currency values stay LTR-oriented within RTL text blocks (see `toHindi()`-style numeral handling in `references/vat-invoice-app`).
  Real RTL is directional meaning, not "swap left and right" (docs/v2/17-ui-system-rtl-themes.md Phase A):
  - **Use logical Tailwind utilities, never physical ones**: `ms-`/`me-` not `ml-`/`mr-`, `ps-`/`pe-`
    not `pl-`/`pr-`, `start-`/`end-` not `left-`/`right-`, `border-s-`/`border-e-` not `border-l-`/
    `border-r-`, `rounded-s-`/`rounded-e-` not `rounded-l-`/`rounded-r-`, `text-start`/`text-end` not
    `text-left`/`text-right`. `scripts/check-rtl.js` (part of `bun run check`) fails the build on any
    physical class in `class`/`:class`/`cn(...)`, with an `/* rtl-ok: <reason> */` escape hatch for the
    legitimate physical cases below.
  - **Not bugs — genuinely physical, leave as `left`/`right`**: anything driven by an explicit
    physical `side` prop (Floating UI's `data-[side=left]`, `Sheet`'s `side="left|right"`, `Sidebar`'s
    `side` branches), and a dialog centered with `left-[50%] translate-x-[-50%]`.
  - **Navigation icons mirror, physical/numeric icons don't.** Back points right in Arabic, forward
    points left. Never import `ChevronLeft`/`ChevronRight`/`ArrowLeft`/`ArrowRight` from `@lucide/vue`
    directly for anything meaning back/next/prev/open — use `dirIcon` + `DirIcon.vue`
    (`modules/core/helpers/dirIcon.ts`) so it flips under `rtl:`. Icons with a real physical or
    numeric direction (trend up/down, sort asc/desc, undo/redo's circular arrow, a transfer
    from→to arrow, a media ▶) are **not** mirrored — `scripts/check-rtl.js` also fails on a raw
    directional-icon import outside the shadcn primitives that already handle their own mirroring.
  - **Slides come from the direction they're anchored to.** A toast pinned to `start-4` must enter
    from that same side, not a hard-coded `-translate-x-4` — see `ToastContainer.vue`'s
    `rtl:translate-x-4` override. The same applies to wizard-step transitions, sheet drawers and any
    other `<Transition>` using `translate-x`.
  - **Progress bars and sliders** fill from the right in RTL.
  - **Numbers stay LTR** inside RTL text (the `num` class / `MoneyText`); phone and IBAN inputs stay
    `dir="ltr"` with `text-end` alignment (not `text-right` — same visual result, but consistent with
    the logical-utilities rule and passes the guard).
  - **Keyboard**: in menus/toggle groups, ArrowRight moves to the *previous* item in RTL — reka-ui
    does this automatically once it gets `dir="rtl"`; any custom keyboard handler for Left/Right
    (there are none as of Phase A — only vertical Up/Down handlers exist in this codebase) must be
    checked the same way if one is added later.
  - See `/dev/ui`'s "RTL — الاتجاه الحقيقي" section for a live example of every case above.
- **Light + dark**, both first-class (unlike the pure-dark Linear marketing site) — this app already scaffolds both via `:root` and `:root.dark`.

## Tokens (already in `design-system.css`)

| Token | Light | Dark | Use |
|-------|-------|------|-----|
| `--color-background` | `#ffffff` | `#0e0f11` | Page canvas |
| `--color-surface` | `#f4f4f5` | `#17181c` | Cards, panels, table rows |
| `--color-surface-hover` | `#e4e4e7` | `#222327` | Hover state on surfaces |
| `--color-border` | `#d4d4d8` | `#2d2e33` | Hairline dividers, card outlines |
| `--color-text-primary` | `#09090b` | `#f2f2f2` | Headings, primary text |
| `--color-text-secondary` | `#71717a` | `#8b8d98` | Muted/secondary text |
| `--color-primary` | `#0e7259` | `#0f7d63` | Primary action buttons, active nav, links |
| `--color-primary-hover` | `#0b6350` | `#0d6b54` | Primary hover |
| `--color-danger` | `#ef4444` | `#e25858` | Destructive actions, negative balances |
| `--color-success` | `#22c55e` | `#42b883` | Paid/positive status |

`--color-primary`/`--color-primary-hover` above are the **default accent preset**, `equal` (the
Equal brand green, logo mark `#10886C`, darkened slightly to clear 4.5:1 white-text contrast —
docs/v2/16-equal-rebrand-and-ui-kit.md Phase B). Users can switch to indigo/teal/rose/amber/blue/
violet/orange (8 total) in Settings → Appearance; see `useAppearance.ts`'s `ACCENTS` map for every
preset's values. Every accent's text-on-accent pair is enforced >= 4.5:1 by `scripts/check-contrast.ts`
(part of `bun run check`), so a new or edited accent can't silently regress contrast.

**Base palette** (docs/v2/17-ui-system-rtl-themes.md Phase E): the 6 grey/text tokens above
(`background`/`surface`/`surface-hover`/`border`/`text-primary`/`text-secondary`) can also be swapped
as a set — neutral (default) / zinc / stone / slate / gray — via `data-base` on `<html>`
(`useAppearance.ts`'s `setBase`/`BASES`), independent of the accent. **4 named presets** (إيكوال,
كلاسيكي, ناعم, حاد) set base + accent + radius together in one click (`THEME_PRESETS`/
`applyThemePreset`). None of this reaches printed documents (invoices, receipts) — see
docs/v2/12-documents-pdf-excel.md §3, which keep their own fixed brand colors regardless of the UI
theme.

Font: `--font-sans` = Cairo (Arabic) → Inter → system-ui fallback stack. Cairo is the correct choice over pure Inter here since this is an Arabic-first UI (Inter has no Arabic glyphs).

## Radii & spacing (carry over from the Linear reference)

- Buttons/inputs: 6px radius (**default** — user-configurable 0/0.25/0.375/0.5/0.75/1rem via
  Settings → Appearance's radius control, `useAppearance.ts`'s `setRadius`; every `rounded-md`/
  `rounded-lg`/`rounded-xl` utility already resolves through `--radius` via the shadcn token bridge
  in `design-system.css`, so no component needs to change to follow it)
- Cards/panels: 12px radius (also follows `--radius`, since `rounded-xl` derives from it)
- Pills/badges: 9999px (status chips) or 4px (small badges) — fixed, not part of the radius scale
- Spacing ladder: 4/8/12/16/24/32/48/64px — base unit 4px
- Card padding: 24px (16px on compact/dense tables)

## Components

**Build with shadcn-vue components** (docs/v2/16-equal-rebrand-and-ui-kit.md Phase C). Every base
primitive — Button, Input, Dialog, Combobox, Table, etc. — lives in
`modules/core/components/shadcn/*`, added via `bunx shadcn-vue@latest add <name>` and owned in-repo.
The existing `App*` wrappers in `modules/core/components/ui/` (`AppButton`, `AppModal`, `DataTable`…)
are built on top of these, keeping their old props/emits API so pages don't need to change; new code
should use the shadcn components directly where no project convention (loading state, Arabic
defaults, an icon prop, `MoneyText`-style formatting) is needed on top. Only add a genuinely custom
component when no shadcn component fits, and keep it in `modules/core/components/ui`.

### Primary button
Background `--color-primary`, white text, 6px radius, 10px/16px padding, weight 510. One per view/section — used for "Save", "New Invoice", "Post Journal Entry", etc.

### Secondary / ghost button
Transparent background, 1px `--color-border`, `--color-text-secondary` text. Used for "Cancel", "Export", "Print Preview".

### Danger button
`--color-danger` text or background (outline by default, filled only for confirm-delete dialogs).

### Card
`--color-surface` background, 12px radius, 1px `--color-border`, 24px padding. No drop shadows — separation comes from the border and the surface-level step up from background.

### Status badge
Small pill, `--color-surface-hover` background, colored text/dot for state:
- Paid / Completed → `--color-success`
- Unpaid / Draft → `--color-text-secondary`
- Overdue / Canceled / Refunded → `--color-danger`
- Partially paid → amber (`#f59e0b`, add as a new token if needed — not yet in the CSS file)

### Error toast (diagnostics error code)
Any error surfaced by the last-resort handlers (`app.config.errorHandler` in `main.ts`, `ErrorBoundary.vue`)
appends a short code to its message: `... — رمز الخطأ: E-7F3A`. The code is the first 4 hex characters
of the same fingerprint `docs/diagnostics/ISSUES.md` groups errors by (`fingerprintOf()` in
`modules/diagnostics/services/logService.ts`) — a non-technical user can read four characters over the
phone to support, who then finds the matching ledger entry or log lines by that fingerprint. Every
error toast gets one; never a raw stack trace or English exception message in the visible text.

### Date picker
`AppDatePicker` (`modules/core/components/ui/AppDatePicker.vue`) — a typeable text field (`dir="ltr"`,
`YYYY-MM-DD`) with a trailing calendar icon button that opens a themed shadcn Popover + Calendar.
Model is a plain `YYYY-MM-DD` string, matching every existing date field's type — no page-level type
changes when adopting it. Replaces native `<input type="date">` everywhere except `DateRangeFilter`
(documented exception — its inline always-visible range inputs have no closed/trigger state to hang a
popover off). A `compact` prop shrinks it for table-cell use (line-item expiry dates).

```vue
<AppDatePicker v-model="form.date" label="التاريخ" required />
<AppDatePicker v-model="line.expiry" compact />
```

### Data table
Dense rows (12px vertical padding), `--color-surface` header row, hairline `--color-border` row dividers, hover state `--color-surface-hover`. Right-aligned numeric columns (amounts, quantities) — remember RTL means "right-aligned" is the natural start-aligned direction for Arabic, so numeric columns should be visually consistent with LTR number rendering inside them.

### Scrollable tab/chip strip (ScrollFade)
`core/components/ui/ScrollFade.vue` wraps a horizontally-scrollable row (settings tabs, chip filters) with the native scrollbar hidden, pointer drag-to-scroll, wheel-to-horizontal-scroll, and an edge fade gradient in `--color-background` that only shows on the side(s) that still have more content — none when everything fits, both in the middle of a long strip, one side at either end. The gradient direction is driven by `:dir()`, not a hard-coded side, so it's correct in RTL without extra flags. Use it instead of letting a tab row wrap or shrink its labels.

### Sidebar navigation
Flat, single-level groups (mirrors the reference system's IA, trimmed to our modules): الرئيسية (Home), المبيعات (Sales/POS), المنتجات والمخزون (Products & Inventory), العملاء والموردين (Customers & Suppliers), المشتريات (Purchases), الحسابات (Accounting), المدفوعات (Payments), التقارير (Reports), الإدارة (Users, Settings). Active item uses `--color-primary` text/left-border accent; nav gated by `role` (see `domain_model.md` §10).

### POS / sale screen
Full-bleed, chrome-free layout (no sidebar) — matches the reference's `blank` layout pattern for its POS screen, since cashiers need maximum screen real estate and minimum distraction. Product grid + cart panel side-by-side (cart pinned to the trailing edge, which is the **left** side in RTL).

### Invoice print preview
Two swappable layouts:
- **A4**: standard printable page, full itemized table, company header/footer, ZATCA QR code bottom-left.
- **Thermal (58mm/80mm)**: narrow single-column receipt layout, condensed line items, QR code centered at bottom — this is the layout the original reference system never actually built (it only ever did A4), so treat it as a first-class addition here, not an afterthought.

## Do / Don't

**Do**
- Use Cairo for all Arabic text; reserve a monospace font only for invoice/journal numbers if desired (optional, not required).
- Keep one primary-color action per screen.
- Use hairline borders for separation, not shadows.
- Keep both light and dark themes fully functional — don't design dark-only screens.

**Don't**
- Don't introduce additional chromatic accents beyond primary/danger/success.
- Don't use heavy font weights (700+) anywhere.
- Don't build LTR-only layouts — every page must work mirrored for RTL.
