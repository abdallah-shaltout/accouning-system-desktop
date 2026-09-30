# 21 · Part 04 — Integration, parity and cutover (the app runs on Rust)

> **Status (2026-09-29): planned.** Written after Part 03 was code complete (306 commands, fast gates
> green, DB tests written but not run). Part 02–03's DB test pass is running now, by the manager, as
> its own step. Part 04 does **not** wait for it: phases A, B, C and D start right away. Phase B2's
> fix loop and phase E need that pass green (see §6). The attachments domain (MariaDB blobs,
> migration m0017) is being added right now. It is part of the cutover set, so E-1 waits for it.
>
> **Deadline rule (user, 2026-09-29):** the deadline has passed. Every choice below picks the
> **fastest safe path** to a shippable desktop app that runs on the real Rust backend and gives
> correct accounting numbers. Real e2e, `bun run desktop`, installer and second-PC checks belong to
> the **final testing plan** (master §8). Part 04 never blocks on them, but it lists what that plan
> must prove (§8).

## 1. Goal

Prove, by machine, that Rust gives the **same DTOs and the same books** as the mock for every ported
service function. Then switch the desktop app to Rust in **one step** and audit the result. Pages and
components do not change. The browser build (`bun run dev`, the e2e suite) stays on the mock (D5, §3
P4-8).

## 2. Read first

1. [`../../../CLAUDE.md`](../../../CLAUDE.md): "Accounting safety", "Rust ↔ Vue", "Diagnostics", ARCHITECTURAL AUTONOMY.
2. [`00-MASTER-PLAN.md`](00-MASTER-PLAN.md): §3 rules, §5 switch, §8 DoD, §9 D5/D10.
3. [`03-DOMAINS-IMPLEMENTATION.md`](03-DOMAINS-IMPLEMENTATION.md): §3.3 (behaviour parity), §3.4 (switch lines), §3.7 (ownership).
4. Each lane's domain files in [`03-domains/`](03-domains/00-import.md): "Known mock quirks" and the **§8(b) parity case list**. Those lists are this part's case inventory. Don't re-derive them.
5. [`../../../docs/v2/02-accounting-review.md`](../../../docs/v2/02-accounting-review.md): read before any fix that touches posting (lanes L2–L4).
6. `AGENT_MEMORY.md`: the IPC table and the service API.

## 3. Decisions (architectural autonomy: strictest/fastest-safe option, logged)

| # | Decision | Reason |
|---|---|---|
| **P4-1** | **Cutover granularity: every `BackendDomain` flips together, behind one switch** (`RUST_DOMAINS` in `src/modules/core/services/backend.ts`), after every lane's parity is green. **This reverses master §1 ("no big-bang switch") and §5 ("cutover is per domain")**. E-7 updates that text. Verification stays incremental: each domain is parity-proven on its own (B2). Only the **user-facing switch** is atomic. | All domains share one database. A half-flipped app shows wrong numbers. Example: a sale posted to MariaDB (invoices on Rust) would be missing from a trial balance read from IndexedDB (reports on mock). The dependency closure of any posting domain covers almost everything: settings, users, parties, products, accounting, reports, and the dashboard that every home page reads (14b I-2: dashboard needs settings; 06b/07: products and purchases must flip together). Templates and diagnostics are the only separable parts, and flipping them alone gains nothing. The dev override `localStorage['equal.backend']` stays for per-domain debugging in dev only. |
| **P4-2** | **Parity runs through the real TS services** (scenario cases in `scripts/parity/cases/`), run twice: once with `usesRust() = false` (mock), once with `usesRust() = true` and a stdio transport to Rust. | One diff tests the switch-line argument mapping, serde (for example Decimal → JSON number), the Rust logic and the DTO shape together. Ids flow through return values, so case code never maps ids by hand. The existing `ReproBundle` cases still replay too (B-10). |
| **P4-3** | **The Rust side runs the real invoke handler on Tauri's `MockRuntime`** (`tauri::test`, cargo feature `parity`) in a new `src-tauri/src/bin/parity_host.rs`. There is no second dispatcher. | It uses the same deserialization, `require`, `with_tx` and error mapping as the app, and there is no 306-arm match to maintain. This needs one shared `invoke_handler::<R>()` in `lib.rs` and 12 commands made runtime-generic (B-1). |
| **P4-4** | **The clock is pinned on both sides.** Mock: a `Date` shim plus `process.env.TZ` = the base's country zone. Rust: `SET timestamp` before every command (`read_business_clock` reads `UTC_TIMESTAMP(3)`, `core/tx.rs:154`). The four direct `Utc::now()` reads in domain/shared business logic move to `cx.clock` (B-3). | Aging, due dates, "today" filters and insights depend on the clock (13b/14/14b already require "clock pinned to the seed's today"). A pinned clock also makes `createdAt` comparable instead of whitelisted. |
| **P4-5** | **Base state = the D10 importer applied to the same snapshot on both sides** (00-import D-1): per case, wipe and `import_snapshot` (debug `replace_existing`). Each lane uses its own database `equal_parity_<lane>` on the one `bun run db:dev` server. | This is already the plan's design, and it also exercises the importer that users will run. Lane-private databases let lanes run at the same time without collisions. Migrations run once per database. |
| **P4-6** | **Diff rules: exact by default.** Ids are mapped as a bijection (importer pairs + learned from results). Error parity is `{code, message}` byte for byte. An **allowlist entry must cite a decision or quirk id** from a domain file (for example `A-D1`, `12 #10`, `Q6`), or the runner rejects it. Epsilon `1e-9` applies only to the fields 13 R-7, 13b §8b and 14b §8b name. Unordered comparison applies only where Q6 or Q-I10 says so. | Nothing gets waved through silently. Every tolerated difference traces back to a written decision. |
| **P4-7** | **The known wrong-number bugs are fixed in both backends before cutover, and Part 04 only verifies them.** A parallel pass on 2026-09-29 already fixed them in the mock and in Rust, each with an `ACC-` issue at `status: fixed` and a `scripts/verify/cases/*.json` case: ACC-0003 (G-25, a refund over-refunds VAT), ACC-0004 (G-31a, landed-cost rounding remainder), ACC-0005 (G-31b, a non-stock receipt debits inventory), ACC-0006 (G-31c, a draft adjustment skips approval), ACC-0007 (12b D-A6, double VAT settlement → `CONFLICT`), ACC-0008 (12b D-A2, reopen mirror date). Lanes L2–L4 add one parity case per fix as task 0. When `verify:replay` and that parity case are green, the issue moves to `verified` (CLAUDE.md "Closing a ledger issue"). | The goal is "correct accounting numbers". Parity then compares fixed against fixed, and the ledger ends with every `ACC-` verified instead of just fixed. |
| **P4-8** | **D5 answered: keep the mock** for browser dev and browser e2e. `bun run parity` keeps it equal to Rust (it joins the DoD). Deleting it is a **later plan**, allowed only after the final testing plan's Tauri e2e suite is fully green (§8). | Browser e2e is the only full UI regression net that exists today. Deleting the mock before the Tauri e2e path has run the whole suite would remove it. |
| **P4-9** | **Legacy (browser/IndexedDB) data:** the Main PC imports it once through the existing `LegacyImportCard` → `setup_import_snapshot`. The importer only works on an empty DB (00-import step 1), so it cannot double-import. The IndexedDB snapshot is **never deleted** by the app; an `importedAt` marker hides the card afterwards. A terminal's own old snapshot is **not merged** (00-import D-3). It stays untouched on that PC. | Zero data loss. Merging two independent sets of books (separate numbering and chart of accounts) is not safe to automate. The only rollback after cutover is restoring a backup (P2-52 takes an automatic backup before migrations). There is no mock fallback (P2-34). |
| **P4-10** | **Tauri e2e path = WebView2 remote debugging (CDP) + Playwright `connect_over_cdp`**, reusing the 21 existing flows. | The fastest route: no tauri-driver or msedgedriver pinned to the fixed WebView2 runtime, and no rewrite of the flows. |
| **P4-11** | **Rust-side regression cases are parity cases**, not repro bundles. Repro recording refuses on Rust (16 D-5). A Rust accounting bug fix ships with a `scripts/parity/cases/…` case plus a `tests/domain_*.rs` test. `scripts/verify/cases/` stays for mock bugs. E-7 updates CLAUDE.md "Accounting safety" to say so. | This keeps CLAUDE.md's "every accounting fix ships a regression case" rule enforceable after the flip. |
| **P4-12** | Leftovers from earlier parts. 12b D-A6/D-A2 are fixed (ACC-0007/0008, P4-7). D-A3 (a purpose-built VAT-settlement reversal) and D-A7 (capping the VAT payment) keep the mock behaviour: both are scope questions, not wrong numbers, and the analysis recommended "no" for both. C-02: `INTERNAL` is adopted, since `backendCall` already emits it. C-16: resolved by the attachments domain (MariaDB blobs, m0017). | Recorded so no reader thinks these block the cutover. |
| **P4-13** | **Parity diff normalization rules (harness, Wave 2)** — written in `scripts/parity/diff.ts`'s header, each covered by `diff.selftest.ts`. **(a)** JSON `null` and a missing key are **equal** (never `[]`/`0`/`false`/`''` vs absent — those stay diffs, decided per field by the owning lane). **(b)** Derived ids translate through their parent's pair: `<parent>-l<n>` (invoice/quotation lines) and `<transfer>-recv` (= Rust `receipt_source_id`); a parent paired elsewhere is an `id-conflict`. **(c)** Object keys that are ids (`stockByBranch.<branch>`, `products.<product>`) translate through the same map; paths keep the mock key. **(d)** A Rust-only `createdAt` is allowed (still listed, reason 00-import D-8) only on an object whose mock `id` is a base-snapshot row that had no `createdAt`; this replaces the two broad `books.*.value[].createdAt` global allow entries. **(e)** The host imports with `adopt_terminal = its own terminal` (00-import D-5) and returns `terminalId`; the runner pairs the mock's `pos-1` with it (unless the importer already paired `pos-1`, i.e. several terminal strings). A mapped mock id that Rust shows verbatim stays an `id-conflict` (it no longer names a row). | (a) The TS types declare these fields `field?: T` / `T \| null` and every reader uses `?.`/`??`, so the two are indistinguishable to the UI, while serde `skip_serializing_if` vs the mock's explicit `null` made hundreds of false diffs. (b–e) The first full run (2026-09-30) showed these as harness artifacts, not behaviour: the ids/keys agreed once translated, the seed rows had no `createdAt` to compare, and the mock's `pos-1` is by construction "this machine". Each rule is row- or path-precise so a real disagreement still diffs. |

## 4. Phases

| File | What | Size | Lane (model) | Status |
|---|---|---|---|---|
| [`04-integration-and-audit/phase-a-contract-checks.md`](04-integration-and-audit/phase-a-contract-checks.md) | Static contract gates: teach `scripts/contract` that `backendCall` is IPC; a switch-line coverage rule; Decimal-serde and mock-read rules; review of the 19 `contract-ok` exceptions; a sync-reader inventory | M | A (Sonnet) | pending |
| [`04-integration-and-audit/phase-b-parity-harness.md`](04-integration-and-audit/phase-b-parity-harness.md) | The harness: `invoke_handler::<R>()`, `parity_host` bin, clock pin, importer id pairs; TS runner, transport hook, case API, diff, books snapshot, bundle replay, `bun run parity` | L | H-RS (Opus) + H-TS (Opus) | pending |
| [`04-integration-and-audit/phase-b2-parity-cases.md`](04-integration-and-audit/phase-b2-parity-cases.md) | ~160 parity cases from the 03-domains §8(b) lists, in 5 domain lanes, each with its own fix loop; parity cases that verify ACC-0003 to ACC-0008 (P4-7) | XL | L1 (Sonnet), L2–L4 (Opus), L5 (Sonnet) | pending |
| [`04-integration-and-audit/phase-c-rust-invariants.md`](04-integration-and-audit/phase-c-rust-invariants.md) | `TestDb::finish()` runs `shared::invariants::run_all` on every test DB; a `test_db_rules` guard so no test skips it; cross-checks that parity runs the invariants after every step | S | C (Sonnet) | pending |
| [`04-integration-and-audit/phase-d-tauri-e2e-path.md`](04-integration-and-audit/phase-d-tauri-e2e-path.md) | `run.py --target tauri` over CDP, `open_page` in 21 flows, per-flow DB reset, e2e save-dir override, `bun run desktop:e2e`, per-target perf baseline. Built and statically checked here; **run in the final testing plan** | M | D (Sonnet) | pending |
| [`04-integration-and-audit/phase-e-cutover-and-audit.md`](04-integration-and-audit/phase-e-cutover-and-audit.md) | The single flip, Rust-mode boot path, mock boot off in Rust mode, legacy-import marker, CLAUDE.md/master updates, the final accounting audit | M | E (Opus) + audit (Opus, read-only) | pending |

## 5. Parallel-lane map

```text
NOW ──────────────────────────────────────────────────────────────────────────────────────────►
Manager:   [Parts 02–03 DB test pass (running)] ─────────────► [host build rounds] ─► [final gates]
Wave 0:    A (Sonnet) · C (Sonnet) · D (Sonnet) · H-RS (Opus) · H-TS (Opus)      ← all disjoint files
                                          H-TS lands case.ts API first (B-6) ──┐
Wave 1:                                    L1 · L2 · L3 · L4 · L5 author cases, run --mock-only
Wave 2:        (host built + Parts 02–03 pass green) ► L1–L5 run on Rust, fix loop, until 0 diffs
Wave 3:                                                                 E (flip + boot path) ► audit
```

- **Wave 0 lanes are fully parallel.** Their file sets are disjoint (ownership table below).
- **Wave 1 starts as soon as `scripts/parity/case.ts` exists** (B-6). Case authoring needs no Rust:
  `bun run parity --mock-only` checks a case against the mock alone (it runs, it is deterministic
  twice in a row, and the invariants hold).
- **Wave 2 needs** the parity host built by the manager (B-4) and the manager's Parts 02–03 test pass
  green enough that `domain_<d>` suites for the lane's domains pass. A lane whose domains fail there
  fixes those tests first, as part of its own fix loop.
- **Wave 3** starts when every lane gate is green, phase A and phase C gates are green, and the
  attachments domain has landed.

### File ownership (disjoint; the manager merges between rounds)

| Owner | Files |
|---|---|
| Manager | `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs` (after B-1 lands), `core/**`, `shared/**`, `entities/**`, `migration/**`, `domains/mod.rs`, `CLAUDE.md`, plan entry files, `AGENT_MEMORY.md`, host build rounds, every `cargo test` run |
| A | `scripts/contract/**`, `docs/backend/contract/**` (generated), `src-tauri/tests/architecture_rules.rs`, fixes to the stay-frontend readers A-4 finds (the listed service files only) |
| H-RS | `src-tauri/src/bin/parity_host.rs`, the B-1 signature edits (`core/diag/mod.rs`, `infrastructure/print/commands.rs`, `domains/settings/commands.rs` + `service/network.rs`, `domains/setup/commands.rs` + `service/device.rs`), the `lib.rs` handler extraction, `infrastructure/import/run.rs` (`ImportReport.id_pairs`), the B-3 clock sites. The manager merges `Cargo.toml` from H-RS's request |
| H-TS | `scripts/parity/*.ts` (runner core), the parity hook in `src/modules/core/services/backend.ts`, `package.json` `parity` script. **After Wave 0 the runner core becomes manager-owned.** Lanes request changes |
| C | `src-tauri/tests/support/mod.rs`, the new `src-tauri/tests/test_db_rules.rs`, and the `.finish().await` edits in every `tests/{domain,shared,db}_*.rs`. C runs in Wave 0 and **hands `tests/domain_<d>.rs` over to the B2 lanes before Wave 2**, so the two never edit the same test file at once |
| D | `scripts/e2e/**`, `scripts/desktop-e2e.js`, `package.json` `desktop:e2e` script, the e2e save-dir branch in `src/modules/core/services/saveFile.ts`, `docs/diagnostics/perf-baseline.json` schema |
| L1–L5 | `scripts/parity/cases/<their domains>/**`; their domains' `src-tauri/src/domains/<d>/**` and `tests/domain_<d>.rs`; their modules' `services/*.ts` switch lines; their mock files in `src/mocks/backend/*` **only for a P4-7 fix or a logged mock bug**. Shared/core/entity/migration changes are requests to the manager (03 §3.7 unchanged) |
| E | `backend.ts` flip, `src/router/index.ts`, `src/main.ts`, `src/mocks/persist.ts` boot gate, `setup/components/LegacyImportCard.vue` marker, master/README/CLAUDE.md text (with the manager) |

`package.json` is shared by H-TS (`parity`) and D (`desktop:e2e`). Each lane adds only its own line, and the
manager merges them.

### Cargo and long runs (binding)

- Agents run **only** `cargo check`, and only through the machine-wide lock:
  `powershell -NoProfile -File scripts/cargo-safe.ps1 <log> check --manifest-path src-tauri/Cargo.toml --workspace --all-targets`.
  To check the host: add `--bin parity_host --features parity`.
- **Host builds are manager-batched rounds:**
  `scripts/cargo-safe.ps1 parity-host build --manifest-path src-tauri/Cargo.toml --bin parity_host --features parity`.
  The manager copies the built exe to `.diagnostics/parity/bin/parity_host-<stamp>.exe`, because a
  running exe blocks the next link (the Part 02 `db_dev_server` lesson). Lanes point `--host` at the copy.
- Every `cargo test` run is manager-run, one filter at a time, on the single `all` binary (`tests/all/main.rs`).
- **Parity runs are cheap** (bun plus a prebuilt exe), so any lane may run them. A case that runs
  longer than 60 s is killed and recorded, and work continues (memory `feedback_dont-block-on-long-runs`).

## 6. Definition of done (Part 04)

- [ ] Phase gates A, B, B2 (all 5 lanes), C, D (static) and E are green, and each phase file has a status note at the top.
- [ ] `bun run parity` (every case plus `--bundles`) reports **0 unexplained diffs**, the allowlist has been
      reviewed in the E-6 audit, and the Rust invariants are green after every step of every case.
- [ ] The manager's `cargo test --manifest-path src-tauri/Cargo.toml --test all` is green
      (`EQUAL_TEST_DATABASE_URL=mysql://root:equal-dev@127.0.0.1:3499`), including `architecture_rules`
      and every `TestDb::finish()` invariant check. `bun run bindings:check` is green.
- [ ] `bun run build`, `check`, `verify:mocks`, `verify:replay`, `contract:check`, `memory:check`,
      `diag:check` are green. Every `ACC-` issue is `verified` (ACC-0001 to ACC-0008 and any new one).
- [ ] `RUST_DOMAINS` = every `BackendDomain`. The browser build is unchanged (`usesRust` is false outside Tauri).
- [ ] The master plan (§1, §5, §6 row 04, §9 D5 → answered), README and CLAUDE.md are updated (E-7).
- [ ] The §8 list is handed to the final testing plan. **Plan 21 stays in `plans/pending/`** until that plan
      has run it (CLAUDE.md "Move on completion": the real desktop check is not done in this part).

## 7. Open question for the user (one; its default is already applied)

1. **Does any beta customer have separate books on more than one PC?** The default (P4-9) imports only the
   Main PC's data, and other PCs' old data stays on those PCs untouched. If the answer is yes, we need a
   small "choose which PC's data to import" step. That is a scope change.

## 8. Handed to the final testing plan (Part 04 does not run these)

- **Required before the mock may be deleted (D5):** the full 21-flow suite green on
  `python scripts/e2e/run.py --target tauri` (phase D) against a Rust-flipped debug build, with no
  console errors. Then a separate small plan moves `ApiError` and the event bus out of `src/mocks`
  (the `backend.ts` header note) and deletes the mock.
- A real `bun run desktop` pass on Windows after the flip: first run as Main PC (provision, legacy import
  of a real IndexedDB snapshot, invariants panel green), a login, a sale, a refund, and a report.
- `bun tauri build` installer, clean-VM install/uninstall (P2-44 data kept), second-PC pairing as a terminal
  and a cross-terminal refresh (the change poller).
- Part 02 DoD items still open (real desktop boot states, installer size, clean-PC run).

## 9. Later (not in Part 04)

Deleting the mock and moving `ApiError`/events (after §8). A "choose which PC to import" flow (only if §7 Q1
says yes). The 12b questions D-A3/D-A7. A parity run in CI. Keyset paging and summary tables (only
when a measurement asks for them). The undo UI (D4). The sync worker (D6).
