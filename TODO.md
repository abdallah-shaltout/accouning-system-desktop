# TODO

## 2026-09-26 — `structuredClone` crash fix: two findings spun off, still open

The crash itself (wizard's Branches step passing a reactive Proxy into `setupService.applyBranches`)
is fixed and committed, along with the same already-fixed pattern in `StepCompany.vue`'s debounced
autosave watcher. Two new findings surfaced while verifying the fix, still open:

- **`BUG-0009`**: the customer-Excel-import wizard times out waiting for a "متابعة" button
  (`python scripts/e2e/run.py --only onboarding` gets past the wizard cleanly, then fails here).
- **`BUG-0010`**: after `finishOnboarding()`, the redirect to `/login` doesn't complete within 15s
  in `setup-wizard-eg` (`page.wait_for_url` timeout). Worth checking `finishOnboarding()`'s
  toast/`router.replace` timing or whether the login route is slow to mount.

## plans/pending/18-countries-a11y-diagnostics/ — Phase B done, C-G pending

Phase A (phone input + switch RTL fix): **done** (`e8ffb8b`). Phase B (diagnostics foundation),
all 7 parts: **done and verified**, committed 2026-09-26 — `ac3fe98` (B1-3: logService's 5
channels, Rust `diag_*` + `tauri-plugin-log`, `wrap()` codemod, `/__diag` middleware), `aab3b85`
(B4: structured audit trail + Settings → سجل التدقيق), `4fb3064` (B5: `/dev/diagnostics` page +
Ctrl+Shift+D overlay), `b174d16` (B6: support-bundle export), `9429326` (B7: the
`docs/diagnostics/ISSUES.md` ledger + `scripts/diagnostics/`), `f0bc75d` (memory regen), `aa4c163`
(wired `updateBranch` to the new audit trail as a real diff example + fixed hardcoded hex colors
in `DiagOverlay.vue`), `f15c9d7` (checkboxes ticked, README Phase B row set to `done`).
Independently re-verified on final `master` HEAD: `bun run build` clean, `bun run verify:mocks`
49/0/0, `bun run check` clean (warning-mode guard only).

Phases C-G of the same plan (contrast/a11y, Egypt+Saudi country profiles incl. the Egypt VAT
rate fix, the address picker, the accounting debugger, closing the dev loop) are still fully
pending — see `plans/pending/18-countries-a11y-diagnostics/README.md` for the phase table and the
open Saudi-data-license decision.

## docs/v2/17-ui-system-rtl-themes.md — Phase F status

**F-2 through F-6 have not started** per doc 17's own phase table (F-1, F-4, F-5, F-5b are ticked
done; F-2/F-3/F-6 are unchecked) — migrate line-item forms next (highest risk — touches accounting
math paths, read `docs/v2/02-accounting-review.md` first), then simple forms, then flip the guard
script to error mode (F-6: zero findings, plus a new rule blocking `@/mocks` imports outside
`modules/*/services`/`helpers` so seam regressions fail the build). Each batch needs its own commit
and a full e2e gate (`python scripts/e2e/run.py`, not just touched flows).

## 2026-09-26 — doc-17 F-3 (simple forms): real gaps found, left open

Migrated the 6 non-line-item forms (`ExpenseFormPage`, `PaymentFormPage`, `VoucherFormPage`,
`ProductFormPage`, `PartyFormPage`, `UserEditorPage`) onto `FormPage` + `FormSection` +
`FormActions`. Two real architecture gaps found and left open, not fixed (out of that pass's
scope):

- **`useForm()` was not used for any of the six forms.** None had a route-leave guard, Ctrl+S, or
  (except Product/User) even a Zod schema before. Porting `problems`-array forms to a Zod schema
  first, then wiring `useForm()`, is real work for whoever picks this up next.
- **`FormField` is not actually usable with any existing `App*` control today.** Every `App*`
  control (`AppInput`, `AppSelect`, `AppCombobox`, `AppDatePicker`, `AppTextarea`, `AppSwitch`)
  renders its own internal `<label>`/hint/error and `useId()`, with no external `id` for
  `FormField`'s scoped-slot `fieldId`/`describedBy` to drive. Needs either (a) an optional external
  `id`/`described-by` prop on every `App*` control, or (b) scoping `FormField` to raw shadcn
  primitives only and updating doc 17 Phase F's F2 rule 3 language accordingly.

Out of scope, left for F-1/F-5 (not F-3): `CategoriesUnitsPage`, `PriceListsPage`,
`BranchesSettingsPage`, `CurrenciesSettingsPage` are `NamedListManager`-based list/CRUD pages, not
single-record routed forms — they don't fit `FormPage`/`useForm()`'s contract.

No dedicated e2e flow exists for parties or users to directly exercise `PartyFormPage`/
`UserEditorPage`'s happy path — only the static gates cover them; consider adding `parties`/`users`
flow files.

## 2026-09-26 — doc 18.E (address picker): BLOCKING — Saudi geo data is a placeholder, needs a real license decision

`src/modules/core/data/geo/sa.json` is **NOT** the real homaily dataset — homaily is **GPL-2.0, an
open licensing decision blocking Saudi shipping** (see
`plans/pending/18-countries-a11y-diagnostics/README.md`). A small, clearly-labeled hand-authored
seed stands in for it (8 major regions, ~15 cities, a handful of districts under Riyadh/Jeddah/
Madinah/Dammam), `license` field literally reading `"placeholder — pending licensing decision..."`.

**Action needed from the user:** decide whether to (a) get a compatible re-license from homaily's
maintainer, (b) find/commission an MIT/permissive Saudi regions/cities/districts dataset, or (c) ship
Egypt only for now and gate Saudi behind a "coming soon" state. Until then, Saudi Arabia's picker
works in the UI but its data is NOT real and NOT license-clean for production.

## 2026-09-26 — doc 18.E: Egypt geo data is a placeholder too — needs a real `bun run geo:build` run once online

`src/modules/core/data/geo/eg.json` is a **hand-authored placeholder** (all 27 governorates, 2-6
real cities each, ~90 total — not the full ~396-city Tech-Labs dataset). `sourceCommit` honestly
reads `"placeholder — not fetched (no network access this session), run bun run geo:build once
available"`. `scripts/geo/build.ts` is written and ready: `EG_GEO_COMMIT=<sha> bun run geo:build`
fetches from a pinned commit SHA (refuses a moving branch), normalizes/dedupes/sorts, overwrites
`eg.json`. Action needed: once online, find the latest commit SHA of
https://github.com/Tech-Labs/egypt-governorates-and-cities-db, run the script, review the diff,
commit the refreshed `eg.json` (MIT — not blocked, just not yet run).

## 2026-09-26 — pre-existing `AppCombobox.vue` bug: reopening a filled combobox shows the raw value in the search box instead of clearing it

Found while writing `address_picker.py`, reproduced on the existing `/dev/ui` combobox demo too —
shared `core/components/ui/AppCombobox.vue` infra used everywhere (invoices, products, party
forms). Repro: pick a value, close it, reopen — the visible search `<input>` shows the raw
selected value instead of being empty, so new search text doesn't reach reka-ui's filter and a
second pick silently fails (closes back to the original selection). `onUpdateOpen()` does reset
the component's own `query` ref to `''` on open, but something in reka-ui's `ComboboxInput`/
`Combobox` internals overrides the visible text with the underlying model value. Needs someone who
owns `AppCombobox.vue` to look at reka-ui's `display-value` prop or force-clear the internal search
state on open. Workaround in `address_picker.py`: use the `clearable` "X" button + a fresh reopen
instead of reopening over an existing value.

## 2026-09-26 — doc 18.G: `onboarding.py` step 2 has a Playwright strict-mode selector ambiguity

`onboarding.py`'s step 2 (`get_by_label("الدولة").select_option("SA")`) crashes with a strict-mode
violation: "الدولة" now resolves to **two** elements — the country `<select>` and a `<switch>`
whose long accessible-name sentence happens to contain "الدولة" as a substring. The crash-handling
side of this is fixed (`run.py` now catches a flow's exception, logs a `BUG-` ledger issue, and
continues to the next flow instead of aborting the whole suite) — the selector ambiguity itself is
**not fixed**. Whoever owns the setup wizard step or `onboarding.py` next should either scope
`get_by_label("الدولة", exact=True)` to the `<select>` specifically, or give the "شامل الضريبة"
switch a shorter/non-overlapping accessible name.

## 2026-09-26 -- doc 18.G: full e2e run found 4 more real, reproducible failures (logged, not fixed)

Each reproduced in isolation (`--only <area>`), each has a `docs/diagnostics/issues/BUG-000N-*.md`
entry:

- **`BUG-0004` `purchases`**: storekeeper's receiving screen — `TimeoutError` waiting for the
  short-delivery/backorder quantity input to become clickable. Possibly related to the doc-17
  F-5b seam-cleanup pass touching `PurchaseFormPage.vue`/`purchaseService.ts` — worth checking
  that diff first.
- **`BUG-0005` `branches-currencies`**: "feature-switches-off: no branch switcher in the sidebar"
  fails — toggling the multi-branch/multi-currency feature flags off doesn't actually remove the
  sidebar switcher, or the flow's flag-toggle step isn't taking effect.
- **`BUG-0006` `reports-v2`**: an export/download action never fires a browser download event
  (15s timeout).
- **`BUG-0007` `full-persona-pass`**: a queued discount-approval request doesn't appear as a
  notification for the manager, despite the request itself being correctly queued.

`bun run verify:mocks` stayed 98/0/0 throughout (these are e2e/UI-level failures, not accounting
invariant breaks).

## 2026-09-30 — plan 23 (subscription platform): phases A-C + D1 done, D2/D3 + E next

Unrelated work stream (`apps/backend` + `apps/dashboard`, not the main desktop app above). Built in
phase pairs per the user's pacing:

- **Phase A** (backend foundation): admin auth (password → TOTP → tokens, refresh rotation +
  family-reuse-detection cascade, designed fresh since the reference server had none), portal auth
  (phone + WhatsApp OTP), the Zod/idempotency/rate-limit middleware, a working `bun run plop`
  generator. `equal_dev`/`equal_test` Postgres databases created on the owner's Coolify server.
- **Phase B** (billing/licensing): organizations/users, plans + entitlement catalog, subscription
  state machine + hourly lapse cron, manual payments + invoicing, device registration + PKCE-style
  activation + Ed25519 license signing, a concurrency-safe credit ledger (real Postgres row-locking,
  tested: 10 parallel requests against a limit of 3 → exactly 3 succeed). End-to-end Gate B scenario
  test: signup → checkout Pro → payment → admin approve → device activation with a real
  cryptographically-verified Pro license → cron lapse to Free.
- **Phase C** (ops): releases (Tauri updater endpoint, deterministic rollout bucket, mandatory-
  bypass), telemetry (error-group ingestion + `IngestFinding`-shaped export for the issue-ledger
  tooling), diagnostics + feedback (remote log-pull with a 14-day expiry cron, every admin download
  audited), admin analytics (active devices, conversion, MRR, churn, credit usage per feature).
- **Phase D1** (dashboard scaffold only): Vite+Vue3+TS strict+Pinia+Vue Router(history)+Tailwind
  v4+real shadcn-vue CLI components+Zod/vee-validate+TanStack Table. Two real login pages (admin+
  portal) verified against the live backend.

All three backend phases' gates are green: `bun run lint`, `bunx drizzle-kit check`, the full test
suite (82 tests/13 files, run twice for stability, sequentially — see the parallel-agent lesson
below), `bun run build && bun run start` → `/health` 200. Dashboard: `bun run build` + `bun run
check` green.

**Two real bugs found during manager re-verification, not by the building agents** (same
cross-process-contamination pattern as this file's other parallel-agent entries above): telemetry's
occurrence-trim issued one DELETE per stale row instead of one query (fixed); a `vi.mock` for
`@@shared/storage/r2` passed in isolation but failed when another test file imported the real
module first in the same vitest worker (`isolate: false` — fixed with `vi.spyOn` on the real
module's exports instead, robust regardless of file run order). **Confirmed lesson, same as this
file's other agent-collision entries**: three separate Phase C agents hit intermittent failures
purely from running `vitest run` concurrently against the one shared `equal_test` Postgres database
(`afterEach` truncates every table) — every failure vanished on a clean sequential re-run. Always
re-verify sequentially before trusting an agent's in-parallel test claim.

**Not verified**: no browser tool was available this session (wmux reported "workspace identity
unknown" both for the scaffolding agent and the manager) — the dashboard's actual rendered pages,
RTL layout and the TOTP step were never seen by a human or a browser tool, only via `curl`/build/
check. The Saudi/Egypt geo-data placeholders and `AppCombobox` bug noted elsewhere in this file are
unrelated to this work stream.

**Next**: D2 + D3 (the real admin/portal pages) in parallel with Phase E (Rust desktop `licensing`
domain) — see `plans/pending/23-subscription-platform/` phase files, each with a Deviations section
recording what happened in A-C/D1.
