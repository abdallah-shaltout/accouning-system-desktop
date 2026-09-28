# 02 — Design System (Tailwind v4 `@theme`, fonts, RTL rules, primitives)

Status: **done** (2026-09-29). Tokens live in `app/assets/css/main.css`; values were re-measured from the reference at implementation time (coral `#F3553B`, forest `#03201F`, cream `#FFE9D4`, container 1320px) and supersede the first-draft values below. Primitives shipped: `LContainer`, `LEyebrow`, `LPillButton`, `LLogo`, `LSplitHeading`, `LStatValue`, `LWindowMock`, `LAccordionBar`, `LBlogCard`, `LPagePanel`, `LBreadcrumb`, `LLegalPage`, `BlogCover`, `BlogArticleBody`. Open: 2 items (below).

The landing has its **own** token file: `apps/landing/app/assets/css/main.css`. It mirrors the
reference palette (not the desktop app's accent presets) but keeps the app's *soul rules*: no pure
black, no weight ≥ 700, hairlines over shadows, logical properties only.

## Tasks

- [x] Create `app/assets/css/main.css` with `@import "tailwindcss";` + the `@theme` block below.
- [x] Add base layer: `html { dir handled in nuxt.config }`, `body` bg/ink defaults, `.num` utility,
      selection color (coral/20), focus-visible ring (coral).
- [x] Register fonts via `@nuxt/fonts` (Alexandria 400/500/600 + IBM Plex Sans Arabic 400/500) —
      subsets `arabic,latin`, `font-display: swap`, preload the two display weights.
- [x] Build the primitives listed below in `app/components/ui/`, each ≤ 80 lines.
- [x] Verify every color used on the page comes from a token (grep for `#` in components = 0 hits).

## `@theme` tokens (final values — from 01-reference-analysis "Global observations")

```css
@import "tailwindcss";

@theme {
  /* palette */
  --color-coral-400: #F97A5E;
  --color-coral-500: #F4553C;   /* primary: CTAs, eyebrows, stat band */
  --color-coral-600: #E14830;   /* hover */
  --color-bone-50:  #F7F3EC;    /* hero gradient light stop */
  --color-bone-100: #F1EFEA;    /* page background */
  --color-bone-200: #EDE7DC;    /* hero gradient dark stop */
  --color-panel:    #EDEDED;    /* grey device panels (S5/S9) */
  --color-ink:      #182721;    /* headings on light — green-black, never #000 */
  --color-ink-soft: #6F746C;    /* muted body on light */
  --color-forest-900: #122019;  /* dark sections, footer */
  --color-forest-950: #0D1813;  /* footer wells, S5 outer */
  --color-lime-300: #D3F36B;    /* charts inside dark UI mocks only */

  /* type */
  --font-display: "Alexandria", "Cairo", ui-sans-serif, system-ui, sans-serif;
  --font-body: "IBM Plex Sans Arabic", "Alexandria", ui-sans-serif, system-ui, sans-serif;

  /* type ramp (desktop / clamp() for fluid) */
  --text-hero: clamp(2.5rem, 6vw, 4.5rem);        /* H1 72px, leading 1.08 (Arabic needs ≥1.08, not 1.02) */
  --text-stat: clamp(3rem, 7vw, 6rem);            /* 96px numerals */
  --text-h2:  clamp(2rem, 4vw, 3.25rem);          /* 52px section heads */
  --text-h3:  clamp(1.75rem, 3vw, 2.75rem);       /* 44px smaller section heads */
  --text-footer-display: clamp(2rem, 4vw, 3.5rem);
  --text-lead: 1.125rem;
  --text-body: 1rem;
  --text-eyebrow: 0.8125rem;                       /* 13px — Arabic eyebrows, weight 500, NO tracking */
  --text-small: 0.8125rem;
  --text-micro: 0.75rem;

  /* shape */
  --radius-card: 1.5rem;     /* 24px cards */
  --radius-panel: 1.75rem;   /* 28px hero/media panels */
  --radius-mini: 1rem;       /* 16px mini app-UIs, glass overlays */
  --radius-bar: 1.25rem;     /* 20px accordion bars, blog images, grey panels */

  /* rhythm */
  --spacing-section: clamp(4.5rem, 9vw, 8rem);   /* 112–128px vertical section padding */
  --spacing-head-gap: clamp(2.5rem, 5vw, 4rem);  /* headline → content */
  --container-max: 75rem;                         /* 1200px content column */

  /* effects */
  --shadow-float: 0 24px 48px -24px rgb(24 39 33 / 0.25);  /* floating UI cards only */
  --ease-out-expo: cubic-bezier(0.16, 1, 0.3, 1);
}
```

Hairlines are utility patterns, not tokens: `border-ink/10` on light, `border-white/25` on coral,
`border-white/12` on forest.

## Base layer rules

```css
@layer base {
  body { @apply bg-bone-100 text-ink font-body antialiased; }
  h1,h2,h3,.display { font-family: var(--font-display); font-weight: 500; }
  b,strong { font-weight: 600; } /* never 700+ anywhere */
  .num { direction: ltr; unicode-bidi: isolate; font-feature-settings: "tnum"; }
  ::selection { background: --alpha(var(--color-coral-500) / 20%); }
}
```

## Typography rules (Arabic-specific — enforce in review)

1. **Never `tracking-*` on Arabic text.** The reference's letterspaced uppercase eyebrows become:
   coral color + weight 500 + 13px + a leading ⊙ icon. Latin fragments (e.g. `POS`) may keep
   tracking.
2. **Line-height ≥ 1.08 for display, ≥ 1.6 for body.** Arabic ascenders/descenders clip at the
   reference's 1.02 leading; 1.08 is the visual equivalent.
3. Headlines end with an Arabic-styled period «.» — keep the template signature.
4. Digits: Latin `0-9`, always inside `.num`. Percent sign leads in Arabic: `%100`.
5. Mixed Latin tokens (`POS`, `Windows`) sit inline with `unicode-bidi: isolate`.

## RTL rules (same as the desktop app)

- Logical utilities only: `ms/me/ps/pe/start-*/end-*/text-start/border-s`. Physical `left/right`
  only for centering transforms or explicit physical needs, marked `/* rtl-ok: reason */`.
- Marquee travels left→right (reading-direction-reversed); progress bars fill from the right;
  "next/open" chevrons point left (◀ becomes the forward direction).
- GSAP: any `x:` value that means "toward reading direction" flips sign vs the LTR reference
  (define `const DIR = -1` helper in the motion composable — see 05-motion.md).

## Primitive components (`app/components/ui/`) — build once, reuse everywhere

| Component | Props / behavior | Used in |
|---|---|---|
| `LContainer.vue` | max-w `--container-max`, px-6 gutters | all sections |
| `LSection.vue` | `bg` variant (bone/coral/forest/black), section padding token | all |
| `LEyebrow.vue` | `label`, `tone` (coral / white70 / ink) — ⊙ icon + 13px label, no tracking | S1,S4,S6,S8,S11 |
| `LHeading.vue` | `as`, `size` (hero/h2/h3), `dim` slot for the 45%-opacity second sentence | all headings |
| `LPillButton.vue` | `tone` (coral/white/ghost), `size` (sm/md/lg), full radius, hover `-translate-y-px` + coral-600, `:active scale-[0.98]` | S0,S1,S12, footer |
| `LHairline.vue` | `tone` (light/coral/forest), horizontal or vertical | S2,S3,S6,S7,S10,S13 |
| `LGlassCard.vue` | white/12 bg + white/25 border + inset top highlight `shadow-[inset_0_1px_0_rgb(255_255_255/0.25)]`, radius-card | S4 |
| `LMarquee.vue` | slot duplicated 2×, CSS keyframe translate loop, `dir`-aware direction | S2 |
| `LStat.vue` | `value` (string with `+`/`%` at 50% opacity), `label`, count-up on enter | S3, S8 |
| `LAccordionRow.vue` | active state, height-auto expand (GSAP), coral active tone | S6, S10 |
| `LSegmented.vue` | two pills, white/10 active fill (forest bg) | S7 |
| `LWindowMock.vue` | desktop-app window chrome (traffic-light-less, Arabic titlebar), image/component slot | S5, S9 |
| `LFloatCard.vue` | dark-glass overlay card (blur + white/10 border), radius-mini, shadow-float | S1(opt), S5, S6 |
| `LChip.vue` | outlined pill, icon + label, 44px | S9 |
| `LBlogCard.vue` | image 4:3 (`NuxtImg`), title, date `.num`, hover zoom | S11 |

Mini app-UI mocks live in `app/components/mocks/` (`MiniInvoiceCard`, `MiniStockCard`,
`MiniReportCard`, `TransferCard`, `ApprovalCard`, `ReceiptOverlayCard`, `PosScreenMock`,
`DashboardMock`) — Arabic, RTL, token-colored, each ≤ 120 lines, no external images.

## Gate

- [ ] `/` renders a token test page (temporarily) showing palette, ramp, primitives in RTL — then delete it. — *Not done: replaced by section-by-section screenshot comparison against the reference (07).*
- [ ] Zero hex colors outside `main.css`; zero `tracking-` on Arabic nodes; zero physical utilities without `rtl-ok`. — *Partly: 0 hex in components except `ui/BlogCover.vue` (illustration palettes, like the app's print-template exemption). 0 physical utilities, `tracking-` only on Latin digits.*
