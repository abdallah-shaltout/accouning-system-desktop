# TODO

## `AGENT_MEMORY.md` now reports 7 additional seam violations

Regenerating memory after merging the two items below surfaced 7 previously-unreported seam
violations (`NavUser.vue`, `useNotifications.ts`, `helpers/attachments.ts`,
`parties/helpers/creditLimit.ts`, `useCatalogStore.ts`, `settings/helpers/backupArchive.ts`,
`src/router/index.ts` — all import `src/mocks/*` outside `services/`). None of these files were
touched by either agent; they were pre-existing and the previous `AGENT_MEMORY.md` on `master`
was simply stale/hadn't caught them. Per the seam rule (CLAUDE.md), don't add more — move them
behind a service when next touching the file. Report-only for now, per user's "review later"
instruction above.

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
49/0/0, `bun run check` clean (warning-mode guard only). Full e2e suite reported green by the
implementing agent (after ruling out transient dev-server contention from the concurrent Phase F-0
agent as the cause of earlier flaky runs) — not independently re-run by the reviewing session.

Phases C-G of the same plan (contrast/a11y, Egypt+Saudi country profiles incl. the Egypt VAT
rate fix, the address picker, the accounting debugger, closing the dev loop) are still fully
pending — see `plans/pending/18-countries-a11y-diagnostics/README.md` for the phase table and the
open Saudi-data-license decision.

**Status 2026-09-26 (later same day):** Phase C and Phase F were relaunched as background agents
(directly on `master`, no worktree) after an earlier attempt collided with a concurrent session's
plan-20 route-object refactor (see the cross-session collision note below) — check `git log` for
whether they've landed by the time you read this. **Phase D and Phase E are paused, not started**:
D touches many of the same form pages (`PartyFormPage`, `ProductFormPage`, `StepCompany`, wizard
step order) that the other session's plan-20 refactor is still working through — do not start D
until plan 20 is finished and committed clean. E depends on D's `countryProfiles.ts` existing, so
it's paused transitively. G depends on F.

## docs/v2/17-ui-system-rtl-themes.md — Phase F: F-0 done, F-1 through F-6 not started

Phases A–E of doc 17 are done and committed (RTL, motion, native save dialogs, sidebar, themes).
**Phase F-0** (shared blocks/layouts groundwork) is now **done**, committed 2026-09-26: `2f9913f`
(blocks: `FormField`, `FormSection`, `FormActions`, `useForm()`, `FilterBar`, `LineItemsEditor`,
`TotalsPanel`, `DetailHeader`, `StatCards`), `83ff1e6` (layouts: `ListPage`, `FormPage`,
`DetailPage`, `SettingsPage`, extended `DataTable` with column types/totals/selection), `6df48b4`
(dev gallery entries, `docs/design_system.md` "Building pages" section,
`scripts/check-ui-rules.js` guard in **warning mode** — 121 findings across still-unmigrated
pages, expected). No existing page was migrated; `LineItemsEditor`/`TotalsPanel` intentionally
contain no totals/VAT math (render + emit only). Re-verified independently on final `master` HEAD:
build/check/verify:mocks all green.

**F-1 through F-6 are still fully pending** — migrate lists, then line-item forms (highest risk —
touches accounting math paths, read `docs/v2/02-accounting-review.md` first), then simple forms,
detail pages, settings pages, seam cleanup (including the 7 newly-surfaced violations above), then
flip the guard script to error mode. Each batch needs its own commit and a full e2e gate
(`python scripts/e2e/run.py`, not just touched flows).

**Status 2026-09-26 (later same day): F-1 through F-6 are PAUSED, do not start.** A concurrent
session (`desktop-app-6f`) is mid-way through plan 20 (`plans/pending/20-named-route-objects/`,
converting navigation to named route objects), touching effectively the same ~100+ page files and
the exact shared components F-1..F-6 need (`AppButton`, `PageHeader`, `KpiCard`, `DetailPage`,
`FormPage`, `ListPage`, `DetailHeader`, `StatCards`). Six agents were launched for F-1 through F-5
plus doc-18 D and F, collided with that refactor within ~2 minutes (one commit, `5a5197f`, briefly
had both plan-20 and F-1/F-3/C edits mixed together), and were stopped. The other session cleaned
up the collision in `edfb39f` (reverted 4 tangled files back to pure plan-20 state) — verify
`git log` shows `edfb39f` or later before assuming this is still current. **Do not relaunch any of
F-1 through F-6, or doc-18 Phase D, until plan 20 is finished and committed in a final green
state** — ask the other session or the user for confirmation first.

## Note on this session's parallel-agent run (2026-09-26)

Two background agents were launched in isolated worktrees for this work; both worktrees were
removed out from under the agents mid-task by the harness within ~60 seconds, causing two rounds
of false "I've launched a background agent to implement..." stub reports with zero real work
before the tasks were relaunched successfully (diagnostics without isolation, directly on this
checkout; Phase F-0 in a fresh worktree that succeeded). **Lesson: don't trust a background
agent's own completion report at face value — verify commits actually exist via `git log`/`git
diff --stat` before treating a task as done**, especially if it returns unusually fast with a
generic summary.

## Cross-session collision (2026-09-26, later same day) — resolved, but watch for a repeat

Launched 7 more agents at once (doc-18 C/D/F, doc-17 F-1..F-6) directly on `master`, no worktrees,
per the user's request for max parallelism. Within ~2 minutes, a concurrent Claude session on the
same machine (`desktop-app-6f`) reported it was mid-way through plan 20
(`plans/pending/20-named-route-objects/`) — a full-tree navigation refactor touching the same
page files and shared components several of my agents were rewriting. All 7 agents were stopped
immediately (none had committed; confirmed via `git log` unchanged). One commit (`5a5197f`) landed
with 4 files containing tangled edits from both efforts; the other session identified and reverted
exactly those 4 files, reapplying only its own edits, and committed the clean result as `edfb39f`.
No work was silently lost, but it required active cross-session coordination to catch — **if you
see multiple Claude sessions active on this repo, check with them before launching agents that
touch shared components or many page files at once**, since neither session has visibility into
the other's file scope by default.

## 2026-09-26 — doc-18 Phase F (accounting debugger): F1-F4 done, pre-existing seed bug found

Implemented on `master` (no worktree), building on the already-merged Phase B (diagnostics) and a
prior partial F2 pass that had left `src/mocks/backend/invariants.ts` + the slimmed
`scripts/verify/*.ts` files uncommitted — committed that as its own commit first, then F1
(posting trace), F3 (inspector in `/dev/diagnostics`'s "المحاسبة" tab — no separate `/dev/accounting`
route was needed, the tab stub already existed for this), F4 (repro bundles + `verify:replay`).
`bun run verify:mocks` stayed 49/0/0 at every checkpoint. F5 (CLAUDE.md rule) still to do.

**Finding, not fixed (out of scope for this pass):** `runAllInvariants()`'s
`checkAllocationsWithinTotal` (§4 "allocations ≤ document total") FAILS on a **fresh seed alone**,
before any replay or new action — `pay-166->inv-628 (48500 > 1000)`. `bun run verify:mocks` never
caught this because none of `scripts/verify/{accounts,inventory,parties,sales,branches}.ts` ever
wired up `checkAllocationsWithinTotal`, `checkDraftsIsolated` or `checkFxConversion` — three of the
14 functions in `invariants.ts` were defined (by whoever did the earlier uncommitted F2 pass) but
never called from the console harness, only from `runAllInvariants()` (which F3's inspector and F4's
replay now both call). `inv-628` looks like the seeded FX example invoice (grand total ~48,500 in
base currency after `rate`) being compared against a payment allocation check that may not be
converting FC "target total" the same way `customerBalance()`/`checkVatControl()` already do
(`invoiceVatBase()`'s `currency && exchangeRate` pattern) — needs someone who can read
`docs/v2/02-accounting-review.md`'s FX section before touching `checkAllocationsWithinTotal` or the
seed data. Reproduce: `seedDatabase()` then `runAllInvariants(db)` — no service call needed, it's
already broken at rest. Left as-is per the accounting-safety rule (don't touch invariant/posting
logic without reading the review doc first) and because fixing it wasn't in this pass's scope
(18.F1-F4, not a bug hunt) — logging here so it isn't lost and doesn't get silently "fixed" by
changing `runAllInvariants()` to skip it.
