# 21 · 02.B — Entities and migrations for the 46 tables (+ `print_templates`)

> **Status:** code complete (B1: m0002–m0007 + org/catalog/inventory/parties + values/doc_date/
> soft_delete + `core/settings.rs` + `tx.rs` B-8 wiring; B2: m0008–m0015 incl. 156 FKs +
> sales/purchases/payments/expenses/journal/platform incl. `print_templates`, `shift_movements` +
> `architecture_rules` 5 rules; B-6/B-7 added in wave 2), not yet compiled against a DB/tested
> (build rule 2026-09-27).
>
> **Corrections to the inventory** (found auditing this file against the real code — fix in place,
> don't re-derive column lists from this file):
> - `branches` uses `active BOOL` (not `is_active`, and not a `deleted_at` soft-delete — confirms
>   this file's own §54 footnote: branch deactivate is a flag, not soft-delete).
> - `currencies`'s PK is the ISO code itself (`code CHAR(3) PRIMARY KEY`), not a UUID `Id` — the
>   one exception beyond `document_counters`/`change_versions` that B-1's PK rule already carves
>   out by name but this table inventory doesn't call out explicitly.
> - `stock_transfers` has a nullable DocDate: `date_day NOT NULL`/`date_instant NULL` for the
>   required `date`, but `sent_at`/`received_at`/`rejected_at` are each a fully optional DocDate
>   (`*_day: Option<NaiveDate>`, `*_instant: Option<DateTime<Utc>>`, `*_key: Option<String>`) —
>   B-1's DocDate list names these three but doesn't flag that, unlike every other DocDate field,
>   they may be entirely absent (a transfer that hasn't been sent/received/rejected yet).
> - `settings.insight_thresholds` (`InsightThresholds`, `entities/values.rs`) is a
>   `BTreeMap<String, JsonDecimal>`, not `BTreeMap<String, f64>` — `JsonDecimal` wraps
>   `rust_decimal::Decimal` via `serde(with = "crate::utils::money::serde_number")` specifically so
>   this JSON column doesn't violate `architecture_rules`' rule 1 (no `f32`/`f64` under
>   `entities/shared/domains`), which a plain `f64` map would have.
> - **m0013_foreign_keys is m0015_foreign_keys.** Every FK is one file, `migration/src/
>   m0015_foreign_keys.rs` (156 FKs, manifest at `migration/src/fk_manifest_b1.md`), added after
>   `m0014_templates.rs` (`print_templates`) — this file's B-2 table and Tasks list both still say
>   "m0013"; the actual migration group order is `m0002_org … m0007_parties, m0008_sales,
>   m0009_purchases, m0010_payments, m0011_expenses, m0012_journal, m0013_platform,
>   m0014_templates, m0015_foreign_keys`.
> - **Entities are not flat.** Every table's entity lives at `entities/<group>/<table>.rs`
>   (`entities/catalog/products.rs`, `entities/parties/parties.rs`, `entities/journal/
> journal_entries.rs`, `entities/payments/vouchers.rs`, …), grouped exactly as `entities/mod.rs`'s
>   doc comment describes ("grouped by owning module so the two parallel phase-B agents never edit
>   the same registration file"), not `entities/<table>.rs` as B-2's task line implies. Every
>   table's entity is re-exported flat from `entities::<table>::{Entity, ActiveModel, ...}`'s owning
>   group module, and also as `entities::<PascalTable>` via each group's `mod.rs` `pub use`.

**Goal:** every `MockDb` table (`src/mocks/db.ts:32-124`) plus `print_templates` (D9) exists
in MariaDB as SeaORM entities with real types, keys, FKs, uniques and checks. The field-level
mapping is **not** repeated here: each module's §2 table in `../01-frontend-analysis/<module>.md`
is the source, plus the TS type in `src/modules/<module>/types/index.ts`. This file fixes the
conventions, the table inventory and every constraint.

**Read first:** [`../01-frontend-analysis/cross-cutting.md`](../01-frontend-analysis/cross-cutting.md)
§3, §6, §7, §8 · §2 of `settings, users, accounting, products, parties, invoices, purchases,
payments, vouchers, expenses, setup, approvals, templates, diagnostics, core` (all in
`../01-frontend-analysis/`) · `src/mocks/db.ts`.

## B-1 — Conventions (apply to every table)

| Concern | Rule | Why |
|---|---|---|
| PK | `id UUID NOT NULL` (`Id`), except `document_counters`/`change_versions` | D2, P2-40 |
| Metadata | `created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3)`, `updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)`, `deleted_at DATETIME(3) NULL`, `sync_status ENUM('local','pending','synced') NOT NULL DEFAULT 'local'` on every business and child table. A TS `createdAt`/`updatedAt` maps to these columns (no duplicate column). No Part 02/03 code writes `sync_status`. | rule 8, D6. The session tz is UTC and the DB clock is the branch clock (P2-08). |
| Money / cost / qty / % / rate / FC / factor | `DECIMAL(19,2)` / `(19,4)` / `(19,4)` (values 2dp) / `(9,4)` / `(19,6)` / `(19,4)` / `(19,6)` | rule 5, P2-19, module §2 tables |
| Counts | `INT` | core.md §2 false-positive notes |
| Enums | MariaDB `ENUM(...)` with exactly the TS literals | module §2 tables |
| Business day | `DATE`; instant `DATETIME(3)` UTC; `DocDate` triple `x_day DATE NOT NULL`, `x_instant DATETIME(3) NULL`, `x_key VARCHAR(24) AS (COALESCE(<instant as %Y-%m-%dT%H:%i:%s.mmmZ>, DATE_FORMAT(x_day,'%Y-%m-%d'))) STORED` | cross-cutting §7, P2-09 |
| Text | names `VARCHAR(200)`, codes `VARCHAR(64)`, document numbers `VARCHAR(40)`, phones `VARCHAR(32)`, free text `TEXT`, base64 images `MEDIUMTEXT`, value objects `JSON` | P2-18 |
| Collation | default `utf8mb4_unicode_ci`. Columns the mock compares exactly (document numbers, account/category/unit/price-list names and codes, `search_normalized`) use `utf8mb4_bin`. Case-insensitive ones (`users.username`, `products.sku`, `branches.code`, `cost_centers.code`) use `utf8mb4_unicode_ci`. | P2-17 |
| Child lists | `position SMALLINT UNSIGNED NOT NULL` plus `UNIQUE(parent_id, position)` | array order is part of the DTO |
| FKs | `RESTRICT` by default. Child → parent `ON DELETE CASCADE` (only ever fires for drafts, because posted documents are never deleted). **No FK** for polymorphic refs: `journal_entries.source_id`, `audit.entity_id`, `activity`, `stock_movements.ref_id`, `product_batches.source_ref_id`, `payment_allocations.target_id`. | rule 7 |
| Names | `uq_<table>_<cols>`, `fk_<table>_<col>`, `ix_<table>_<cols>`, `ck_<table>_<rule>` | P2-22 |
| Partial unique | generated `…_key … AS (CASE WHEN <cond> THEN <cols> END) STORED` + `UNIQUE` | C-15 |
| Soft delete | tables: `accounts, categories, units, price_lists, custom_field_defs, taxes, payment_methods, cost_centers, expense_categories, recurring_expenses, journal_templates, print_templates`. Each uniqueness rule becomes a generated `<col>_live AS (CASE WHEN deleted_at IS NULL THEN <col> END) STORED` + `UNIQUE`. | P2-16 |
| Engine | `ENGINE=InnoDB`, `DEFAULT CHARSET=utf8mb4` | A-6 |

**DocDate fields** (confirm each against its mock writer before coding — a field that only ever
holds `YYYY-MM-DD` becomes plain `DATE`, and one that only ever holds instants becomes
`DATETIME(3)`): `journal_entries.date`, `journal_drafts.date`, `invoices.date`, `refunds.date`,
`quotations.date`, `purchase_orders.date`, `purchase_returns.date`, `payments.date`,
`payment_allocations.date`, `vouchers.date`, `expenses.date`, `card_settlements.date`,
`stock_adjustments.date`, `stock_movements.date`, `stock_transfers.{date,sent_at,received_at,rejected_at}`,
`debit_note_drafts.date`, `product_batches.received_date`, `activity.date`, `audit.at`.

## B-2 — Table inventory (46 `MockDb` keys → tables)

Every row below is one `MockDb` key from `src/mocks/db.ts:32-124`. "Migration group" is which
`m00NN_*` file creates it (grouped by owning module, in master §7's dependency order, so later
groups can FK into earlier ones without a forward reference until m0013). Columns are **not**
relisted here — derive them from the named type + its module's §2 table in `01-frontend-analysis/`,
applying B-1. Child tables (arrays nested inside a parent type) get their own row, `position`,
and a `parent_id` FK per B-1.

| Migration group | Table(s) | `MockDb` key | Source type(s) | Notes |
|---|---|---|---|---|
| m0002_org | `branches` | `branches` | `settings/types` `Branch` | `uq code_live` (ci soft-delete-aware, though branches aren't in the soft-delete list today — confirm against settings.md §2 whether branch deactivate is soft-delete or an `active` flag; branches.md/settings.md's activate/deactivate pair suggests an `active BOOL`, not `deleted_at` — use `active` here, not the soft-delete convention) |
| | `cost_centers` | `costCenters` | `settings/types` `CostCenter` | self `parent_id`; `branch_id`; `uq code_live`; soft-delete |
| | `cost_center_budgets` | child of `CostCenter.budgets` | — | child table; `uq(cost_center_id, fiscal_year_id)` |
| | `fiscal_years` | `fiscalYears` | `accounting/types` `FiscalYear` | `start_date`, `end_date DATE`; `ix(start_date, end_date)`; `closing_entry_id`, `closed_by` FKs added in m0013 |
| | `currencies` | `currencies` | `settings/types` `Currency` | `uq code CHAR(3)`; `fixed_rate (19,6)` |
| | `exchange_rates` | `exchangeRates` | `settings/types` `ExchangeRate` | `uq(currency, date)` |
| | `taxes` | `taxes` | `settings/types` `Tax` | `rate (9,4)`; soft-delete |
| | `settings` | `settings` | `settings/types` `StoreSettings` | singleton (`singleton TINYINT NOT NULL DEFAULT 1`, `uq`, `ck singleton=1`); **new** `timezone VARCHAR(64) NULL` (P2-08) and `default_branch_id NOT NULL` (P2-20); JSON sub-policies per module §2; device-only fields (printer connection etc., per cross-cutting §3) are **not** stored here — they live in `device-settings.json` |
| | `payment_methods` | `paymentMethods` | `settings/types` `PaymentMethod` | soft-delete |
| m0003_accounts | `accounts` | `accounts` | `accounting/types` `Account` | self `parent_id`; soft-delete; **no** uniqueness on `system_role` (C-15) |
| m0004_users | `users` | `users` | `users/types` `User` | `uq username_live` (ci) |
| | `credentials` | `credentials` | — (mock: `Record<username,password>`) | becomes `credentials(user_id UUID PK/FK -> users.id, password_hash VARCHAR(255))`, keyed by the immutable user id, not username (cross-cutting §1's fix for the mock's rekey-on-rename fragility) |
| m0005_catalog | `categories` | `categories` | `products/types` `Category` | self `parent_id`; soft-delete |
| | `units` | `units` | `products/types` `Unit` | soft-delete |
| | `price_lists` | `priceLists` | `products/types` `PriceList` | soft-delete |
| | `product_prices` | child of `PriceList`/`Product` override | — | child table (C-08: named `product_prices`, not `price_list_values`) |
| | `custom_field_defs` | `customFieldDefs` | `products/types` `CustomFieldDef` | soft-delete |
| | `products` | `products` | `products/types` `Product` | `uq sku_live` (ci); `search_normalized` (P2-38); qty/value/cost per B-1 |
| | `product_branch_stock` | child of `Product` per-branch stock | — | child table; `uq(product_id, branch_id)` |
| | `product_batches` | `productBatches` | `products/types` `ProductBatch` | `product_id`; `source_ref_id` no FK (polymorphic) |
| m0006_inventory | `stock_adjustments` | `stockAdjustments` | `products/types` `StockAdjustment` | header + lines child table |
| | `stock_movements` | `stockMovements` | `products/types` `StockMovement` | append-only; `ref_id` no FK |
| | `stock_counts` | `stockCounts` | `products/types` `StockCount` | header + lines child table |
| | `debit_note_drafts` | `debitNoteDrafts` | `products/types` `DebitNoteDraft` | worklist row, hard-deletable |
| | `stock_transfers` | `stockTransfers` | `products/types` `StockTransfer` | header + lines child table; `sent_at`/`received_at`/`rejected_at` DocDate |
| m0007_parties | `parties` | `customers` + `suppliers` | `parties/types` `Customer`, `Supplier` | **one table**, `kind ENUM('customer','supplier')` (P2-15); `search_normalized` |
| | `party_groups` | `partyGroups` | `parties/types` `PartyGroup` | |
| | `party_history` | `partyHistory` | `parties/types` `PartyHistoryEntry` | append-only; `link JSON` (RouteRef) |
| m0008_sales | `invoices` | `invoices` | `invoices/types` `Invoice` | header + `invoice_lines` + `invoice_tenders` child tables |
| | `refunds` | `refunds` | `invoices/types` `Refund` | header + `refund_lines` child table |
| | `quotations` | `quotations` | `invoices/types` `Quotation` | header + `quotation_lines` child table |
| | `held_sales` | `heldSales` | `invoices/types` `HeldSale` | per-terminal cart, JSON cart payload (P2-18); hard-deletable |
| | `shifts` | `shifts` | `invoices/types` `Shift` | generated `open_key AS (CASE WHEN status='OPEN' THEN terminal_id END) STORED` + `UNIQUE` (C-15, one open shift per terminal) |
| m0009_purchases | `purchase_orders` | `purchaseOrders` | `purchases/types` `PurchaseOrder` | header + `purchase_order_lines` child table |
| | `purchase_returns` | `purchaseReturns` | `purchases/types` `PurchaseReturn` | header + `purchase_return_lines` child table |
| m0010_payments | `payments` | `payments` | `payments/types` `Payment` | header + `payment_allocations` child table; `target_id`/`target_type` no FK (polymorphic, per B-1) |
| | `vouchers` | `vouchers` | `vouchers/types` `Voucher` | one flat table, nullable kind-specific columns (`kind ENUM('RECEIPT','PAYMENT','TRANSFER','OWNER')`, per `01-frontend-analysis/vouchers.md`'s modeling recommendation) |
| | `card_settlements` | `cardSettlements` | `vouchers/types` `CardSettlement` | header + `card_settlement_groups` child table; `uq_card_settlement_groups_date_method(date, payment_method_id)` (`01-frontend-analysis/vouchers.md`'s concurrency recommendation) |
| m0011_expenses | `expense_categories` | `expenseCategories` | `expenses/types` `ExpenseCategory` | soft-delete |
| | `expenses` | `expenses` | `expenses/types` `Expense` | |
| | `recurring_expenses` | `recurringExpenses` | `expenses/types` `RecurringExpense` | soft-delete; `day` gets a `CHECK (day BETWEEN 1 AND 28)` (accounting.md §9's clamp, applied here too since it's the same field shape) |
| m0011b_journal | `journal_entries` | `journalEntries` | `accounting/types` `JournalEntry` | header + `journal_lines` child; `search_normalized`; `ck_journal_entries_balanced` (P2-39) |
| | `journal_drafts` | `journalDrafts` | `accounting/types` `JournalEntry` (draft shape) | header + `journal_draft_lines` child; **separate** from `journal_entries` (P2-14) |
| | `journal_templates` | `journalTemplates` | `accounting/types` `JournalTemplate` | soft-delete |
| m0012_platform | `activity` | `activity` | `core/types` `ActivityEntry` | append-only; `link JSON` (RouteRef) |
| | `audit` | `audit` | `diagnostics/types` `AuditEntry` | append-only; `before`/`after JSON`; `link JSON`; **undo columns** (02.E): `action_type VARCHAR(64) NULL`, `payload JSON NULL`, `is_undoable BOOL NOT NULL DEFAULT FALSE`, `undo_of`/`undone_by` self FKs, `terminal_id UUID NULL`; `ix(entity, entity_id)` |
| | `approval_requests` | `approvalRequests` | `approvals/types` `ApprovalRequest` | `link JSON` (RouteRef) |
| m0012_templates | `print_templates` | D9 (`localStorage` today, not a `MockDb` key) | `templates/types` `PdfTemplate` | `branch_id`; generated `default_key AS (CASE WHEN is_default AND deleted_at IS NULL THEN CONCAT(branch_id,':',kind) END) STORED` + `UNIQUE` (C-15); soft-delete |
| m0015_foreign_keys | — | — | — | **every FK** is added here (156 FKs, manifest `migration/src/fk_manifest_b1.md`), so the create order never matters and the circular pairs (`branches`↔`accounts`, `branches`↔`cost_centers`, `fiscal_years.closing_entry_id`↔`journal_entries`) resolve. Includes the composite `(party_id, party_kind) → parties(id, kind)` on `journal_lines` and `(target_id, target_type) → parties(id, kind)` on `payments` (only where `target_type` is a party). Actual group order: `m0002_org…m0007_parties, m0008_sales, m0009_purchases, m0010_payments, m0011_expenses, m0012_journal, m0013_platform, m0014_templates, m0015_foreign_keys` (this row was written as "m0013" before the templates/platform split was finalized — corrected here). |

`document_counters` and `change_versions` are created in `m0001_infrastructure` (Phase A, A-7),
not here — they are the two exceptions to the per-table pattern (P2-40).

The **`counters`** `MockDb` key (`Record<DocumentKind, number>`) is **not** a separate table: it
is exactly `document_counters` (C-3, Phase A/C), already created in m0001.

## Tasks
- [x] B-1: write the migrations (`m0002`…`m0015_foreign_keys`) in `src-tauri/migration/src/`
      with sea-query builders, one file per migration group in B-2, applying every rule in B-1.
      Generated/CHECK columns use `ColumnDef::extra`/raw DDL where sea-query can't express them.
      Each migration's `down` drops its own tables. `m0015`'s `down` drops its FKs. ⏳ runs in the
      single final build/test pass (no DB server existed this session).
- [x] B-2: one entity file per table, grouped as `src-tauri/src/entities/<group>/<table>.rs`
      (not flat `entities/<table>.rs` — see "Corrections to the inventory"; hand-written
      `DeriveEntityModel`, `Id`/`Decimal`/`chrono` types, `Relation`s for every FK), plus
      `entities/mod.rs` (flat re-exports per group) — there is no separate `prelude.rs` file;
      `entities/mod.rs` itself serves that role via `pub use <group>::*`. Generated columns are
      read-only fields that no code sets.
- [x] B-3: `entities/soft_delete.rs`: a `SoftDelete` trait with `find_live()` and
      `soft_delete(conn, id, at)` (plus `restore`), implemented for the soft-delete tables in B-1.
- [x] B-4: the `DocDate` bridge (`entities/doc_date.rs`: `read`/`write`/`key`/`generated_key_extra`).
      Each entity with a DocDate field exposes `fn <field>(&self) -> DocDate` (or
      `Option<DocDate>` for `stock_transfers`' optional `sent_at`/`received_at`/`rejected_at`) and a
      setter that writes `_day`/`_instant` — confirmed present on `journal_entries`, `vouchers`,
      `stock_adjustments`, `stock_transfers`.
- [x] B-5: JSON value types (`entities/values.rs`: `Address`, `NationalAddress`, `RouteRef`,
      settings sub-policies incl. `InsightThresholds`/`JsonDecimal`, `AuditFieldDiff` lists,
      held-sale cart payload), shared structs deriving `FromJsonQueryResult`.
- [x] B-6/B-7 (added in wave 2 — were the only genuinely missing pieces): `ActiveModelBehavior::
      before_save` on exactly the 4 tables named in P2-38 computes `search_normalized` via
      `utils::text::search_haystack` from each table's own real search field set (found by
      grepping the mock/frontend search call sites, not guessed):
      - `products` (`entities/catalog/products.rs`): `name, sku, barcode` — `ProductListPage.vue`'s
        `matchesSearch([p.name, p.sku, p.barcode], search.value)`.
      - `parties` (`entities/parties/parties.rs`): `name, name_en, code, phone, vat_number,
        contact_person` — the union of `partyService.ts`'s `getCustomers`
        (`includesText([c.name, c.nameEn, c.code, c.phone, c.vatNumber], ...)`) and `getSuppliers`
        (`includesText([s.name, s.nameEn, s.code, s.phone, s.contactPerson, s.vatNumber], ...)`);
        `contact_person` is simply NULL on a customer row (P2-15 single table).
      - `journal_entries` (`entities/journal/journal_entries.rs`): `number, description,
        source_number` — `accountingService.ts`'s `matchesJournalFilter`'s
        `includesText([e.number, e.description, e.sourceRef?.number], filter.search)`, matching
        `JournalListPage.vue`'s search placeholder ("رقم القيد، البيان، أو المستند").
      - `vouchers` (`entities/payments/vouchers.rs`): `number, description, note` —
        `voucherService.ts`'s `includesText([v.number, v.description, v.note], filter.search)`.
      Recomputes whenever any searched field is `Set`; a field left `Unchanged`/`NotSet` on a
      partial update is read back from `self` via `ActiveValue::try_as_ref()` (works for both
      `Set` and `Unchanged`) — the strict "recompute from the row's full current state" choice, not
      "only recompute when every searched field happens to be present in this one update". No
      other entity gets a behaviour hook. Caveat: `before_save` only fires through
      `ActiveModel::insert/update/save` (SeaORM 1.1.20), never `insert_many`/`update_many` — any
      Part 03 bulk-write path for these 4 tables must go through the per-row form or compute the
      haystack itself.
- [x] B-8: `core/settings.rs`: `load(conn)` and `load_shared_locked(conn)` for the singleton row.
      `core/tx.rs` wires `TxCtx`: the clock's `tz` comes from `settings.timezone` (read together
      with `UTC_TIMESTAMP(3)`), and `cx.require(conn, area, access)` delegates to
      `core::settings::require` (`check_access` + `settings.role_access_overrides`, A-5).
- [x] B-9: `tests/architecture_rules.rs` (P2-31 — 5 rules, C/D/E's already folded in): no
      `f32`/`f64` under `entities/shared/domains`; no `begin`/`transaction(` outside `core/tx.rs`;
      no bare `find()` on soft-delete entities outside `entities/`; no `delete`/`delete_many`/
      `delete_by_id` on posted-document entities; audit/activity writes confined to
      `shared::activity`.

## Tests
- [x] `entities_match_schema_for_every_table` (written; runs in the final pass — needs a live
      MariaDB, none available this session): for every entity, its column set equals
      `information_schema.COLUMNS` for its table, and `SELECT <all columns> … LIMIT 0` succeeds.
- [x] Constraints (written; runs in the final pass): `journal_lines_checks_reject_two_sided_and_
      negative` rejects a two-sided or a negative line; the `journal_entries` balance CHECK; the
      open-shift generated unique (second OPEN on the same terminal → 1062); the default-template
      unique; `soft_deleted_category_name_can_be_recreated`; `uq_card_settlement_groups_date_method`
      rejects a duplicate day×method; the composite party FK rejects `party_kind='supplier'`
      pointing at a customer row — all in `tests/db_entities_foundation.rs` /
      `tests/db_entities_documents.rs`.
- [x] Round-trip (written; runs in the final pass): `invoice_with_lines_and_tenders_round_trips`,
      `settings_row_with_json_policies_round_trips_field_equal` (Decimal scale kept, DocDate both
      shapes via `doc_date_round_trips_both_shapes_on_stock_adjustments`); a party with phones is
      covered by `tests/db_entities_search.rs`'s parties test (this wave).
- [x] Collation (written; runs in the final pass): `sku_conflicts_case_insensitively_on_products`
      (`SKU-1` vs `sku-1`); `category_name_does_not_conflict_case_sensitively` (`Food` vs `food`).
- [x] `tests/db_entities_search.rs` (new, this wave; runs in the final pass): for each of the 4
      B-6/B-7 tables, inserting without `search_normalized` set fills it via `before_save`, and a
      partial update touching only one searched field still recomputes the full haystack from the
      row's current state (asserted byte-for-byte against `utils::text::search_haystack`, incl. an
      Arabic hamza-variant case on `products`).

## Gate
- [ ] `cargo build` + `cargo test` green (incl. `architecture_rules`, `entities_match_schema`,
      `db_entities_search`). ⏳ not run this session — no cargo/build commands permitted (agent
      constraint), single throttled build+test happens at the end per the manager's plan.
- [ ] Fresh DB: `Migrator::up` then `down` to zero then `up` again succeeds (test written:
      `migrator_up_down_up_succeeds`; not executed this session).
- [ ] `bun run build`, `check`, `verify:mocks` (128/0), `contract:check`, `memory` + `memory:check`, `diag:check`.
- [x] Status note at the top of this file.
