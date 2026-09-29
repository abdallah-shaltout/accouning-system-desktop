# 05 — Motion & GSAP ScrollTrigger Choreography (Phase E)

Status: **done** (2026-09-29). Change from plan: first-paint motion (hero panel, headline lines, nav) runs as CSS keyframes (`.intro-*` in main.css) so it starts at first paint; accordions animate with CSS `grid-template-rows`; the marquee is a CSS keyframe. GSAP (loaded async) owns all scroll-linked motion: reveals, count-ups, parallax, stacked-card spread, progress bars. Open: 2 measurement items. **Motion never stops (user decision 2026-09-29):** no reduced-motion code of any kind, no hover pause on the ticker, and the feature accordion and business types keep auto-advancing after clicks and hover — verified with `prefers-reduced-motion: reduce` emulated.

Motion philosophy: **calm, physical, expensive.** The reference is not a scroll-jacking site —
it's hairlines, soft reveals, one marquee, and floating UI cards. Every animation is
transform/opacity only, `ease: 'power3.out'` (or `--ease-out-expo`) for reveals, `ease: 'none'`
for anything scrubbed. No reduced-motion variants (D9). All code goes through
`useLandingMotion()` (03) so `ctx.revert()` cleans up and the RTL `dir = -1` sign is available.

Global rules (from the gsap-scrolltrigger skill):
- Register ScrollTrigger once in the plugin; never on child tweens of a timeline — the
  ScrollTrigger sits on the timeline.
- Sections mount in DOM order ⇒ triggers are created top-to-bottom; call
  `ScrollTrigger.refresh()` once on `document.fonts.ready`.
- Reveal pattern default: `toggleActions: 'play none none none'`, `start: 'top 78%'`, `once: true`
  for content reveals (never replay text). Scrub only where listed.
- `markers: true` allowed while building; the QA gate greps for it (07).

## Per-section choreography

### S0 Navbar
- On mount: bar fades/slides down (`y: -16 → 0`, 0.5s) after hero headline starts.
- Scrolled state: standalone `ScrollTrigger.create({ start: 80 })` toggles a `.is-scrolled` class →
  backdrop-blur + shadow via CSS transition (not GSAP).

### S1 Hero (the "wow" moment — load timeline, no ScrollTrigger)
Timeline on mount, total ~1.1s:
1. Panel clip-reveal: hero panel `clip-path: inset(6% round var(--radius-panel)) → inset(0)` + `scale 1.02 → 1`.
2. Eyebrow fade-up (`y: 16*dirY`), then **H1 lines reveal** — each line wrapped in an overflow
   clip, `yPercent: 110 → 0`, stagger 0.12. **Do NOT split Arabic text into chars/words** —
   SplitText per-character breaks Arabic ligatures. Line-level masks only (manual `<span
   class="line">` wrappers in the data, not SplitText).
3. CTA pops (`scale 0.9 → 1`, back.out(1.4)) + microcopy fade.
4. Abstract visual: slow perpetual drift — halftone sphere `y: ±12px` loop (8s, sine.inOut,
   yoyo), glow opacity breathe. Scroll parallax: `scrub: true`, visual `y: -6%`, text `y: 4%`
   between `start: 'top top', end: 'bottom top'`.

### S2 Ticker
- CSS keyframe marquee (no GSAP — cheaper): duplicated track, `translateX(0 → 50%)` (RTL
  direction: positive X), 28s linear infinite. GSAP only nudges `timeScale`-like speed via a
  CSS var on scroll velocity — **optional**, skip if it jitters.

### S3 Stats
- Hairlines draw in: vertical separators `scaleY: 0 → 1` (transform-origin top), 0.6s stagger.
- Count-up (`useCountUp`): once, on `start: 'top 70%'`, 1.2s, `snap` to integer, `+`/`%` glyphs
  fade in after. Numbers render final value in SSR markup (SEO) — animation sets `textContent`
  from 0 only after trigger fires.

### S4 Product cards
- H2 two-tone reveal (line masks). Cards: `ScrollTrigger.batch('.product-card', …)` —
  `y: 48 → 0, opacity 0 → 1`, stagger 0.12.
- Inside mocks (perpetual, CSS where possible): progress bars fill from the right on enter;
  MiniReportCard bars grow `scaleY` staggered; MiniInvoiceCard total ticks once.

### S5 Devices
- Panel reveal: outer light panel `y: 64, opacity 0 → 1`.
- Parallax scrub (`scrub: 0.8`): window mock `y: -4%`, floating cards `y: -10%` — different
  speeds sell depth. Cards get idle float loops (`y: ±8px`, 6s, sine, yoyo, offset phases).

### S6 Feature accordion
- Rows cascade in (`x: 32*dir → 0`, stagger 0.08).
- **Accordion behavior** (component logic + GSAP): open = `gsap.to(desc, { height: 'auto',
  opacity: 1, 0.45s, ease: power3.out })`, close previous in parallel; title color tweens to
  coral via CSS class. Auto-advance every 6s and **never stops**: a click jumps to that row and restarts the cycle from it (user decision 2026-09-29: motion always runs).
- Visual cross-fade: image stack, active `opacity 1 / scale 1` others `opacity 0 / scale 1.03`.
- Overlay glass card: enters `y: 24, opacity 0` when its image becomes active.

### S7 Rules
- Headline + sub reveal; hairline rows `scaleX: 0 → 1` from start side.
- Segmented switch: content swap = `opacity/y` 0.3s out-in (GSAP timeline, no layout jump —
  min-height reserved).
- 3D card visual: scroll scrub `rotateZ: -6° → 4°, y: 40 → -40` (`scrub: 1`); idle specular
  sweep via CSS gradient animation.

### S8 Trust cluster
- Avatars pop in (`scale 0 → 1, back.out(1.7)`, stagger 0.07 random order), connector lines
  draw (`stroke-dashoffset` — SVG lines), label pills last. Idle: each avatar drifts `y: ±6px`
  desynced. Mini-stats reuse `useCountUp`.

### S9 Team
- H2 line masks; panels `y: 56, opacity 0` staggered; chips `ScrollTrigger.batch`,
  `scale 0.9 + y: 16 → 0`, stagger 0.06.

### S10 Business types + S-FAQ
- Same accordion mechanic as S6 (shared `LAccordionRow` logic), single-open, height-auto tweens.
  Bars cascade in on scroll.

### S11 Blog
- Cards batch-reveal; hover zoom is CSS (`scale 1.04`, 0.5s ease-out-expo) — GSAP not needed.

### S12 CTA + S13 Footer
- CTA headline reveal + button pop. Footer display line: line mask reveal; columns fade-up
  stagger 0.08. Optional signature move: display line scrubbed `yPercent: 20 → 0` as footer
  enters (`scrub: true, start: 'top bottom', end: 'bottom bottom'`).

## Performance guardrails

- Only `transform`/`opacity`/`clip-path` are animated. No `height` animations except accordions
  (contained, short).
- `will-change: transform` only on: marquee track, hero visual layers, floating cards. Remove
  after load timeline completes where static.
- Idle loops capped: ≤ 6 concurrent loops on screen; all `sine.inOut`, all GPU-composited.
- No `window.addEventListener('scroll')` anywhere — ScrollTrigger only.
- Test with 6× CPU throttle: scroll must stay ≥ 50fps.

## Gate

- [x] Every section's reveal fires exactly once at the right threshold, in RTL direction.
- [x] Accordions: rapid-clicking never desyncs heights (tweens `overwrite: 'auto'`).
- [ ] `grep -r "markers" app/` → 0 hits; `ctx.revert()` verified via route hop to `/blog` and back (no duplicated triggers). — *markers grep = 0 ✓; revert-on-unmount is in `useLandingMotion` but a route-hop trigger count was not measured.*
- [ ] 6× CPU throttle scroll test recorded in the QA doc. — *Not done.*
