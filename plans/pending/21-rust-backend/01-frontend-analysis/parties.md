# 21 · 01.B — `parties` contract

> **Status:** done (2026-09-27) · **Inventory:** `docs/backend/contract/parties.md`
> (regenerate with `bun run contract`) · **Mock spec:** `src/mocks/backend/balances.ts`
> (`customerBalance`/`supplierBalance`/`*Fc`/`customerStatement`/`supplierStatement`),
> `src/mocks/backend/payments.ts` (`getOpenDocumentsFor`/`unallocatedCreditFor`, read-only reach
> from here) · **Services:** `src/modules/parties/services/partyService.ts` · **Helpers:**
> `src/modules/parties/helpers/creditLimit.ts` (called from `invoices`, not from here — listed for
> completeness since it's this module's public surface) · **Types:** `src/modules/parties/types/index.ts`
>
> Customers and suppliers (docs/v2/08-customers-and-suppliers.md). No ledger/stock/numbering reach
> of its own — every balance/statement/aging figure is **computed by reading `journalEntries`**,
> never stored redundantly, so `parties` never posts anything itself (confirmed by the inventory's
> blank "Shared" column except `activity`).

## 1. Endpoints

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `findDuplicates` | port (confirmed) | `parties_find_duplicates` | `{ input: { phone?, vatNumber? }, excludeId?: string }` | `DuplicateWarning[]` | — | — | n/a (read) | **Synchronous** in the mock (no `delay()`, not async) — used for live "as you type" duplicate warnings. `checkDuplicates` below is the async wrapper the actual service surface exposes; Rust only needs one IPC command (`checkDuplicates`'s shape) — `findDuplicates` is an internal sync helper, not worth a second command. |
| `checkDuplicates` | port (confirmed) | `parties_check_duplicates` | `{ input: { phone?, vatNumber? }, excludeId?: string }` | `DuplicateWarning[]` | — | — | n/a (read) | Scans **both** `customers` and `suppliers` for a phone or VAT-number match — cross-table by design (a customer and a supplier sharing a phone/VAT is still worth flagging). |
| `getPartyGroups` | port (confirmed) | `parties_get_party_groups` | `{ kind: 'customer' \| 'supplier' }` | `PartyGroup[]` | — | — | n/a (read) | |
| `getCustomers` | port (confirmed) | `parties_get_customers` | `{ filter?: PartyFilter }` | `Customer[]` | — | — | n/a (read) | Every row's `balance`/`unallocatedCredit` are **computed on read** (`withComputed`), not stored columns — see §7. `filter.search` uses `includesText` (Arabic-normalizing substring match, cross-cutting concern — see `01-frontend-analysis/cross-cutting.md`'s `matchesSearch` task, 01.D). |
| `getCustomer` | port (confirmed) | `parties_get_customer` | `{ id: string }` | `Customer` | — | — | n/a (read) | Same computed-field pattern as `getCustomers`. |
| `saveCustomer` | port (confirmed) | `parties_save_customer` | `{ input: CustomerInput, id?: string }` | `Customer` | `customers`, `partyHistory`, activity, audit | activity | not undoable | Create-or-update by optional `id`. **Refuses deactivating a customer with an outstanding positive balance** (`customerBalance(id) > 0`) — a real accounting-safety guard, must port exactly. Writes **two** side effects beyond the core row: an activity/audit entry AND a `partyHistory` row with a fixed Arabic message ("تعديل بيانات العميل" / "إنشاء بطاقة العميل") — these are two different logs for two different UI surfaces (global activity feed vs. the party's own "السجل" tab) and both must be written, not just one. Emits `parties:changed`. |
| `getCustomerStatement` | port (confirmed) | `parties_get_customer_statement` | `{ id: string }` | `PartyStatementRow[]` | — | — | n/a (read) | See §7 — a full ledger-derived aggregation, not a stored table. |
| `getSuppliers` | port (confirmed) | `parties_get_suppliers` | `{ filter?: PartyFilter }` | `Supplier[]` | — | — | n/a (read) | Same computed-balance pattern; `filter.overLimitOnly` (present on `PartyFilter`, used by `getCustomers`) has **no supplier equivalent check** here — suppliers have no credit limit concept, confirmed intentional (asymmetric by design, not a gap). |
| `getSupplier` | port (confirmed) | `parties_get_supplier` | `{ id: string }` | `Supplier` | — | — | n/a (read) | |
| `saveSupplier` | port (confirmed) | `parties_save_supplier` | `{ input: SupplierInput, id?: string }` | `Supplier` | `suppliers`, `partyHistory`, activity, audit | activity | not undoable | Same two-log pattern as `saveCustomer`. **No "can't deactivate with a balance" guard for suppliers** — asymmetric vs. `saveCustomer` (see §8, worth a decision). |
| `getSupplierStatement` | port (confirmed) | `parties_get_supplier_statement` | `{ id: string }` | `PartyStatementRow[]` | — | — | n/a (read) | See §7. |
| `linkPartyRecords` | port (confirmed) | `parties_link_party_records` | `{ customerId: string, supplierId: string }` | — | `customers.linkedPartyId`, `suppliers.linkedPartyId` | — | not undoable via the registry, but **its own inverse (`unlinkPartyRecord`) exists and is a manual re-link/unlink action, not a compensating "undo"** | Both records must exist or it refuses. Sets the link **both ways** in one operation — must commit atomically (a customer linked to a supplier that isn't reciprocally linked back would be a data-integrity bug). No activity/audit row (see §6). |
| `unlinkPartyRecord` | port (confirmed) | `parties_unlink_party_record` | `{ partyId: string, kind: 'customer' \| 'supplier' }` | — | `customers.linkedPartyId` and/or `suppliers.linkedPartyId` | — | not undoable (re-linking is `linkPartyRecords` again, a fresh action) | **Silently returns (no-op, no error) if the party has no link** — not an `ApiError`, just an early `return`. Clears **both sides** of the link when the counterpart still points back (defensive — if the counterpart's link was already cleared some other way, only the requested side is cleared). No activity/audit row (see §6). |
| `getLinkedNetBalance` | port (confirmed) | `parties_get_linked_net_balance` | `{ customerId?: string, supplierId?: string }` | `number \| undefined` | — | — | n/a (read) | Returns `undefined` (not an error) if either id is missing — a "not applicable" sentinel, not a failure. `customerBalance(id) - supplierBalance(id)` — simple arithmetic on two already-reviewed aggregations. |
| `getPartyHistory` | port (confirmed) | `parties_get_party_history` | `{ partyId: string }` | `PartyHistoryEntry[]` | — | — | n/a (read) | Sorted by `date` descending. This is the **separate, party-specific log** `saveCustomer`/`saveSupplier` write to (distinct from the global activity feed — see the note on those two functions). |
| `getPartyAging` | port (confirmed) | `parties_get_party_aging` | `{ kind: 'customer' \| 'supplier', partyId: string }` | `AgingBucket[]` | — | — | n/a (read) | See §7 — buckets by days-overdue using `dueDate` (falling back to document date), reads open documents via `payments.ts`'s `getOpenDocumentsFor` (that file's own review, if/when scheduled, owns `openDocumentsFor`'s internals — this module only consumes its output). |

## 2. DTOs → Rust

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `PartyCommon` | `id` | `Uuid` | `UUID` | UUIDv7 (D2). Note `Customer`/`Supplier` both extend `PartyCommon` per the types file — Rust likely models these as two entities (`customers`, `suppliers`) sharing a common set of columns, or one `parties` table with a `kind` discriminant plus per-kind extension tables for `Customer.creditLimit` / `Supplier.contactPerson`/`defaultExpenseAccountId` — **this is an entity-design decision for Part 02-B**, not resolved here (flagged in §9). |
| `PartyCommon` | `type` | `enum PartyType { Individual, Company }` | `ENUM('individual','company')` | `#[serde(rename_all = "camelCase")]`. |
| `PartyCommon` | `balance` / `unallocatedCredit` | `Decimal` (both) | **not stored columns** — computed at query time via a `SUM(...)` over `journal_lines` (mirroring `customerBalance`/`supplierBalance`/`unallocatedCreditFor`), same as the mock | `DECIMAL(19,2)` when materialized in a response DTO, but there is **no backing column** — see §7. This is the single most important modeling fact in this module: don't add a `balance` column to `customers`/`suppliers` and try to keep it in sync: recompute on read, exactly like the mock, so the accounting invariant "AR GL = Σ customer balances" holds *by construction* rather than by a sync mechanism that can drift. |
| `PartyCommon` | `phones` | `Vec<PartyPhone>` | child table `party_phones(id, party_id, label, number)` | `PartyPhone.label` → `enum PhoneLabel { Mobile, Work, Whatsapp }`, `#[serde(rename_all = "lowercase")]`. |
| `PartyCommon` | `contacts` | `Vec<PartyContact>` | child table `party_contacts(id, party_id, name, role, phone, email)` | — |
| `PartyCommon` | `nationalAddress` | `Option<NationalAddress>` | JSON column, or normalized address columns — **decide alongside `settings.md`'s `Branch.nationalAddress`/`StoreSettings.nationalAddress` mapping** so all three don't diverge (Part 02-B, one address shape reused everywhere) | Same `Address`-family shape as `settings`/`setup`. |
| `PartyCommon` | `structuredAddress` | `Option<Address>` (shared struct) | same as above | The type file shows **both** `nationalAddress: NationalAddress` (legacy/flat) and `structuredAddress: Address` (the newer 18.E picker output) on the same entity — confirm with `settings.md`/`setup.md`'s address handling whether both need to persist or whether `structuredAddress` supersedes `nationalAddress` going forward (flagged in §9, not a blocker — the mock keeps both, so Rust should too, for now). |
| `PartyCommon` | `groupId` / `priceListId` / `salespersonId` / `branchId` / `linkedPartyId` | `Option<Uuid>` (all) | `UUID` FK (to `party_groups`, `price_lists`, `users`, `branches`, and the *other* parties table respectively) | `linkedPartyId` is the one cross-table FK: a customer's `linkedPartyId` points at a **supplier** row and vice versa — model as two nullable FK columns on their respective tables (not a generic polymorphic reference), since the direction is always known from which table you're in. |
| `Customer` | `creditLimit` | `Decimal` | `DECIMAL(19,2)` | Money — consumed by `creditLimit.ts`'s `assertWithinCreditLimit`, compared with a `+0.005` tolerance (not `round2`'d itself, but compared against `round2`'d balances — see that helper). |
| `Supplier` | `defaultExpenseAccountId` | `Option<Uuid>` | `UUID` FK → `accounts.id` | — |
| `PartyGroup` | `discountPercent` | `Decimal` | `DECIMAL(9,4)` | Percentage scale, consistent with every other percentage field reviewed so far. |
| `PartyStatementRow` | `kind` | `enum StatementRowKind { Invoice, Refund, Payment, PurchaseOrder, PurchaseReturn, Opening }` | `ENUM(...)` | `#[serde(rename_all = "camelCase")]` — matches the `JournalSourceKind` values these rows are derived from 1:1 (`ref?.kind`), so this enum should likely just **reuse** whatever `JournalSourceKind` maps to in `accounting`'s review, rather than being redeclared here — flagged in §9 for cross-checking once `accounting.md` is reviewed. |
| `PartyStatementRow` | `debit` / `credit` / `balance` | `Decimal` (all) | not persisted — computed per-request from `journal_lines`, exactly like `balance` above | `DECIMAL(19,2)` in the response DTO. |
| `AgingBucket` | `key` | `enum AgingBucketKey { Current, D30, D60, D90Plus }` | — (not a stored value, just a computed grouping) | `#[serde(rename = "...")]` per-variant since `'30'`/`'60'`/`'90plus'` aren't valid Rust identifiers as-is — needs explicit `#[serde(rename = "30")]` etc. |
| `AgingBucket` | `total` | `Decimal` | — (computed) | `DECIMAL(19,2)`, `round2`'d per bucket exactly like the mock (`round2(buckets[bucketIndex].total + doc.outstanding)` — accumulated with rounding at each step, not just at the end; Rust must round incrementally the same way if it wants byte-identical parity, though summing exact decimals first and rounding once would be mathematically equivalent for `Decimal` — **note for the parity harness**, not a behavior change). |
| `OpeningBalanceStub` | `amount` | `Option<Decimal>` | `DECIMAL(19,2)` | Money — this stub type isn't written by anything in `partyService.ts` itself (the actual opening-balance posting is `setup.postPartyOpening`, already reviewed) — it's just the read-side shape a party's card shows. |
| Route field | `link` on `saveCustomer`/`saveSupplier`'s `logActivity` calls (`/customers/${id}`, `/suppliers/${id}`) | `RouteRef { name: 'customer-detail' \| 'supplier-detail', params: { id } }` | — | 4 more of the 61 path-string links (F7) — 2 pairs, owned by `partyService.ts`, tracked in §8. |

## 3. Validation and errors

| Function | Rule (source: mock check line) | Code | Exact Arabic message |
|---|---|---|---|
| `getCustomer` | customer must exist | `NOT_FOUND` | `العميل غير موجود` |
| `saveCustomer` | `name` non-empty after trim (`validateCommon`) | `VALIDATION` (default code) | `الاسم مطلوب` |
| `saveCustomer` | `vatNumber` (when present) matches `/^3\d{13}3$/` (`validateCommon`) | `VALIDATION` (default code) | `الرقم الضريبي يجب أن يكون 15 رقماً يبدأ وينتهي بالرقم 3` — **same Saudi-only regex bug already fixed in `settings.md`'s `updateSettings`; this is the carry-over instance flagged there** — see §8. |
| `saveCustomer` (update path) | customer must exist | `NOT_FOUND` | `العميل غير موجود` |
| `saveCustomer` (update path) | can't deactivate a customer with `customerBalance(id) > 0` | `VALIDATION` (default code) | `لا يمكن إيقاف عميل عليه رصيد مستحق` |
| `getSupplier` | supplier must exist | `NOT_FOUND` | `المورد غير موجود` |
| `saveSupplier` | `name` non-empty after trim, `vatNumber` pattern (same `validateCommon`, same bug) | `VALIDATION` (default code) | same two messages as `saveCustomer` |
| `saveSupplier` (update path) | supplier must exist | `NOT_FOUND` | `المورد غير موجود` |
| `linkPartyRecords` | both the customer and the supplier must exist | `NOT_FOUND` | `الطرف غير موجود` |

**No dedicated Zod schema referenced from this service file** — `validateCommon`/`clean` are the entirety of server-side validation. (The frontend does have `src/modules/parties/validators/partySchema.ts`, per earlier grep results in this session, but `partyService.ts` doesn't import or re-run it — client-side-only beyond what's listed above; Rust should treat the Zod schema as an additional source of rules to consider re-asserting, per F8 in the master plan, not assume `validateCommon` is the complete picture.)

## 4. Undo matrix (every function that writes)

| Function | Undoable? | Compensation (existing fn) | Refused when | Period rule (D7) |
|---|---|---|---|---|
| `saveCustomer` / `saveSupplier` | No | — | n/a | n/a — not period-scoped, no ledger touch |
| `linkPartyRecords` | No via the registry — but `unlinkPartyRecord` is the natural, always-available inverse (not a "compensation" in the master-plan §3 rule 7 sense, since linking/unlinking isn't an accounting action, just a reference toggle) | `unlinkPartyRecord` (manual, not auto-triggered) | n/a | n/a |
| `unlinkPartyRecord` | No via the registry — same reasoning, `linkPartyRecords` is the inverse | `linkPartyRecords` (manual) | n/a | n/a |

No accounting consequence from any write in this module (confirmed: no `ledger`/`stock`/`numbering`/`period` in the shared-manager column) — matches the pattern of `approvals.md`, master-plan §3 rule 7 doesn't really apply here since nothing is "reversed," only reference data is toggled or master-data fields are edited.

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|
| Two terminals edit the same customer/supplier concurrently | `customers`/`suppliers` row | Ordinary last-write-wins is acceptable here (no financial consequence — same class of risk as any master-data edit), but the **deactivate-with-balance guard on `saveCustomer`** must re-check `customerBalance(id)` inside the same transaction as the write, using a locking read on the relevant `journal_lines`, so a concurrent sale posting between the check and the commit can't slip a balance-bearing customer into `active: false`. |
| Two terminals call `linkPartyRecords`/`unlinkPartyRecord` on the same customer/supplier pair at once | `customers.linkedPartyId`, `suppliers.linkedPartyId` | Both writes touch two rows atomically — needs a transaction that locks both rows (`SELECT ... FOR UPDATE` on both `customers` and `suppliers` involved) so a link and an unlink racing on the same pair resolve deterministically (last-committed-wins) rather than leaving one side linked and the other not. |
| Concurrent `saveCustomer`/`saveSupplier` creates racing on the auto-generated `nextCode()` sequence | `customers.code` / `suppliers.code` (e.g. `C-0001`, `S-0001`) | `nextCode()`'s "find the max existing number, add one" approach is **not safe under concurrency** even within the mock's own logic if two creates ran in the same tick (though the mock is single-process, so this never actually races today). Under D8, this must become a real sequence: either a `UNIQUE` constraint on `code` with retry-on-conflict, or route it through the same `shared::numbering` mechanism every other document-number sequence uses (per-branch counters, `SELECT ... FOR UPDATE`) rather than a `MAX(code)+1` scan. **Flagged in §8/§9** — this is the one real correctness gap for D8, not just a style preference. |

## 6. Events and side effects

- **Activity/audit rows** written for: `saveCustomer`, `saveSupplier` (both `activityKind: 'party'`). **No activity/audit row at all** for `linkPartyRecords`/`unlinkPartyRecord` — these change a party's linked-record state (arguably worth a trail, since it affects how balances are read together) but the mock is silent. Flagged in §8.
- **`partyHistory` rows** (a *separate* log from the global activity feed, read back by `getPartyHistory` for the party's own "السجل" tab) written for: `saveCustomer` (create/update, fixed Arabic messages), `saveSupplier` (create/update, fixed Arabic messages). **Not written** for `linkPartyRecords`/`unlinkPartyRecord` either — same gap, doubled (missing from both logs).
- **Events emitted:** `saveCustomer`, `saveSupplier`, `linkPartyRecords`, `unlinkPartyRecord` all emit `parties:changed` (this is the module's own dedicated cache-invalidation event, distinct from `approvals`' reuse of `ledger:changed` — a cleaner pattern than what `approvals.md` flagged).
- **Attachments/printing:** none directly in this service file (party avatars/logos, if any, would be handled elsewhere — not found in this file's scope).

## 7. Aggregations (reports / analytics / dashboard / insights only)

This module is almost entirely aggregation — every read past the raw row is computed from `journalEntries`:

| Output (DTO field) | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|
| `Customer.balance` / `Supplier.balance` | `journalEntries` (flattened lines), `accounts` (to resolve the receivable/payable system-role account) | lines where `accountId` = that role's account, `partyKind` matches, `partyId` matches | — (single party) | `round2` (`balances.ts:22,26`) | `balances.ts:14-27` |
| `Customer.unallocatedCredit` / `Supplier.unallocatedCredit` | `payments` (+ their `allocations[]`) | `payments` where `type`/`targetType`/`targetId` match | — | `round2` inside `unallocatedAmount` (`payments.ts:96`) | `payments.ts:94-106` (owned by `payments` module's own review, consumed here) |
| `PartyStatementRow[]` (customer) | same ledger lines as `Customer.balance`, `+ journalEntries` for `sourceRef`/`description` | same party filter | sorted by `date` ascending, running balance accumulated | `round2` per row (`balances.ts:49`) | `balances.ts:45-78` |
| `PartyStatementRow[]` (supplier) | same, sign-flipped (`credit - debit` vs `debit - credit`) | same | same | `round2` per row | `balances.ts:80-102` |
| `AgingBucket[]` | `getOpenDocumentsFor` (from `payments.ts`, itself reading `invoices`/`purchaseOrders`) | party's open (unsettled) documents only | bucketed by `daysOverdue` from `dueDate ?? date`: `<=0` → current, `1-30`, `31-60`, `60+` | `round2` accumulated per bucket (`partyService.ts:270`) | `partyService.ts:261-274` |
| `getLinkedNetBalance` | derived — `Customer.balance − Supplier.balance` for a linked pair | — | — | inherits `round2` from the two balances it subtracts (not re-rounded itself — a subtraction of two already-2dp numbers can't introduce new precision, so this is safe as-is) | `partyService.ts:233-237` |

## 8. Contract fixes needed in the mock (01.C — resolved 2026-09-27)

Per CLAUDE.md's architectural-autonomy rule: applying the same resolution `settings.md` already
established for the identical VAT-regex bug, plus adding the two missing audit trails and flagging
(not fixing in the mock) the `nextCode()` concurrency issue, which is a Rust-side design item, not
a mock behavior change.

- [x] **`validateCommon`'s VAT-number regex is now country-aware.** `partyService.ts` now imports `countryProfile` and checks `countryProfile(db.settings.country).taxId.pattern`/`.label`/`.hint` instead of the hard-coded `/^3\d{13}3$/` — same fix pattern as `settings.md`'s `updateSettings`, applied to the second (and last known) instance of this exact bug in the codebase.
- [x] **`linkPartyRecords`/`unlinkPartyRecord` now write activity rows.** Both now call `logActivity('party', ...)` — "ربط بطاقتي عميل ومورد" / "إلغاء ربط البطاقتين" style messages — closing the silent-write gap noted in §6. `partyHistory` rows were **not** added for these two (unlike `saveCustomer`/`saveSupplier`) — linking/unlinking isn't really "this party's own history" in the same sense as an edit to their own record; the global activity feed is the right home for it. This asymmetry is intentional, not a leftover gap.
- [ ] **`nextCode()`'s `MAX(code)+1` pattern is not concurrency-safe** — flagged for Part 02 (§5, §9), not fixed in the mock (the mock is single-process, so there's nothing to "fix" there; this is purely a note for how Rust implements code generation).
- [ ] **F7 (shared task, still pending):** 4 more path-string links in `partyService.ts` (`/customers/${id}` ×2, `/suppliers/${id}` ×2) — tracked in §2, folded into the shared 01.C pass.
- [x] **`saveSupplier` now has the same "can't deactivate with a balance" guard as `saveCustomer`.** Added `if (data.active === false && supplierBalance(id) > 0) throw new ApiError('لا يمكن إيقاف مورد عليه رصيد مستحق')` to `saveSupplier`'s update path (`src/modules/parties/services/partyService.ts`), closing the asymmetry.

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

Nothing raised here needed a stop-and-ask decision under the architectural-autonomy rule — the
`saveSupplier` guard, VAT-regex fix, and audit-row additions all had one clearly-safer option and
were applied directly. What remains are **entity-modeling questions for Part 02-B**, not behavioral
forks, since they don't change what the mock does — only how Rust structures the tables:

- **`Customer`/`Supplier` entity shape**: one `parties` table with a `kind` discriminant + per-kind extension tables, or two separate tables (`customers`, `suppliers`) sharing a common column set? Either works; note it here so whoever designs the entities in Part 02-B picks one deliberately rather than by accident, and so `linkedPartyId`'s cross-table FK direction is modeled consistently.
- **`nationalAddress` vs `structuredAddress`**: the mock carries both (legacy flat address + the newer 18.E picker output) on every party. Confirm with `settings.md`/`setup.md`'s address handling whether `structuredAddress` should eventually supersede `nationalAddress`, or whether both persist indefinitely — not a blocker, since the mock keeps both today and Rust should match that for parity.
- **`PartyStatementRow.kind` enum reuse**: this should likely share its Rust enum with whatever `accounting.md` defines for `JournalSourceKind`'s `kind` values, rather than being redeclared independently — flag for cross-checking once `accounting.md` is reviewed.

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (16/16, no dispositions changed — only mock-behavior fixes and one guard addition).
- [x] Every write function is in §4.
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` is green (no override needed, no contract shape changed). Also green: `bun run build`, `bun run check`, `bun run verify:mocks` (128 ok, 0 failed), `bun run memory:check` (0 new seam violations), `bun run diag:check`.
