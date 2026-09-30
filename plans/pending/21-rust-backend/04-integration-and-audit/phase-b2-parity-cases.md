# 04 · Phase B2 — Parity cases per domain lane, and the fix loop until 0 diffs

> **Status:** pending. Five parallel lanes. **Wave 1** (authoring, `--mock-only`) starts as soon as
> `scripts/parity/case.ts` (B-6) exists. **Wave 2** (Rust runs and the fix loop) starts once the
> manager has built `parity_host` and the Parts 02–03 test pass is green for the lane's `domain_*`
> suites. The case inventory is the **§8(b) list of each 03-domains file**. Case names below are
> those ids; don't re-derive them.

## Lanes (disjoint files; see the entry file's ownership table)

| Lane | Model | Domains (03-domains files) | ≈ cases | Size |
|---|---|---|---|---|
| **L1 platform** | Sonnet | settings (01, except revaluation), users (03), approvals (04), templates (15), diagnostics (16), backup (17), import (00), attachments (its domain file, when it lands) | 30 | M |
| **L2 masters & stock** | Opus | parties (05), products (06), inventory (06b), purchases (07) + verify ACC-0004/5/6 | 40 | L |
| **L3 sales & cash** | Opus | invoices (08), POS shifts (08b), payments (09), vouchers (10) + verify ACC-0003 | 42 | L |
| **L4 ledger** | Opus | accounting (12), period close (12b), expenses (11), setup (02: wizard + openings), settings revaluation (01) + verify ACC-0007/8 | 39 | L |
| **L5 read models** | Sonnet | reports (13), operational reports (13b), analytics/dashboard (14), insights (14b) | 15 (large) | M |

Why this split: posting logic (L2–L4) needs Opus and a read of `docs/v2/02-accounting-review.md`.
L1 and L5 are mechanical (CRUD, settings, read-only aggregation on seeded data). Each lane owns the
Rust domains its cases exercise, so a lane can fix its own diffs without touching another lane's files.

## The case file convention

- Path: `scripts/parity/cases/<domain>/<case-id>.ts`, one `defineCase` per file (B-6 API), with `name: '<domain>/<case-id>'`
  and `source: '03-domains/<nn>-<d>.md §8(b)'`.
- **Base:** `demo-sa` by default. Use `demo-eg` wherever a case's behaviour depends on the country (VAT 14 %, EGP,
  timezone), and at least once per posting lane. Use `edge` for the importer cases. Use `empty` for the setup wizard.
- **One case = one business story.** Assert the documents through their own read services (`getInvoice`,
  `getJournalEntriesForSource`, …). The runner adds the books snapshot (trial balance, parties, stock) and the
  invariants on its own (B-9).
- Every validation case uses `s.expectError` for **each** Arabic message the domain file's §3 lists, so error
  parity is byte-exact.
- Cross-domain calls are fine, because in the rust pass the whole app is on Rust (P4-1). A diff whose cause
  lies in another lane's domain goes to the manager, who routes it. Don't edit that lane's files.

## The fix loop (every lane, Wave 2)

For each unexplained diff, pick **exactly one** of these, and record it in the lane's section of this file's status note:

1. **Rust is wrong.** Fix it in `src-tauri/src/domains/<d>/**`. If money, stock or a balance is involved,
   add the scenario to `tests/domain_<d>.rs` too (a regression test on the DB). Run
   `scripts/cargo-safe.ps1 <lane> check --manifest-path src-tauri/Cargo.toml --workspace --all-targets`,
   then ask the manager for the next host build round.
2. **The mock is wrong, and it is not a listed quirk.** Don't fix it silently (03 §3.3). Open an issue:
   `ACC-` for a wrong number, `BUG-` otherwise (`docs/diagnostics/issues/…`, then `bun run diag`).
   For a **wrong number**, apply the P4-7 rule: fix both backends in this lane, add a
   `scripts/verify/cases/*.json` regression case, keep `bun run verify:mocks` and `verify:replay` green, and
   add a line to the domain file's "Known mock quirks". Anything that is not a wrong number (a missing guard
   that changes scope, wording) stays as-is and goes to the "later" note.
3. **A representation difference that a written decision allows.** Add an `allow` entry citing that id
   (P4-6). The runner rejects an entry without one.
4. **It needs a `shared/**`, `core/**`, `entities/**` or `migration/**` change.** Request it from the
   manager (a new migration is `m0018+`, and m0017 belongs to attachments). Keep working on other cases meanwhile.

A lane is done when `bun run parity --lane <Ln>` shows **0 unexplained diffs**, and the Rust invariants hold
after every step (B-9).

## L1 — platform (Sonnet)

> **Wave 2 status (2026-09-30, mock vs Rust): 31/31 green** (30 cases + `_selftest`; first run 8/31). Fix log (one choice per diff, per the fix loop):
> - **Rust wrong:** users/approvals `parse_id` refused a non-UUID id as `VALIDATION` "معرّف غير صالح" →
>   now `Id::unknown_from_text` so the service's own `NOT_FOUND` answers, like the mock. Importer: absent
>   boolean flags defaulted to `true` (`default_true`) — now `false`, matching the mock's truthy reads
>   (00-import D-13; `edge` admin login refused on both sides); held sales read a `cart` key no mock
>   `HeldSale` has, dropping every parked line — now reads `lines` (regression in `domain_import`
>   `edge_fixture_quirks`). `save_attachment` silently re-keyed a non-UUID client id (the caller then
>   could never fetch its file) — now `VALIDATION` (+ `domain_attachments` test). `wipe_business_rows`
>   left `attachments` rows behind. `settings_save_backup_settings` dropped the device delta, so a
>   chosen backup folder was never persisted (auto backup always `NoFolder`). `settings_build_backup_archive`
>   stamped the wall clock instead of the business clock. `shared::balances::{customer,supplier}_balances`
>   resolved the control account even for an empty party list (mock returns `[]`).
> - **Mock/frontend wrong (not a number):** fflate's raw "invalid zip data" leaked from `readManifest`
>   (now the Arabic `VALIDATION`, same as Rust); `persist.ts` didn't resync the `unk-N` audit placeholder
>   counter from `entityId`; attachments needed a UUID id on Rust (`attachmentService.newAttachmentId()`).
> - **Harness:** posting-trace ring not cleared per pass (`pass.ts`); parity host shared one app-data
>   dir (device file leaked across cases/lanes) — now per process, device file removed on every reset.
> - **Case wrong:** payment-method inputs lacked `active`; seeded ids not passed through `s.baseId`;
>   cashier reading the approvals queue (Approvals:Read, D-2); `users-verify-pin-roles` books read as a
>   cashier; backup archives built from the (empty on Rust) live `db` instead of the base; auto-run with
>   no folder took different branches; `import-edge` signed out (Rust reads need a session).
> - **Allowed (decision ids):** 03-users D-1 (restore session), 17-backup D-3 (browser archive restores
>   through the importer: re-keyed ids, format fields), 06-products Q-7 (`prices: []`), 00-import
>   D-13/D-14/C-14 (edge fixture flags/currency/theme).
> - **Edge fixture** (manager decision): accounts/branch/product carry `active: true`; the admin keeps no
>   `active` (pinned refusal); TS-required fields added (`invoiceNumberPrefix`, `thermalWidthMm`,
>   `refundedAmount`, line id), free-text line `freetext-0`, held sale in the real `lines` shape; the two
>   TS-format backup zips regenerated from it.

> **Status note (2026-09-29, gap-closing pass):** the three harness-only gaps Wave 1 found and routed
> around are now fixed in the harness (not in `src/mocks/**`, per scope): `scripts/parity/polyfills.ts`
> (new) installs an in-memory `localStorage`/`sessionStorage`, a real `indexedDB` (the `fake-indexeddb`
> devDependency), a `window` alias, and a minimal `document` stub for `saveFile.ts`'s
> `browserDownload()` — all guarded to only install when the global is genuinely absent, so production
> behaviour is unchanged. `run.ts` imports it first (before any service-touching import) and
> `pass.ts`'s `prepareFrontend` calls `resetPolyfills()` every pass (mock and rust alike) so both start
> from an empty "browser profile". With that: `templates/templates-lifecycle` is rewritten to the real
> chained lifecycle (each step uses the id an earlier step returned); `backup/backup-counts`,
> `backup/backup-restore-browser-archive` and `backup/backup-auto-run` are rewritten to call the real
> `backupService` functions end to end (including a real `backupNow`/`restoreFromArchive` round trip
> and a real `initAutoBackup`/`stopAutoBackup` cycle). `attachments/attachments-lifecycle` and
> `attachments/attachments-size-refusal` are new (attachments landed as a Rust domain,
> `usesRust('core')`, no `03-domains/` file of its own — cases cite the seam module doc comment
> instead of a §8(b) list): save → fetch (by owner) → fetch-by-ids → remove, overwrite-by-id, and the
> unknown-id no-ops, plus a dedicated case pinning a **known, unfixed cross-backend gap**: Rust's
> `save_attachment` refuses a blob over 10MB server-side, the mock's `putAttachment` has no such
> check at all — reported in that case's own doc comment (P4-7 needs a decision), not fixed here.
> `import/import-edge` is now written too: the harness's invariants crash on this base is fixed
> (`books.ts`'s `mockInvariants()` now catches a thrown invariant check — `checkArApControl`/
> `checkInventoryGl` calling `accountFor()` for a role `mock-snapshot-edge.json` doesn't have — and
> returns one synthetic failing row instead of aborting `invariantsNow()`; the fixture itself was never
> touched). The fixture's one user has no `active: true`, so login genuinely fails there (pinned with
> `s.expectError`, `user: null`) — a real property of that fixture, not routed around; `getShifts`/
> `getShift` are skipped for the same reason (the fixture's `shift-1` row has no `movements` array,
> which `shifts.ts`'s summary math needs — a fixture-shape gap, not a harness one). `bun run parity
> --mock-only --lane L1`: **31/31 green**, 0 unexplained diffs (checked twice for determinism).
> `bun run parity --mock-only` (all 5 lanes, 172 cases): **all green**, 0 unexplained diffs anywhere —
> the other lanes' previously-open mock bugs (L2's two purchase cases, L3's two FX/invariant cases)
> are green too now, fixed by their owning lanes in the interim. `bun run check` clean (pre-existing
> warnings only, unrelated to this pass). The Rust host still doesn't exist, so the cargo test filters
> below and a real mock-vs-rust run are still Wave 2, unchanged from before.
- [x] **settings:** `settings-update-merge`, `settings-tax-crud`, `settings-payment-methods-order`,
      `settings-branch-lifecycle`, `settings-cost-centers`, `settings-currency-rates`.
- [x] **users:** `users-login-audit`, `users-create-rename-login`, `users-self-protection`,
      `users-verify-pin-roles`, `users-list-order`.
- [x] **approvals:** `approvals-submit-approve`, `approvals-reject-requires-comment`,
      `approvals-double-decide`, `approvals-list-order`.
- [x] **templates:** `templates-lifecycle` (the 15 §8(b) sequence: list (seed) → create → save → set default →
      duplicate → reset → import valid and invalid → delete → list). Template list order is compared as a set,
      plus a spot check (12 Q6). Restored to the real chained lifecycle now that the localStorage harness gap
      is fixed (`polyfills.ts`) — see status note.
- [x] **diagnostics:** `diagnostics-audit-filter`, `diagnostics-audit-entities`, `diagnostics-invariants`,
      `diagnostics-balances-around`, `diagnostics-drift-clean-db`.
- [x] **backup:** `backup-settings-save`, `backup-counts`, `backup-restore-browser-archive`, `backup-auto-run`.
      All four now call the real service functions end to end (`backupNow`/`restoreFromArchive`/
      `initAutoBackup`) now that the indexedDB/document/window harness gaps are fixed — see status note.
- [x] **import:** `import-demo-sa`, `import-demo-eg` (base only; the steps call every list service and diff
      them). `import-edge` (base `edge`) is now written too — the harness's invariants-crash gap is fixed
      (`books.ts`); see status note for what it covers and skips. Order differences from backdated seed
      rows would be allowlisted with 00-import Q-2/D-8, one entry per affected list, never a wildcard —
      none were needed.
- [x] **attachments:** `attachments-lifecycle` (save → fetch by owner → fetch-by-ids → remove, overwrite,
      unknown id) and `attachments-size-refusal` (10MB boundary — pins a real, unfixed cross-backend gap,
      see status note). No `03-domains/` file exists for attachments yet, so these cite the seam module
      (`core/services/attachmentService.ts`) doc comment as `source` instead.
- [x] L1 gate: `bun run parity --lane L1` shows 0 unexplained diffs — **31/31 green** (2026-09-30,
      host `parity_host-20260930-125520`, run `.diagnostics/parity/2026-09-30T09-55-40-120Z`);
      `--mock-only --lane L1` green; `domain_settings/users/approvals/templates/diagnostics/backup/import/
      attachments/parties::` + `shared_ledger::` green on test exe `all-20260930-095036`.

## L2 — masters and stock (Opus) — read `docs/v2/02-accounting-review.md` first

> **L2 status (Wave 2 close, 2026-09-30):** **L2 green, 44/44** (L3 44/44 incl. new `invoices/sale-non-base-unit`); m0020 landed (line `unit_id` → `VARCHAR(64)`, no FK — it is the product's own `ProductUnit.id`); full Rust suite 402/402; ACC-0004/5/6 → `verified` (manager: run `bun run diag`).

> **L2 status (Wave 2, 2026-09-30):** `bun run parity --lane L2` (mock vs Rust): 8/43 (first run) → **43/44 pass**
> (44 = the 43 + new `products/p-c11-unit-prices`). The one red case, `purchases/p-p6-tracked-batches`, is
> blocked on a **migration** requested from the manager: every line table's `unit_id`
> (`purchase_order_lines`, `invoice_lines`, `quotation_lines`, `stock_transfer_lines`) is a `uuid` FK to
> `units`, but the value the UI stores is the product's own `ProductUnit.id` (`pu-panadol-box`, a free
> string — `PurchaseFormPage` `unitOptionsFor`, `usePosStore` `l.unit?.id`), so any unit line fails the FK
> (`CONFLICT`). Needs m0020: drop the four `fk_*_unit_id`, make the columns `VARCHAR(64) NULL`, entity/DTO
> `unit_id: Option<String>`, importer copies the raw string. Rust fixes this pass: transfer `-recv` source id
> (MariaDB 11.4 refuses a v8 UUID with byte 8 in `0x01..=0x80`; also `Id::unknown_from_text`),
> `landedCosts` absent-vs-`[]` kept like the mock, `supplierInvoiceDate` accepts an instant (was an IPC
> decode `INTERNAL`) + importer keeps seeded instants (07 D-U8), duplicate-supplier-invoice match is
> app-side (padded numbers), `updatedAt` not bumped by link/unlink/follow-up updates, unit presets'
> `allowsDecimals` only where the mock sets it, `ProductUnitPrice.unitId` is a string (06 D-P7). Mock
> fixes: BUG-0020 (purchase detail's stale supplier balance), BUG-0021 (`getLinkedNetBalance` float noise).
> Allow entries: 06 Q-7 (`prices` `[]`≡absent), 05 D-6 (`phones`), 07 Q-U8 (return `taxRate`), 07 D-U8
> (`supplierInvoiceDate` instant vs its day) — the first two/last use the new narrow `emptyArrayOnly` /
> `dayOfInstantOnly` allow flags (`case.ts`/`allow.ts`). L2 gate stays open until the migration lands.

> **L2 status (Wave 1, 2026-09-29):** 42 cases authored (parties 8, products 11 incl. ACC-0006, inventory 10,
> purchases 13 incl. ACC-0004/0005 and two split-outs). `bun run parity --mock-only --lane L2`: 41 of 43 pass
> (incl. `_selftest/identity`), 0 unexplained diffs, no allow entries yet. The 2 failures are mock bugs,
> reported and not fixed (P4-7 needs a decision + `ACC-` issue): `purchases/p-p7-return-variance-guard`
> (purchase-return variance line has the wrong sign → unbalanced entry, non-atomic partial write) and
> `purchases/p-p4-other-supplier-landed` (other-supplier landed-cost AP has no payable document →
> `supplier-allocation` invariant breaks). ACC-0004/5/6 cases are green on the mock; the verify:replay /
> `tests/domain_*` / `verified` steps of L2-0 and the Rust pass are Wave 2.

- [x] **L2-0 Verify the G-31 fixes (P4-7).** (Wave 2: 3 cases green mock-vs-Rust, replay green, pinned by `domain_purchases`/`domain_products`, ACC-0004/5/6 `verified`) They are already fixed in both backends by the 2026-09-29 pass,
      at `status: fixed`. Add one parity case per fix, check that `bun run verify:replay` replays its
      `scripts/verify/cases` file green, and check that a `tests/domain_*` test pins it (add one if missing).
      Then set the issue to `verified` and run `bun run diag`:
      `purchases/acc-0004-landed-cost-remainder` (100.00 over 3 lines → shares sum to 100.00, receipt balanced),
      `purchases/acc-0005-non-stock-receipt` (expensed to the purchase account, invariant `inventory-gl` holds),
      `products/acc-0006-draft-adjustment-approval` (completing an over-threshold draft without a grant is refused
      with the same message on both sides, and succeeds with the Rust-side G-P3 grant). If any fix has not landed
      in one of the two backends by Wave 2, this lane completes it in its own files, following that issue file.
- [x] **parties:** (Wave 1: 8 cases, mock-only green) `parties-create-codes`, `parties-update-absent-keys`, `parties-deactivate-guard`,
      `parties-statement-and-balance`, `parties-aging-buckets`, `parties-link-unlink`, `parties-duplicates`,
      `parties-search-arabic`.
- [x] **products (06):** (Wave 1: 10 cases + `acc-0006`, mock-only green) P-C1 … P-C10 (`products/p-c1-list-filter-search` … `products/p-c10-custom-fields`), including the
      Arabic normalization inputs `أحمد`/`احمد` and `٣`/`3`.
- [x] **inventory (06b):** (Wave 1: 10 cases, mock-only green) P-I1 … P-I10. P-I8 compares string sorts as multisets (Q-I10), with numeric sorts exact.
      P-I5 runs with the Rust-side grant.
- [x] **purchases (07):** (Wave 1: 9 cases + 2 split-outs authored; Wave 2: all 13 green mock-vs-Rust) P-P1 … P-P9. P-P8 continues P-I10 (`ExpiryReportPage` flow:
      `returnBatchesToSupplier` then `postDebitNoteDraft`, the reason products and purchases share one lane).
      P-P4 uses a non-divisible landed amount after L2-0(a).
- [x] L2 gate: `bun run parity --lane L2` shows 0 unexplained diffs, and `bun run verify:mocks` and
      `verify:replay` are green. The manager runs `domain_parties::`, `domain_products::` and `domain_purchases::`, and all are green.

## L3 — sales and cash (Opus) — read `docs/v2/02-accounting-review.md` first

> **L3 status (2026-09-29, Wave 1):** 42 cases authored (`invoices` 17 incl. `acc-0003-refund-inclusive-vat-exact`,
> `shifts` 8, `payments` 8, `vouchers` 9). `bun run parity --mock-only --lane L3`: **40/42 green**, and every
> `expectError` records the intended §3 message. The 2 red cases are blocked by shared-invariant findings outside L3's
> files (manager): (1) `allocations-within-total` compares an FC allocation's **base** `amount` with the FC
> document's `grandTotal` (`invariants.ts` `checkAllocationsWithinTotal`, and its Rust port
> `shared/invariants/documents.rs` `check_allocations_within_total`). It fails on the plain seed
> (`pay-135->inv-634 48500 > 1000`) and on `payment-fx-gain-worked-example`. The seed and `payments.ts` are
> correct: `verify:mocks`' `branches.ts` pins 48,500. (2) `payment-allocate-later-fx` also breaks
> `one-active-entry` (the allocate-later FX entry is a second SYSTEM entry on the payment, which 09 §3.3 specifies)
> and `fx-conversion`: `payments.ts` `allocatePayment` tags the FX-delta control line with `amountFc` = the full FC
> settled at `payment.rate`, which also moves the party's FC balance the wrong way. Not added to the §8(b) cases:
> the period-lock refusal. On the mock, `recordSale`, `recordPayment` and the voucher `record*` functions write the
> document and counter (and stock, for a sale) before `postJournal` throws. Any lock date covering today also trips
> `lock-date` on the seeded history. Both are reported to the manager. ACC-0003 is left at `fixed` until the Rust
> pass runs its case, although `verify:replay` of its bundle is green. Wave 2 (Rust) not started.

> **L3 status (2026-09-30, Wave 2 — mock vs Rust):** `bun run parity --lane L3` **43/43 green** (the 42 L3 cases + `_selftest/identity`; first Rust run: 1/42). Real bugs fixed:
> (1) **Rust** `RefundInput.lines[].invoiceLineId` was decoded as a UUID, but the UI sends the invoice line's DTO id
> (`{invoiceId}-l{n}`) — every refund failed `INTERNAL`. It is now a string resolved against
> `common::line_display_id`, and `Refund.lines[].invoiceLineId` / `InvoiceDetail.returnedQty` keys use the same id
> (`invoices/dto.rs`, `service/{refund,reads,common}.rs`, `tests/domain_invoices.rs`).
> (2) **Rust** importer stored `payments.target_ref` raw (`po-26`), so `Payment.targetRef` was dropped on read; it now
> maps like the allocation target (`infrastructure/import/tables/payments.rs`).
> (3) **Rust** `OpenDocument.dueDate` was `YYYY-MM-DD` where `Invoice.dueDate` (and the mock) give the DocDate key
> (`shared/balances.rs` `due_date_key`, `payments/dto.rs`).
> (4) **Rust** `createCardSettlement` with an empty deposit (`NaN` → `null`) failed to decode (`INTERNAL`); it is now
> the mock's `VALIDATION أدخل مبلغ الإيداع البنكي` (`vouchers/dto.rs` `deposit_amount: Option`, `service/settlements.rs`).
> (5) **Mock** `getInvoice`/`getInvoicePrintData` embedded the raw customer row (stale `balance: 0`, no
> `unallocatedCredit`) against the type's "computed on read"; they now use `partyService.withComputed` (08 D-I9
> updated), and `getInvoicesPaged` totals are `round2`ed. Neither touches posting (verify:mocks 128/0, replay green).
> Case fixes: every `'x-does-not-exist'` id became a well-formed missing UUID (`MISSING_ID`) — a free-form id tests
> Rust's id decoding (`VALIDATION معرّف غير صالح` / `INTERNAL`), not the §3 lookup refusal; the UI never sends one.
> Allow entries: `print-sample` `steps.sample.value.sample` (08 **D-I11**, new: the unsaved sample's id is a UUID),
> `sale-batch-fefo` `steps.*.value.prices` (`emptyArrayOnly`, 06 Q-7). Harness: `diff.ts` `derivedKey` pairs a
> `<parent>-lN` **object key** through its parent (selftest added). ACC-0003 → `verified`.

- [x] **L3-0 Verify the G-25 fix (ACC-0003, P4-7).** (Wave 2: case green mock-vs-Rust, replay green, ACC-0003 `verified`) It is already fixed in the mock (`sales.ts`, the shared
      `refundLineShare` in `src/modules/invoices/helpers/totals.ts`) and in Rust (`domains/invoices/service/refund.rs`,
      including its unit test "1 × 115 refunds 115, not 130"). Add the parity case `invoices/acc-0003-refund-inclusive-vat-exact`
      with a partial refund, then a final refund, on a tax-inclusive sale that has a line discount and an invoice
      discount (the discount order line → invoice → VAT, per 02-accounting-review). Confirm `verify:replay` is green
      for `ACC-0003-refund-vat-inclusive.json`, then set ACC-0003 to `verified`.
- [x] **invoices (08):** (Wave 2: all green mock-vs-Rust) `sale-cash-inclusive`, `sale-split-tender`, `sale-credit-partial-due-date`, `sale-fx-usd`,
      `sale-free-text`, `sale-empties-stock`, `sale-batch-fefo`, `sale-validation-each`,
      `sale-credit-limit-block-and-override`, `refund-partial-then-final`, `refund-writeoff`,
      `refund-customer-credit`, `quotation-save-convert`, `invoice-list-filters-search-paged`,
      `invoice-detail-joins`, `print-sample`.
- [x] **shifts (08b):** (Wave 2: all green mock-vs-Rust) `shift-open-close-exact`, `shift-close-over`, `shift-close-short`, `shift-close-drop`,
      `shift-force-close`, `shift-cash-in-out`, `shift-xreport-with-sales-refunds`, `held-sale-hold-resume-discard`.
- [x] **payments (09):** (Wave 2: all 8 green mock-vs-Rust; the Wave 1 FX invariant blockers were fixed upstream) `payment-receipt-unallocated`, `payment-receipt-multi-invoice`, `payment-supplier-po`,
      `payment-fx-gain-worked-example` (the docs/v2/10 §2 numbers), `payment-allocate-later-fx`,
      `payment-remove-allocation`, `payment-validation-each`, `payment-list-filters-paged`.
- [x] **vouchers (10):** (Wave 2: all green mock-vs-Rust) `voucher-receipt`, `voucher-payment`, `voucher-transfer-fee`,
      `voucher-owner-both-directions`, `voucher-validation-each`, `voucher-list-search`,
      `settlement-card-with-fee`, `settlement-mixed-card-wallet`, `settlement-already-settled-conflict`.
- [x] L3 gate (2026-09-30: lane 43/43, verify:mocks 128/0, verify:replay green; `domain_invoices/payments/vouchers/parties/reports/analytics` 95/95 and `domain_import` 8/8 on the 09:43 build): `bun run parity --lane L3` shows 0 unexplained diffs, and `verify:mocks` and `verify:replay` are
      green. The manager runs `domain_invoices::`, `domain_payments::` and `domain_vouchers::`, and all are green.

## L4 — ledger (Opus) — read `docs/v2/02-accounting-review.md` first

> **L4 status (2026-09-30, Wave 2 — mock vs Rust):** `bun run parity --lane L4` **40/40 green** (was 8/39 in the
> first run; the 40th is `_selftest/identity`); `verify:mocks` 128/0, `verify:replay` clean; `domain_accounting::`,
> `domain_accounting_period::`, `domain_expenses::`, `domain_setup::`, `domain_import::` green on
> `all-20260930-093806.exe`. Rust fixes: non-UUID id text decodes to a never-existing id (`utils/id.rs`,
> unknown ids now reach the mock's NOT_FOUND/VALIDATION refusals instead of an args-decode INTERNAL); importer D-8
> synthetic `created_at` base = `min(savedAt, now) − 1 day` so imported rows sort before rows made under the pinned
> clock (`ORDER BY created_at, id` = mock array order); next-year name fallback = closed year's start year + 1;
> paged journal sort keeps insertion order for ties/unknown keys; journal reads batch-load lines (no N+1 — timeouts);
> account code clash checked before parent rules; `Account.parentId` serialised as `null`; template/shell accounts
> store `requiresParty` only when true; `getOnboardingProgress` projects the mock's 6 keys. UI: `SetupWizardPage`
> records the `ready` step before `finishOnboarding` (Rust ends the bootstrap session on finish, D-1). Cases:
> `settings-revaluation-post` runs as admin (Settings:Write); `setup-wizard-eg` records `done-10` before `finish` and
> allows `phones` `[]` (05-parties D-6); `setup-wizard-sa` allows the journal user fields (02-setup Q-1).
> ACC-0007/ACC-0008 set to `verified` (manager: run `bun run diag`).

> **L4 status (2026-09-29, Wave 1 — authoring, `--mock-only`):** 39 cases written (accounting 12 +
> period close 9 + ACC-0007/0008 = 23 under `cases/accounting/`, expenses 9, setup 6,
> `settings/settings-revaluation-post` 1). `bun run parity --mock-only --lane L4`: 33 green, 6 red, all 6 red
> **only** because of mock invariant defects (no step diffs, no aborts except the harness one below) — reported to
> the manager, not fixed (P4-7 / "report, don't fix"): (1) §4.5 `vat-output`/`vat-input` ignore VAT-settlement entries →
> `vat-settlement-payable`, `vat-settlement-refundable`; (2) §4.6 `customer-/supplier-allocation` ignore opening balances,
> credit expenses and manual party lines → `setup-party-opening-before-after-golive`, `setup-party-opening-reverse`,
> `expenses/create-credit-supplier`; (3) `runAllInvariants` throws on a chart with no role account (empty DB, or the
> `basic` template without card clearing) → `setup-wizard-eg` (base `empty`) is a harness error; `setup-wizard-sa`,
> `setup-coa-switch`, `setup-branches-after-coa` therefore start from the empty-company shell (custom base). Other
> findings worked around in the cases and reported: §4.8 `lock-date` flags every entry before the lock date (cases use
> 2026-04-15, before the demo history); §4.9 3900 checked before onboarding ends; the reclose entry reuses the
> `onboarding-close` source (`one-active-entry`); FX allocate-later breaks three invariants; the demo seeds duplicate
> account ids (`acc-1`/`acc-2` = group "1"/"2" and branch 1111); `clock.ts` `unpinClock` deletes `process.env.TZ`, after
> which Bun ignores every later TZ change (EG cases run in the Riyadh zone in lane runs); demo base ids depend on build
> order (no `resetIdCounters` in `bases.ts`) — L4 cases look uid-generated ids up through services; `s.login` cannot be
> called twice for the same user. ACC-0007 and ACC-0008 cases are green on the mock; the issues stay `fixed` until the
> Rust pass (Wave 2). Allowlist entries: `draft-lifecycle` A-D1, `fiscal-year-crud` P-D1 (outcome-recorded step);
> `template-crud` lists compared unordered (Q6).

- [x] **accounting (12):** `accounting/coa-crud`, `coa-refusals`, `reparent`, `manual-entry-post`,
      `manual-entry-b1-refusals`, `draft-lifecycle` (allowlist the added `postJournalDraft` audit row, A-D1),
      `reverse-manual`, `reverse-refusals`, `journal-list-filters`, `journal-detail-related`, `template-crud`,
      `recurring-post-advance`. Allow `attachmentIds` `[]` vs absent (12 #10) only where it appears.
- [x] **period close (12b):** `fiscal-year-crud`, `lock-date`, `close-year-prechecks`, `close-year`,
      `close-year-next-year-autocreate`, `reopen-year`, `vat-settlement-payable`, `vat-settlement-refundable`,
      `vat-pay`, plus the two ACC verifications (P4-7): `accounting/acc-0007-vat-overlap-conflict` (both of the issue
      file's overlap examples refused with the same `CONFLICT` message; reversing the settlement frees the period)
      and `accounting/acc-0008-reopen-mirror-date`. Set ACC-0007/0008 to `verified` once they are green.
      D-A3/D-A7 are pinned as the mock has them (P4-12).
- [x] **expenses (11):** `create-cash-no-vat`, `create-tax-invoice-15`, `create-credit-supplier`,
      `create-period-locked` (error only, Q1), `category-crud`, `category-delete-refusals`,
      `list-filter-search`, `recurring-crud`, `post-due-and-advance`.
- [x] **setup (02):** `setup-wizard-eg` and `setup-wizard-sa` (base `empty`, `user: null` until the wizard's
      bootstrap login, 03 H-1), `setup-coa-switch`, `setup-branches-after-coa`,
      `setup-party-opening-before-after-golive`, `setup-party-opening-reverse`. Map `'' ↔ admin` as 02 §8(b) says.
- [x] **revaluation (01):** `settings-revaluation-post` (GL after).
- [x] L4 gate: `bun run parity --lane L4` shows 0 unexplained diffs, and `verify:mocks` is green. The manager
      runs `domain_accounting::`, `domain_accounting_period::`, `domain_expenses::` and `domain_setup::`, and all are green.

## L5 — read models (Sonnet)

> **Status note (this wave, mock-only):** all 18 L5 cases written and green under
> `bun run parity --mock-only --lane L5` (19/19 including the shared `_selftest/identity`, 0
> unexplained diffs, 0 harness/invariant problems). Rust host not built yet, so only the
> determinism check (`--mock-only`) ran this wave — Wave 2's mock-vs-rust pass and the
> `domain_reports::`/`domain_analytics::` cargo runs are the manager's next step once `parity_host`
> exists. One real pre-existing mock bug found while authoring `reports/vat-multicurrency-refund`
> (see that file's header comment and the note below) — reported, not fixed, per this wave's scope.
> No allowlist entries beyond the one the plan already calls for (`freetext-fx-demo`).

All L5 cases use base `demo-sa` (plus one `demo-eg` pass each for the statements and home KPIs), with the
clock pinned to the seed's "today" (`2026-06-30`).

- [x] **reports (13):** `reports/statements-full`, `statements-month`, `statements-midyear` (commands 1–14 for
      each of the three ranges), `reports/dims` (commands 1, 2 and 6 × branch, cost center, currency),
      `reports/vat-multicurrency-refund` (a multi-currency invoice and a partial refund, then the VAT report and
      detail), `reports/account-code-order` (codes `"2"`, `"10"`, `"1101"`, R-4). There are no epsilon fields here (13 §8(b)).
      Plus `reports/statements-full-eg` for the header's one `demo-eg` statements pass. Written and green
      (mock-only, determinism check). `account-code-order` uses the seed's actual codes (`"1"`…`"6"` roots,
      two-digit groups, four-digit leaves) rather than the plan's literal `"2"`/`"10"`/`"1101"` example strings,
      since those exact codes don't exist in this chart — the byte-order behavior (R-4) is exercised all the same.
- [x] **operational (13b):** `reports/operational-full`, `operational-month`, `operational-midyear` (commands 1,
      3–7, 11–14), `reports/operational-once` (2, 8–10, 15, 16). Epsilon applies only to `SalesByProduct.qty`,
      `SalesByCategory.qty`, `ReturnsReportRow.qty` and `LowStockRow.suggestedQty` (13 R-7). Added the allowlist entry for the
      `'freetext-fx-demo'` → `freetext-<position>` id (13b §8(b), cited as `13b #8`) on `SalesByProduct.productId` in
      `operational-full`/`-month`/`-midyear`. Written and green.
- [x] **analytics/dashboard (14):** `analytics/commands-defaults` (commands 1–10), `analytics/home-kpis-week-month`,
      `analytics/ymd-arab`. Synchronous mirrored functions are awaited through their async loaders
      (phase A A-5 table), never read through `mirrored()`'s fallback. Plus `analytics/home-kpis-eg` for the
      header's one `demo-eg` home-KPIs pass. Written and green.
- [x] **insights (14b):** `dashboard/insights-latin`, `dashboard/insights-arab` (roles filtered the same way on
      both sides; the backup store is loaded first, I-4), `dashboard/product-hints` (low stock, below cost, dead
      stock, found dynamically from the seeded catalog rather than a hardcoded id). Epsilon applies only to
      `InsightDto.value` for `discount-leak` — the path syntax can't discriminate by `ruleKey` inside the array,
      so the epsilon covers `value` on every insight row in these two cases (harmless on `--mock-only`; a
      narrower expression is the manager's call once Rust exists, per case-file comments). Written and green.
- [x] L5 gate: `bun run parity --lane L5` shows 0 unexplained diffs. The manager runs `domain_reports::` and
      `domain_analytics::`, and both are green. *(Done 2026-09-30: 19/19 PASS mock-vs-Rust on
      parity_host-20260930-125021 after the colon-composite id rule in diff.ts; domain_reports/analytics green in
      the full 400-test run.)*

**Mock bug found (reported, not fixed — CLAUDE.md "if a case exposes a real mock bug, do NOT fix
the mock"):** `checkAllocationsWithinTotal` (`src/mocks/backend/invariants.ts:382`,
`allocations-within-total`) compares a payment allocation's **base-currency** `amount` directly
against an FX invoice's **FC-denominated** `grandTotal`, with no currency conversion. Confirmed
independently of any case: seeding a fresh `demo-sa` snapshot and running
`checkAllocationsWithinTotal` against it (before any new document is posted) already reports `"1
over-allocated: pay-135->inv-634 (48500 > 1000)"` — the seed's own FX worked example
(`seed/branches9.ts`'s `cus-usd-1` USD invoice + its settling payment) trips this invariant as
shipped. `reports/vat-multicurrency-refund` works around it by reusing that pre-existing invoice
(the parity harness's per-case `baseline` already absorbs pre-existing failures) instead of posting
a new FX invoice/payment of its own, which would add a *second* offender and fail the case. Expected
vs actual: expected `passed: true` (or at least a stable pre-existing message), actual `passed:
false` with the offender list growing by one entry per additional FX-invoice payment. No `ACC-`/`BUG-`
issue filed from this lane (out of scope — L2/L3 own the invoices/payments domains this bug lives
in); flagging here for the manager/those lanes to open the ledger issue.

## Gate (phase B2)

- All five lane gates are green, and `bun run parity --bundles` is green. It replays every
  `scripts/verify/cases/*.json` (ACC-0001 to ACC-0008, plus any new one) on both backends.
- `bun run build`, `check`, `verify:mocks`, `verify:replay`, `contract:check`, `diag` then `diag:check` are green.
  ACC-0003 to ACC-0008, and every `ACC-` issue opened in the fix loop, are `verified` (CLAUDE.md "Closing a ledger issue").
- Status note: per lane, the case count, the diffs found by kind (Rust bug, mock bug, allowed), and the
  allowlist entries with their cited ids. That list is the input to the E-6 audit.
