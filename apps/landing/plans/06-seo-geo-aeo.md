# 06 — SEO · GEO · AEO · JSON-LD (Phase F)

Status: pending

Everything runs through `@nuxtjs/seo` (site config in 03) + hand-written JSON-LD via
`useSchemaOrg`. Target queries (Arabic, Egypt/MENA): «برنامج محاسبة للمحلات», «برنامج كاشير»,
«برنامج فواتير ضريبية», «برنامج محاسبة بدون انترنت», «برنامج مبيعات ومخزون», «برنامج حسابات
سوبر ماركت». AEO target: the FAQ answers; GEO target: being the citable entity for «برنامج
محاسبة يعمل أوفلاين وبياناته محلية».

## Tasks — classic SEO

- [ ] `useSeoMeta` on `index.vue`:
  - title: «ايكوال المحاسبي — برنامج محاسبة وكاشير للمحلات يعمل بدون إنترنت» (≤ 60 chars visual).
  - description (150–160): «برنامج محاسبة سطح مكتب للمحلات: فواتير ضريبية، كاشير POS، مخزون
    وجرد، و28 تقريرًا جاهزًا. يعمل بدون إنترنت وبياناتك تبقى على جهازك. حمّله مجانًا لويندوز.»
  - `ogTitle/ogDescription/ogImage/ogLocale: 'ar_AR'`, `twitterCard: 'summary_large_image'`.
- [ ] OG image: static designed 1200×630 (coral bg, Arabic headline, app window mock) in
      `public/og.png` — designed once, not runtime-generated (SSG simplicity). Blog articles may
      use `nuxt-og-image` template later.
- [ ] Heading hierarchy: exactly one `h1` (hero). Section heads `h2`, card/accordion titles `h3`.
      Verify with an outline check.
- [ ] Semantic landmarks: `header/nav/main/section[aria-labelledby]/footer`; skip-link
      («تخطَّ إلى المحتوى») first in DOM.
- [ ] Every `NuxtImg`: descriptive Arabic `alt`, explicit `width/height`, `format="webp"`,
      hero visual `preload` + `fetchpriority="high"`, below-fold `loading="lazy"`.
- [ ] Canonical (module handles from `site.url`); trailing-slash consistency.
- [ ] `robots.txt` (module): allow all, point at sitemap. **Do not block AI crawlers** (GPTBot,
      ClaudeBot, PerplexityBot) — GEO wants them in.
- [ ] Sitemap (module): `/`, `/blog/<3 slugs>`.
- [ ] Internal links: blog cards → articles; articles link back to `/#features`, `/#faq` with
      descriptive Arabic anchors.

## Tasks — JSON-LD (one `@graph`, via `useSchemaOrg` on `app.vue`)

- [ ] `Organization`: name «ايكوال المحاسبي», alternateName "Equal Accounting", `url`, `logo`,
      `sameAs: []` (fill when social profiles exist).
- [ ] `WebSite`: name + `inLanguage: 'ar'`.
- [ ] `SoftwareApplication` (the money block):
  ```json
  {
    "@type": "SoftwareApplication",
    "name": "ايكوال المحاسبي",
    "alternateName": "Equal Accounting",
    "applicationCategory": "BusinessApplication",
    "applicationSubCategory": "Accounting software",
    "operatingSystem": "Windows 10, Windows 11",
    "inLanguage": "ar",
    "offers": { "@type": "Offer", "price": "0", "priceCurrency": "EGP",
                "description": "نسخة تجريبية مجانية" },
    "featureList": ["فواتير ضريبية", "نقطة بيع POS", "إدارة مخزون وجرد",
                    "تقارير مالية", "تعدد الفروع", "يعمل دون اتصال بالإنترنت",
                    "نسخ احتياطي مشفّر"]
  }
  ```
  **No `aggregateRating`/`review` markup** — no real reviews yet; fabricated ratings risk manual
  actions.
- [ ] `FAQPage` from `data/faq.ts` — the exact on-page Q/A strings (parity between markup and
      visible content is a policy requirement).
- [ ] `BreadcrumbList` + `Article` (with `datePublished`, `author` = Organization, `inLanguage`)
      on each blog page.
- [ ] Validate every block in Google's Rich Results Test + schema.org validator.

## Tasks — AEO (answer engines / featured snippets)

- [ ] FAQ section (S-FAQ, 04) uses **question-phrased `h3`s** + 40–60-word direct answers —
      the extractable unit for snippets and voice.
- [ ] Each blog article opens with a 40–60-word direct answer under the H1 («الجواب المختصر:»
      pattern), then detail; use numbered `ol` steps in the جرد article (list-snippet bait).
- [ ] Definition sentence on the page (hero sub or S4 intro) that states plainly:
      «ايكوال المحاسبي هو برنامج محاسبة سطح مكتب للمحلات يعمل بدون إنترنت.» — the "X is Y"
      sentence engines quote.
- [ ] Table in the offline article comparing «برنامج سحابي vs برنامج سطح مكتب» (table-snippet bait).

## Tasks — GEO (generative engines: Perplexity/ChatGPT/AI Overviews)

- [ ] **Entity clarity:** brand named identically everywhere («ايكوال المحاسبي», Latin "Equal
      Accounting" once in footer + JSON-LD `alternateName`); one clear claim + category in the
      first 100 words of the page.
- [ ] **Factual density:** the real numbers (28 تقرير، 14 فحصًا، 4 أدوار، ويندوز 10/11) appear in
      crawlable text, not only in animated counters (SSR renders final values — 05).
- [ ] `public/llms.txt`: short English+Arabic factsheet — what Equal is, category, OS, key
      features, FAQ links, contact. (Emerging convention; costs one file.)
- [ ] About-block in footer or `/#why` naming who builds it (E-E-A-T signal); contact info real.
- [ ] Ship prerendered HTML (SSG, D8) — zero JS-only content; AI crawlers read everything.

## Performance (CWV — ranking + GEO crawlability)

- [ ] LCP = hero H1 or hero image < 2.0s (preload display font weights 500/600 as woff2,
      `font-display: swap`; hero abstract ≤ 120KB webp).
- [ ] CLS < 0.05: fonts size-adjusted fallback (`@nuxt/fonts` handles), all media sized,
      accordion min-heights reserved.
- [ ] INP: GSAP work off-main where possible, no long tasks > 200ms at load.
- [ ] Total JS ≤ 150KB gz (gsap ~70KB — import core + ScrollTrigger only, no other plugins).

## Gate

- [ ] Lighthouse SEO = 100; Rich Results Test green for SoftwareApplication, FAQPage, Article.
- [ ] `curl` the generated `index.html`: all copy, stats and FAQ text present in raw HTML.
- [ ] `llms.txt`, `robots.txt`, `sitemap.xml` reachable in `bun run preview`.
- [ ] Meta title/description render correctly in an Arabic SERP preview tool (no truncation mid-word).
