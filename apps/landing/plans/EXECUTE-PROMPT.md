# Execute Prompt (paste this into the implementation session)

> Copy everything below the line into a fresh session (Opus-class model is fine). It assumes the
> working directory is the repo root `desktop-app/`.

---

Implement the Equal landing page exactly as specified by the plan set in `apps/landing/plans/`.

**Read first, in this order (do not start coding before finishing all of them):**
1. `apps/landing/plans/README.md` — decisions D1–D11, phases, gates. The decisions are final; do not reopen them.
2. `apps/landing/plans/01-reference-analysis.md` — the layout source of truth. Also open the two
   reference images `apps/landing/reference/01-256119823.png` and `03-256119823.png` and keep
   them as your visual target throughout.
3. `apps/landing/plans/02-design-system.md`, `03-setup.md`, `04-sections-copy.md`,
   `05-motion.md`, `06-seo-geo-aeo.md`, `07-qa-pixel-perfect.md`.

**What you're building:** an Arabic-only, fully-RTL landing page for «ايكوال المحاسبي» (Equal), a
Windows desktop accounting app — a pixel-faithful RTL mirror of the Nero reference (same section
order S0–S13 + FAQ, same geometry, radii, palette roles and rhythm), with the Arabic copy from
plan 04, GSAP/ScrollTrigger motion from plan 05, and the SEO/GEO/AEO/JSON-LD work from plan 06.
Stack: Nuxt 4 (SSG) + Tailwind v4 `@theme` + @nuxt/image + @nuxt/fonts + @nuxtjs/seo + GSAP.
Everything lives in `apps/landing/` (own package, bun). Never import from the repo's `src/`.

**Execution order:** Phase A (03-setup) → B (02-design-system) → C (sections S0–S5) →
D (S6–S13 + FAQ) → E (05-motion) → F (06-seo) → G (blog articles, plan 04 §S11) →
H (07-qa). Tick the `- [ ]` boxes in the plan files as you complete tasks, add a one-line status
note at the top of each phase file when its gate passes, and update the phase table in the plans
README. Commit at the end of each phase with a scoped message (`feat(landing): phase A — scaffold`).

**Rules:**
- Do not ask questions; the plans decide everything. If you hit a genuine gap, pick the option
  most consistent with pixel-parity (D1) and the reference, note it in the plan file under a
  "Deviations" heading, and continue.
- Pixel-parity beats convenience: when Tailwind's default scale fights a measurement in plan 01,
  use the token, not the nearest default.
- RTL discipline: logical utilities only (`ms/me/ps/pe/start/end`), digits inside `.num`,
  **never letter-space Arabic**, never split Arabic text per-character for animation (line masks
  only).
- All copy comes from `app/data/*.ts` as written in plan 04 — do not paraphrase it.
- Honesty constraints D4–D6: no fake logos, no fake testimonials, no fabricated stats or
  aggregateRating markup.
- Skills: invoke `/gsap-scrolltrigger` before Phase E, and `/copywriting` only if you must write
  copy the plans don't already contain (blog article bodies in Phase G). Use
  `/frontend-design:frontend-design` or `/design-taste-frontend-v1` judgment for micro-polish,
  but the reference always wins over any skill's generic taste rules.
- Verification loop: keep `bun run dev` running; after each section, screenshot it and compare
  against the reference crop before moving on (Playwright or the webapp-testing skill). Phase H
  is the formal gate — run it fully, record Lighthouse scores in plan 07.

**Definition of done:** every checklist box in plans 02–07 ticked, the README phase table all
`done`, `bun run generate` clean, and the Phase H gates green (pixel overlay, responsive 1920→390,
RTL grep = 0 violations, Lighthouse ≥ 95/100/95/95, Rich Results Test green, no console errors).

Start with Phase A now.
