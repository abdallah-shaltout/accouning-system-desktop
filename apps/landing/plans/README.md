# Equal Landing Page — Master Plan (ايكوال المحاسبي)

> **Status (2026-09-29): implemented, not yet deployed.** All phases A–H are built and verified
> locally (production SSG build, Lighthouse, keyboard, 6 widths, 3 browser engines). What is still
> missing before this can be called finished:
> 1. **User inputs:** final domain (`NUXT_PUBLIC_SITE_URL`), the Windows installer URL
>    (`runtimeConfig.public.downloadUrl`, now `#download`), real contact channels (WhatsApp) and
>    who builds Equal (E-E-A-T), pricing wording for the FAQ.
> 2. **Needs the public URL:** Google Rich Results Test, Arabic SERP preview.
> 3. **Mobile Lighthouse performance is 89–91, not ≥ 95** — caused by the display font swap
>    (see 07); `font-display: optional` fixes it at a typography cost. Your call.
> 4. Not measured: 6× CPU-throttled scroll fps; a 50%-opacity overlay against the reference.
>
> **Run it:** `cd apps/landing && bun install && bun run dev` (http://localhost:3000).
> Build: `bun run generate` → static site in `.output/public/`. Typecheck: `bunx nuxi typecheck`.
> Visual QA: `python scripts/scrollshots.py <url>` (viewport steps + contact sheet),
> `python scripts/shots.py <url> <width>`; regenerate `public/og.png` + `logo.png` with
> `python scripts/og.py` while the dev server runs on :3100.

Arabic-only, fully-RTL marketing landing page for the Equal desktop accounting app, built as a
**pixel-faithful clone of the "Nero" fintech reference** (`apps/landing/reference/01-256119823.png`
+ `03-256119823.png`) — same layout, same geometry, same rhythm — re-skinned with Equal's brand,
real features, and Arabic copy.

**Stack:** Nuxt 4 (SSG) · Tailwind CSS v4 (`@theme` tokens) · `@nuxt/image` · `@nuxt/fonts` ·
`@nuxtjs/seo` (schema-org / sitemap / robots / og-image) · GSAP 3 + ScrollTrigger.

**Location:** everything lives in `apps/landing/` (its own package, its own `package.json`,
its own lockfile). It never imports from `../../src` — brand constants and tokens are **copied**
into the landing (see decisions below).

---

## Goal

One page that a non-technical Egyptian/Gulf shop owner scrolls and immediately *gets*:
what Equal is (محاسبة سطح مكتب تعمل بدون إنترنت), what it does (فواتير، كاشير، مخزون، تقارير),
why to trust it (بياناتك على جهازك، فحوصات محاسبية تلقائية), and one obvious action (تحميل/تجربة).
Visually indistinguishable in quality from the reference; culturally and linguistically native Arabic.

## Reading order (implementation session)

1. This README (decisions + phases + gates).
2. [01-reference-analysis.md](01-reference-analysis.md) — the pixel anatomy of every section. **The source of truth for layout.**
3. [02-design-system.md](02-design-system.md) — tokens, fonts, type ramp, RTL rules, component inventory.
4. [03-setup.md](03-setup.md) — scaffold, modules, folder structure, GSAP wiring.
5. [04-sections-copy.md](04-sections-copy.md) — final Arabic copy per section + rationale.
6. [05-motion.md](05-motion.md) — GSAP/ScrollTrigger choreography per section.
7. [06-seo-geo-aeo.md](06-seo-geo-aeo.md) — meta, JSON-LD, AEO/GEO, sitemap, llms.txt.
8. [07-qa-pixel-perfect.md](07-qa-pixel-perfect.md) — verification gates.
9. [EXECUTE-PROMPT.md](EXECUTE-PROMPT.md) — paste-ready prompt for the implementation session.

## Decisions (made, not open)

| # | Decision | Why |
|---|----------|-----|
| D1 | **Pixel-parity first, content second.** Layout, spacing, radii, color roles, section order copy the reference 1:1 (mirrored for RTL). Only *content* (language, brand, features, imagery subjects) changes. | User requirement: "100% the same pixel perfect". |
| D2 | **Fonts:** Display = `Alexandria` (Google Fonts, Arabic+Latin, geometric — closest Arabic analog of the reference grotesque). Body = `IBM Plex Sans Arabic`. Max weight **600** anywhere (brand rule: never bold ≥700). **Never letter-space Arabic text** — Arabic script is connected; tracking breaks it. Eyebrows get color+weight+size instead of tracking. | Brand soul continuity with the app (Cairo-family look, Linear-style weights) + reference fidelity. |
| D3 | **Digits stay Latin (0-9) and LTR** inside RTL text, wrapped in a `.num` utility (`direction:ltr; unicode-bidi:isolate`), matching the desktop app's convention and the reference's numeral look. | Same rule as the app; numerals are a visual signature of the reference. |
| D4 | **Logo strip → feature ticker.** No real client logos exist. Same visual treatment (white wordmarks + vertical hairlines on coral, infinite marquee) but the items are Equal capability wordmarks (فواتير ضريبية، كاشير POS، مخزون، تقارير…). | Honesty (copywriting skill: never fabricate) with zero layout deviation. |
| D5 | **Stats are real product facts:** `+28` تقرير جاهز · `14` فحصًا محاسبيًا تلقائيًا · `%100` بياناتك على جهازك. | Grounded in the repo (28 report routes, 14 invariants, offline-first desktop). No fake "50K customers". |
| D6 | **Testimonials section → business-type accordion.** Same layout (one expanded light card + collapsed dark bars) but rows are business types (سوبر ماركت، محل ملابس، صيدلية…) with a scenario paragraph instead of fabricated quotes. Swappable to real testimonials later without layout change. | No fake people/quotes; identical geometry. |
| D7 | **Mini app-UIs are rebuilt as Vue components, not screenshots** (the 3 product cards, the transfer card, the glass overlay card). Crisp at any DPI, natively RTL/Arabic, and animatable. The dark phone dashboard uses a real Equal screenshot when provided; until then a component mock in the same composition. | Reference's mock-UI cards must read Arabic; screenshots of the English-less app at card size would blur. |
| D8 | **SSG output** (`nuxi generate`), deployable to any static host. No server runtime. | Fastest CWV, simplest hosting, best SEO. |
| D9 | **No reduced-motion variants.** Animations always run at full power (inherited product decision, CLAUDE.md rule 20 — applies repo-wide). | Repo rule. |
| D10 | Landing **does not import** desktop `src/`; it copies `APP_NAME_AR = 'ايكوال المحاسبي'`, `APP_NAME_EN = 'Equal Accounting'`, `APP_SHORT = 'Equal'` into `app/utils/brand.ts` with a comment pointing at the source of truth. | Separate package; seam stays clean. |
| D11 | All URLs/config that depend on deployment go through `runtimeConfig.public` (`siteUrl`, `downloadUrl`, `contactWhatsApp`). Defaults are placeholders that build fine. | Unblocks implementation before hosting decisions. |

## Open items for the user (do NOT block on these — build with defaults)

- **Final domain** (`siteUrl`) — needed before deploy for canonical/OG/JSON-LD (placeholder `https://equal-app.com`).
- **Download link** for the Windows installer (placeholder `#download` anchor with "قريبًا" state).
- Real app screenshots (Arabic UI, dark theme) for the phone/dashboard panels — component mocks ship first.

## Phases

| Phase | File | What | Size | Status |
|-------|------|------|------|--------|
| A | [03-setup.md](03-setup.md) | Scaffold Nuxt 4 + Tailwind v4 + modules + fonts + GSAP plumbing | M | done |
| B | [02-design-system.md](02-design-system.md) | `@theme` tokens, base styles, primitives (Container, Eyebrow, PillButton, Hairline, GlassCard, Marquee, SectionHeading, `.num`) | M | done (2 open) |
| C | [01-reference-analysis.md](01-reference-analysis.md) §S0–S5 + [04-sections-copy.md](04-sections-copy.md) | Top half: Navbar, Hero, Ticker, Stats, Product cards, Feature accordion | L | done |
| D | §S6–S13 of the same two files | Bottom half: Dark "rules" section, Trust cluster, Team, Business-type accordion, Blog cards, CTA band, Footer + FAQ | L | done |
| E | [05-motion.md](05-motion.md) | GSAP/ScrollTrigger choreography, marquee, counters, accordions, parallax | M | done (2 open) |
| F | [06-seo-geo-aeo.md](06-seo-geo-aeo.md) | Head/meta, JSON-LD graph, FAQ (AEO), sitemap/robots/llms.txt, OG image | M | done (6 open) |
| G | [04-sections-copy.md](04-sections-copy.md) §Blog | 3 real articles (`/blog/<slug>`) matching the blog cards | S | done |
| H | [07-qa-pixel-perfect.md](07-qa-pixel-perfect.md) | Pixel overlay vs reference, responsive, Lighthouse, RTL audit | M | mostly done (4 open) |

Each phase file contains its own `- [ ]` checklist and gate. Tick boxes and add a status note at the
top of the phase when done (same convention as the main repo).

## Deviations from this plan (made during implementation, 2026-09-29)

| # | What changed | Why |
|---|---|---|
| X1 | **S5 "device showcase" was dropped as its own section**; its content (dark dashboard + transfer/limit cards) lives in S9 exactly as in the full-page reference. | Re-slicing `03-*.png` at full resolution showed crop 01's black panel is a presentation shot of the S9 panels, not a separate section. |
| X2 | Page structure is **rounded white panels on a coral → forest frame** (hero panel, accordion panel, one long panel for trust → blog), not flat bone sections. Panels inset 10px, content column 1320px. | Measured from the reference (frame 1400px, gutters 39px, panel radius ~28px). |
| X3 | S9 feature row is a **hairline 3 + 2 cell grid**, not pills; S10 collapsed rows are **light** bars with a dark square arrow button. | Same re-measurement. |
| X4 | Hero H1 line 2 → «وبياناتك في محلّك.» | The planned line wrapped to 4 lines at display size; the new one keeps the 2-line composition and the privacy promise. |
| X5 | First-paint motion is **CSS keyframes**; **GSAP loads asynchronously** after `onNuxtReady` (`$motion`); below-the-fold sections use Nuxt **lazy hydration** (`hydrate-on-visible`) and `content-visibility: auto`. | Mobile Lighthouse went 84 → 90 and TBT 250 → <100ms; SSR HTML (SEO) unchanged. Side effect: full-page screenshots can't show those sections — use `scripts/scrollshots.py`. |
| X6 | Accordions animate with CSS `grid-template-rows` instead of GSAP height tweens. | No height desync possible under rapid clicking (verified: 21 rapid clicks → exactly one row open). |
| X7 | Hero art, product/app mocks, role tiles and blog covers are **SVG/CSS components**; no raster images, so `@nuxt/image` is installed but unused. | Crisp at any DPI, Arabic/RTL-native, animatable, zero image requests. Real app screenshots can replace the scene mocks later. |
| X8 | S8 portrait photos → **role tiles** (كاشير، محاسب، أمين مخزن، مدير…); footer has **no social icons**. | No real people or social profiles exist yet (honesty rule, like D4–D6). |
| X9 | Logo is a simple coral **"=" mark** (`LLogo.vue`, `public/favicon.svg`, `public/logo.png`). | The app icon is a detailed portrait illustration that doesn't read at nav size. Swap in a final logo when available. |
| X10 | Extra pages: `/blog` index, `/privacy`, `/terms`, and a noindexed `/og-card` (source of `public/og.png`). | Footer/nav links must resolve (0 broken links verified); OG image needs browser-shaped Arabic. |
| X11 | Added `lucide-vue-next` (icons) and pinned `typescript@5` + `vue-tsc`. | Icons for ticker/features/mocks; `nuxi typecheck` does not run on TypeScript 7. |

## Definition of done (the landing's own gates)

```bash
cd apps/landing
bun install
bun run dev        # visual check at http://localhost:3000
bun run generate   # SSG build must succeed with zero warnings that matter
bun run preview    # serve dist and re-check
```

- [ ] Side-by-side + 50%-opacity overlay comparison against `reference/03-*.png`: section order, proportions, radii, colors match (mirrored). See [07-qa-pixel-perfect.md](07-qa-pixel-perfect.md). — *side-by-side done at 1400px; no opacity overlay.*
- [ ] Lighthouse (mobile + desktop) ≥ 95 Performance / 100 SEO / ≥ 95 Accessibility / ≥ 95 Best Practices. — *desktop ✓; mobile performance 89–91 (see 07).*
- [x] `dir="rtl" lang="ar"` on `<html>`; zero physical `left/right` utilities except `/* rtl-ok */`-marked lines; all digits `.num`-wrapped.
- [ ] Rich Results Test passes for every JSON-LD block; no fabricated review/rating markup. — *no fabricated rating markup ✓; Rich Results Test not run (needs public URL).*
- [ ] Every GSAP animation runs 60fps (transform/opacity only), no `markers: true` left in. — *transform/opacity only ✓, markers 0 ✓; fps not measured under throttling.*
- [x] Checked at 1920, 1440, 1280, 768, 390 widths; no horizontal scroll anywhere.
