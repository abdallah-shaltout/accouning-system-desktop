# 18.C — Contrast & accessibility

**Measured today** (WCAG 2.x ratios; text needs 4.5:1, control boundaries and icons need 3:1):

| Pair | Ratio | Verdict |
|---|---|---|
| Input border `#d4d4d8` on white | **1.48** | fails 3:1 (1.4.11): input edges are almost invisible |
| Input border on surface `#f4f4f5` (cards, like the wizard) | **1.34** | fails |
| Dark: border `#2d2e33` on surface | **1.31** | fails |
| Secondary text `#71717a` on surface (labels, hints on cards) | **4.40** | fails 4.5 |
| Secondary text on surface-hover | **3.81** | fails |
| Danger text `#ef4444` on surface (every field error) | **3.42** | fails |
| Success `#22c55e` / warning `#d97706` as text on white | **2.28 / 3.19** | fail |
| Switch off track on surface | **1.15** | fails |
| Dark **teal** accent: white on `#2dd4bf` | **1.86** | fails badly |
| Dark **rose** accent: white on `#fb7185` | **2.69** | fails |
| Default *equal* green, both modes | 5.1–7.2 | passes |

**Candidate fixes** (the script decides the final values):

| Token | Light | Dark | Result |
|---|---|---|---|
| new `--color-border-control` (inputs, selects, switch off, checkbox) | ~`#85858f` | ~`#6b6d78` | ≥3:1 on background **and** surface |
| `--color-border` (hairlines, dividers) | unchanged | unchanged | decorative only, stays calm (rule 15) |
| `--color-text-secondary` | `#5f5f69` (5.74 on surface) | unchanged (5.37) | passes |
| `--color-danger` (text use) | `#c81e1e` (5.22) | unchanged (4.87) | passes |
| `--color-success` / `--color-warning` text | `#15803d` / `#a15c07` | check | ≥4.5 |
| dark teal / rose `--color-on-primary` | dark ink `#042f2e` / `#2a0610` | | 7.8 / 6.9 |

**Tasks**
- [ ] `scripts/check-contrast.ts` (this also closes doc 17 Phase E's task): parses `design-system.css`, checks every base × accent × mode for text/bg pairs (4.5), control borders and focus ring (3), on-primary (4.5). Wired into `bun run check`.
- [ ] Apply the token fixes; add `--color-border-control`; move `.control`, shadcn `input`, `select`, `switch`, `checkbox` onto it.
- [ ] Placeholder: drop `opacity: 0.8` (it takes the placeholder under 4.5). Use the secondary color as-is.
- [ ] Focus: one visible ring (≥3:1) on every control; verify in the dev gallery by Tab-walking.
- [ ] Field semantics: `aria-invalid` + `aria-describedby` → the error `<p>` (with `role="alert"` on first show) in `AppInput`, `AppSelect`, `AppPhoneInput`, `AppCombobox`, `AddressFields`. Required marker `*` is `aria-hidden`, with `aria-required` on the control.
- [ ] Wizard step list: `aria-current="step"`, and the "اختياري" chip is readable (≥4.5).
- [ ] Screenshots: wizard company step, invoice form, dashboard × light/dark × every accent (gate).

