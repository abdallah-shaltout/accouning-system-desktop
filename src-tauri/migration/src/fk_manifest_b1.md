# FK manifest — B1 tables (m0002…m0007)

Written by B1 for B2, who owns `m0015_foreign_keys` (per B-1: no FK is added outside that one
file, so create order never matters). Format: `table.column → referenced_table(col)` `ON DELETE
<rule>`. Composite FKs are called out explicitly.

## m0002_org

| Column | References | ON DELETE | Notes |
|---|---|---|---|
| `branches.cash_account_id` | `accounts(id)` | RESTRICT | auto-created cash-drawer account |
| `branches.bank_account_id` | `accounts(id)` | RESTRICT | |
| `branches.default_price_list_id` | `price_lists(id)` | RESTRICT | |
| `branches.cost_center_id` | `cost_centers(id)` | RESTRICT | circular with `cost_centers.branch_id` — resolve in m0015 |
| `cost_centers.parent_id` | `cost_centers(id)` | RESTRICT | self-referential tree |
| `cost_centers.manager_user_id` | `users(id)` | RESTRICT | |
| `cost_centers.branch_id` | `branches(id)` | CASCADE | auto-created with its branch; circular with `branches.cost_center_id` |
| `cost_center_budgets.cost_center_id` | `cost_centers(id)` | CASCADE | child row |
| `cost_center_budgets.fiscal_year_id` | `fiscal_years(id)` | RESTRICT | |
| `fiscal_years.closing_entry_id` | `journal_entries(id)` | RESTRICT | forward ref — journal_entries created in m0011b/m0012_journal |
| `fiscal_years.closed_by` | `users(id)` | RESTRICT | |
| `exchange_rates.currency` | `currencies(code)` | RESTRICT | |
| `settings.default_tax_id` | `taxes(id)` | RESTRICT | |
| `settings.default_branch_id` | `branches(id)` | RESTRICT | P2-20: every settings row must resolve to a real branch |
| `settings.currency` | `currencies(code)` | RESTRICT | base currency must be an enabled currency row (confirm with B2/master plan whether base currency is *required* to also exist in `currencies` — if not, drop this one) |

## m0003_accounts

| Column | References | ON DELETE | Notes |
|---|---|---|---|
| `accounts.parent_id` | `accounts(id)` | RESTRICT | self-referential tree |
| `accounts.branch_id` | `branches(id)` | RESTRICT | branch-scoped cash drawer etc. |
| `accounts.currency` | `currencies(code)` | RESTRICT | nullable — only for FC cash/bank accounts |

## m0004_users

| Column | References | ON DELETE | Notes |
|---|---|---|---|
| `users.price_list_id` | `price_lists(id)` | RESTRICT | |
| `users.home_branch` | `branches(id)` | RESTRICT | |
| `credentials.user_id` | `users(id)` | CASCADE | PK is also the FK — a credentials row has no meaning without its user |

## m0005_catalog

| Column | References | ON DELETE | Notes |
|---|---|---|---|
| `categories.parent_id` | `categories(id)` | RESTRICT | self-referential tree |
| `categories.purchase_account_id` | `accounts(id)` | RESTRICT | |
| `categories.revenue_account_id` | `accounts(id)` | RESTRICT | |
| `categories.cogs_account_id` | `accounts(id)` | RESTRICT | |
| `categories.sale_tax_id` | `taxes(id)` | RESTRICT | |
| `categories.purchase_tax_id` | `taxes(id)` | RESTRICT | |
| `products.category_id` | `categories(id)` | RESTRICT | |
| `products.unit_id` | `units(id)` | RESTRICT | |
| `products.purchase_account_id` | `accounts(id)` | RESTRICT | |
| `products.sale_tax_id` | `taxes(id)` | RESTRICT | |
| `products.purchase_tax_id` | `taxes(id)` | RESTRICT | |
| `products.revenue_account_id` | `accounts(id)` | RESTRICT | |
| `products.cogs_account_id` | `accounts(id)` | RESTRICT | |
| `products.preferred_supplier_id` | `parties(id)` | RESTRICT | plain FK to `parties.id` is enough here (no kind check needed — a UI-level "must be a supplier" concern, not a DB constraint by itself, unless the composite `(id,kind)` unique lets a composite FK enforce it too: `(preferred_supplier_id, 'supplier') → parties(id, kind)` is possible if desired — flagged as B2's call) |
| `product_prices.product_id` | `products(id)` | CASCADE | child row |
| `product_prices.price_list_id` | `price_lists(id)` | CASCADE | child row |
| `product_prices.unit_id` | `units(id)` | RESTRICT | |
| `product_branch_stock.product_id` | `products(id)` | CASCADE | child row |
| `product_branch_stock.branch_id` | `branches(id)` | CASCADE | child row |
| `product_batches.product_id` | `products(id)` | CASCADE | child row |
| `product_batches.supplier_id` | `parties(id)` | RESTRICT | (no kind check by plain FK — see products.preferred_supplier_id note) |

(`product_batches.source_ref_id` and `stock_movements.ref_id` are polymorphic — **no FK**, per B-1.)

## m0006_inventory

| Column | References | ON DELETE | Notes |
|---|---|---|---|
| `stock_adjustments.offset_account_id` | `accounts(id)` | RESTRICT | STOCK_IN reason=other only |
| `stock_adjustments.approved_by` | `users(id)` | RESTRICT | |
| `stock_adjustment_lines.stock_adjustment_id` | `stock_adjustments(id)` | CASCADE | child row |
| `stock_adjustment_lines.product_id` | `products(id)` | RESTRICT | |
| `stock_movements.product_id` | `products(id)` | RESTRICT | append-only ledger, never cascaded away |
| `stock_movements.batch_id` | `product_batches(id)` | RESTRICT | |
| `stock_counts.category_id` | `categories(id)` | RESTRICT | |
| `stock_counts.started_by` | `users(id)` | RESTRICT | |
| `stock_counts.adjustment_id` | `stock_adjustments(id)` | RESTRICT | set once applied |
| `stock_count_lines.stock_count_id` | `stock_counts(id)` | CASCADE | child row |
| `stock_count_lines.product_id` | `products(id)` | RESTRICT | |
| `debit_note_drafts.supplier_id` | `parties(id)` | RESTRICT | (supplier-kind check at app layer, or composite FK per products note above) |
| `stock_transfers.from_branch_id` | `branches(id)` | RESTRICT | |
| `stock_transfers.to_branch_id` | `branches(id)` | RESTRICT | |
| `stock_transfers.sent_by` | `users(id)` | RESTRICT | |
| `stock_transfers.received_by` | `users(id)` | RESTRICT | |
| `stock_transfers.rejected_by` | `users(id)` | RESTRICT | |
| `stock_transfer_lines.stock_transfer_id` | `stock_transfers(id)` | CASCADE | child row |
| `stock_transfer_lines.product_id` | `products(id)` | RESTRICT | |
| `stock_transfer_lines.unit_id` | `units(id)` | RESTRICT | |
| `stock_transfer_lines.batch_id` | `product_batches(id)` | RESTRICT | |

## m0007_parties

| Column | References | ON DELETE | Notes |
|---|---|---|---|
| `parties.group_id` | `party_groups(id)` | RESTRICT | |
| `parties.price_list_id` | `price_lists(id)` | RESTRICT | |
| `parties.salesperson_id` | `users(id)` | RESTRICT | |
| `parties.branch_id` | `branches(id)` | RESTRICT | |
| `parties.linked_party_id` | `parties(id)` | RESTRICT | self-referential "both roles" link |
| `parties.default_expense_account_id` | `accounts(id)` | RESTRICT | supplier-only field |
| `party_phones.party_id` | `parties(id)` | CASCADE | child row |
| `party_groups.price_list_id` | `price_lists(id)` | RESTRICT | |
| `party_history.party_id` | `parties(id)` | CASCADE | child-of-party (per cross-cutting §8) |
| **composite** `party_history.(party_id, party_kind)` | `parties(id, kind)` | CASCADE | enforces the history row's kind matches its party's actual kind |

## Composite FKs this file's tables need from OTHER migration groups (informational, for m0015)

These aren't m0002-m0007 columns but are called out in the phase-b spec as depending on the
`parties` composite unique this file creates:

- `journal_lines.(party_id, party_kind) → parties(id, kind)` — B2's table, composite FK against
  B1's `uq_parties_id_kind`.
- `payments.(target_id, target_type) → parties(id, kind)` — only where `target_type` is a party
  kind (B2's table/column, same composite unique).

## Notes for B2

- Every `branches`/`accounts`/`cost_centers` circular pair above is exactly why they're all
  deferred to m0015 — none of m0002/m0003 can add these FKs themselves without a forward
  reference.
- `fiscal_years.closing_entry_id`/`closed_by` and `settings.default_tax_id` are also forward refs
  (into `journal_entries`/`taxes`, `taxes` is same-migration so that one *could* be added in m0002
  if you'd rather keep it local — flagged, not applied, since B-1 says "no FK in these files" without
  exception).
- `parties.preferred_supplier_id`/`product_batches.supplier_id`/`debit_note_drafts.supplier_id`
  wanting "must be a supplier, not a customer" is exactly the same composite-FK shape as the
  `journal_lines`/`payments` cases above (`(col, 'supplier') → parties(id, kind)`) if you want DB-level
  enforcement rather than an app-layer check — your call since it wasn't explicitly asked for by the
  phase-b spec beyond the two composite FKs it names.
