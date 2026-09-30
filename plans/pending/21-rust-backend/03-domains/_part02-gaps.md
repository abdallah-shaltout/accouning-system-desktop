# 21 · Part 03 — Part 02 gaps found while planning (manager-owned)

Each gap is fixed by the manager before the first wave that needs it (entry file §3.7). A schema
change is a new migration `m0016_…`, never an edit to m0001–m0015 once Part 02's tests pass.
Status: `open` → `fixed (<file>)`.

| Id | Gap | Found by | Needed by | Status |
|---|---|---|---|---|
| G-1 | `shared::balances::OpenDocument` lacks fields the payments/parties screens show | 05-parties | 05, 09 | fixed (W0-B, shared, 2026-09-28) |
| G-2 | `shared::balances` has no batch readers for list screens (N+1 per party) | 05-parties | 05 | fixed (W0-B, shared, 2026-09-28) |
| G-3 | `AppError::map_unique` can never match (errno/constraint-name lookup) | 05-parties | every domain with a unique rule | fixed (W0-A, core, 2026-09-28) |
| G-4 | `with_read` gives no `BusinessClock` (reads that need "today") | 05-parties | 05, 13, 14 | fixed (W0-A, core, 2026-09-28) |
| G-5 | statement line order not fixed (stable sort key) | 05-parties | 05 | fixed (W0-B, shared, 2026-09-28) |
| G-6 | `TxCtx::require_any(areas)` for commands callable from several areas | 04-approvals | 04 | fixed (W0-A, core, 2026-09-28) |
| G-7 | `Role`/`Area`/`Access` lack `TS` derives (DTOs reference them) | 03-users | 03 | fixed (W0-A, core, 2026-09-28) |
| G-8 | bindings: no hook to export domain DTOs; `ipc.gen.ts` emitted without imports; `AuditEntry.link` import path wrong | 03-users | every domain | fixed (W0-A, core, 2026-09-28) |
| G-9 | every `*_instant` column is `TIMESTAMP(0)` — ms lost, ordering/keys differ from the mock (must be `DATETIME(3)`) | 05-parties | every domain | fixed (m0001, m0008–m0014: `.timestamp()` → `DATETIME(3)`, 2026-09-28) |
| G-10 | shared tax-id validation rule (EG/SA) | 05-parties | 01, 05 | fixed (W0-B, shared, 2026-09-28) |
| G-11 | unused party `balance` columns (balances are always derived from the ledger) — drop | 05-parties | 05 | fixed (W0-C m0016, 2026-09-28) |
| G-12 | `SystemRole` (shared::ledger::accounts) lacks serde/`TS` derives and a `FromStr` from the DB string | 12-accounting | 12, 01 | fixed (W0-B, shared, 2026-09-28) |
| G-13 | `RouteRef` has no `query` field (links with query params) | 12-accounting | 12, 13 | fixed (W0-A, core, 2026-09-28) |
| G-14 | `PagedResult.totals` is `f64` — must be `Decimal` as JSON number (rule 5) | 12-accounting | every paged list with totals | fixed (W0-A, core, 2026-09-28) |
| G-15 | ledger drafts: `save_draft`/`update_draft` take a plain date (DocDate instant lost); `post_draft` sets the wrong `posted_by`, drops currency, skips the Parties touch | 12-accounting | 12 | fixed (W0-B, shared, 2026-09-28) |
| G-16 | `ledger::post` doesn't lock the account rows it resolves (a concurrent deactivate/delete race) | 12-accounting | every poster | fixed (W0-B, shared, 2026-09-28) |
| G-17 | `accounts.code` has no unique index — needs `m0016` | 12-accounting | 12, 00 | fixed (W0-C m0016, 2026-09-28) |
| G-18 | `journal_entries.search_normalized` never filled on the `ledger::post` insert path | 12-accounting | 12 | fixed (W0-B, shared, 2026-09-28) |
| G-19 | compensators receive no `&UndoRegistry` (their own `record` call needs one) | 12-accounting | 12, 02 | fixed (W0-B, shared, 2026-09-28) |
| G-20 | importer must de-duplicate names under the new unique keys (expense categories, journal templates) | 12-accounting | 00 | open |
| G-21 | one shared totals/VAT helper in Rust (tax-inclusive, discount line → invoice → VAT) used by invoices **and** purchases — not two copies | 08-invoices | 07, 08 | fixed (W0-B, shared, 2026-09-28) |
| G-22 | migration: free-text invoice lines need `product_id` NULL; due/expiry dates stored as full timestamps should be `DATE` | 08-invoices | 07, 08, 00 | fixed (W0-C m0016, 2026-09-28) |
| G-23 | read which unique key a duplicate-key error hit (constraint name → message) — pairs with G-3 | 08-invoices | 08, 05 | fixed (W0-A, core, 2026-09-28) |
| G-24 | open-documents list in `shared::balances` must be complete (pairs with G-1) | 09-payments | 09 | fixed (W0-B, shared, 2026-09-28) |
| **G-25** | **ACCOUNTING BUG in the mock: a refund on a tax-inclusive sale over-refunds VAT (`sales.ts:437-444`: refunding 115 pays back 130); invariants miss it (entry still balances). Needs an `ACC-` ledger issue + mock fix + `scripts/verify/cases` regression case before `invoices_create_refund` is ported — user to confirm.** | 08-invoices | 08 (`createRefund` only) | fixed (2026-09-29, user delegated the decision: refund = exact proportional net + VAT share; ACC-0003, refined by ACC-0017 gross-first rounding) |
| G-26 | helpers: branch document prefix, default cost center, default purchase tax (shared, one copy) | 06/07 | 06, 07 | fixed (W0-B, shared, 2026-09-28) |
| G-27 | somewhere to record manager-PIN approvals (server re-checks the PIN) | 06b-inventory | 06b, 08 | fixed (design: in-memory `AppState.approval_grants` per 06b (no schema) — added by 06b's implementer via manager, 2026-09-28) |
| G-28 | `m0016`: category/unit/price-list name uniques are `unicode_ci` but P2-17 says exact (`utf8mb4_bin`); unique barcode index; received dates keep their time; a lock row to serialize barcode checks | 06-products | 06, 06b, 07 | fixed (W0-C m0016, 2026-09-28) |
| G-29 | `architecture_rules` false positive on a legitimate products write path (see 06 §7) | 06-products | 06 | fixed (W0-C architecture_rules, 2026-09-28) |
| G-30 | `Payment`/`PaymentStatus` DTOs are needed by purchases before 09 lands — define them in `domains/payments/dto.rs` first (W3 ordering) | 07-purchases | 07, 09 | open |
| **G-31** | **ACCOUNTING BUGS in the mock (kept in the port until decided): (a) landed cost that doesn't split evenly makes the receive entry unbalanced (100.00 / 3 lines → 99.99) — the rounding gap needs the largest-line rule; (b) receiving a non-stock item debits inventory without stock value (breaks `inventory-gl`); (c) completing a draft adjustment skips the approval threshold/PIN. Each fix = `ACC-` issue + regression case.** | 06b/07 | 06b, 07 | fixed (2026-09-29, user delegated: (a) remainder on the largest line ACC-0004, (b) non-stock → purchase account ACC-0005, (c) same approval rule on completion ACC-0006) |
| G-32 | `core::tx::with_read_ctx` (widens G-4): read tx whose closure gets `ReadCtx { actor, clock, terminal_id }` + `ReadCtx::require(area, access)`, and refuses with `UNAUTHORIZED` without a session | 13-reports | 13, 13b, 14, 14b | fixed (W0-A, core, 2026-09-28) |
| G-33 | `indexmap = "2"` as a direct dependency (JS `Map` insertion order + stable sort in every group-by) | 13-reports | 13, 13b, 14, 14b | fixed (W0-C Cargo.toml, 2026-09-28) |
| G-34 | `utils::text::compare_ar` via ICU4X `icu_collator` (locale `ar`) for the mock's `localeCompare(…, 'ar')` | 13-reports | 13b | fixed (W0-C utils::text::compare_ar, 2026-09-28) |
| G-35 | lock-order text conflict: entry file §3.3 vs `core/lock.rs:3-6` header — pick one order, fix the other | 13-reports | all writers | fixed (entry file §3.3 now cites `core/lock.rs`, 2026-09-28) |
| G-36 | `utils::format` (`Numerals`, `DateStyle`, `format_money`, `format_number`, `format_date_key`) — port of `format.ts` formatters for strings the mock builds with them | 14-analytics | 14, 14b | fixed (W0-C utils::format, 2026-09-28) |
| G-37 | frontend `core/services/backendMirror.ts` (`mirrored`, `clearMirrors`): reactive mirror so **synchronous** service functions read inside `computed()` can be Rust-backed without page changes | 14-analytics | 14, 14b | fixed (W0-C backendMirror.ts, 2026-09-28) |
| G-38 | `Simplify<T>` in `core/types/contract.ts` for `contract.check.ts` entries whose TS type is an intersection | 14-analytics | 14, 14b | fixed (W0-A, core, 2026-09-28) |
| G-39 | public DTO assemblers other domains reuse: `Product` + `StockTransfer` (06), `ApprovalRequest` (04), `Invoice` (08) | 14-analytics | 14 | open |
| G-40 | `scripts/contract/config.ts` override `templates.exportTemplate → frontend` (pure transform, 15 T-6) | 15-templates | 15 | fixed (W0-C contract config, 2026-09-28) |
| G-41 | `architecture_rules` must allow `src/infrastructure/import/**` and `src/infrastructure/backup/**` to write raw rows (importer/restore) | 00/17 | 00, 17 | fixed (W0-C architecture_rules, 2026-09-28) |
| G-42 | `shared::numbering::set_counter(conn, kind, value)` (importer/restore set counters) | 00 | 00, 17 | fixed (W0-B, shared, 2026-09-28) |
| G-43 | on Windows always attach a managed `ServerHandle` to `AppState`, even before provisioning (setup's role step needs it) | 02-setup | 02 | fixed (W0-A, core, 2026-09-28) |
| G-44 | `with_read_on(state, conn)` / a read helper that takes an explicit connection (importer/backup) | 00/17 | 00, 17 | fixed (W0-A, core, 2026-09-28) |
| G-45 | pre-migration automatic backup in `core::db::migrate` (P2-52) + `DbStatus::MigrationBackupFailed` | 17-backup | 17 | fixed (W0-A, core, 2026-09-28) |
| G-46 | `ServerPaths::backups()` directory | 17-backup | 17 | fixed (W0-A, core, 2026-09-28) |
| G-47 | crates: `zip`, `aes-gcm`, `pbkdf2`, `sha2`, `base64` | 17-backup | 17 | fixed (W0-C Cargo.toml, 2026-09-28) |
| G-48 | `03-users` makes `to_authenticated` public (setup's bootstrap session) | 02-setup | 02, 03 | open |
| G-49 | `journal_draft_lines` has no `currency`/`amount_fc`/`rate` columns, so a draft can't carry FC lines into `post_draft` — add to `m0016` + entity | W0-B | 12 | fixed (manager: m0016 + entity + ledger, 2026-09-28) |
| G-50 | `SystemRole` TS export: add `SystemRole::export_all` to `domains::export_bindings` (or rely on an embedding DTO) | W0-B | 12, 01 | fixed (manager: domains::export_bindings, 2026-09-28) |
