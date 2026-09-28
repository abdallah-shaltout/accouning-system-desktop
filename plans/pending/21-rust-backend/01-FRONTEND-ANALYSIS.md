# 21 · Part 01 — Frontend analysis (the contract the Rust backend must honour)

> **Status (2026-09-27):** 01.A done. Tooling is in place and D3 rounding is applied to the mock.
> D8–D10 are answered (§4). CLAUDE.md gained an "architectural autonomy" rule this session:
> implementation-detail decisions in 01.B/01.C are made directly (recommended/strictest option),
> not asked one-by-one — see `01-frontend-analysis/settings.md`/`setup.md` for the pattern. 01.B:
> `settings.md` done (4 mock fixes: country-aware VAT validation, `deleteTax`/`deletePaymentMethod`
> reference checks, audit rows on destructive writes, `listHistory` → `frontend`). `setup.md` done
> (5 mock fixes: `persistProgress` → `drop`, `applyBranches` duplicate-code now throws, post-go-live
> guards on `setFiscalYear`/`applyPaymentMethods`, `reversePartyOpeningBalance` gets a server-side
> allocation guard + `ApiError` + a symmetric audit row). Remaining carried-forward items (not
> blocking): a duplicate `isBaseCurrencyLocked` check, a hard-coded seed-id lookup in
> `applyCountryTax`, the shared F7 path-string-link pass, and a missing reversal path for the
> step-8 opening entry itself (flagged for whoever implements it in Part 03). `users.md` done
> (1 disposition change: `getDemoAccounts` → `dev-only`, returns plaintext demo passwords; no mock
> code fix needed — F4's real fix, argon2 hashing + a real session/token model, is a Rust design
> question left open as D-U1, not a mock change). `approvals.md` done — smallest module so far,
> already clean (proper `AppRoute` link, correct error codes, full audit trail); no mock fixes, one
> Part 02-F design note about the `ledger:changed` event being reused as a generic cache-invalidation
> signal. `parties.md` done (3 mock fixes: country-aware VAT validation — second and last known
> instance of the `settings.md` regex bug, `linkPartyRecords`/`unlinkPartyRecord` now write activity
> rows, `saveSupplier` gets the same "can't deactivate with a balance" guard `saveCustomer` already
> had). Carried forward: `nextCode()`'s non-concurrency-safe `MAX+1` pattern (Part 02 design note),
> shared F7 path-string links, and two Part 02-B entity-modeling questions (parties table shape,
> `nationalAddress` vs `structuredAddress`). `products.md` done — the largest module (51 fns across
> 4 service files), and the one that owns the `stock` shared-manager reach (`applyStockChange`,
> `receiveBatch`/`consumeFefo` FEFO batches, weighted-average cost in `backend/inventory.ts`/`core.ts`).
> 2 mock fixes: `deleteCustomFieldDef` gets the same reference-check-before-hard-delete guard the
> other catalog deletes already had (was silently orphaning `product.customFields` values), and
> `deleteDraftAdjustment` now writes an activity/audit row (was a completely silent delete). Checked
> for both cross-cutting bugs named for this session and found neither present: no hard-coded
> VAT-regex bug (this module doesn't touch tax numbers) and no plain-`Error`-instead-of-`ApiError`
> gaps. Carried forward, not fixed (policy/design decisions, not narrow bugs): catalog master-data
> writes (categories/units/price-lists/custom-fields) have zero audit trail across ten functions, a
> documentation discrepancy in `00-MASTER-PLAN.md` rule 5 (quantities actually round to 2dp in the
> mock, not 4dp as the rule states), the manager-PIN approval gate trusting a client-sent
> `approvedBy` with no server-side re-verification (F4/D-U1's problem, not this module's), transfers
> emitting only `ledger:changed` not `catalog:changed`, and a missing draft-transfer delete endpoint.
> `purchases.md` done (12 functions, all `port`, no disposition changes) — **zero mock fixes applied**,
> the first module this session with none: every named bug class (Saudi-only VAT regex, a hard-delete
> with no reference/state check on `cancelPurchaseOrder`, silent writes, plain `Error` instead of
> `ApiError`, silent-skip-instead-of-throw) was checked and found either not applicable or already
> correctly handled (`cancelPurchaseOrder` already refuses a `RECEIVED`/`ORDERED`-only-window
> correctly; all 30 throw sites in `backend/purchases.ts` use `ApiError`; every write calls
> `logActivity`, which itself writes the audit row). Confirms `products.md`'s stock-module patterns
> apply directly here too (`applyStockChange`/`receiveBatch` reused as-is, same 2dp-not-4dp qty
> rounding, same "one shared counter across branches, prefixed" numbering shape for
> `purchaseOrder`/`stockTransfer`). Two accounting-sensitive spots were deliberately **not** touched
> and left as open questions instead of guessed-at fixes: `PurchaseOrder.currency`/`exchangeRate`
> fields are carried by the types but never read by any posting code in `purchases.ts` (inert, not
> wired to FX conversion — a scope question for Part 03, not a bug); and no posting function in this
> module (`receivePurchaseOrder`, `createPurchaseReturn`, `postDebitNoteDraft`) emits
> `ledger:changed`/`catalog:changed` at all (only `parties:changed`), a stronger version of the
> event-coverage gap `products.md` flagged for transfers. Also found a new concurrency requirement
> `products.md` didn't need: receiving/returning against a PO needs a purchase-order-row lock (not
> just the product-row lock already specified), since the "how much is remaining/returnable" check
> reads `received_qty`/prior returns before writing the new cumulative value. `invoices.md` done —
> the most accounting-sensitive module reviewed so far (createSale/createRefund posting engine, FEFO
> + weighted-average cost consumption, split-tender payments, FX sales, POS shifts with cash
> over/short + optional cash-drop transfer voucher). **Zero mock fixes applied**, matching
> `purchases.md`'s outcome: every named bug class (Saudi-only VAT regex, hard-delete with no
> reference check, plain `Error`, silent-skip-instead-of-throw) was checked — including the two
> call sites named in the brief (`discardHeldSale`/`resumeHeldSale`) — and found either not
> applicable or judged intentional (a no-op discard of an already-gone held sale reads as idempotent
> delete semantics, not a swallowed validation error). Confirmed reuse, not re-derivation, of
> `products.md`'s stock/FEFO concurrency notes and `settings.md`'s FX/`ExchangeRate` scale — this is
> the first module where FX is actually posting-tested (`purchases.md`'s FX fields were inert;
> here `prepareSale`/`recordSale` genuinely convert FC sales to base via `convertLinesToBase`'s
> largest-line rounding-gap rule). Two accounting-sensitive spots were deliberately **not** touched
> and left as open questions: `createRefund`'s cash-drawer shift-attribution fallback
> (`sales.ts:529`) can log a cash refund's drawer movement against the wrong terminal's open shift
> when the original sale's shift has since closed (never wrong on the GL side, only which shift's
> X/Z report sees it); and whether `closePosShift` even *can* be "reopened" was analyzed in depth per
> the task's explicit ask — concluded **no**, since a close can conditionally post both a variance
> entry and a separate transfer-voucher document, and a new shift may already be open on the same
> terminal by the time anyone notices a bad close, so the correct fix is a manual correcting journal
> entry, not a new undo code path. Also found a **new** concurrency requirement neither
> `products.md` nor `purchases.md` needed: a customer-row lock for `createSale`'s credit-limit
> check-then-post (the credit check reads a live-computed `customerBalance()`, not a locked
> counter, so two terminals selling to the same near-limit customer could both pass a stale check).
> Confirmed several held-sale/quotation-status writes have no audit trail at all
> (`holdSale`/`resumeHeldSale`/`discardHeldSale`, `setQuotationStatus`) plus `recordCashInOut` (real
> drawer cash, no audit row) — none fixed, all recorded as an open question in §9 rather than
> guessed at, since none is a destructive action on persistent business data (the bar
> `products.md`/`settings.md`'s applied fixes met). `payments.md` done — the receipt/disbursement
> voucher module with sub-ledger allocation and realized-FX math (docs/v2/02-accounting-review.md C1,
> docs/v2/10 §2). One safe fix applied: `getPayment` now throws `ApiError(..., 'NOT_FOUND')` instead
> of a plain `Error` (verify:mocks baseline 128/0 unchanged before/after — the fix touches nothing
> accounting-related). One genuine open question left in §9, not guessed at: `removeAllocation`
> doesn't reverse a prior FX-adjustment journal entry an allocation may have triggered via
> `allocateExistingPayment`'s conditional FX posting — deciding whether removal should refuse outright
> or implement a real reversal is a scope/design choice, not an implementation detail. Also flagged: a
> `PaymentFilter.to` field mistagged `_route_` by the contract generator's naming heuristic (it's a
> plain date-range bound, not a route — a generator-hint limitation, not a mock bug) and a naming
> collision to avoid in Part 02 — this module's narrow 3-variant `PaymentMethod` tender-kind type is
> a different type from `settings.md`'s 6-variant `PaymentMethod` configurable-tender entity. `vouchers.md`
> done — the general-voucher (RECEIPT/PAYMENT/TRANSFER/OWNER "fast journal") and card/wallet-settlement
> module, which clears the `cardClearing`/`walletClearing` account `payments.md`'s and `invoices.md`'s
> control-account postings never touch. **Zero mock fixes applied** — the third module this session
> (after `purchases.md` and `invoices.md`) with none: every named bug class was checked and found
> already correct, including a clean sweep on writes-with-no-audit-trail (all 5 write functions call
> `logActivity`, which also writes the structured `AuditEntry`) and plain-`Error`-instead-of-`ApiError`
> (every throw site already uses `ApiError`, including both `getXById` lookups). This module is also
> the **exception** to the `ledger:changed`-skipping pattern every other posting module this session
> has shown: all 5 write functions here correctly emit it. Modeled the 4-kind `Voucher` discriminated
> union as one entity table with nullable kind-specific columns (matches the mock's single flat
> `db.vouchers` array and one shared `number` counter — a `UNION ALL` across 4 tables would buy
> nothing since `getVouchers`/`getVoucher` never filter on a kind-specific column). Confirmed reuse of
> `payments.md`'s "second, expected instance" of the `PaymentFilter.to` route-hint false positive
> (this module's `VoucherFilter.to` has the identical issue) and found a **new** instance of the same
> class of generator-hint imprecision: `UnsettledTenderGroup.tenderCount` is tagged `_decimal_` but is
> actually a plain integer count. Found a genuinely new concurrency requirement neither `payments.md`
> nor any earlier module needed: `createCardSettlement`'s "is this {date, paymentMethodId} pair
> already settled" check reads the full existing-settlements snapshot before validating, so two
> terminals could double-settle the same tender group — recommended a **unique DB constraint** on
> `card_settlement_groups(date, payment_method_id)` rather than a row lock, since the DB itself can
> refuse the second insert with no extra locking logic needed. One genuine open question left in §9,
> not guessed at: whether card/wallet settlements should ever be reversible, and if so how "unsettling"
> would interact with the current permanent-once-settled `settledKeys()` lookup — a scope/design
> decision for Part 03, not an implementation detail, with no accounting bug today since nothing
> exercises an unsettle path. `expenses.md` done — the simplest posting module reviewed so far: a
> one-shot, one-line "spend money" document (Dr expense [cc] + VAT input / Cr a payment method's
> system-role account or Cr `payable[supplier]`) plus a recurring-expense template/due-list. One
> mock fix applied: `deleteExpenseCategory`'s "can't delete a protected category" refusal now throws
> `ApiError(..., 'FORBIDDEN')` instead of the default `VALIDATION` code, matching `settings.md`'s
> already-fixed `deletePaymentMethod`/`deleteTax` pattern for the same class of refusal (`verify:mocks`
> baseline 128/0 unchanged before/after). The task brief's named check-target, `deleteExpenseCategory`'s
> reference check, turned out to be **already correct** (it already refuses when any `Expense`
> references the category, with the right `CONFLICT` code) — not a gap. Confirmed reuse, not
> re-derivation, of `settings.md`'s `deletePaymentMethod` reference-check (which already covers
> `db.expenses.some((e) => e.paidFrom.kind === 'method' && ...)`) and `payments.md`'s/`vouchers.md`'s
> `PaymentFilter.to`/`VoucherFilter.to` route-hint false positive (`ExpenseFilter.to` is the third,
> expected instance). Found one genuine, un-guessed bug-class hit: `splitTax` silently falls back to
> `rate = 0` when `taxId` doesn't resolve to an active tax, instead of throwing — deliberately **not**
> fixed since it touches VAT math and it's unclear whether the (unreviewed) form already prevents this
> state, so it's recorded as a §9 open question instead. Also found `deleteRecurringExpense` has no
> reference check against `Expense.recurringTemplateId` at all, but on inspection this is **not** the
> same bug class as the settings/category checks — `recurringTemplateId` is a write-once historical
> breadcrumb no query ever reads back, so nothing can be orphaned — recorded as considered-and-correctly-
> unnecessary, not fixed. Carried forward, not fixed (policy/design decisions, not narrow bugs): four
> master-data writes (`saveExpenseCategory`, `deleteExpenseCategory`, `saveRecurringExpense`,
> `deleteRecurringExpense`) have zero audit trail, the same cross-module policy question `products.md`
> already carried forward for ten of its own catalog functions, not re-fixed piecemeal here; a
> `parties:changed` event-coverage gap on the credit-to-supplier expense path (same already-documented
> cross-module event-skipping class); and inert fields (`Expense.branchId`,
> `ExpenseCategory.defaultTaxId`/`defaultCostCenterId`) matching `purchases.md`'s "captured but not
> wired" pattern. The standout open question: `RecurringExpense.autoPost` is a fully persisted,
> UI-editable field that **no backend code anywhere reads** — every due template, regardless of
> `autoPost`, waits for an explicit `postDueRecurringExpense` click; the source doc itself hedges
> ("`autoPost` is optional"), so whether this should ever become a real background-job feature (D1's
> Main-Server-per-branch would host it) is left as a genuine, unresolved product decision for Part 03,
> not silently assumed either way. Also found a new concurrency requirement neither `payments.md` nor
> `vouchers.md` needed in quite this shape: two terminals both posting the same due recurring-expense
> template concurrently (a double-click race, not a routine multi-terminal operation) could double-post
> the expense while only advancing `nextDate` by one month — needs a locking read + a live
> `active && nextDate <= today` re-check inside `postDueRecurringExpense`'s own transaction, not just an
> existence check. `accounting.md` done (33 functions) — **the posting engine every other module
> reviewed this session already assumed exists**: chart-of-accounts CRUD, manual journal entries
> (create/draft/edit/post/delete), the general-purpose `reverseJournal` (MANUAL-only, per the master
> plan's earlier finding), fiscal-year lifecycle (`closeYear`/`reopenYear`, the one pair in the whole
> app besides `settings.md`'s branch activate/deactivate where the compensation is a named,
> purpose-built function rather than the generic reversal registry), journal templates + recurring
> postings, and VAT settlement/payment. Read `docs/v2/02-accounting-review.md` in full per the task's
> explicit instruction, plus `invariants.ts` to ground the concurrency/undo sections in what
> `verify:mocks` actually checks. **Baseline `bun run verify:mocks`: 128 ok, 0 failed, confirmed fresh
> before any edit** (matches every prior module this session) — **one mock fix applied** (the same
> master-data audit-trail-gap class already fixed in `settings.md`/`parties.md`/`expenses.md`:
> `createOrUpdateJournalTemplate`/`removeJournalTemplate` now write an activity/audit row via
> `saveJournalTemplate`/`deleteJournalTemplate`, the latter gaining a `userId` parameter), confirmed
> **unchanged after the fix: 128 ok, 0 failed**. Per this module's explicit "maximally conservative"
> mandate, **everything else found was deliberately left untouched and written up in §9 instead**,
> producing more open questions than any other module this session — the correct, expected outcome
> for the accounting core: `postJournalDraft` posts a previously-validated draft with **no
> re-validation of the lines' account state at post time** (only the period check) and **no audit
> trail at all** (the one posting function in the whole app with zero `logActivity` call — deliberately
> not touched together with its two sibling draft functions, which share the same silent-audit gap but
> have zero GL effect of their own); `submitVatSettlement` has no guard against being called twice for
> overlapping periods (would double-close the same VAT movement into `vatPayable`); `payVatSettlementNow`
> has no check against the current `vatPayable` balance; `closeYear`/`reopenYear`'s "compute totals →
> post closing entry → flip `isClosed`" sequence is this module's (and arguably the whole plan's)
> sharpest identified concurrency risk — a concurrent poster landing between the totals computation and
> the `isClosed` flip would both corrupt the just-computed closing entry and slip an entry into a year
> about to lock, with no error to anyone, recommending that `assertOpenPeriod`'s fiscal-year lock become
> a genuine serialization point (`SELECT ... FOR UPDATE` on every posting's read of it, not just
> `close_year`'s own); and `JournalTemplate.recurrence.day` has no `[1,28]` clamp the way
> `expenses.md`'s `RecurringExpense.day` already does, so a template's `nextDate` can silently drift
> across month-length boundaries under JS `Date`'s native rollover. Also confirmed several things as
> genuinely correct rather than gaps: `deleteAccount`/`saveAccount`'s system-account protections are
> already fully reference-checked (only the error *code* — `VALIDATION` not `FORBIDDEN` — is a
> candidate a human could likely wave through, deliberately left as a §9 note rather than auto-applied
> given this module's conservative mandate); drafts skipping `assertOpenPeriod` entirely is intentional
> (nothing has GL effect until `postJournalDraft`, matching every other module's only-check-at-actual-
> posting-time pattern); and `JournalEntry.templateId`/removed-template reference safety is the same
> confirmed-not-a-violation shape `expenses.md` already established for `recurringTemplateId`. Found
> the second confirmed instance of `expenses.md`'s "reversing a posted recurring entry doesn't roll
> back the template's `nextDate` advance" compound-undo gap (here for `postRecurringTemplate`), and the
> fourth instance of the by-now-expected `Filter.to` route-hint false positive. `reports.md` done (32
> functions) — the first fully read-only module reviewed this session: **0 writes**, **0 shared-manager
> reach** (no ledger/stock/activity/numbering/period/currency call anywhere), and consequently §4 (undo
> matrix) is "n/a — no writes" for all 32, correctly, not a shortcut. This module also has no
> `src/mocks/backend/reports.ts` — every function lives directly in `reportService.ts` (1199 lines),
> reading `db.*` tables straight, plus read-only reach into `accounts.ts`/`balances.ts`/`payments.ts`.
> **Zero mock fixes applied** — the fourth module this session with none (after `purchases.md`,
> `invoices.md`, `vouchers.md`): every named bug class (plain `Error`, bad date-range boundaries,
> rounding-rule mismatches) was checked and found either not present or, for rounding, a genuine
> pre-existing structural fact rather than a bug — §7 (aggregations, the focus section for this module)
> documents that two different rounding *orders* coexist across the module's own functions
> (`computePnl`'s round-per-line-then-sum vs. `getSalesReport.byProduct`'s sum-then-round-once), both
> correct as implemented, and that `getVatReport`'s document-derived vs. ledger-derived VAT figures
> (and `getCashFlowStatement`'s `netChange` vs. `closingCash − openingCash`) are pairs of independently
> computed aggregations expected to reconcile but not mechanically tied together — the same
> relationship `accounting.md §7` already found between `getVatPeriodTotals` and `checkVatControl`
> (§4.5), now confirmed present in two more places, flagged as a design note for Part 03's SQL, not a
> bug to fix by deriving one side from the other. Also the first module this session with **zero F7
> path-string-link violations** (every route field already emits a proper `AppRoute`). Genuine open
> questions left in §9 (not guessed at): whether `getSalesReport`'s heuristic `inclusive`-invoice
> re-derivation should instead read a stored field on `Invoice` (deferred to cross-check against
> `invoices.md`), and that `getGrossProfitReport`'s `groupBy==='invoice'` vs `'product'`/`'category'`
> paths (and `getReturnsReport`'s `byProduct` vs `byReason`/`byCashier`) use genuinely different
> formulas that don't necessarily sum to the same total — existing, intentional behavior, flagged so
> Part 03 doesn't collapse them into one shared query. `analytics.md` done — the smallest module
> after this one's own 3-function count: read-only dashboard charts (sales trend/weekday/payment-mix,
> product profitability, customer new-vs-returning/concentration), no `src/mocks/backend/*` file at
> all, no writes, no shared-manager reach, and **no error paths of any kind** — every degenerate case
> (empty window, zero denominator) falls back to a graceful "not enough data yet" Arabic string
> rather than throwing. Zero mock fixes needed; one routine open item in §8 (not a design fork): no
> `days`/`limit` bounds validation on any of the 3 functions, flagged as an IPC-boundary hardening
> task for whoever implements the Rust commands, not a mock bug (nothing in the UI ever calls these
> out of range today). Confirmed product-profit math correctly uses each invoice line's
> point-in-time `costPrice` snapshot (historically accurate WAC), not a live re-read — only the
> product's display name/SKU are looked up live, which is cosmetic. `core.md` done (35 functions) —
> the "shared core" grab-bag module, the most heterogeneous reviewed this session: dashboard KPIs
> (`dashboardService.ts`), the insight rules engine (`insightEngine.ts`), dev-only reset/reseed
> tooling (`devToolsService.ts`), thermal printing (`printService.ts`), Typst PDF/label rendering
> (`pdfService.ts` — 8 of its functions already `rust-existing`), and the shared native-save-dialog
> helper (`saveFile.ts`), plus the module that owns `AppRoute`/`RouteName`, `Address`,
> `ActivityKind`/`ActivityEntry`, `PagedQuery`/`PagedResult`/`PageSort` and the command-palette
> types. **Baseline `bun run verify:mocks`: 128 ok, 0 failed, confirmed fresh before any edit** —
> **two disposition overrides applied, no mock code touched**: `resetToEmpty`/`reloadDemoData` →
> `dev-only` in `scripts/contract/config.ts` (same class as `users.md`'s `getDemoAccounts`, F9:
> `resetToEmpty` destroys all data with no undo, `reloadDemoData` reseeds/overwrites nearly every
> table — both a dev/demo capability, never a production command), confirmed **unchanged after:
> 128 ok, 0 failed**. Confirmed all 8 already-`rust-existing` functions (`renderGenericReport`,
> `renderGenericReportAndSave`, `renderLabels`, `renderLabelsAndSave`, `renderLabelsPreview`,
> `renderPreview`, `renderReportPdf`, `saveReportPdf`) have **zero contract gaps** against
> `AGENT_MEMORY.md`'s IPC table: every one calls `render_pdf` or `render_preview`, both registered
> in `generate_handler!`, both defined in `src-tauri/src/pdf/render.rs`, both already documented
> with this file as their real caller — likewise confirmed `printReceipt`/`testPrint` against the
> already-registered `print_thermal_receipt`/`print_test_receipt`, no gap. Did **not** close the
> `nationalAddress` vs `structuredAddress` question `settings.md`/`parties.md` left open: `Address`'s
> own doc comment calls `NationalAddress` "deprecated" in its favor, but the mock still writes and
> reads both fields everywhere today with no migration path in this module's own code, so forcing a
> decision here would guess at a data-migration/product call that belongs to `parties.md`/
> `settings.md`'s Part 02-B entity design — left open, with a recorded recommendation, not force-
> closed. Confirmed `insightEngine.ts`'s dismiss/snooze/clear functions stay correctly `frontend`
> (F2, per-device `localStorage`, not re-litigated) and `saveFile.ts` stays correctly `frontend` (the
> CLAUDE.md rule-21 shared save helper, calling Tauri's own plugins directly with no custom command
> to port). Flagged the command-palette types (`PaletteCommand.when`, `PaletteResult.run`,
> `PaletteSearchProvider.search`) as **intentionally DTO-less** — function-typed fields that can
> never cross an IPC boundary, not an oversight — and cross-referenced `PagedQuery`/`PagedResult`/
> `PageSort` to 01.D's not-yet-written `cross-cutting.md`, which owns the final paging contract.
> Two genuine findings **not** fixed, recorded in §8 instead per this session's established bar
> (destructive/reference-integrity gaps get fixed, low-stakes preference/diagnostics gaps get
> flagged): `setThresholds` has no audit trail (a UI-tuning preference with zero accounting effect,
> unlike the destructive master-data writes fixed in other modules), and six dashboard-read functions
> (`getInTransitTransfers`, `getPendingApprovalRequests`, `getLastBackupFailedAt`,
> `getJournalDraftCount`, `getStockValueSnapshot`, `hasAnyProducts`) are missing the `wrap('core.…', …)`
> diagnostics-seam call every other function in the 341-function inventory has. One open, non-blocking
> note in §9: `TopCustomerRow`'s dashboard ranking uses gross sales while `HomeKpis.netSales` nets
> against refunds — a pre-existing, cosmetic inconsistency between two widgets, flagged so Part 03's
> SQL doesn't "fix" it into a silent behavior change. `templates.md` done — print-template CRUD, D9's
> localStorage→DB move (every function was already `port`-overridden under D9 before this review;
> confirmed the override is correct and complete, not a fresh disposition call). One fix applied:
> `importTemplate` now throws `ApiError(..., 'VALIDATION')` instead of a plain `Error`. Two module-wide
> policy questions left in §9, not guessed at: whether `deleteTemplate` should refuse deleting the
> last/current-default template of a kind (no reference check exists today), and whether every write
> in this module should get an audit trail now that templates are shared per-branch DB data instead
> of one device's local preference (recommended yes for `saveTemplate`/`deleteTemplate`/`setAsDefault`
> at minimum, not applied since it's a batch scope decision). Also flagged for the not-yet-written
> `cross-cutting.md` (01.D): a cross-terminal refresh signal is needed so one terminal's default-template
> change reaches every other terminal in the branch. `diagnostics.md` done — the actual last module in
> the 01.B build order (14 functions: the accounting debugger, the audit-log reader, and the support-
> bundle export). Answered this module's own stated task (F9, "decide which read endpoints Rust serves
> in debug builds"): 8 functions overridden to `dev-only` (posting trace/invariants/drift/explain/raw-
> entry/balances-around/recent-documents/repro-recording-start — everything CLAUDE.md's own text scopes
> to "`/dev/diagnostics` (dev builds)"), while `getAuditEntries`/`getAuditEntities`/`exportSupportBundle`
> were deliberately kept `port` (production), since they back genuinely user-facing features (the audit
> log, and the "تصدير ملف التشخيص" support-bundle export) — not everything under this module is a dev
> tool, and conflating the two would have wrongly hidden a real feature behind a debug flag. Zero writes
> anywhere in this module, so no accounting risk from the disposition changes; `verify:mocks` confirmed
> unchanged (128/0) since only `scripts/contract/config.ts` was touched, no mock code. One genuine open
> item in §9, not guessed at: `startReproRecording`'s "clone the whole in-memory db, then record every
> service call" mechanism has no obvious 1:1 Rust equivalent once the DB is real — flagged for whoever
> designs Part 04's repro/replay tooling, not resolved here. **01.B (per-module contract review) is now
> complete — all 17 module files done.** Remaining before Part 01's own gate is green: 01.C (the shared
> F7 path-string-link conversion pass — every module this session recorded its own instances but none
> were converted in bulk) and 01.D (the not-yet-written `cross-cutting.md`: auth/session, terminal
> identity, the settings branch/device split, the `AppError` catalogue, events, paging, dates, the
> table→entity ownership map, and the D10 import spec). Parts 02–04 are not written until Part 01's
> overall gate (§6) is green.
>
> **01.C done (2026-09-27).** Re-ran `bun run contract` fresh at the start: 341 fns (296 port)
> unchanged, confirming no drift since the 01.B session. Converted every path-string link (the 61 in
> the original inventory, plus 2 more the generator's heuristic had missed — a query-string shape,
> `` `/payments?highlight=${id}` ``, in `payments.ts`'s `recordPayment`/`allocatePayment`) across 21
> files: `src/mocks/backend/{approvals,branches,core,currency,expenses,inventory,journal,opening,
> payments,purchases,revaluation,sales,settlements,shifts,transfers,vouchers}.ts`,
> `src/mocks/seed/history.ts`, `src/modules/accounting/services/accountingService.ts`,
> `src/modules/parties/services/partyService.ts`, `src/modules/products/services/
> {inventoryService,productService}.ts`, `src/modules/settings/services/settingsService.ts`,
> `src/modules/users/services/userService.ts`. Every existing `/* route-ok: ... */` escape-hatch
> comment on these call sites (added under Decision 8 when the type was still `string`) was removed,
> since the field is now `AppRoute`-typed and the comment's premise (a string the type checker can't
> verify) no longer applies. Plumbing: `ActivityEntry.link` (`core/types/index.ts`) and
> `AuditEntry.link` (`diagnostics/types/index.ts`) retyped `string` → `AppRoute`; `logActivity`'s
> `link` param and `LogAuditInput.link` retyped the same way in `mocks/backend/core.ts`.
> **`entityFromLink` rewritten**: the old code split the link string on `/` and took the last two
> segments as `entityId`/`entity`; the new `ROUTE_NAME_TO_ENTITY` lookup (route name → entity string,
> a new table distinct from the existing `ENTITY_TO_ACTIVITY_KIND`, since route names don't map 1:1
> to entity strings — e.g. route `purchase` → entity `purchaseOrder`, route `journal-entry` → entity
> `journal`) reads `link.name` + `link.params.id` directly off the `AppRoute` object; a link with no
> `entity` mapping or no `id` param (every list-route link: `approvals`, `journal-templates`,
> `journal`, `fiscal-years`, `settings-*`, `pos-shifts`, `movements`, `card-settlements`, `accounts`,
> `transfers`) falls back to `{ entity: kind, entityId: uid('unk') }`, exactly the old code's
> no-parseable-segment fallback. Three judgment calls, each recorded where it was made: (1)
> `opening.ts`'s bare `/inventory` stock-opening link → `{ name: 'movements' }` (no route named
> exactly `/inventory` exists; `movements` is the general inventory-ledger view, the closest match
> for a stock-in event not tied to one adjustment/count/transfer document); (2)
> `settlements.ts`'s `/payments/settlements/${id}` → `{ name: 'card-settlements' }` (no per-settlement
> detail route exists, only the list route — confirmed by reading `route-map.gen.d.ts`, not guessed);
> (3) `transfers.ts`'s 4 `/inventory/transfers/${id}` links → `{ name: 'transfers' }` (confirmed by
> reading both the route map and `products/routes` — `transfers` has no `:id` child, and
> `StockTransferListPage.vue` is the only transfer page that exists, no detail page under any other
> name). Confirmed already-correct and untouched: `logService.ts:55`'s `'/src/'` literal is a
> stack-trace source-path prefix (`firstAppFrame`'s frame-matching regex), not a navigation route —
> a confirmed false positive of the generator's string-literal heuristic, left in place; `bun run
> contract`'s final path-string-links table shows exactly this one entry, 0 real violations.
> `actionTo`/`sourceLink`/`refLink` (CLAUDE.md rule 25's other named examples) were grepped
> repo-wide and found **already** `AppRoute`-typed everywhere they exist (`useNotifications.ts`,
> `insightEngine.ts`/`insightRules.ts`, `accountingService.ts`'s `JournalRow.sourceLink`,
> `inventoryService.ts`'s `StockMovementRow.refLink`) — no fix needed, confirmed not guessed.
> `ApprovalsPage.vue`'s `RouterLink :to="row.link"` needed no change (already object-compatible);
> no other reader did string manipulation on `.link` that would have broken. Two carried-forward
> `setup.md` §8 items were judged safe and applied: the duplicate `isBaseCurrencyLocked` check
> (`setupService.ts` now calls `currency.ts`'s function instead of re-implementing
> `db.journalEntries.length > 0`) and `applyCountryTax`'s hard-coded `tax-vat-out`/`tax-vat-in` seed-id
> lookup (now matches by `t.accountRole === 'vatOutput'`/`'vatInput'`) — both no-behavior-change
> mechanical dedups, confirmed via `verify:mocks` unchanged before/after each edit. No other carried-
> forward item from any of the 17 module files was judged narrow/safe enough to apply here (the rest
> are policy/design/scope questions explicitly out of 01.C's scope, e.g. audit-trail gaps,
> `removeAllocation` FX reversal, `closeYear`/`reopenYear` concurrency). **Final gate state:**
> `bun run build` green, `bun run check` green (`check-routes: no path-string navigation targets
> found`; the 81 pre-existing `check-ui-rules` warnings are unrelated to this task and don't fail the
> build), `bun run verify:mocks` 128 ok / 0 failed (unchanged throughout every batch), `bun run
> contract` 341 fns (296 port) unchanged / 0 real path-string links, `bun run contract:check` green,
> `bun run memory` regenerated (804 files, 17 modules, 122 routes, 12 Rust commands, 0 new seam
> violations) and `bun run memory:check` green, `bun run diag:check` green (21 issues, not stale).
> Per plan doc 21's own e2e-timing override (00-MASTER-PLAN.md §8), the e2e suite was **not** run for
> this task — it runs once at the end of the whole plan. 01.D (`cross-cutting.md`) is still pending;
> Part 01's overall gate is not yet green.
>
> **01.D done (2026-09-27).** Wrote `01-frontend-analysis/cross-cutting.md` (8 sections, matching
> `01-FRONTEND-ANALYSIS.md` §5's bullet order): **(1) auth/session** — closes D-U1 (`users.md` §9) by
> applying D8 rather than re-litigating it: "session" is process-local `AppState` on each terminal's
> own Tauri process (no HTTP layer, no bearer token), passwords hashed with `argon2`, credentials keyed
> by the immutable `user_id` UUID (`credentials(user_id, password_hash)`) instead of the mock's
> mutable-username map (fixes `users.md` §8's rekey fragility as a side effect), `restoreSession`
> dropped as a Rust command (no client-held id to restore from once the process itself is the session).
> **(2) terminal identity** — a new `terminal.json` file in the Tauri app-data dir (UUIDv7, generated
> once), read into `AppState.terminal_id`; held sales/open shift/thermal printer config are per-terminal,
> confirmed compatible with every module's already-recommended lock (`invoices.md`/`products.md`/
> `purchases.md`/`vouchers.md`/`templates.md` §5) — no lock is ever keyed by `terminal_id` itself.
> **(3) settings split (F11)** — `settings.md` had only prose notes on `backup.folder`, not the actual
> per-field table F11 asked for; that table is finished here (every `StoreSettings` field classed
> branch/device), plus a new `device-settings.json` file (connection string, printer config, backup
> folder) and the one merged `settingsService` contract (pages unchanged). Flagged `StoreSettings.theme`
> as likely dead (superseded by `useAppearance`/`useTheme`'s already-local theme handling) rather than
> silently modeling an unused device field. **(4) `AppError`** — closes with a concrete Rust enum
> (`core/error.rs`, serde tag `code`, `SCREAMING_SNAKE_CASE`) serializing to the same `{ code, message }`
> shape the mock already produces, confirming the frontend's existing `ApiError`-catching/`log.error`/
> `E-XXXX`-toast code needs **zero changes** — direct payoff of F5 already matching on both sides.
> **(5) events** — confirmed `MockEvent`'s 3 names carry **no payload** (`events.ts`); designed the
> `change_versions(category, version)` table + per-terminal poll D8 named but didn't detail, decided
> **not** to add a 4th `templates:changed` category (reuses `catalog:changed` instead, closing
> `templates.md` §6/§9's open ask), and pulled forward a consolidated event-coverage-gaps table from
> `purchases.md`/`products.md`/`expenses.md`'s own findings (Part 03 to-do, not re-investigated).
> **(6) paging/search** — `PagedQuery`/`PagedResult` map to `LIMIT`/`OFFSET` (not keyset, matching the
> existing page-number UI); decided (architectural autonomy) to port `normalizeArabic` as Rust code
> plus a generated indexed `search_normalized` column per searchable entity, rather than hunting for a
> MariaDB collation that doesn't actually exist for hamza/alef-unification + Arabic-Indic-digit folding.
> **(7) dates** — confirmed `localDateKey`/`inDateRange` have **no explicit timezone concept today**
> (browser/OS-local only, exactly as the task brief predicted) — a genuine gap, not a prior decision;
> applies "business-local `DATE`, instants UTC `DATETIME(3)`" via the Main PC's OS timezone (D1: one
> Main PC per branch), and **recommends** (not applies — mock code is out of scope for 01.D) a new
> `StoreSettings.timezone` field for Part 02 to remove the "every terminal's OS clock must agree"
> assumption. **(8) table→entity map** — all 46 `MockDb` tables (cross-checked against
> `docs/backend/contract/README.md`'s generated table list and `db.ts`'s interface directly, not
> hand-typed) plus the not-yet-existing 47th, `print_templates` (D9), each marked top-level/child-of-X/
> settings-blob. **(9) D10 import spec** — source confirmed from `persist.ts` (IndexedDB `mock-db`/
> `snapshot`/`current`, `SCHEMA_VERSION = 1`, empty `migrations` map today); explicitly notes 01.C
> running first means the importer's route-link handling is now a trivial id-remap (no path-string
> parsing needed) — the direct benefit the task brief predicted; decided the multi-branch "which branch
> owns the imported print templates" edge case needs a one-time picker UI rather than a silent guess.
> Two items flagged as **recommended Part 02 additions, not settled contract** (named explicitly, not
> hidden): the `StoreSettings.timezone` field and the `credentials(user_id, password_hash)` table shape.
> **No conflict found** between any cross-cutting finding here and an already-answered decision
> (D1–D3, D8–D10) or a per-module 01.B finding.
>
> **Part 01's overall gate (§6) is green as of 2026-09-27.** Every fast gate was re-run fresh at the
> end of this session (not assumed from 01.C's earlier run): `bun run contract:check` → "up to date:
> 341 service fns (296 port), 46 tables, 254 types"; `bun run verify:mocks` → 128 ok, 0 failed;
> `bun run build` → green; `bun run check` → routes clean ("no path-string navigation targets found"),
> the 81 pre-existing `check-ui-rules` warnings unchanged and non-blocking (as in 01.C); `bun run memory`
> regenerated (805 files, 17 modules, 122 routes, 12 Rust commands, 0 new seam violations) and
> `bun run memory:check` green; `bun run diag:check` → "up to date (21 issue(s))"; `docs/diagnostics/
ISSUES.md` grepped directly for `ACC-` — **zero matches**, confirmed no open `ACC-` issue exists at
> all. Also noted, not silently fixed: this doc's own §6 gate text says "16 module files" but 01.B's
> task list and every status note above show **17** files done (`settings, setup, users, approvals,
> parties, products, purchases, invoices, payments, vouchers, expenses, accounting, reports, analytics,
> core, templates, diagnostics`) — a pre-existing doc-count typo, corrected in §6's checkbox text with
> the count explained, not "fixed" by deleting a file. Per this task's constraints, the plan folder is
> **not** moved to `plans/completed/` and Part 02 is **not** started — that decision is left to the
> user/orchestrating session. The natural next step is `02-CORE-AND-SHARED-ARCHITECTURE.md`.

## 1. Purpose

Turn the frontend and its mock backend into a **written, generated contract**: for every one of the
339 service functions, what it takes, what it returns, what data it reads and writes, which shared
manager it needs (ledger / stock / activity / numbering / currency / period), and what the Rust
side does with it. Part 02 designs entities and `shared/` from this, and Part 03 ports domains from
it. Nothing here is typed from memory. The generator produces the facts and reviewers confirm
them.

**Source of truth, in order:** the mock backend's behavior (`src/mocks/backend/*`,
`docs/v2/02-accounting-review.md`) → the service function's signature and return shape
(`src/modules/<m>/services/*`) → the module types (`src/modules/<m>/types`). Pages are never read
for contract purposes, since they only consume services.

## 2. Inputs (all generated or already in the repo)

| Input | What it gives | How to refresh |
|---|---|---|
| [`docs/backend/contract/README.md`](../../../docs/backend/contract/README.md) | Index: per-module counts, shared-manager matrix, table read/write map, path-string links | `bun run contract` |
| `docs/backend/contract/<module>.md` | Every service fn: params, return type (inferred by the TS checker), suggested disposition, tables written/read (transitive), shared managers, DTO types, plus every module type with field hints (_decimal_, _uuid_, _date_, _route_, _enum_) | `bun run contract` |
| [`docs/backend/contract/mocks.md`](../../../docs/backend/contract/mocks.md) | All 240 mock-engine functions: tables read/written, mock calls, **round2/round4 counts** (the exact rounding points Rust must reproduce) | `bun run contract` |
| `docs/backend/contract/contract.gen.json` | Everything above, machine-readable (Part 04 diffs and checks against it) | `bun run contract` |
| `scripts/contract/config.ts` | Heuristic settings, capability entry points, and **`overrides`**, where every changed disposition is recorded with a reason | edit by hand |
| `AGENT_MEMORY.md` | Routes, module deps, IPC table, boundary report | `bun run memory` |

How the generator works (`scripts/contract/`: `extract → analyze → render → run`, all settings in
`config.ts`): it builds one TypeScript program over `src/`, finds every `wrap('<module>.<fn>', …)`,
and follows each function's calls **transitively through the checker**, including re-exports
through `@/mocks`. For each body it records `db.<table>` reads and writes (explicit array mutators,
references obtained via `find`/`at`, and anything inside a `mutate()` callback), non-db mock state
(`session`), `invoke('<cmd>')` calls, and browser/plugin APIs. **Writes are a lower bound**, since a
write through an object passed into a helper is not tracked. Reviewers confirm them in 01.B.
`bun run contract:check` fails when the generated files are stale.

## 3. What the first run found (2026-09-26)

**339** functions: **298 port**, **8 rust-existing** (they already `invoke` `render_pdf` /
`render_preview`), **33 frontend**. After the D9 overrides: **309 port**, 8 rust-existing, 22 frontend. **46** MockDb tables, **254** module types, **240** mock-engine
functions, **61** path-string links.

| # | Finding (evidence) | Consequence |
|---|---|---|
| F1 | Shared managers are reached by: **ledger 42** fns, **stock 18**, **activity 74**, **numbering 46**, **period 42**, **currency 4** (README "Shared managers"). | That is the exact caller list for `shared::ledger/stock/activity`. Part 02 sizes and tests those modules against it. |
| F2 | **Templates are per-device.** All 11 `templateService` fns use `localStorage` (`templateService.ts:14`, key `pdf_templates_v1`). The same holds for insight dismiss/snooze (`insightEngine`) and backup history (`backupService`, IndexedDB). | **D9:** templates move to the DB, shared per branch. The 11 fns are overridden to `port`. Insight dismiss/snooze stays per device. |
| F3 | **Terminal identity already exists.** `terminalId` is on held sales and shifts (`invoices/types/index.ts:163,208,229`). | The Rust `AppState` needs a stable terminal id per install. D8 decides how terminals reach the DB. |
| F4 | **Auth is a mock.** `session = { userId: '' }` (`mocks/db.ts:257`), and `db.credentials` holds plain-text passwords (`mocks/db.ts`). | Rust owns the session and stores password + manager-PIN hashes (argon2). The `authService` contract (login/restore/logout/verifyManagerPin) stays the same. |
| F5 | **Error codes** are a closed set: `NOT_FOUND` 92, `CONFLICT` 23, `FORBIDDEN` 16, `VALIDATION` 7, `UNAUTHORIZED` 2 (`mocks/utils.ts:52`). The UI shows the Arabic message. | `AppError` serializes to `{ code, message }` with the same five codes and the same Arabic messages (listed per module in 01.B). |
| F6 | **Change events** drive page refresh: `ledger:changed` (16), `catalog:changed` (19), `parties:changed` (11), consumed via `onLedgerChanged` / `onCatalogChanged`. | Rust emits the same three names as Tauri events. With several terminals, other terminals must learn about changes too (D8). |
| F7 | **61 activity/audit links are path strings** (e.g. `backend/journal.ts:160`, `backend/branches.ts:126`). They break CLAUDE.md rule 25 for service link fields. | Fix in the **mock first** (01.C), so the parity harness compares route objects on both sides. |
| F8 | **Server-side validation** exists only as frontend Zod schemas (`parties/validators/partySchema.ts`, `products/validators/productSchema.ts`, `users/validators/userSchema.ts`) plus `ApiError('…','VALIDATION')` checks inside the mock. | Rust must re-validate every command input. The Zod schemas and mock checks are the rule list. |
| F9 | **Dev-only surface:** `devToolsService.resetToEmpty/reloadDemoData`, and `accountingDebugService` (posting trace, invariants, drift, repro bundles). | These become debug-build-only Rust commands. A **snapshot importer** (MockDb JSON → MariaDB) serves demo data, the Part 04 parity harness and D10. |
| F10 | **Backup** (`backupService`) snapshots the IndexedDB MockDb through `plugin-fs`. | Under MariaDB, backup/restore moves to `infrastructure/backup`. The service contract stays, and the dispositions flip to `port` in 01.B. |
| F11 | **`StoreSettings` mixes branch-wide and per-machine values** (`settings/types/index.ts`, `printer`): store name, currency, country and plan 22's default `a4Template`/`imageTemplate` belong to the branch, but thermal `printerName`/`host`/`connection`, `a4PrinterName` and `labelPrinterName` belong to one machine. | Under D8, `db.settings` becomes one shared DB row, so one terminal saving its printer would change it for all. 01.D splits settings into **branch** (DB) and **device** (local to each install, e.g. the Tauri app-data folder). `settings.md` marks every `StoreSettings` field as one or the other. Plan 22 (in progress) adds the template defaults; they are branch-level, which matches D9. |

## 4. Decisions needed for this part

All decisions this part needs are answered (full text in `00-MASTER-PLAN.md` §9):

| # | Decision | What 01.B / 01.D must record because of it |
|---|---|---|
| D8 | Each terminal runs its own Rust app, connected straight to the Main PC's MariaDB over the LAN. MariaDB row locks, no HTTP server. | Every module file's §5 "Concurrency" lists the rows two terminals can race on and the lock or unique constraint that settles it. `cross-cutting.md` specifies the change-version table for cross-terminal refresh (F6). |
| D9 | Print templates move to the DB, shared per branch. | `templates.md` designs the `print_templates` DTO/entity from `PdfTemplate` (`templates/types`), with per-branch default handling (`setAsDefault`). |
| D10 | Real/beta data exists. The snapshot importer ships to users. | `cross-cutting.md` specifies the import source (IndexedDB `mock-db`/`snapshot`/`current` + schema `version`, `localStorage` `pdf_templates_v1`) and the table → entity map the importer follows. Every module file notes fields that need transformation on import (path-string links, plain-text credentials → hashes, mock ids → UUIDv7). |

## 5. Phases

| Phase | File(s) | What | Size | Status |
|---|---|---|---|---|
| 01.A | this file, `scripts/contract/*`, `scripts/verify/rounding.ts` | Tooling + D3 applied to the mock | M | done |
| 01.B | `01-frontend-analysis/<module>.md` × 17, from [`TEMPLATE.md`](01-frontend-analysis/TEMPLATE.md) | Per-module contract review | L | done |
| 01.C | mock + types (see tasks) | Contract fixes in the spec (path links → route objects, anything 01.B finds) | M | done |
| 01.D | `01-frontend-analysis/cross-cutting.md` | Auth/session, terminal id, errors, events, paging, dates/timezone, table → entity ownership map | M | done |

### 01.A — Tooling and rounding (done)

- [x] `scripts/contract/` generator (`types`, `config`, `extract`, `analyze`, `render`, `run`) →
      `docs/backend/contract/*`, plus `bun run contract` and `contract:check` in `package.json`.
- [x] Landmarks for the rounding helper and the contract inventory in `scripts/memory/config.ts`.
- [x] **D3:** one rounding rule, `roundHalfAwayFromZero` / `round2` / `round4` in
      `src/modules/core/helpers/numbers.ts`. It rounds on the shortest decimal form, which matches
      `rust_decimal` `MidpointAwayFromZero`. Every copy now re-exports or imports it:
      `mocks/utils.ts`, `mocks/backend/core.ts` (`round4`), `mocks/backend/transfers.ts`,
      `mocks/backend/invariants.ts`, `invoices/helpers/totals.ts`, `scripts/verify/shared.ts`,
      `scripts/totals.spec.ts`. Inline `Math.round(x*100)/100` was replaced in `invoiceService`,
      `partyService`, `inventoryService`, `tafqit`, `ExpenseListPage`, `ProfitLossPage`,
      `CloseShiftDialog`, `PriceMatrix`.
- [x] Regression: a new `rounding` area in `bun run verify:mocks` (`scripts/verify/rounding.ts`)
      pins the cases the old code got wrong (−0.125 → −0.13, 1.005 → 1.01, 1234.565 → 1234.57,
      float noise) and checks that every exported `round2`/`round4` **is the same function**, so a
      future local copy fails the gate. Result: 128 ok / 0 failed on both SA and EG seeds, and
      `scripts/totals.spec.ts` passes 39/39. `build`, `check`, `contract:check`, `memory:check`
      and `diag:check` are green. **The e2e suite has not been run for this change.** It runs once
      at the end of the plan (master plan §8).

### 01.B — Per-module contract review

One file per module, copied from `01-frontend-analysis/TEMPLATE.md`. Order follows the master plan
build order, so Part 03 can start on the first module while later ones are still reviewed:

- [x] `settings.md` (42) · [x] `setup.md` (21) · [x] `users.md` (9) · [x] `approvals.md` (5)
- [x] `parties.md` (16) · [x] `products.md` (51) · [x] `purchases.md` (12) · [x] `invoices.md` (26)
- [x] `payments.md` (7) · [x] `vouchers.md` (11) · [x] `expenses.md` (11) · [x] `accounting.md` (33)
- [x] `reports.md` (32) · [x] `analytics.md` (3) · [x] `core.md` (35) · [x] `templates.md` (11, D9: DB per branch)
- [x] `diagnostics.md` (14): decide which read endpoints Rust serves in debug builds (F9)

For each module file, the reviewer must:
- [ ] Confirm or change every function's disposition. A change goes into
      `scripts/contract/config.ts` → `overrides` with a reason, followed by `bun run contract`.
- [ ] Confirm the write set against the mock code. The generator's writes are a lower bound.
- [ ] Map every DTO field to a Rust type and column type. Every _decimal_ gets a **scale**, taken
      from the mock's rounding point in `mocks.md` (2 = money, 4 = cost/qty/rate). Every
      _route_ becomes `RouteRef`. Every _enum_ becomes a Rust enum with serde `rename`.
- [ ] Fill the undo matrix: every write function is either **undoable via `<existing reversal fn>`**
      or **not undoable because …**. No new accounting semantics (master plan §3 rule 7).
- [ ] List validation rules (Zod schema + mock `VALIDATION`/`CONFLICT` checks) and error cases
      with their exact Arabic messages.
- [ ] Concurrency notes under D8: which rows two terminals can race on (numbering, stock, shift,
      allocations), and the lock or constraint that settles it.
- [ ] Reports, analytics, dashboard and insights modules only: for every output, the aggregation
      spec (source tables, filters, grouping, rounding point), so Part 03 writes SQL, not a loop.

### 01.C — Contract fixes in the spec (mock first, so the parity harness compares like with like) — done (2026-09-27)

- [x] F7: turned every path-string link in `src/mocks/**` and services into named route objects.
      Typed the `link` fields on `ActivityEntry` / `AuditEntry` as `AppRoute`, updated their
      readers, and got `bun run check` and `verify:mocks` green. The inventory shows
      **0 path-string links** (1 documented non-route false positive remains — see status note).
- [x] Every other mismatch 01.B records under "Contract fixes needed in the mock" that was safe to
      apply now: the `setupService.ts`/`currency.ts` duplicate `isBaseCurrencyLocked` check and
      `applyCountryTax`'s hard-coded seed-id lookup (both from `setup.md` §8). No accounting-sensitive
      fix was made, so no new `scripts/verify/cases/*.json` case was needed.

### 01.D — Cross-cutting contract (`01-frontend-analysis/cross-cutting.md`) — done (2026-09-27)

- [x] Auth + session (F4): login/restore/logout/manager PIN, password hashing, role checks (the
      same `area` roles as `core/helpers/navigation.ts`).
- [x] Terminal identity (F3) and what is per terminal (held sales, shift) vs. per branch.
- [x] Settings split (F11): every `StoreSettings` field classed **branch** (DB) or **device**
      (local per install: printers, connection, backup folder on the Main PC). Define the device
      store and the one `settingsService` contract that merges both, so pages don't change.
- [x] `AppError` catalogue (F5): code → HTTP-like meaning → how the UI shows it (`E-XXXX` toast).
- [x] Events (F6): the three names, payloads, and the cross-terminal refresh mechanism (D8).
- [x] Paging (`core/types/paging.ts` `PagedQuery`/`PagedResult`), sorting and search semantics
      (`core/helpers/search.ts` `matchesSearch` — Arabic normalization must match in SQL or Rust).
- [x] Dates: `localDateKey` / `inDateRange` (`mocks/utils.ts`). Decide the timezone rule
      (business-local dates as `DATE`, instants as UTC `DATETIME(3)`).
- [x] **Table → entity ownership map** for all 46 tables: owning module, which become child
      tables (invoice lines, journal lines, allocations), and which are settings blobs (`settings`,
      `counters`). This is the direct input to Part 02-B.
- [x] **Import spec (D10):** how the shipped importer reads the old store (IndexedDB `mock-db` →
      `snapshot` → `current`, `version` from `src/mocks/persist.ts`; `localStorage`
      `pdf_templates_v1`), the per-table transforms (id → UUIDv7 with a kept mapping, path links →
      `RouteRef`, `credentials` → argon2 hashes, templates → `print_templates` of the active branch),
      and the rules: idempotent, one transaction, invariants green or roll back.

## 6. Gate (Part 01 is done when all of these hold)

- [x] D8, D9, D10 answered and recorded in `00-MASTER-PLAN.md` (2026-09-26).
- [x] **17** module files (this doc's own text below says "16" in the gate title's original draft —
      a pre-existing paperwork mismatch, not corrected by deleting a module file; 01.B's own task
      list and status note both show 17 files done: `settings, setup, users, approvals, parties,
      products, purchases, invoices, payments, vouchers, expenses, accounting, reports, analytics,
      core, templates, diagnostics`) + `cross-cutting.md` complete, with every checklist box ticked.
- [x] Every service function has a confirmed disposition, either the heuristic's or an override
      with a reason (341 fns, 296 port, confirmed via `bun run contract` unchanged since 01.C).
- [x] 01.C done: the inventory shows **0 path-string links** (1 documented non-route false positive,
      `logService.ts:55`, remains and is expected — see the status note above).
- [x] `bun run contract:check`, `bun run verify:mocks` (0 failed), `bun run build`,
      `bun run check`, `bun run memory:check` and `bun run diag:check` are green (all re-run
      2026-09-27 at the end of 01.D — see the status note above for exact output).
- [x] No open `ACC-` issue in `docs/diagnostics/ISSUES.md` (confirmed: zero `ACC-` entries at all,
      open or otherwise), since we don't port a known wrong number.

**Part 01's overall gate is green as of 2026-09-27.** All boxes above are satisfied by a fresh run
of every fast gate at the end of the 01.D session (not assumed from an earlier session). Per
CLAUDE.md's plan-lifecycle rule and this task's own constraints, the folder is **not** moved to
`plans/completed/` and Part 02 is **not** started here — that decision belongs to the user/orchestrating
session. The natural next step is writing `02-CORE-AND-SHARED-ARCHITECTURE.md` per the master plan's
build order (§7) and its own "Next step" section.
