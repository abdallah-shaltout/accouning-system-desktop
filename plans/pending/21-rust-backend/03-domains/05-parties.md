# 21 · 03.05 — `parties` (customers/suppliers, codes, statements, balances, aging, linking)

> **Status:** planned 2026-09-28, not implemented. Wave **W2** (entry file §4). Depends on: 01-settings
> (`settings.country` for the tax-id rule), 03-users (session), Part 02 `shared::balances`,
> `shared::numbering::lock`, `shared::activity`, entities `parties/*`, and Part 02 gaps
> G-1, G-2, G-3, G-4, G-5, G-6, G-8, G-10, G-11 (manager adds them **before W2**).

**Goal.** Port the 15 async functions of `partyService.ts` to `domains/parties/` over the single
`parties` table (P2-15). Codes stay the mock's `MAX+1` rule, serialised by
`numbering::lock(SequenceLock::…)` (P2-21). Balances, statements and aging are **computed on read**
through `shared::balances` (parties.md §2/§7 — never a stored balance). Every DTO equals `Customer`/`Supplier`/…

**Read first.** [`../01-frontend-analysis/parties.md`](../01-frontend-analysis/parties.md) (§1, §3, §5,
§7, §8) · mock `src/modules/parties/services/partyService.ts:16-289`, `src/mocks/backend/balances.ts:14-102`,
`src/mocks/backend/payments.ts:29-106` · types `src/modules/parties/types/index.ts:1-165` · form
`src/modules/parties/pages/PartyFormPage.vue:218-262` (what the client actually sends) · Rust
`shared/balances.rs`, `shared/numbering.rs` (`lock`, `SequenceLock`), `entities/parties/{parties,party_phones,party_groups,party_history}.rs`,
migration `m0007_parties.rs` (`uq_parties_kind_code`, `uq_parties_id_kind`), `utils/text.rs`
(`normalize_arabic`, `matches_search`, `like_contains`).

## 1. Commands

All reads `with_read` + Parties:Read (every role has Parties ≥ Read, `permissions.ts`); all writes
`with_tx` + Parties:Write and touch `ChangeCategory::Parties` (mock `emit('parties:changed')`).

| Mock fn | Disposition | Rust command | Args → Return | Tx | Events |
|---|---|---|---|---|---|
| `findDuplicates` (sync) | port as `checkDuplicates` only | — (no command, no switch line: no caller outside the service) | — | — | — |
| `checkDuplicates` | port | `parties_check_duplicates` | `{ input: DuplicateCheckInput, excludeId?: Id }` → `Vec<DuplicateWarning>` | read | — |
| `getPartyGroups` | port | `parties_get_party_groups` | `{ kind: PartyKind }` → `Vec<PartyGroup>` | read | — |
| `getCustomers` | port | `parties_get_customers` | `{ filter?: PartyFilter }` → `Vec<Customer>` | read | — |
| `getCustomer` | port | `parties_get_customer` | `{ id }` → `Customer` | read | — |
| `saveCustomer` | port | `parties_save_customer` | `{ input: CustomerInput, id?: Id }` → `Customer` | write | `Parties` |
| `getCustomerStatement` | port | `parties_get_customer_statement` | `{ id }` → `Vec<PartyStatementRow>` | read | — |
| `getSuppliers` | port | `parties_get_suppliers` | `{ filter?: PartyFilter }` → `Vec<Supplier>` | read | — |
| `getSupplier` | port | `parties_get_supplier` | `{ id }` → `Supplier` | read | — |
| `saveSupplier` | port | `parties_save_supplier` | `{ input: SupplierInput, id?: Id }` → `Supplier` | write | `Parties` |
| `getSupplierStatement` | port | `parties_get_supplier_statement` | `{ id }` → `Vec<PartyStatementRow>` | read | — |
| `linkPartyRecords` | port | `parties_link_party_records` | `{ customerId, supplierId }` → `()` | write | `Parties` |
| `unlinkPartyRecord` | port | `parties_unlink_party_record` | `{ partyId, kind }` → `()` | write | `Parties` (only when something changed) |
| `getLinkedNetBalance` | port | `parties_get_linked_net_balance` | `{ customerId?, supplierId? }` → `Option<Decimal>` | read | — |
| `getPartyHistory` | port | `parties_get_party_history` | `{ partyId }` → `Vec<PartyHistoryEntry>` | read | — |
| `getPartyAging` | port | `parties_get_party_aging` | `{ kind, partyId }` → `Vec<AgingBucket>` | read (+ business clock, G-4) | — |

15 commands. Args struct names follow `Parties<Fn>Args` (entry §3.2).

## 2. DTOs (`domains/parties/dto.rs`, `#[ts(export_to = "parties/types/gen/")]`)

| Rust DTO | TS type (file:line) | Notes |
|---|---|---|
| `PartyPhone` | `types/index.ts:2-7` | `label` enum lowercase (`mobile`/`work`/`whatsapp`). `id: String` — Rust returns the child row's `Id` as text (D-6). |
| `PartyContact`, `PartyBankInfo`, `OpeningBalanceStub`, `NationalAddress` | `:10-16`, `:52-56`, `:44-50`, `:25-36` | TS-derived mirrors of the entity JSON structs (`entities/parties/parties.rs`, `entities/values.rs`), `From` both ways. `OpeningBalanceStub.amount` `serde_number::option`, `asOfDate` as the stored string. |
| `Customer` | `:58-112` | Flat struct (no `serde(flatten)` — `Equals` needs one object type): every `PartyCommon` field + `credit_limit`. `balance: Decimal`, `unallocated_credit: Option<Decimal>` (always `Some`), `credit_limit` → `serde_number` + `#[ts(type="number")]`; ids `#[ts(type="string")]`; `structured_address` `#[ts(optional, type="import('@/modules/core/types/address').Address")]`; `phones: Option<Vec<PartyPhone>>` always `Some` (D-6); `created_at: Option<String>` = ISO of `created_at`; `updated_at` present only when `updated_at != created_at` (the mock sets `updatedAt` only on update, `:129`). |
| `Supplier` | `:116-120` | Same common fields + `contact_person`, `default_expense_account_id`. Both built by one `fn common_fields(model, phones, balance, credit) -> CommonParts` so the mapping exists once. |
| `CustomerInput`, `SupplierInput` | `:114`, `:122` | Request-only mirrors of the TS `Omit<…>` types (all fields, so `Equals` holds). Server-owned parts are accepted but ignored (D-3). |
| `PartyGroup` | `:125-132` | `discount_percent` `serde_number::option`. |
| `PartyStatementRow` | `:135-146` | From `shared::balances::PartyStatementRow`: `date = date_key`, `kind` enum camelCase (`invoice`,`refund`,`payment`,`purchaseOrder`,`purchaseReturn`,`opening`), `ref_id`, money `serde_number`. |
| `PartyHistoryEntry` | `:149-156` | `date` = ISO of `party_history.created_at` (D-7). |
| `AgingBucketKey`, `AgingBucket`, `AgingDocument` | `:158-165` | keys renamed per variant: `current`, `"30"`, `"60"`, `"90plus"`; `documents[]` = `{ id, number, date, dueDate?, outstanding }`. |
| `PartyFilter`, `DuplicateWarning`, `DuplicateCheckInput` | `partyService.ts:16-28,66` | `DuplicateWarning.field` enum `phone`/`vatNumber`. |
| `PartyKind` | `'customer' \| 'supplier'` literals | lowercase enum. |

`src/modules/parties/types/contract.check.ts` (new): one `Equals` per DTO above; `PartyFilter`/
`DuplicateWarning` imported as types from `../services/partyService`. `getLinkedNetBalance`:
`Option<Decimal>` → `number | null`; the switch line converts `null → undefined`
(`// contract-ok: null→undefined at the switch line`).

## 3. Service logic (`domains/parties/service/{read,write,link,aging}.rs`)

**Shared helpers.**
- `validate_common(conn, input)` (`partyService.ts:30-38`): `input.name.trim()` empty → `VALIDATION`
  `الاسم مطلوب`; then, when `input.vat_number` is a non-empty string (**untrimmed**, as the mock
  tests it), `utils::country::tax_id_rule(settings.country)` (G-10; `countryProfile` falls back to
  `EG`) must match, else `VALIDATION` `format!("{} يجب أن يكون {}", rule.label, rule.hint)`
  (SA: `الرقم الضريبي يجب أن يكون 15 رقماً يبدأ وينتهي بالرقم 3`; EG: `رقم التسجيل الضريبي يجب أن يكون 9 أرقام`).
- `clean(input)` (`:40-53`): `name` trimmed; `name_en`, `phone`, `email`, `address`, `vat_number`,
  `cr_number`, `national_id`, `notes` trimmed and `""` → `None` — these keys are **always assigned**
  (JS sets them to `undefined` explicitly); supplier adds `contact_person` the same way (`:181`).
- `next_code(conn, kind)` (`:55-63`): after `numbering::lock(conn, SequenceLock::CustomerCode|SupplierCode)`,
  read every `code` of that kind with `find_including_deleted()` (the `(kind, code)` unique is not
  live-scoped); for each, strip the **first** occurrence of the prefix (`C-`/`S-`, JS `String.replace`),
  trim JS whitespace, `""` → 0, otherwise parse as `Decimal` (unparsable → skipped, like a non-finite
  `Number`); `max` over values `> m`; result `format!("{prefix}{:0>4}", js_number_string(max + 1))`.
- `with_computed(conn, model)` (`:95-97,154-156`): `balance` = `shared::balances::customer_balance`/
  `supplier_balance`; `unallocated_credit` = `unallocated_credit_for(kind, id)`; lists use the batch
  variants (G-2) so a list is 3 queries, not 2N+.
- Phones: `party_phones WHERE party_id IN (…) ORDER BY party_id, position`.

**`check_duplicates(input, exclude_id)`** (`:66-84`): candidates = live parties (both kinds)
`WHERE (phone = ? OR vat_number = ? OR id IN (SELECT party_id FROM party_phones WHERE number = ?))`
`ORDER BY kind, created_at, id` (ENUM order = customers first, the mock's `[...customers, ...suppliers]`);
then exact (byte) comparison in Rust: skip `excludeId`; `input.phone` non-empty and (`p.phone == phone`
or any phone number `== phone`) → `{ field: phone }`; `input.vat_number` non-empty and `p.vat_number == vat`
→ `{ field: vatNumber }` — phone warning before VAT warning per party (`:71-76`).

**`get_party_groups(kind)`** (`:88-91`): `WHERE kind = ? ORDER BY created_at, id`.

**`get_customers(filter)`** (`:99-111`) / **`get_suppliers`** (`:158-169`): SQL `kind = ?`, live,
`active = 1` unless `includeInactive`, `group_id = ?` when set, and when `search` is non-empty
`search_normalized LIKE like_contains(normalize_arabic(search))` (narrowing only); `ORDER BY created_at, id`.
Then compute (batch) and filter in Rust exactly as the mock: `withBalanceOnly` → `balance > 0`;
customers only `overLimitOnly` → `credit_limit.unwrap_or(0) > 0 && balance > credit_limit`;
`matches_search([name, name_en, code, phone, vat_number], search)` for customers,
`[name, name_en, code, phone, contact_person, vat_number]` for suppliers (the exact haystacks).

**`get_customer(id)` / `get_supplier(id)`** (`:113-118,171-176`): `id` and `kind` must match, else
`NOT_FOUND` `العميل غير موجود` / `المورد غير موجود`; return `with_computed`.

**`save_customer(conn, cx, registry, input, id)`** (`:120-145`; `save_supplier` `:178-203` is the same
with supplier texts, `SequenceLock::SupplierCode`, `contact_person`, `default_expense_account_id`):
1. `validate_common` (before anything else, `:122`), then `clean`.
2. **Update path** (`id` given): `lock::for_update_by_id(conn, "parties", id)`; row with `kind = customer`
   → else `NOT_FOUND` `العميل غير موجود` (`:126-127`).
3. `input.active == Some(false)` and `customer_balance(id) > 0` → `VALIDATION`
   `لا يمكن إيقاف عميل عليه رصيد مستحق` (supplier: `لا يمكن إيقاف مورد عليه رصيد مستحق`, `:128`/`:186`).
4. Assign (`Object.assign`, `:129`): cleaned keys always; every other input key **only when present**
   (`type`, `group_id`, `tags`, `active`, `contacts`, `structured_address`, `national_address`,
   `currency`, `price_list_id`, `payment_terms_days`, `salesperson_id`, `branch_id`, `bank`,
   `credit_limit` (`round2`, D-4), `default_expense_account_id`); `opening_balance` per D-3;
   `linked_party_id` never (D-3); `updated_at = cx.clock.now`. When `phones` is present: delete the
   party's `party_phones` rows and insert the sent list in order (`position` 0…, `id = Id::new()`).
5. `log(ActivityKind::Party, format!("تعديل العميل {}", name), None, Some(RouteRef::detail("customer", id)))` (`:131`).
6. Insert `party_history { id: Id::new(), party_id, party_kind: "customer", date: cx.clock.today(),
   message: "تعديل بيانات العميل", user_id: actor.id, created_at: cx.clock.now }` (`:132-134`).
2′. **Create path**: `next_code` (lock first); insert `parties` with `id = Id::new()`, `kind`, `code`,
   `balance = 0` (column never read, D-1), `unallocated_credit = None`, the cleaned/present fields,
   `created_at = updated_at = now`, `sync_status = local`; insert phones. A `uq_parties_kind_code`
   violation (should be impossible under the lock) maps to `CONFLICT` via G-3.
5′. `log(Party, format!("إضافة العميل {}", name), None, Some(RouteRef::detail("customer", id)))` (`:138`).
6′. `party_history` message `إنشاء بطاقة العميل` (`:139-141`).
7. `cx.touch(ChangeCategory::Parties)` (`:143`); return `with_computed(reloaded row)`.

**`get_customer_statement(id)` / `get_supplier_statement(id)`** (`:147-150,205-208`): map
`shared::balances::customer_statement`/`supplier_statement` rows to the DTO. No existence check
(the mock returns `[]` for an unknown id). Tie order depends on G-5.

**`link(customer_id, supplier_id)`** (`:213-224`): `lock::for_update_many_sorted(conn, "parties", [c, s])`;
customer row (`kind=customer`) and supplier row (`kind=supplier`) must both exist → else `NOT_FOUND`
`الطرف غير موجود`; set `c.linked_party_id = s`, `s.linked_party_id = c` (previous partners untouched,
Q-3); `log(Party, format!("ربط بطاقتي \"{}\" (عميل) و\"{}\" (مورد)", c.name, s.name), None,
Some(RouteRef::detail("customer", c)))`; touch `Parties`.

**`unlink(party_id, kind)`** (`:226-245`): plain read of the party (`id`, `kind`); missing or no
`linked_party_id` → `Ok(())` (no activity, no touch, `:231`). Otherwise lock both ids sorted
(`for_update_many_sorted`), re-read both; clear the party's link and, **if the counterpart row of the
other kind exists, clear its link unconditionally** (`:235` — the code, not parties.md §1's wording);
`log(Party, format!("إلغاء ربط بطاقة \"{}\"{}", p.name, counterpart.map(|c| format!(" و\"{}\"", c.name)).unwrap_or_default()),
None, Some(RouteRef::detail(if customer {"customer"} else {"supplier"}, p.id)))`; touch `Parties`.

**`get_linked_net_balance(c, s)`** (`:248-252`): either absent → `None`; else
`customer_balance(c) − supplier_balance(s)` (no extra rounding; both are already `round2`).

**`get_party_history(party_id)`** (`:256-259`): `WHERE party_id = ? ORDER BY created_at DESC, id ASC`.

**`get_party_aging(kind, party_id)`** (`:263-289`): `today = business_clock(conn).today()` (G-4);
docs = `open_invoices_for` / `open_purchase_orders_for` (G-1: with `number`, `date`, `due_date`,
ordered by date key then `created_at, id`); four buckets with the exact labels
`حتى تاريخ الاستحقاق`, `1–30 يوم`, `31–60 يوم`, `90+ يوم` (U+2013 dashes); for each doc
`ref_day = due_date.unwrap_or(date.day)`, `days = (today − ref_day).num_days()`, bucket
`≤0 → 0, ≤30 → 1, ≤60 → 2, else 3`; `total = round2(total + outstanding)` **per step** (`:285`);
push `{ id, number, date: date.key(), dueDate, outstanding }`.

## 4. Concurrency (D8)

- Codes: `numbering::lock(SequenceLock::…)` (`document_counters` row `FOR UPDATE`) serialises the
  `MAX+1` read; `uq_parties_kind_code` backs it (P2-21, parties.md §5).
- Edits: the party row is locked `FOR UPDATE` first (party step of `core/lock.rs`'s order); two
  edits are last-write-wins after serialising (parties.md §5).
- Deactivate-with-balance: the row lock only closes the race if posters also lock the party.
  **Cross-domain rule R-1 (manager propagates to 07/08/09/10/11/12/02):** every command that posts
  journal lines tagged with a party takes `lock::share_lock_by_id(conn, "parties", id)` for each
  distinct party (sorted) in the party step of the lock order, so a sale can't post between this
  command's balance read and its commit.
- Lock-order conflict (surfaced, not resolved here): entry §3.3 lists "settings S → fiscal year →
  parties → products → documents", but `core/lock.rs`'s doc comment and the code (`shared::ledger::post`
  takes the settings/fiscal-year share locks *inside* posting, after the domain's own row locks) use
  "documents → parties → products → settings → fiscal year → `document_counters` → `change_versions`".
  This file follows the code; the manager should correct entry §3.3.
- Link/unlink: both rows locked in sorted id order (`for_update_many_sorted`); unlink reads first
  unlocked only to learn the pair, then locks both and re-reads.

## 5. Undo

Not undoable via the registry (parties.md §4; link/unlink are each other's manual inverse; E-5 lists none).

## 6. Frontend switch lines (`src/modules/parties/services/partyService.ts`)

`if (usesRust('parties')) return backendCall('<cmd>', { … })` in: `checkDuplicates` `{ input, excludeId }`,
`getPartyGroups` `{ kind }`, `getCustomers` `{ filter }`, `getCustomer` `{ id }`, `saveCustomer`
`{ input, id }`, `getCustomerStatement` `{ id }`, `getSuppliers` `{ filter }`, `getSupplier` `{ id }`,
`saveSupplier` `{ input, id }`, `getSupplierStatement` `{ id }`, `getPartyHistory` `{ partyId }`,
`getPartyAging` `{ kind, partyId }`. Special forms:
- `linkPartyRecords`: `if (usesRust('parties')) { await backendCall('parties_link_party_records', { customerId, supplierId }); return; }`
- `unlinkPartyRecord`: same form with `{ partyId, kind }`.
- `getLinkedNetBalance`: `if (usesRust('parties')) return (await backendCall('parties_get_linked_net_balance', { customerId, supplierId })) ?? undefined;`
- `findDuplicates` (sync): untouched.

## 7. Known mock quirks (kept) and decisions

**Quirks kept:**
- Q-1 Update audits `action = 'create'` (`logActivity` default).
- Q-2 Absent input keys keep old values — the form omits `groupId`/`structuredAddress`/`currency`/`bank`/`creditLimit` when empty (`PartyFormPage.vue:229-251`), so those can't be cleared from the form.
- Q-3 Linking doesn't clear a previous partner's back-link; unlinking clears the counterpart even if it points elsewhere.
- Q-4 The `90+ يوم` bucket starts at 61 days.
- Q-5 VAT is validated untrimmed, then stored trimmed.
- Q-6 Statements for unknown ids return `[]`; `getLinkedNetBalance` doesn't check existence.
- Q-7 The mock's `localDateKey('YYYY-MM-DD')` shifts a date-only value one day earlier west of UTC; Rust uses the stored day (identical for the supported UTC+ countries EG/SA).

**Decisions:**
- D-1 `parties.balance`/`unallocated_credit` columns (m0007 comment: "kept in sync by Part 03") are **never read or maintained**: balances are recomputed from the ledger (parties.md §2 — AR GL = Σ customers by construction). Insert `0`/`NULL`; G-11 asks the manager to drop them.
- D-2 Codes keep the mock's exact `MAX+1` format; exotic JS `Number()` forms (hex, exponent, `Infinity`) aren't emulated — every stored code is `C-NNNN`/`S-NNNN`.
- D-3 Server authority on save: `linkedPartyId` in the input is ignored (links change only through link/unlink, which keep both sides consistent); `openingBalance.journalEntryId`/`locked` are server-owned (setup's posting sets them) — when the stored stub has a `journalEntryId`, the whole stored stub is kept; otherwise the client's `amount`/`side`/`asOfDate` are stored with the stored `journalEntryId`/`locked`. The form echoes these values, so normal use is unchanged.
- D-4 `credit_limit` is `round2`'d to its DECIMAL(19,2) column.
- D-5 Area choice: every read Parties:Read (all roles), writes Parties:Write (admin/manager; accountant, cashier and storekeeper have Read — matches the `customer-new`/`supplier-new` routes' `access: 'write'`).
- D-6 Phone row ids are server `Id`s (the client's `phone-N` ids can't be UUIDs; they're only `v-for` keys); `phones` is always an array. Parity maps ids and treats absent ≡ `[]`.
- D-7 `party_history.date` in the DTO is the ISO of `created_at` (every mock writer stores an ISO instant, `:133,140,191,198`); the 00-import handoff: set `created_at` from the mock `date` and `date` from its local day.

**Part 02 gaps this file needs** (G-3/G-6/G-8 as defined in [`03-users.md`](03-users.md) §7, G-9 in
[`04-approvals.md`](04-approvals.md) §7; manager adds them before W2):
- G-1 `shared::balances::OpenDocument` has only `id`/`outstanding`; aging needs `number`, `date: DocDate`, `due_date: Option<NaiveDate>` (and 09-payments will need the rest of the mock `OpenDocument`: `kind`, `total`, `currency`, `fc_outstanding`, `rate`, `payments.ts:29-70`). `open_invoices_for`/`open_purchase_orders_for` order by `date_day` only; the mock sorts by the full date string, so order by (`date_day`, `date_instant`, `created_at`, `id`).
- G-2 Batch readers in `shared::balances` (one copy of the math): `customer_balances(conn, &[Id])`, `supplier_balances`, `unallocated_credits(conn, kind, &[Id])` → `HashMap<Id, Decimal>`, each `round2` per party exactly like the single-party functions.
- G-4 `with_read` gives no `BusinessClock`; make `core::tx::read_business_clock` public (or hand the clock to the closure) — aging needs the business "today".
- G-5 `shared::balances::party_ledger_lines` has no `ORDER BY`; the statement's stable date sort then breaks same-day ties differently from the mock (journal-entry array order, then line order). Add `ORDER BY journal_entries.created_at, journal_entries.id, journal_lines.position`.
- G-10 `utils::country::tax_id_rule(country: Option<&str>) -> TaxIdRule { label, hint, matches }` porting `countryProfiles.ts` `taxId` (EG `^\d{9}$`, SA `^3\d{13}3$`, fallback EG; ASCII digits, no regex crate) — shared with 01-settings' `updateSettings` check.
- G-11 `m0007_parties.rs` stores `balance`/`unallocated_credit` with a comment saying Part 03 keeps them in sync; that contradicts parties.md §2/§7 and D-1. Drop both columns (in place, Part 02 tests have not passed) or leave them unused — never maintain them.

## 8. Tests

**(a) `src-tauri/tests/domain_parties.rs`:**
- create customer → code `C-0001`, then `C-0002`; codes independent per kind (`S-0001`); a code `C-0010` imported → next `C-0011`; odd codes (`X-1`) ignored.
- 10 parallel creates → 10 distinct consecutive codes (lock), no `CONFLICT`.
- validation: blank name → `الاسم مطلوب`; SA settings + bad VAT → SA message; EG settings + bad VAT → EG message; name check runs before VAT check.
- update: unknown id / supplier id through `save_customer` → `العميل غير موجود`; absent `groupId` keeps it, `nameEn: ""` clears it; phones replaced in order; client `linkedPartyId` ignored; posted opening stub kept.
- deactivate a customer with an open invoice balance → `VALIDATION` text; with zero balance → OK; same for a supplier.
- history: create + update write `إنشاء بطاقة العميل` then `تعديل بيانات العميل`, returned newest first; audit/activity messages exact, `entity='customer'`.
- list: order `created_at, id`; `includeInactive`, `withBalanceOnly`, `groupId`, `overLimitOnly`, Arabic search (`أحمد` finds `احمد`, Arabic-Indic digits find a phone); balances equal `shared::balances` per party.
- statement final row = balance; kinds mapped; same-date rows keep posting order (G-5).
- duplicates: phone in `phones[]` of a supplier flagged for a customer form; `excludeId` skips self; customers listed before suppliers.
- link/unlink: both sides set; unlink from the supplier side clears both; unlink of an unlinked party writes no activity and bumps no version; link with a missing supplier → `الطرف غير موجود`.
- aging: documents bucketed by `dueDate` else date against the business "today"; bucket totals `round2`'d stepwise.
- `run_all` invariants pass after a posted-sale + deactivate-refused scenario (no side effects on the ledger).

**(b) Parity cases:** `parties-create-codes`, `parties-update-absent-keys`, `parties-deactivate-guard`,
`parties-statement-and-balance`, `parties-aging-buckets`, `parties-link-unlink`, `parties-duplicates`, `parties-search-arabic`.

## 9. Checklist

- [ ] Confirm G-1, G-2, G-3, G-4, G-5, G-6, G-8, G-10, G-11 are in place (manager, before W2).
- [ ] `domains/parties/{mod,dto,commands}.rs` + `service/{mod,read,write,link,aging}.rs` (files stay < 400 lines).
- [ ] `dto.rs` per §2 (mirrors + `From`, one `common_fields` builder).
- [ ] `service`: helpers (`validate_common`, `clean`, `next_code`, `with_computed`, phones loader), then reads, `save_customer`/`save_supplier` (one generic core parameterised by kind + texts), link/unlink, net balance, history, aging — §3 order, mock line comments.
- [ ] `commands.rs`: 15 commands, Parties:Read/Write, `with_read`/`with_tx`.
- [ ] Manager: register 15 commands, `pub mod parties;`, hooks; propagate rule R-1 to the posting domains' files.
- [ ] Switch lines (§6); `src/modules/parties/types/contract.check.ts` (§2).
- [ ] `tests/domain_parties.rs` (§8a); parity list to Part 04; 00-import handoff note (D-7).
- [ ] Status note at the top of this file.

## Gate

`cargo check` clean (manager run) · tests written · switch lines · `contract.check.ts` compiles ·
`memory:check` 0 contract gaps for the 15 commands. DB tests/parity in the deferred time-boxed pass.
