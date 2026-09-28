# 07 — QA: Pixel-Perfect, Responsive, RTL, Perf (Phase H)

Status: **mostly done** (2026-09-29). Lighthouse (production build, `serve`): **desktop 99–100 / 100 / 100 / 100**, **mobile 89–91 / 100 / 100 / 100** (Perf / A11y / BP / SEO); CLS 0.001–0.003, TBT < 100ms. Mobile LCP is 3.3s in the lab: the remaining delay is the Alexandria web font swapping in (measured: disabling every intro animation did not move LCP). `font-display: optional` would remove it at the cost of first-visit visitors on slow networks seeing the fallback font — left as a user decision. No horizontal overflow at 1920/1440/1280/1024/768/390; keyboard, stress and link checks pass; Chromium, Firefox and WebKit render without errors. Open: opacity overlay, 6× CPU fps recording, mobile perf ≥ 95, Rich Results screenshots.

The final gate. Run after Phases A–G are code-complete. Playwright (via the webapp-testing
skill's tooling or a local script in `apps/landing/scripts/shots.mjs`) captures full-page
screenshots for comparison.

## 1 · Pixel-parity vs reference

- [x] Capture full-page screenshot at **1600px width** (the reference's frame) → — *done at 1400px: the reference's page frame is 1400px inside its 1600px canvas. Full-page shots can't show `content-visibility` sections, so `scripts/scrollshots.py` captures viewport steps instead.*
      `apps/landing/.qa/full-1600.png`.
- [x] Side-by-side against `reference/03-256119823.png` (mirror the reference horizontally for a
      fair RTL comparison): check **section order, section heights ratio, background boundaries,
      content column width, card counts and proportions, radii, hairline placement**.
- [ ] 50%-opacity overlay for the top fold vs the hero crop in `01-256119823.png`: navbar height, — *Not done as an overlay; compared side by side at 1400px (the reference frame width).*
      hero panel inset, eyebrow→H1→CTA vertical rhythm within ±8px.
- [x] Color-picker audit: coral, bone, forest, ink sampled from the built page match the tokens
      (which match the reference within perceptual tolerance).
- [x] Typography: H1/H2/stat sizes within ±4px of the scale in 01-reference-analysis; two-tone — *compared visually against slice crops, not pixel-measured.*
      dim opacity ≈ 50%; no Arabic tracking anywhere.

## 2 · Responsive

- [x] 1920 / 1440 / 1280: content column stays 1200px, outer bleeds grow. — *column is 1320px (measured from the reference: 39px gutters in a 1400px frame), not 1200px.*
- [x] 1024: product cards 3→1 wide or 3→2+1 per reference behavior (choose 1-col stack, cards
      full width — verify nothing squishes); accordion+visual stack vertically (visual first on
      mobile? **No — text first**, matches template mobile behavior).
- [x] 768 / 390: single column everywhere; stats stack with horizontal hairlines; ticker still
      loops; nav → hamburger sheet (full-screen, forest bg, big links); **no horizontal
      scrollbar at any width** (`document.documentElement.scrollWidth === innerWidth`).
- [x] Hero uses `min-h-[100dvh]`-safe sizing (no `h-screen`), text never overlaps visual at
      tablet widths.

## 3 · RTL audit

- [x] `<html dir="rtl" lang="ar">` in generated output.
- [x] Grep gate: `grep -rE "(ml-|mr-|pl-|pr-|left-|right-|text-left|text-right)" app/ | grep -v "rtl-ok"` → 0.
- [x] Visual sweep: marquee direction, progress-bar fill direction, accordion `+` icon side,
      chevrons, avatar-cluster mirroring, footer column order.
- [x] Digits: every number LTR inside `.num`; phone/latin tokens isolated; dates readable.

## 4 · Interaction & motion

- [x] Keyboard: tab order follows visual RTL order; accordions/segmented operable with
      Enter/Space + arrows; focus-visible ring on every interactive element; skip-link works.
- [x] Accordion stress: rapid clicks, switching while auto-advance fires — no height desync.
- [ ] 6× CPU throttle scroll: ≥ 50fps, no layout thrash (DevTools performance recording). — *Not done.*
- [x] `markers` grep = 0; no console errors/warnings on load or full scroll.

## 5 · SEO/A11y final numbers (records go in this file's status note)

- [ ] Lighthouse mobile + desktop: Perf ≥ 95 / SEO 100 / A11y ≥ 95 / BP ≥ 95 (record scores). — *Desktop 99–100 / 100 / 100 / 100 ✓. Mobile 89–91 / 100 / 100 / 100 — performance below 95 (see status note).*
- [x] axe scan: 0 critical/serious (color-contrast: white/70-on-coral labels are ≥ 3:1 for
      large text — verify, bump to white/80 if failing).
- [ ] Rich Results Test screenshots saved to `.qa/`. — *Not done (needs public URL).*
- [x] `bun run generate` → `preview` → click every link and anchor (including footer + blog).

## 6 · Cross-browser

- [x] Chromium, Firefox, WebKit (Playwright's three engines): fonts, blur/glass, clip-path hero
      reveal, marquee.

Done = every box ticked, scores recorded at top of this file, and the plans folder's README
phase table set to `done` for A–H.
