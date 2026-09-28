# 03 — Project Setup & Architecture (Phase A)

Status: pending

Greenfield scaffold in `apps/landing/`. Own package; never touches the desktop app's build.
Package manager: **bun** (same as the repo).

## Tasks

- [ ] Scaffold: `cd apps/landing && bunx nuxi@latest init . --package-manager bun` (Nuxt 4,
      `app/` directory structure). Keep `reference/` and `plans/` folders untouched.
- [ ] Add deps: `bun add gsap` · `bun add -D tailwindcss @tailwindcss/vite`
- [ ] Add modules: `bunx nuxi module add image fonts seo` (`@nuxt/image`, `@nuxt/fonts`,
      `@nuxtjs/seo` — the umbrella that brings schema-org, sitemap, robots, og-image, seo-utils).
- [ ] Wire Tailwind v4 as a **Vite plugin** (not PostCSS): `vite: { plugins: [tailwindcss()] }`
      in `nuxt.config.ts`, css: `['~/assets/css/main.css']`.
- [ ] `nuxt.config.ts` essentials (below).
- [ ] Folder structure (below).
- [ ] GSAP client plugin + `useLandingMotion` composable skeleton (below).
- [ ] `app/utils/brand.ts` — copied constants (D10): `APP_NAME_AR`, `APP_NAME_EN`, `APP_SHORT`,
      plus landing-only: `TAGLINE_AR`, nav labels, download/contact links read from runtimeConfig.
- [ ] `bun run dev` boots; `bun run generate` produces `dist/` (add `"generate": "nuxt generate"`,
      `"preview": "nuxt preview"` scripts).

## `nuxt.config.ts` (the load-bearing parts)

```ts
export default defineNuxtConfig({
  compatibilityDate: '2026-09-01',
  modules: ['@nuxt/image', '@nuxt/fonts', '@nuxtjs/seo'],
  css: ['~/assets/css/main.css'],
  vite: { plugins: [tailwindcss()] },          // import tailwindcss from '@tailwindcss/vite'
  app: {
    head: {
      htmlAttrs: { lang: 'ar', dir: 'rtl' },   // THE line everything else depends on
      meta: [{ name: 'theme-color', content: '#F4553C' }],
    },
  },
  fonts: {
    families: [
      { name: 'Alexandria', weights: [400, 500, 600], subsets: ['arabic', 'latin'] },
      { name: 'IBM Plex Sans Arabic', weights: [400, 500], subsets: ['arabic', 'latin'] },
    ],
  },
  site: {                                       // consumed by @nuxtjs/seo
    url: process.env.NUXT_PUBLIC_SITE_URL ?? 'https://equal-app.com',   // D11 placeholder
    name: 'ايكوال المحاسبي',
    description: 'برنامج محاسبة سطح مكتب للمحلات — فواتير، كاشير، مخزون وتقارير. يعمل بدون إنترنت وبياناتك تبقى على جهازك.',
    defaultLocale: 'ar',
  },
  runtimeConfig: {
    public: {
      siteUrl: 'https://equal-app.com',
      downloadUrl: '#download',                 // real installer URL later (D11)
      contactWhatsApp: '',
    },
  },
  nitro: { prerender: { crawlLinks: true, routes: ['/'] } },
})
```

## Folder structure

```
apps/landing/
├─ plans/                     ← these files
├─ reference/                 ← the two PNGs (never shipped; exclude from dist)
├─ nuxt.config.ts
├─ package.json
├─ public/
│  ├─ favicon.svg  og-fallback.png  robots handled by module
├─ app/
│  ├─ app.vue                 ← shell: <NuxtPage/>, skip-link, SchemaOrg setup
│  ├─ assets/css/main.css     ← @theme tokens (02)
│  ├─ assets/img/             ← hero-abstract.webp, halftone.svg, noise.png, blog covers
│  ├─ pages/
│  │  ├─ index.vue            ← composes S0–S13, ≤ 120 lines (sections are components)
│  │  └─ blog/[slug].vue      ← Phase G (3 articles via Nuxt Content? no — simple md-in-component pages)
│  ├─ components/
│  │  ├─ ui/                  ← primitives (02)
│  │  ├─ mocks/               ← Arabic mini app-UI components (02)
│  │  └─ sections/            ← one file per section:
│  │     SiteNav.vue HeroSection.vue TickerSection.vue StatsSection.vue
│  │     ProductCardsSection.vue DevicesSection.vue FeatureAccordion.vue
│  │     RulesSection.vue TrustSection.vue TeamSection.vue BusinessTypes.vue
│  │     BlogSection.vue CtaSection.vue SiteFooter.vue FaqSection.vue*  (*see 06 — AEO)
│  ├─ composables/
│  │  ├─ useLandingMotion.ts  ← GSAP context helper (below)
│  │  └─ useCountUp.ts
│  ├─ utils/brand.ts
│  └─ data/
│     ├─ features.ts  stats.ts  nav.ts  businessTypes.ts  blog.ts  faq.ts
│        ← ALL copy lives here as typed const arrays (04), never inline in components
└─ plugins/gsap.client.ts
```

Rule carried over from the app: **section components stay under ~250 lines**; copy lives in
`app/data/*` so text edits never touch markup.

## GSAP wiring (client-only, Vue 3 lifecycle-safe)

`plugins/gsap.client.ts`:

```ts
import gsap from 'gsap'
import { ScrollTrigger } from 'gsap/ScrollTrigger'
export default defineNuxtPlugin(() => {
  gsap.registerPlugin(ScrollTrigger)
  return { provide: { gsap, ScrollTrigger } }
})
```

`composables/useLandingMotion.ts` — every section calls this instead of raw GSAP:

```ts
export function useLandingMotion(setup: (ctx: { gsap: GSAP; dir: number }) => void) {
  const { $gsap } = useNuxtApp()
  let ctx: gsap.Context
  onMounted(() => { ctx = $gsap.context(() => setup({ gsap: $gsap, dir: -1 /* RTL */ })) })
  onUnmounted(() => ctx?.revert())
}
```

`dir: -1` is the RTL sign helper (05-motion.md): "enter from reading-direction" = `x: 40 * dir`.
ScrollTriggers are created in DOM order (sections mount top-to-bottom in `index.vue`), so no
`refreshPriority` juggling is needed; call `ScrollTrigger.refresh()` once after fonts load
(`document.fonts.ready`).

## Gate (Phase A done when)

- [ ] `bun run dev` serves an RTL Arabic page with tokens + fonts applied (Alexandria visible in headings).
- [ ] `bun run generate && bun run preview` works; `dist/` contains prerendered `index.html` with `dir="rtl"`.
- [ ] GSAP demo tween runs (then removed); no SSR errors (`gsap` never imported server-side).
- [ ] `reference/` and `plans/` excluded from the build output.
