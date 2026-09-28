# 21 · 03.16 — `diagnostics` (audit-log reads, support-bundle data, debug-build accounting debugger)

> **Status (2026-09-28, updated):** **Slices A, A2 and B all code-complete, not yet compiled/tested
> by this implementer** (hard rule: implementers write code, never run cargo — one throttled
> manager build+test at the end). Slice A (W1) unchanged from the note below. Slice A2 (this pass,
> after 17-backup landed): `service/support.rs`'s `support_snapshot` now calls
> `infrastructure::backup::dataset::dump_snapshot` (H-3), drops the `credentials` table
> (`SUPPORT_BUNDLE_EXCLUDED_TABLES`, D-3), and `redact`s the result — the old `VALIDATION` refusal is
> gone. Slice B (this pass, after 12-accounting landed): `service/debugger.rs` (7 functions),
> `dto.rs`'s `AccountingDocSummary`/`BalanceAround`/`DriftRow(Kind)`/`ExplainLine` plus the
> `InvariantResultDto`/`PostingTraceDto` mirror DTOs (`From<&shared::invariants::InvariantResult>` /
> `From<&shared::ledger::trace::PostingTrace>`), and 7 new `#[tauri::command]`s in `commands.rs`,
> each opening with a runtime `debug_only()` gate (D-4: `if !cfg!(debug_assertions) { return
> Err(FORBIDDEN) }`) before `ctx.require(Accounting, Read)` inside its `with_read_ctx` snapshot —
> **not** `#[cfg(debug_assertions)]` on the function, since `ipc_manifest_matches_handler`
> (`core/ipc.rs`) parses `lib.rs`'s `generate_handler!` list as plain text against `all_signatures()`
> with no build-profile awareness, so a `#[cfg]`-hidden command in a release build would break that
> check. All 10 commands (3 + 7) are therefore always registered; only their bodies branch by build
> profile. Frontend switch lines added to `accountingDebugService.ts` (6 reads +
> `startReproRecording`'s D-5 refusal); `contract.check.ts` gained the slice-B drift-check entries.
> **Not done by this implementer** (manager-owned files, out of edit scope — see "Needs from
> manager" in the handoff note): registering the 7 new commands in `lib.rs`'s `generate_handler!`,
> and adding 11 new `export_all` lines to `domains/mod.rs`'s `export_bindings` (that file already
> inlines this domain's slice-A DTO exports rather than calling a per-domain hook, so slice B follows
> the same inline convention — see `domains/diagnostics/mod.rs`'s own NOTE comment for the exact
> list). DB tests for A2/B added to `tests/domain_diagnostics.rs` (fixture copied from
> `domain_accounting.rs`'s `seed_role_account`/`seed_fixture`, per lessons.md) — not run.
>
> **Previous note (W1, slice A only):** `domains/diagnostics/{mod,dto,commands}.rs` +
> `service/{mod,audit,support}.rs` and `tests/domain_diagnostics.rs` written for slice A's 3 commands
> (`diagnostics_get_audit_entries`, `diagnostics_get_audit_entities`,
> `diagnostics_export_support_bundle`); frontend switch lines in
> `auditService.ts`/`supportBundleService.ts`, `SupportSnapshot`/`ServerDiagnostics` added to
> `types/index.ts`, and `contract.check.ts` entries added for `AuditFilter`/`ServerDiagnostics`/
> `SupportSnapshot`. The two support-bundle tests noted as deferred there
> (`any_signed_in_user_gets_a_support_snapshot_without_db` and its DB-snapshot sibling) are now
> written (this pass) using a `seed_branch_and_settings` fixture copied from `domain_settings.rs`.
>
> Depends on: 01-settings (the settings DTO), 03-users (session), 17-backup (`infrastructure::backup::dataset`),
> 12-accounting (`domains::accounting::{dto,service::rows}`), Part 02
> `core/dto.rs` (`AuditEntry`), `infrastructure::database::errors::diagnostics_snapshot`,
> `shared::invariants::run_all`, `AppState.traces` (`shared::ledger::trace::TraceRing`), gaps G-6/G-8/G-9.

**Goal.** Serve the 3 production `port` functions (Part 02 C-10: `getAuditEntries`,
`getAuditEntities`, `exportSupportBundle`) from `domains/diagnostics/`, with the support bundle's
data coming from the real DB and the Main PC's server snapshot, **no secrets**. Then give the
`/dev/diagnostics` → المحاسبة tab its 7 debug-build-only reads (F9) over `shared::invariants::run_all`
and the posting-trace ring, so the debugger shows the real DB instead of the mock.

**Read first.** [`../01-frontend-analysis/diagnostics.md`](../01-frontend-analysis/diagnostics.md) (§1
dispositions, §7 aggregations, §8 dev-only overrides, §9 repro redesign) · services
`src/modules/diagnostics/services/auditService.ts:10-48`, `supportBundleService.ts:64-127`,
`accountingDebugService.ts:24-249` · `src/modules/diagnostics/config.ts:32-43` (`REDACTED_KEYS`) ·
callers `settings/pages/AuditLogSettingsPage.vue:46,59` (route area `users`),
`settings/pages/AboutSettingsPage.vue:26` (no area — any signed-in user), `components/dev/DevAuditTab.vue:15`
· Rust `core/dto.rs` (`AuditEntry`, `AuditFieldDiff`, `AuditAction`), `entities/platform/audit.rs`,
`infrastructure/database/errors.rs:108-135`, `shared/invariants/mod.rs:95`, `shared/ledger/trace.rs:103-290`.

## 1. Commands

| Mock fn | Disposition | Rust command | Args → Return | Area / Access | Tx | Slice |
|---|---|---|---|---|---|---|
| `getAuditEntries` | port | `diagnostics_get_audit_entries` | `{ filter?: AuditFilter }` → `Vec<AuditEntry>` | Users:Read | `with_read` | A |
| `getAuditEntities` (+ alias `getAuditEntityKinds`) | port | `diagnostics_get_audit_entities` | `()` → `Vec<String>` | Users:Read | `with_read` | A |
| `exportSupportBundle` | port (data part only) | `diagnostics_export_support_bundle` | `{ includeDbSnapshot?: bool }` → `SupportSnapshot` | session; `includeDbSnapshot` needs Settings:Write (D-2) | `with_read` | A (+A2) |
| `listRecentDocuments` | dev-only | `diagnostics_list_recent_documents` | `{ limit?: u32 }` → `Vec<AccountingDocSummary>` | debug build + Accounting:Read | `with_read` | B |
| `getPostingTrace` | dev-only | `diagnostics_get_posting_trace` | `{ entryId }` → `Option<PostingTrace>` | debug + Accounting:Read | none (ring) | B |
| `getJournalEntryRaw` | dev-only | `diagnostics_get_journal_entry_raw` | `{ id }` → `Option<JournalEntry>` | debug + Accounting:Read | `with_read` | B |
| `getBalancesAround` | dev-only | `diagnostics_get_balances_around` | `{ entryId }` → `Vec<BalanceAround>` | debug + Accounting:Read | `with_read` | B |
| `getInvariantResults` | dev-only | `diagnostics_get_invariant_results` | `()` → `Vec<InvariantResult>` | debug + Accounting:Read | `with_read` (RR snapshot) | B |
| `getDriftReport` | dev-only | `diagnostics_get_drift_report` | `()` → `Vec<DriftRow>` | debug + Accounting:Read | `with_read` | B |
| `explainAccountBalance` | dev-only | `diagnostics_explain_account_balance` | `{ accountId, partyId? }` → `Vec<ExplainLine>` | debug + Accounting:Read | `with_read` | B |
| `startReproRecording` | dev-only, **not ported** (diagnostics.md §9 → Part 04) | — | switch line refuses (D-5) | — | — | — |
| `stopReproRecording`, `isReproRecording`, `exportReproBundle` | frontend | — | untouched | — | — | — |

3 production + 7 debug commands = 10. No writes, no events, no audit rows (diagnostics.md §4/§6).
Debug commands are always registered (so `ipc_manifest_matches_handler` stays one list) and start with
`if !cfg!(debug_assertions) { return Err(AppError::forbidden("أداة التشخيص المحاسبي متاحة في نسخة التطوير فقط").into()) }` (D-4).

## 2. DTOs (`domains/diagnostics/dto.rs`, `#[ts(export_to = "diagnostics/types/gen/")]`)

| Rust DTO | TS type | Notes |
|---|---|---|
| `AuditEntry`, `AuditFieldDiff`, `AuditAction` | `types/index.ts:69-95` | **Reused** from `core/dto.rs` (already exported + checked in `diagnostics/types/contract.check.ts`). `fn audit_to_dto(m: &audit::Model) -> AuditEntry`: `at = m.at().key()` (DocDate key), `before`/`after` = `serde_json::from_value::<Vec<AuditFieldDiff>>` (malformed → `None`), `link` from `RouteRefValue`, entity enum → `core::dto::AuditAction`. Fix the `link` ts path (G-8c). |
| `AuditFilter` | `auditService.ts:10-17` | `user_id: Option<String>`, `entity`, `action: Option<AuditAction>`, `from`, `to`, `search` (strings, compared as the mock does). |
| `SupportSnapshot` | **new** `SupportSnapshot` in `types/index.ts` (task, the TS type is the contract) | `{ settingsRedacted: unknown; dbSnapshot?: unknown; server?: ServerDiagnostics }` → `settings_redacted: serde_json::Value` `#[ts(type="unknown")]`, `db_snapshot: Option<Value>` `#[ts(optional, type="unknown")]`, `server: Option<ServerDiagnosticsDto>`. |
| `ServerDiagnosticsDto` | **new** `ServerDiagnostics` in `types/index.ts` | camelCase mirror of `infrastructure::database::errors::ServerDiagnostics` (`state`, `version?`, `port?`, `lanSharing`, `lastFailure?`, `errorLogTail?`). |
| `AccountingDocSummary`, `DriftRow`, `ExplainLine` | `accountingDebugService.ts:24-34,106-115,205-213` | money `serde_number`; optional fields `#[ts(optional)]`. |
| `BalanceAround` | inline return of `getBalancesAround` (`:87-97`) | `{ accountId, accountName, accountCode?, before, after }`. |
| `InvariantResultDto` (`#[ts(rename = "InvariantResult")]`), `PostingTraceDto` (`rename = "PostingTrace"`) + its step/line/totals/vat/cost/fx mirrors | `@/mocks` types (`invariants.ts`, `posting-trace.ts:27-36`) | Needed because `shared::invariants::InvariantResult`/`shared::ledger::trace::PostingTrace` have no `TS` derive and serialise `Decimal` as **strings** (`rust_decimal` `serde-with-str`); the DTOs use `serde_number` and are built by `From<&…>`. |
| `JournalEntry` | accounting's DTO | **Reused** from `domains::accounting::dto` (12-accounting, H-2) — not redeclared. |

`src/modules/diagnostics/types/contract.check.ts` gains: `SupportSnapshot`, `ServerDiagnostics`,
`AuditFilter` (type import from `../services/auditService`), `AccountingDocSummary`/`DriftRow`/
`ExplainLine` (from `../services/accountingDebugService`), `BalanceAround` as
`Awaited<ReturnType<typeof getBalancesAround>>[number]`, `InvariantResult` as
`Awaited<ReturnType<typeof getInvariantResults>>[number]`, `PostingTrace` as
`NonNullable<Awaited<ReturnType<typeof getPostingTrace>>>` (no direct `@/mocks` import from a types file).
`Option<T>` returns (`getPostingTrace`, `getJournalEntryRaw`) are `T | null` vs the TS `T | undefined`:
switch lines add `?? undefined` (`// contract-ok: null→undefined at the switch line`).

## 3. Service logic (`domains/diagnostics/service/{audit,support,debugger}.rs`)

**`get_audit_entries(conn, filter)`** (`auditService.ts:19-38`):
1. SQL narrows by exact `user_id`, `entity`, `action` when set; `ORDER BY created_at, id` (insertion order).
2. Rust filters, exactly as `matches` (`:19-32`): `from`: `at_key[..10] < from` → drop; `to`: `at_key[..10] > to`
   → drop (the **UTC** slice of the ISO key, not the business day — Q-1); `search`: `q = search.trim().to_lowercase()`,
   non-empty and none of `message`, `entity_id` (text), `entity_label` (or `""`) `.to_lowercase().contains(q)` → drop.
3. Stable sort by `at` key descending (`b.at.localeCompare(a.at)`). Map with `audit_to_dto`.

**`get_audit_entities(conn)`** (`:42-45`): `SELECT DISTINCT entity`, then sort **in Rust** by code
point (`[...new Set()].sort()` is a UTF-16 code-unit sort; SQL `ORDER BY` under `utf8mb4_unicode_ci`
would case-fold). No hard-coded list (diagnostics.md §1).

**`support_snapshot(state, conn, actor, include_db)`** (`supportBundleService.ts:103-127`, data part):
1. `include_db == Some(true)` → `require(conn, actor, Area::Settings, Access::Write)` (D-2).
2. `settings_redacted = redact(serde_json::to_value(<01-settings get_settings DTO>))` — the same
   camelCase JSON the mock's `clone(db.settings)` produces (H-1).
3. `redact(value, depth)` ports `:67-78`: depth > 8 or null → as is; arrays map; objects replace the
   value of any key whose lowercase is in `REDACTED_KEYS` with `[محجوب]`. `REDACTED_KEYS` is a Rust
   `const` with a drift test that parses `diagnostics/config.ts:32-43` (the `auth.rs` `permissions.ts`
   pattern) — one list, checked.
4. `server`: `#[cfg(windows)]` and `device.role == Main` → `Some(diagnostics_snapshot(&state.server.paths))`
   mapped to `ServerDiagnosticsDto`; otherwise `None` (a terminal has no local server; P2 handoff §9 "no secrets" —
   `server.json` + error-log tail hold no credentials; keyring secrets are never read).
5. **Slice A2:** `db_snapshot` = the table exporter 17-backup builds for P2-52 (reads every table
   through SQL in the same `with_read` snapshot), **excluding `credentials`** (argon2 hashes are still
   secrets, D-3), then `redact`ed. Until slice A2 lands, the command refuses `includeDbSnapshot: true`
   with `VALIDATION` `لقطة قاعدة البيانات غير متاحة بعد` — removed by A2 (this is sequencing, not
   placeholder logic: the checkbox is opt-in and the rest of the bundle works).

The frontend keeps collecting logs, `meta.json`, zipping and `saveFile` (diagnostics.md §1).

**Slice B — debugger reads** (all over one `with_read` snapshot; mock lines cited):
- `list_recent_documents(limit = 100)` (`:38-55`): journal entries → Rust stable sort by date key desc,
  then `number` desc (string compares); take `limit`; `has_trace = state.traces.for_entry(id).is_some()`;
  `type`, totals, `source_kind` from the entry.
- `get_posting_trace(entry_id)` (`:61-64`): `state.traces.for_entry(entry_id)` → `PostingTraceDto` (no DB).
- `get_journal_entry_raw(id)` (`:66-70`): posted entry DTO, else draft DTO, via 12-accounting's builders (H-2); else `None`.
- `get_balances_around(entry_id)` (`:76-99`): missing → `[]`; accounts touched (first-seen order of the
  entry's lines); `before` = entries with `date < d || (date == d && number < n)`, `after` = `<=` —
  string comparisons on date keys and numbers, exactly as the mock; per account `round2(Σ debit − credit)`;
  `accountName` = account name or the id, `accountCode` optional.
- `get_invariant_results()` (`:101-104`): `shared::invariants::run_all(tx)` → DTOs (the one copy of
  the 14 invariants; CLAUDE.md "Accounting safety").
- `get_drift_report()` (`:124-201`): display-only port with `Decimal` — customers (`ORDER BY created_at, id`):
  `outstanding = round2(Σ non-DRAFT invoices (grand_total − Σ refunds.settled_to_receivable − paid_amount))`,
  `credit = unallocated_credit_for`, `subledger = round2(outstanding − credit)`, `gl = customer_balance`,
  `diff = round2(subledger − gl)`, row only when `|diff| > 0.01`, `first_diverging` by walking entries
  sorted (date key, number) with `running = round2(running + delta)`; suppliers the same with RECEIVED
  POs and returns `refund_method == credit ? grand_total : settled_to_payable` (same columns as
  `shared/invariants/parties.rs:75,110`), `sign −1`, compared against `−gl`; products: inventory-role
  GL `round2(Σ)` vs `round2(Σ products.stock_value of type product)`, rows per product only when they
  differ by > 0.01 (`subledger = stock_value`, `gl = invGl`, `diff = round2(invSum − invGl)`). Pass/fail
  is **never** decided here — that stays in `run_all` (diagnostics.md §1).
- `explain_account_balance(account_id, party_id)` (`:215-237`): lines on the account (and party when
  given) joined to entries, base order `je.created_at, je.id, jl.position`; row `{ docId, docNumber,
  docDate: key, sourceKind, description: line ?? entry, debit, credit }`; stable sort by `docDate` desc.

## 4. Concurrency (D8)

Read-only. `with_read` gives one REPEATABLE READ snapshot, so the bundle's settings and DB snapshot
are mutually consistent and the invariants see one state (P2-06). The trace ring is per process: a
terminal shows only traces of postings it committed (diagnostics.md §5, P2-35).

## 5. Undo

None — no writes.

## 6. Frontend switch lines

`src/modules/diagnostics/services/auditService.ts`:
- `getAuditEntries`: `if (usesRust('diagnostics')) return backendCall('diagnostics_get_audit_entries', { filter });`
- `getAuditEntities`: `… return backendCall('diagnostics_get_audit_entities');` (`getAuditEntityKinds` is the same function object — no line).

`src/modules/diagnostics/services/supportBundleService.ts` (`exportSupportBundle`, D-1 — the one
function whose Rust part is data only):
```ts
const rust = usesRust('diagnostics')
  ? await backendCall('diagnostics_export_support_bundle', { includeDbSnapshot: options.includeDbSnapshot })
  : null;
// 'settings.redacted.json' ← rust ? rust.settingsRedacted : redact(clone(db.settings))
// 'server-diagnostics.json' ← only when rust?.server
// 'db-snapshot.json'        ← options.includeDbSnapshot ? (rust ? rust.dbSnapshot : clone(db)) : absent
```

`src/modules/diagnostics/services/accountingDebugService.ts` (slice B): `listRecentDocuments`
`{ limit }`, `getBalancesAround` `{ entryId }`, `getInvariantResults`, `getDriftReport`,
`explainAccountBalance` `{ accountId, partyId }` return `backendCall(...)`; `getPostingTrace`
`{ entryId }` and `getJournalEntryRaw` `{ id }` return `(await backendCall(...)) ?? undefined`;
`startReproRecording`: `if (usesRust('diagnostics')) throw new ApiError('تسجيل إعادة الإنتاج غير متاح مع قاعدة البيانات الحقيقية بعد', 'FORBIDDEN');` (D-5; `ApiError` from `@/mocks` like the file's other imports).

## 7. Known mock quirks (kept) and decisions

**Quirks kept:** Q-1 audit date filters use the UTC `at.slice(0,10)`, not the business day. Q-2
`getAuditEntries` returns the whole filtered log (no paging). Q-3 the drift product rows repeat the
aggregate diff on every product. Q-4 debugger reads return `[]`/`undefined` for unknown ids, never `NOT_FOUND` (diagnostics.md §3).

**Decisions:**
- D-1 `exportSupportBundle` keeps zip/logs/save in the frontend; Rust returns only the settings, server and DB parts (diagnostics.md §1). This is the documented exception to entry §3.4's one-line switch.
- D-2 A DB snapshot holds all business data, so it needs Settings:Write (the backup bar, `roleCanRestoreBackup`); the rest of the bundle stays open to any signed-in user as today.
- D-3 The DB snapshot never contains `credentials` (the mock's `clone(db)` even includes plaintext passwords — a mock-only leak not carried over) and is `redact`ed with the same key list.
- D-4 Debug commands are registered in every build but refuse in release (`cfg!(debug_assertions)`), so the handler list and `ipc_sig!` manifest stay one list (F9).
- D-5 Repro recording is not ported: the mock's "clone the whole DB then record calls" has no Rust equivalent yet (diagnostics.md §9 → Part 04). On Rust it refuses instead of recording mock data.
- D-6 Slices: A in W1 (audit + settings + server), A2 after 17-backup (W2), B after 12-accounting (W5, needs its journal DTO builders).

**Part 02 gaps this file needs:** G-6 and G-8 (incl. G-8c — the `AuditEntry.link` binding path) as
defined in [`03-users.md`](03-users.md) §7; G-9 (`audit.at_instant` is `TIMESTAMP(0)`, so audit `at`
keys lose milliseconds and same-second ordering) as defined in [`04-approvals.md`](04-approvals.md) §7.

**Handoffs:** H-1 01-settings exposes its `get_settings` service fn (the `settings_get_settings` DTO) for step 2. H-2 12-accounting exposes `journal_entry_dto(conn, id)` / `journal_draft_dto(conn, id)`. H-3 17-backup exposes its per-table JSON exporter with a table-exclusion list (used here with `["credentials"]`).

## 8. Tests

**(a) `src-tauri/tests/domain_diagnostics.rs`:**
- audit: rows written by `shared::activity::record` come back newest first; `userId`/`entity`/`action` filters; `from`/`to` compare the UTC slice (an entry at `…T22:30Z` on a UTC+3 business day is filtered by its UTC date); search matches `message`, `entityId`, `entityLabel` case-insensitively.
- entities: distinct, code-point sorted (`Invoice` before `branch`).
- access: cashier `diagnostics_get_audit_entries` → `FORBIDDEN`; any signed-in user gets a support snapshot without DB; cashier `includeDbSnapshot` → `FORBIDDEN`.
- redaction: a nested settings key `apiKey`/`PIN` becomes `[محجوب]`; the drift test against `config.ts` passes.
- A2: DB snapshot has no `credentials` table and no `password_hash` string anywhere.
- B (debug build): release-mode refusal message; `get_invariant_results` equals `run_all`; `get_posting_trace` returns a trace pushed by a committed posting and `None` for a rolled-back one; balances before/after on same-day entries follow `number`; explain rows newest first.

**(b) Parity cases:** `diagnostics-audit-filter`, `diagnostics-audit-entities`, `diagnostics-invariants`,
`diagnostics-balances-around`, `diagnostics-drift-clean-db` (all rows empty on a clean replay).

## 9. Checklist

**Slice A (W1)**
- [x] Confirm G-6, G-8 (incl. G-8c `link` path fix) and G-9 are in place. (Confirmed fixed per `_part02-gaps.md`.)
- [x] `domains/diagnostics/{mod,dto,commands}.rs` + `service/{mod,audit,support}.rs`; `ipc_signatures()` + `export_bindings(cfg)`. (`export_bindings(cfg)` hook itself is `domains/mod.rs`, manager-owned — this implementer's DTOs need one `export_all` line added there; see "Needs from manager" in the final report.)
- [x] `audit_to_dto`, `get_audit_entries`, `get_audit_entities`, `redact` + `REDACTED_KEYS` drift test, `support_snapshot` (settings + server; DB refusal until A2).
- [x] Add `SupportSnapshot` and `ServerDiagnostics` to `src/modules/diagnostics/types/index.ts`; contract checks (§2).
- [x] Switch lines in `auditService.ts` and `supportBundleService.ts` (§6).
- [ ] Manager: register the 3 commands + `pub mod diagnostics;`.

**Slice A2 (after 17-backup, W2)**
- [x] Call 17-backup's exporter excluding `credentials`, `redact` it; remove the refusal; add the A2 tests. (`service/support.rs`'s `support_db_snapshot` calls `infrastructure::backup::dataset::dump_snapshot`, drops `credentials` via `SUPPORT_BUNDLE_EXCLUDED_TABLES`, then `redact`s. Not yet compiled — see status note.)

**Slice B (after 12-accounting, W5)**
- [x] `service/debugger.rs`: the 7 reads (§3), debug gate + Accounting:Read in each command; `PostingTraceDto`/`InvariantResultDto` mirrors with `serde_number`. (Written; not yet compiled — see status note.)
- [x] Switch lines in `accountingDebugService.ts` (incl. the `startReproRecording` refusal); contract checks; slice B tests.
- [ ] Manager: register the 7 commands in `lib.rs`'s `generate_handler!` and add the 11 `export_all` lines to `domains/mod.rs` (exact lines in this implementer's final report / `domains/diagnostics/mod.rs`'s NOTE comment).

- [x] `tests/domain_diagnostics.rs` (§8a — slice A, A2 and B all written, not run, incl. the two
  support-bundle tests slice A's note deferred: `any_signed_in_user_gets_a_support_snapshot_without_db`
  and `admin_include_db_snapshot_has_no_credentials_table_and_no_password_hash_anywhere`). ⏳ deferred
  time-boxed test pass for the actual DB run (needs the manager's `cargo check` first — these tests
  were written blind against code this implementer could not compile).
- [x] Parity list to Part 04: `diagnostics-audit-filter`, `diagnostics-audit-entities`,
  `diagnostics-invariants`, `diagnostics-balances-around`, `diagnostics-drift-clean-db` (all 5 of
  §8b's cases; the last 3 are now in scope since slice B is written).

## Gate

Per slice: `cargo check` clean (manager run) · tests written · switch lines · `contract.check.ts`
compiles · `memory:check` 0 contract gaps for the slice's commands. DB tests/parity in the deferred
time-boxed pass.
