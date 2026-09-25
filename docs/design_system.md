# Design System — "Linear-style" Light/Dark

Source of truth for tokens: `src/assets/styles/design-system.css` (already scaffolded — this doc explains the intent behind it and fills in component-level guidance, adapted from `references/vat-invoice-app/design.md`, a Linear-inspired reference).

## Principles

- **Midnight precision instrument**: near-black/near-white surfaces, hairline borders instead of heavy shadows, one accent color used sparingly for the single primary action per view.
- **Compact density**: tight paddings (8–12px), small radii (6px buttons/inputs, 12px cards), Inter-family type at 400–510 weight — never bold (700+).
- **RTL-first**: Arabic is the primary UI language. Layout mirrors for RTL; numbers and currency values stay LTR-oriented within RTL text blocks (see `toHindi()`-style numeral handling in `references/vat-invoice-app`).
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
docs/v2/16-equal-rebrand-and-ui-kit.md Phase B). Users can switch to indigo/teal/rose/amber in
Settings → Appearance; see `useAppearance.ts`'s `ACCENTS` map for every preset's values.

Font: `--font-sans` = Cairo (Arabic) → Inter → system-ui fallback stack. Cairo is the correct choice over pure Inter here since this is an Arabic-first UI (Inter has no Arabic glyphs).

## Radii & spacing (carry over from the Linear reference)

- Buttons/inputs: 6px radius
- Cards/panels: 12px radius
- Pills/badges: 9999px (status chips) or 4px (small badges)
- Spacing ladder: 4/8/12/16/24/32/48/64px — base unit 4px
- Card padding: 24px (16px on compact/dense tables)

## Components

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

### Data table
Dense rows (12px vertical padding), `--color-surface` header row, hairline `--color-border` row dividers, hover state `--color-surface-hover`. Right-aligned numeric columns (amounts, quantities) — remember RTL means "right-aligned" is the natural start-aligned direction for Arabic, so numeric columns should be visually consistent with LTR number rendering inside them.

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
