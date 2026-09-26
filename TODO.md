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

## Note on this session's parallel-agent run (2026-09-26)

Two background agents were launched in isolated worktrees for this work; both worktrees were
removed out from under the agents mid-task by the harness within ~60 seconds, causing two rounds
of false "I've launched a background agent to implement..." stub reports with zero real work
before the tasks were relaunched successfully (diagnostics without isolation, directly on this
checkout; Phase F-0 in a fresh worktree that succeeded). **Lesson: don't trust a background
agent's own completion report at face value — verify commits actually exist via `git log`/`git
diff --stat` before treating a task as done**, especially if it returns unusually fast with a
generic summary.
