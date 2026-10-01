# Phase G — Validation, keys, deploy, final gate

> **Status:** pending. This is the last phase.

## G1 — Price validation (01 §5)

- [ ] Run the Van Westendorp interview with 15–20 shop owners in El Marg and the surrounding areas.
- [ ] Record the answers in a sheet that computes the four cumulative curves and the acceptable price range.
- [ ] If Pro at 299 falls outside the range, publish a new `plan_version` from the admin. The code doesn't change.

## G2 — Landing copy (conflict fix)

- [ ] `apps/landing/app/data/faq.ts` «هل في اشتراك شهري؟»: change the answer to «البرنامج مجاني مدى الحياة للمحل الصغير،
      والاشتراك اختياري لو عايز مميزات أكتر».
- [ ] Microcopy: `apps/landing/app/components/sections/HeroSection.vue:36` «مجانًا للتجربة · بدون اشتراك شهري» →
      «مجاني مدى الحياة · الاشتراك اختياري», and `CtaSection.vue:20` «… · بدون اشتراك شهري» → «… · مجاني مدى الحياة».
      Mirror the change in `apps/landing/plans/04-sections-copy.md`.
- [ ] `apps/landing/app/data/blog.ts:114` and `:133` stay true under freemium (no *mandatory* subscription), so no change.
- [ ] Add a pricing section or link to `/pricing` on the portal.

## G3 — Keys and secrets

- [ ] Generate the license Ed25519 key pair (`bun run keys:generate`). The private key goes to server env only; the
      public key goes into `licensing/keys.rs`.
- [ ] Generate the Tauri updater key pair. The private key goes into the build machine secret; the public key goes
      into `tauri.conf.json`.
- [ ] Record the key-rotation procedure in `apps/backend/README.md`: add a new `kid`, ship the desktop with both
      public keys, then switch `LICENSE_ACTIVE_KID`.

## G4 — Coolify handoff (D15: the user deploys, we deliver code)

- [ ] `apps/backend/DEPLOY.md` covers:
  - the full env list (`apps/backend/docs/07-environment.md`) with which keys are secrets;
  - build `bun run build`, start `bun run start`, pre-deploy `bun run db:migrate`, health check path `/health`,
    and port from `PORT`;
  - the one-time seed command `bun run seed`.
  No Dockerfile or compose file.
- [ ] `apps/dashboard/DEPLOY.md`: a static site, build `bun run build`, publish directory `dist/`, SPA fallback to
      `index.html`, and the single env var `VITE_API_URL`.
- [ ] Storage: the user creates the two Cloudflare R2 buckets (`R2_BUCKET_RELEASES` public-read with a custom domain,
      `R2_BUCKET_PRIVATE` private) and an R2 API token. The code only reads the `R2_*` env keys.
- [ ] The user provides `DB_URL` and `REDIS_URI` for staging and production. We run the gate A–C integration tests
      against staging before the desktop test in G5.

## G5 — Final gate (the single cargo run)

- [ ] Through `scripts/cargo-safe.ps1`: `cargo build` and the batched `cargo test` (the `all` binary suites for
      `licensing` and `update`), then `bun run bindings:check`.
- [ ] The full desktop gate from `CLAUDE.md`.
- [ ] A real `bun run desktop` against staging, covering the README end-to-end scenario plus: the update
      downloads in the background and installs on close; the terminal version-mismatch screen; the admin «اسحب
      التشخيص» → bundle received with no DB inside; a declined pull with the toggle off.
- [ ] When everything is green: move this folder to `plans/completed/`, set every status to `done`, and run
      `bun run memory`.
