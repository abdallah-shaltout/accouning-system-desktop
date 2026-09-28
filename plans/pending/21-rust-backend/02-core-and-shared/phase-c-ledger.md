# 21 · 02.C — `shared::ledger`, `shared::numbering`, `shared::currency`

> **Status:** code complete, not yet compiled/tested (build rule 2026-09-27). Implemented blind
> against 02.A/02.B's real code (no `document_counters` SeaORM entity exists — B1/B2 left it as a
> migration-only table per its own doc comment [C-11: no UUID PK, never synced], so `numbering.rs`
> talks to it via raw `Statement`s, which also happens to be exactly what keeps it the one place
> touching that table). `SystemRole` has no Rust enum anywhere else in the repo (the `accounts`
> entity stores `system_role` as a plain nullable `String`), so `shared::ledger::accounts::SystemRole`
> is a new 36-variant enum defined here — every other module should use this one, not invent a
> second. Deviations from the phase spec, recorded per CLAUDE.md's architectural-autonomy rule:
> `next_number`/`lock` in `shared::numbering` take no `cx: &TxCtx` parameter (the fixed API contract
> given to this wave says `next_number(conn, kind) -> TxResult<String>`, no `cx` — the row lock
> already comes from the `UPDATE`/`FOR UPDATE` statement itself, not from `TxCtx`). See the final
> report for the full API and "Needs from manager" list.

**Goal:** the **only** code that writes `journal_entries`/`journal_lines` (master rule 3).
It is a behaviour-exact port of `postJournal`/`resolvePosting`/`assertOpenPeriod`
(`src/mocks/backend/core.ts:55-169`), `reverseJournal`/reopen's mirror (`journal.ts:130-162`,
`core.ts:640-677`), `accountFor` and the product resolvers (`accounts.ts`), `nextNumber`
(`db.ts:232-254`) and `currency.ts`, plus the row locks D8 requires. It is balanced or refused
(rule 6) and never "fixes" numbers.

**Read first:** [`../../../../docs/v2/02-accounting-review.md`](../../../../docs/v2/02-accounting-review.md)
B1–B3, §3, §4 · [`../01-frontend-analysis/accounting.md`](../01-frontend-analysis/accounting.md)
§2–§5 · the mock files above · `src/mocks/backend/posting-trace.ts`.

## Tasks

### C-1 — `shared/ledger/accounts.rs`
- [x] `resolve_account(conn, role, ctx { branch_id, currency })`: candidates are live rows with
      `system_role = role AND active`, `ORDER BY created_at, id` (the mock's array order). Pick the
      branch match, else the currency match, else a row with neither, else the first
      (`accounts.ts:61-69`). None → `NOT_FOUND` with the role label
      (`لا يوجد حساب في شجرة الحسابات لدور "{label}" — أضف حساباً بهذا الدور أولاً`). The 36-label
      `ROLE_LABEL` table is copied from `accounts.ts:11-48`. This is the **only** role → account
      lookup in the backend (accounting.md §2).
- [x] `account_by_id` (`NOT_FOUND` `الحساب غير موجود في شجرة الحسابات`), `settlement_account_for(method)`,
      `revenue_account_for`, `cogs_account_for`, `purchase_account_for(…, default_purchase_account_id)`,
      `sale_tax_id_for`, `purchase_tax_id_for`, all ported 1:1 from `accounts.ts:85-153`.

### C-2 — `shared/ledger/period.rs` (P2-13)
- [x] `assert_open_period(conn, date: &NaiveDate, allow_closed_period)`. **Always**: take a shared
      lock on the settings row, then the first fiscal year covering `date.day` (`ORDER BY
      created_at, id`, `LOCK IN SHARE MODE`). **Only if `!allow_closed_period`**: if a lock date is
      set and `day <= lock_date` → `AppError::period_locked_by_date`; if the covering year is
      closed → `period_locked_by_year`. No covering year → allowed (the mock's behaviour,
      `core.ts:112`; recorded, not changed). The date check runs before the year check, as in `core.ts:108-115`.
- [x] `lock_fiscal_year_exclusive(conn, id)` for Part 03's `close_year`/`reopen_year`. The doc
      comment states the lock order: settings (S) first, then the year (X).

### C-3 — `shared/numbering.rs` (P2-21)
- [x] `next_number(conn, kind) -> TxResult<String>`: `UPDATE document_counters SET value=value+1 WHERE kind=?`,
      then read `value` back (the row stays X-locked until commit). Prefix: `settings.invoice_number_prefix`
      for `invoice`, else the fixed map from `db.ts:232-247`. Zero-pad to 6 digits. Gapless (a rollback releases the number).
- [x] `lock(conn, SequenceLock::{CustomerCode, SupplierCode})`: `SELECT … FOR UPDATE` on the lock
      row, so Part 03 can run the mock's `MAX+1` code rule (`partyService.ts:55-63`) safely.

### C-4 — `shared/currency.rs`
- [x] `base_currency(conn)` (= `settings.currency`), `is_base_currency`. `latest_rate(conn, code,
      as_of_day)`: a fixed rate wins, else the latest `exchange_rates` row with `date <= cutoff`
      (`currency.ts:77-83`). `require_rate` → `VALIDATION` `لا يوجد سعر صرف لعملة {code}`.
      `to_base(fc, rate) = round2(fc × rate)`. `convert_lines_to_base(fc[], rate)` puts the whole
      rounding gap on the largest-|fc| line, first index on ties (`currency.ts:97-112`).

### C-5 — `shared/ledger/post.rs`
- [x] Types: `AccountRef::{Role(SystemRole), Id(Id)}`. `PostingLine { account, debit, credit,
      description, party: Option<PartyRef{kind,id}>, branch_id, cost_center_id, currency,
      amount_fc, rate, trace: Option<LineTrace> }` (mirrors `core.ts:22-52`). `PostJournal { date:
      DocDate, description, entry_type, source: Option<SourceRef{kind,id,number}>, lines,
      allow_closed_period, attachment_ids, template_id }`. `created_by` = `cx.actor`
      (`UNAUTHORIZED` if there is none).
- [x] `resolve_posting(conn, lines)`: for each line, resolve the account (C-1); `branch_id` defaults
      to `settings.default_branch_id` (P2-20); `cost_center_id` defaults to that branch's cost
      center (`core.ts:73`); `currency` defaults to the base currency; `debit`/`credit` →
      `round2`. Keep lines with `debit > 0 || credit > 0`. Totals = `round2(Σ)`. If
      `|Σd − Σc| > 0.001` → `AppError::unbalanced`. Refuse `is_group` accounts (P2-36; same message
      as `journal.ts:24`). Also used by the draft path, which has no period check (`core.ts:178-205`).
- [x] `post(conn, cx, PostJournal) -> journal_entries::Model`, in this order: `assert_open_period`
      → `resolve_posting` → fewer than 2 kept lines → `VALIDATION`
      `يجب أن يحتوي القيد على سطرين على الأقل` → `next_number(Journal)` → insert the entry
      (`status` POSTED implied, `posted_by = created_by`, `posted_at = cx.clock.now`) and the lines
      in input order → `cx.touch(Ledger)`, plus `cx.touch(Parties)` if any line has a party
      (P2-12) → queue a posting trace (C-7).
- [x] Drafts: `save_draft`, `update_draft`, `delete_draft`, and `post_draft(id, allow_closed)`,
      which moves the row into `journal_entries` with the **same id and number**
      (`core.ts:233-245`), deletes the draft (its lines cascade), and touches Ledger.

### C-6 — `shared/ledger/reverse.rs`
- [x] `reverse(conn, cx, ReverseRequest { original_id, date, description, entry_type,
      allow_closed_period, reason: Option<ReversalReason { text, stamp_original: bool }>,
      dims: MirrorDims::{Keep, Default} })`. Steps: lock the original (`FOR UPDATE`, `NOT_FOUND`
      `القيد غير موجود`); if `reversed || reversal_of_id.is_some()` → `VALIDATION`
      `هذا القيد معكوس بالفعل`; post the mirror (debit ⇄ credit, keep account, description,
      party; `Keep` also copies branch and cost center, like `reverseJournal`; `Default` omits
      them, like reopen at `core.ts:651-658`; currency and FC are never copied, as in both mock
      paths); set `reversal_of_id` and `reversal_reason` on the mirror; set `original.reversed =
      true` and, only if `stamp_original`, `original.reversal_reason`. The `MANUAL`-only rule and
      the required reason stay in the accounting domain (Part 03), because the mock checks them
      there (`journal.ts:133-135`).

### C-7 — `shared/ledger/trace.rs` (P2-35)
- [x] `PostingTrace`/`PostingTraceStep`, the shape of `posting-trace.ts:21-36`. Steps come from
      `LineTrace` plus an `accountResolution` step for every line (`posting-trace.ts:60-95`).
      `Effects` gains `traces`. After commit, `with_tx` pushes them into `AppState.traces` (a ring
      of 500; in debug builds also indexed by entry id) and logs `log::debug!` on the
      `accounting` target. A rolled-back command leaves no trace.

### C-8 — Rules and docs
- [x] `architecture_rules`: only `src/shared/ledger/**` may build an ActiveModel for or insert into
      `journal_entries`, `journal_lines`, `journal_drafts`, `journal_draft_lines`; only
      `src/shared/numbering.rs` may touch `document_counters`.
- [x] C-9: `00-MASTER-PLAN.md` rule 6: replace "returns `AppError::Unbalanced` / `PeriodLocked`" with
      "returns `AppError::unbalanced(…)` (code `VALIDATION`) / `period_locked_*` (code `FORBIDDEN`),
      the mock's exact codes and messages" (C-01). Rule 5: "`round2` for money **and quantity**,
      `round4` for cost/rate" (C-03).

## Tests (DB-backed)
- [x] A balanced post persists the entry and its lines in order; the number goes `JE-000001`,
      `JE-000002`; the branch, cost-center and currency defaults are applied; `10.005` debit is stored as `10.01`.
- [x] Zero lines are dropped; one kept line → the exact message; an unbalanced post gives
      `القيد غير متوازن: المدين 100 ≠ الدائن 99.99`; a group account is refused; a missing role → the label message.
- [x] Period: lock-date refusal; closed-year refusal; `allow_closed_period` bypasses both; no covering year is allowed.
- [x] **Concurrency (two connections):** (a) 20 concurrent posts get 20 distinct sequential numbers
      with no gaps; (b) transaction A holds the year's X lock, then B's post blocks; A sets
      `is_closed` and commits; B is then refused with `period_locked_by_year` (a timeout-guarded test).
- [x] Reverse: `Keep` vs `Default` dimensions, both reason variants, and refusal of a second reversal.
- [x] Draft: `post_draft` keeps the id and number; the draft rows are gone.
- [x] Effects: Ledger is touched on every post; Parties only with a party line; the trace ring
      gets the entry after commit and nothing after a rollback.
- [x] Currency: the fixed-rate path; the cut-off date; the `convert_lines_to_base` gap goes to the
      largest line (tie → first); the `require_rate` message.
- [x] `resolve_account` precedence (branch > currency > neutral > first); inactive and soft-deleted accounts are excluded.

## Gate
- [ ] `cargo build` + `cargo test` green; `architecture_rules` includes the C-8 rules. ⏳ runs in the single final build/test pass
- [ ] `bun run build`, `check`, `verify:mocks` (128/0), `contract:check`, `memory:check`, `diag:check`. ⏳ runs in the single final build/test pass
- [x] Master plan rules 5 and 6 reworded (C-9). Status note at the top of this file.
