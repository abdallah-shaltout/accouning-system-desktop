//! m0015_foreign_keys (21.02-B, owner B2): every FK across the whole schema, added last so table
//! creation order never matters (B-1 rule; resolves the circular pairs branches<->accounts,
//! branches<->cost_centers, fiscal_years.closing_entry_id<->journal_entries).
//!
//! Table/column names for B1's groups (org/accounts/users/catalog/inventory/parties) are taken
//! directly from `m0002_org.rs`..`m0007_parties.rs` as written by B1 in this same working tree.
//! `down` drops every FK added here (tables themselves are dropped by their owning migration's
//! own `down`).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// One `ALTER TABLE ... ADD CONSTRAINT fk_<table>_<col> FOREIGN KEY (...) REFERENCES ...` statement.
struct Fk {
    name: &'static str,
    table: &'static str,
    columns: &'static str,
    ref_table: &'static str,
    ref_columns: &'static str,
    on_delete: Option<&'static str>,
}

const CASCADE: Option<&str> = Some("CASCADE");

/// The full FK manifest. Grouped by the table the FK lives ON, in an order safe to apply given
/// every table already exists (m0002..m0014 all ran first). `down` drops these by name in reverse.
///
/// B1's section below is transcribed exactly from `fk_manifest_b1.md` (written by B1 for this
/// file) — table/column names there corrected several assumptions this file started with
/// (`categories`/`products` carry per-row tax/account fields, `stock_adjustments.approved_by`,
/// `stock_movements.batch_id`, `parties.linked_party_id` self-ref, a separate `party_phones` child
/// table, `parties.preferred_supplier_id`, etc.) — see the final report's "corrections" section.
/// Backtick-quotes each name in a `"a, b"` column list — some columns are reserved words
/// (`shift_movements.by`), which MariaDB rejects unquoted (error 1064).
fn quote_columns(list: &str) -> String {
    list.split(',').map(|c| format!("`{}`", c.trim())).collect::<Vec<_>>().join(", ")
}

fn manifest() -> Vec<Fk> {
    vec![
        // --- m0002_org (B1, per fk_manifest_b1.md) -----------------------------------------------
        Fk { name: "fk_branches_cash_account_id", table: "branches", columns: "cash_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_branches_bank_account_id", table: "branches", columns: "bank_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_branches_default_price_list_id", table: "branches", columns: "default_price_list_id", ref_table: "price_lists", ref_columns: "id", on_delete: None },
        Fk { name: "fk_branches_cost_center_id", table: "branches", columns: "cost_center_id", ref_table: "cost_centers", ref_columns: "id", on_delete: None },
        Fk { name: "fk_cost_centers_parent_id", table: "cost_centers", columns: "parent_id", ref_table: "cost_centers", ref_columns: "id", on_delete: None },
        Fk { name: "fk_cost_centers_manager_user_id", table: "cost_centers", columns: "manager_user_id", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_cost_centers_branch_id", table: "cost_centers", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: CASCADE },
        Fk { name: "fk_cost_center_budgets_cost_center_id", table: "cost_center_budgets", columns: "cost_center_id", ref_table: "cost_centers", ref_columns: "id", on_delete: CASCADE },
        Fk { name: "fk_cost_center_budgets_fiscal_year_id", table: "cost_center_budgets", columns: "fiscal_year_id", ref_table: "fiscal_years", ref_columns: "id", on_delete: None },
        Fk { name: "fk_fiscal_years_closing_entry_id", table: "fiscal_years", columns: "closing_entry_id", ref_table: "journal_entries", ref_columns: "id", on_delete: None },
        Fk { name: "fk_fiscal_years_closed_by", table: "fiscal_years", columns: "closed_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_exchange_rates_currency", table: "exchange_rates", columns: "currency", ref_table: "currencies", ref_columns: "code", on_delete: None },
        Fk { name: "fk_settings_default_tax_id", table: "settings", columns: "default_tax_id", ref_table: "taxes", ref_columns: "id", on_delete: None },
        Fk { name: "fk_settings_default_branch_id", table: "settings", columns: "default_branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_settings_currency", table: "settings", columns: "currency", ref_table: "currencies", ref_columns: "code", on_delete: None },

        // --- m0003_accounts (B1, per fk_manifest_b1.md) ------------------------------------------
        Fk { name: "fk_accounts_parent_id", table: "accounts", columns: "parent_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_accounts_branch_id", table: "accounts", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_accounts_currency", table: "accounts", columns: "currency", ref_table: "currencies", ref_columns: "code", on_delete: None },

        // --- m0004_users (B1, per fk_manifest_b1.md) ---------------------------------------------
        Fk { name: "fk_users_price_list_id", table: "users", columns: "price_list_id", ref_table: "price_lists", ref_columns: "id", on_delete: None },
        Fk { name: "fk_users_home_branch", table: "users", columns: "home_branch", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_credentials_user_id", table: "credentials", columns: "user_id", ref_table: "users", ref_columns: "id", on_delete: CASCADE },

        // --- m0005_catalog (B1, per fk_manifest_b1.md) -------------------------------------------
        Fk { name: "fk_categories_parent_id", table: "categories", columns: "parent_id", ref_table: "categories", ref_columns: "id", on_delete: None },
        Fk { name: "fk_categories_purchase_account_id", table: "categories", columns: "purchase_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_categories_revenue_account_id", table: "categories", columns: "revenue_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_categories_cogs_account_id", table: "categories", columns: "cogs_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_categories_sale_tax_id", table: "categories", columns: "sale_tax_id", ref_table: "taxes", ref_columns: "id", on_delete: None },
        Fk { name: "fk_categories_purchase_tax_id", table: "categories", columns: "purchase_tax_id", ref_table: "taxes", ref_columns: "id", on_delete: None },
        Fk { name: "fk_products_category_id", table: "products", columns: "category_id", ref_table: "categories", ref_columns: "id", on_delete: None },
        Fk { name: "fk_products_unit_id", table: "products", columns: "unit_id", ref_table: "units", ref_columns: "id", on_delete: None },
        Fk { name: "fk_products_purchase_account_id", table: "products", columns: "purchase_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_products_sale_tax_id", table: "products", columns: "sale_tax_id", ref_table: "taxes", ref_columns: "id", on_delete: None },
        Fk { name: "fk_products_purchase_tax_id", table: "products", columns: "purchase_tax_id", ref_table: "taxes", ref_columns: "id", on_delete: None },
        Fk { name: "fk_products_revenue_account_id", table: "products", columns: "revenue_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_products_cogs_account_id", table: "products", columns: "cogs_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_products_preferred_supplier_id", table: "products", columns: "preferred_supplier_id", ref_table: "parties", ref_columns: "id", on_delete: None },
        Fk { name: "fk_product_prices_product_id", table: "product_prices", columns: "product_id", ref_table: "products", ref_columns: "id", on_delete: CASCADE },
        Fk { name: "fk_product_prices_price_list_id", table: "product_prices", columns: "price_list_id", ref_table: "price_lists", ref_columns: "id", on_delete: CASCADE },
        Fk { name: "fk_product_prices_unit_id", table: "product_prices", columns: "unit_id", ref_table: "units", ref_columns: "id", on_delete: None },
        Fk { name: "fk_product_branch_stock_product_id", table: "product_branch_stock", columns: "product_id", ref_table: "products", ref_columns: "id", on_delete: CASCADE },
        Fk { name: "fk_product_branch_stock_branch_id", table: "product_branch_stock", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: CASCADE },
        Fk { name: "fk_product_batches_product_id", table: "product_batches", columns: "product_id", ref_table: "products", ref_columns: "id", on_delete: CASCADE },
        Fk { name: "fk_product_batches_supplier_id", table: "product_batches", columns: "supplier_id", ref_table: "parties", ref_columns: "id", on_delete: None },

        // --- m0006_inventory (B1, per fk_manifest_b1.md) -----------------------------------------
        Fk { name: "fk_stock_adjustments_offset_account_id", table: "stock_adjustments", columns: "offset_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_adjustments_approved_by", table: "stock_adjustments", columns: "approved_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_adjustment_lines_stock_adjustment_id", table: "stock_adjustment_lines", columns: "stock_adjustment_id", ref_table: "stock_adjustments", ref_columns: "id", on_delete: CASCADE },
        Fk { name: "fk_stock_adjustment_lines_product_id", table: "stock_adjustment_lines", columns: "product_id", ref_table: "products", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_movements_product_id", table: "stock_movements", columns: "product_id", ref_table: "products", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_movements_batch_id", table: "stock_movements", columns: "batch_id", ref_table: "product_batches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_counts_category_id", table: "stock_counts", columns: "category_id", ref_table: "categories", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_counts_started_by", table: "stock_counts", columns: "started_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_counts_adjustment_id", table: "stock_counts", columns: "adjustment_id", ref_table: "stock_adjustments", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_count_lines_stock_count_id", table: "stock_count_lines", columns: "stock_count_id", ref_table: "stock_counts", ref_columns: "id", on_delete: CASCADE },
        Fk { name: "fk_stock_count_lines_product_id", table: "stock_count_lines", columns: "product_id", ref_table: "products", ref_columns: "id", on_delete: None },
        Fk { name: "fk_debit_note_drafts_supplier_id", table: "debit_note_drafts", columns: "supplier_id", ref_table: "parties", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_transfers_from_branch_id", table: "stock_transfers", columns: "from_branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_transfers_to_branch_id", table: "stock_transfers", columns: "to_branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_transfers_sent_by", table: "stock_transfers", columns: "sent_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_transfers_received_by", table: "stock_transfers", columns: "received_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_transfers_rejected_by", table: "stock_transfers", columns: "rejected_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_transfer_lines_stock_transfer_id", table: "stock_transfer_lines", columns: "stock_transfer_id", ref_table: "stock_transfers", ref_columns: "id", on_delete: CASCADE },
        Fk { name: "fk_stock_transfer_lines_product_id", table: "stock_transfer_lines", columns: "product_id", ref_table: "products", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_transfer_lines_unit_id", table: "stock_transfer_lines", columns: "unit_id", ref_table: "units", ref_columns: "id", on_delete: None },
        Fk { name: "fk_stock_transfer_lines_batch_id", table: "stock_transfer_lines", columns: "batch_id", ref_table: "product_batches", ref_columns: "id", on_delete: None },

        // --- m0007_parties (B1, per fk_manifest_b1.md) -------------------------------------------
        Fk { name: "fk_parties_group_id", table: "parties", columns: "group_id", ref_table: "party_groups", ref_columns: "id", on_delete: None },
        Fk { name: "fk_parties_price_list_id", table: "parties", columns: "price_list_id", ref_table: "price_lists", ref_columns: "id", on_delete: None },
        Fk { name: "fk_parties_salesperson_id", table: "parties", columns: "salesperson_id", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_parties_branch_id", table: "parties", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_parties_linked_party_id", table: "parties", columns: "linked_party_id", ref_table: "parties", ref_columns: "id", on_delete: None },
        Fk { name: "fk_parties_default_expense_account_id", table: "parties", columns: "default_expense_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_party_phones_party_id", table: "party_phones", columns: "party_id", ref_table: "parties", ref_columns: "id", on_delete: CASCADE },
        Fk { name: "fk_party_groups_price_list_id", table: "party_groups", columns: "price_list_id", ref_table: "price_lists", ref_columns: "id", on_delete: None },
        Fk { name: "fk_party_history_party_id", table: "party_history", columns: "party_id", ref_table: "parties", ref_columns: "id", on_delete: CASCADE },
        // Composite (per fk_manifest_b1.md): enforces the history row's kind matches its party's.
        Fk { name: "fk_party_history_party_composite", table: "party_history", columns: "party_id, party_kind", ref_table: "parties", ref_columns: "id, kind", on_delete: CASCADE },

        // --- m0008_sales (B2, this migration group's own tables) ----------------------------------
        Fk { name: "fk_invoices_customer_id", table: "invoices", columns: "customer_id", ref_table: "parties", ref_columns: "id", on_delete: None },
        Fk { name: "fk_invoices_cashier_id", table: "invoices", columns: "cashier_id", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_invoices_shift_id", table: "invoices", columns: "shift_id", ref_table: "shifts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_invoices_branch_id", table: "invoices", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_invoices_cost_center_id", table: "invoices", columns: "cost_center_id", ref_table: "cost_centers", ref_columns: "id", on_delete: None },
        Fk { name: "fk_invoice_lines_product_id", table: "invoice_lines", columns: "product_id", ref_table: "products", ref_columns: "id", on_delete: None },
        Fk { name: "fk_invoice_lines_tax_id", table: "invoice_lines", columns: "tax_id", ref_table: "taxes", ref_columns: "id", on_delete: None },
        Fk { name: "fk_invoice_lines_unit_id", table: "invoice_lines", columns: "unit_id", ref_table: "units", ref_columns: "id", on_delete: None },
        Fk { name: "fk_invoice_lines_batch_id", table: "invoice_lines", columns: "batch_id", ref_table: "product_batches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_invoice_lines_revenue_account_id", table: "invoice_lines", columns: "revenue_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_invoice_tenders_payment_method_id", table: "invoice_tenders", columns: "payment_method_id", ref_table: "payment_methods", ref_columns: "id", on_delete: None },
        Fk { name: "fk_refund_lines_invoice_line_id", table: "refund_lines", columns: "invoice_line_id", ref_table: "invoice_lines", ref_columns: "id", on_delete: None },
        Fk { name: "fk_quotations_customer_id", table: "quotations", columns: "customer_id", ref_table: "parties", ref_columns: "id", on_delete: None },
        Fk { name: "fk_quotations_salesperson_id", table: "quotations", columns: "salesperson_id", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_quotation_lines_product_id", table: "quotation_lines", columns: "product_id", ref_table: "products", ref_columns: "id", on_delete: None },
        Fk { name: "fk_quotation_lines_tax_id", table: "quotation_lines", columns: "tax_id", ref_table: "taxes", ref_columns: "id", on_delete: None },
        Fk { name: "fk_quotation_lines_unit_id", table: "quotation_lines", columns: "unit_id", ref_table: "units", ref_columns: "id", on_delete: None },
        Fk { name: "fk_quotation_lines_batch_id", table: "quotation_lines", columns: "batch_id", ref_table: "product_batches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_quotation_lines_revenue_account_id", table: "quotation_lines", columns: "revenue_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_held_sales_customer_id", table: "held_sales", columns: "customer_id", ref_table: "parties", ref_columns: "id", on_delete: None },
        Fk { name: "fk_held_sales_held_by", table: "held_sales", columns: "held_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_shifts_branch_id", table: "shifts", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_shifts_opened_by", table: "shifts", columns: "opened_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_shifts_closed_by", table: "shifts", columns: "closed_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_shifts_force_closed_by", table: "shifts", columns: "force_closed_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_shift_movements_by", table: "shift_movements", columns: "by", ref_table: "users", ref_columns: "id", on_delete: None },

        // --- m0009_purchases (B2) -----------------------------------------------------------------
        Fk { name: "fk_purchase_orders_supplier_id", table: "purchase_orders", columns: "supplier_id", ref_table: "parties", ref_columns: "id", on_delete: None },
        Fk { name: "fk_purchase_orders_cost_center_id", table: "purchase_orders", columns: "cost_center_id", ref_table: "cost_centers", ref_columns: "id", on_delete: None },
        Fk { name: "fk_purchase_orders_branch_id", table: "purchase_orders", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_purchase_order_lines_product_id", table: "purchase_order_lines", columns: "product_id", ref_table: "products", ref_columns: "id", on_delete: None },
        Fk { name: "fk_purchase_order_lines_unit_id", table: "purchase_order_lines", columns: "unit_id", ref_table: "units", ref_columns: "id", on_delete: None },
        Fk { name: "fk_purchase_order_lines_tax_id", table: "purchase_order_lines", columns: "tax_id", ref_table: "taxes", ref_columns: "id", on_delete: None },
        Fk { name: "fk_purchase_returns_supplier_id", table: "purchase_returns", columns: "supplier_id", ref_table: "parties", ref_columns: "id", on_delete: None },
        Fk { name: "fk_purchase_return_lines_product_id", table: "purchase_return_lines", columns: "product_id", ref_table: "products", ref_columns: "id", on_delete: None },
        Fk { name: "fk_purchase_return_lines_batch_id", table: "purchase_return_lines", columns: "batch_id", ref_table: "product_batches", ref_columns: "id", on_delete: None },

        // --- m0010_payments (B2) ------------------------------------------------------------------
        Fk { name: "fk_payments_branch_id", table: "payments", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        // Composite (per fk_manifest_b1.md's "informational" note): only meaningful where
        // target_type is a party kind ('customer'/'supplier') — payments.target_type shares the
        // same two literal values as parties.kind, so the plain composite FK applies directly.
        Fk { name: "fk_payments_target_party", table: "payments", columns: "target_id, target_type", ref_table: "parties", ref_columns: "id, kind", on_delete: None },
        Fk { name: "fk_vouchers_cost_center_id", table: "vouchers", columns: "cost_center_id", ref_table: "cost_centers", ref_columns: "id", on_delete: None },
        Fk { name: "fk_vouchers_created_by", table: "vouchers", columns: "created_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_vouchers_payment_method_id", table: "vouchers", columns: "payment_method_id", ref_table: "payment_methods", ref_columns: "id", on_delete: None },
        Fk { name: "fk_vouchers_credit_account_id", table: "vouchers", columns: "credit_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_vouchers_debit_account_id", table: "vouchers", columns: "debit_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_vouchers_source_account_id", table: "vouchers", columns: "source_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_vouchers_destination_account_id", table: "vouchers", columns: "destination_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_vouchers_fee_account_id", table: "vouchers", columns: "fee_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_vouchers_cash_account_id", table: "vouchers", columns: "cash_account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_card_settlements_created_by", table: "card_settlements", columns: "created_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_card_settlement_groups_payment_method_id", table: "card_settlement_groups", columns: "payment_method_id", ref_table: "payment_methods", ref_columns: "id", on_delete: None },

        // --- m0011_expenses (B2) ------------------------------------------------------------------
        Fk { name: "fk_expense_categories_account_id", table: "expense_categories", columns: "account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_expense_categories_default_tax_id", table: "expense_categories", columns: "default_tax_id", ref_table: "taxes", ref_columns: "id", on_delete: None },
        Fk { name: "fk_expense_categories_default_cost_center_id", table: "expense_categories", columns: "default_cost_center_id", ref_table: "cost_centers", ref_columns: "id", on_delete: None },
        Fk { name: "fk_expenses_tax_id", table: "expenses", columns: "tax_id", ref_table: "taxes", ref_columns: "id", on_delete: None },
        Fk { name: "fk_expenses_cost_center_id", table: "expenses", columns: "cost_center_id", ref_table: "cost_centers", ref_columns: "id", on_delete: None },
        Fk { name: "fk_expenses_paid_from_payment_method_id", table: "expenses", columns: "paid_from_payment_method_id", ref_table: "payment_methods", ref_columns: "id", on_delete: None },
        Fk { name: "fk_expenses_paid_from_supplier_id", table: "expenses", columns: "paid_from_supplier_id", ref_table: "parties", ref_columns: "id", on_delete: None },
        Fk { name: "fk_expenses_recurring_template_id", table: "expenses", columns: "recurring_template_id", ref_table: "recurring_expenses", ref_columns: "id", on_delete: None },
        Fk { name: "fk_expenses_created_by", table: "expenses", columns: "created_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_expenses_branch_id", table: "expenses", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_recurring_expenses_tax_id", table: "recurring_expenses", columns: "tax_id", ref_table: "taxes", ref_columns: "id", on_delete: None },
        Fk { name: "fk_recurring_expenses_paid_from_payment_method_id", table: "recurring_expenses", columns: "paid_from_payment_method_id", ref_table: "payment_methods", ref_columns: "id", on_delete: None },
        Fk { name: "fk_recurring_expenses_paid_from_supplier_id", table: "recurring_expenses", columns: "paid_from_supplier_id", ref_table: "parties", ref_columns: "id", on_delete: None },

        // --- m0012_journal (B2) -------------------------------------------------------------------
        Fk { name: "fk_journal_entries_created_by", table: "journal_entries", columns: "created_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_journal_entries_posted_by", table: "journal_entries", columns: "posted_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_journal_entries_template_id", table: "journal_entries", columns: "template_id", ref_table: "journal_templates", ref_columns: "id", on_delete: None },
        Fk { name: "fk_journal_lines_account_id", table: "journal_lines", columns: "account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_journal_lines_branch_id", table: "journal_lines", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_journal_lines_cost_center_id", table: "journal_lines", columns: "cost_center_id", ref_table: "cost_centers", ref_columns: "id", on_delete: None },
        // P2-15 composite party FK: (party_id, party_kind) -> parties(id, kind).
        Fk { name: "fk_journal_lines_party", table: "journal_lines", columns: "party_id, party_kind", ref_table: "parties", ref_columns: "id, kind", on_delete: None },
        Fk { name: "fk_journal_drafts_created_by", table: "journal_drafts", columns: "created_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_journal_drafts_template_id", table: "journal_drafts", columns: "template_id", ref_table: "journal_templates", ref_columns: "id", on_delete: None },
        Fk { name: "fk_journal_draft_lines_account_id", table: "journal_draft_lines", columns: "account_id", ref_table: "accounts", ref_columns: "id", on_delete: None },
        Fk { name: "fk_journal_draft_lines_branch_id", table: "journal_draft_lines", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_journal_draft_lines_cost_center_id", table: "journal_draft_lines", columns: "cost_center_id", ref_table: "cost_centers", ref_columns: "id", on_delete: None },
        Fk { name: "fk_journal_templates_created_by", table: "journal_templates", columns: "created_by", ref_table: "users", ref_columns: "id", on_delete: None },

        // --- m0013_platform (B2) ------------------------------------------------------------------
        Fk { name: "fk_activity_user_id", table: "activity", columns: "user_id", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_activity_audit_id", table: "activity", columns: "audit_id", ref_table: "audit", ref_columns: "id", on_delete: None },
        Fk { name: "fk_audit_user_id", table: "audit", columns: "user_id", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_audit_branch_id", table: "audit", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
        Fk { name: "fk_audit_terminal_id", table: "audit", columns: "terminal_id", ref_table: "terminals", ref_columns: "id", on_delete: None },
        Fk { name: "fk_approval_requests_requested_by", table: "approval_requests", columns: "requested_by", ref_table: "users", ref_columns: "id", on_delete: None },
        Fk { name: "fk_approval_requests_decided_by", table: "approval_requests", columns: "decided_by", ref_table: "users", ref_columns: "id", on_delete: None },

        // --- m0014_templates (B2) -----------------------------------------------------------------
        Fk { name: "fk_print_templates_branch_id", table: "print_templates", columns: "branch_id", ref_table: "branches", ref_columns: "id", on_delete: None },
    ]
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        use sea_orm::{ConnectionTrait, Statement};
        let db = manager.get_connection();

        // One `ALTER TABLE` per table (not per FK), in manifest order.
        let mut groups: Vec<(&'static str, Vec<String>)> = Vec::new();
        for fk in manifest() {
            // `audit.terminal_id` references a `terminals` table that doesn't exist in this schema
            // (terminal identity is a local file per cross-cutting §2, not a branch-DB table) — skip
            // it; the column stays a plain nullable UUID with no FK, matching B-1's "no FK for
            // polymorphic/out-of-DB refs" spirit. Every other row above is applied for real.
            if fk.ref_table == "terminals" {
                continue;
            }
            let on_delete = fk.on_delete.map(|a| format!(" ON DELETE {a}")).unwrap_or_default();
            let clause = format!(
                "ADD CONSTRAINT `{name}` FOREIGN KEY ({columns}) REFERENCES `{ref_table}` ({ref_columns}){on_delete}",
                name = fk.name,
                columns = quote_columns(fk.columns),
                ref_table = fk.ref_table,
                ref_columns = quote_columns(fk.ref_columns),
            );
            match groups.iter_mut().find(|(t, _)| *t == fk.table) {
                Some((_, clauses)) => clauses.push(clause),
                None => groups.push((fk.table, vec![clause])),
            }
        }

        for (table, clauses) in groups {
            let alter = format!("ALTER TABLE `{table}` {}", clauses.join(", "));
            // An empty table has nothing to validate, so the FKs are added in place (metadata only,
            // no table copy) with checks off for this one statement — on a fresh schema that is
            // every table, and it turns minutes of table copies into seconds. A table that already
            // holds rows takes the normal checked path, so an orphan row still fails the migration.
            let empty_row = db
                .query_one(Statement::from_string(db.get_database_backend(), format!("SELECT NOT EXISTS(SELECT 1 FROM `{table}`) AS e")))
                .await?;
            let is_empty = match empty_row {
                Some(row) => row.try_get::<bool>("", "e").unwrap_or(false),
                None => false,
            };
            let sql = if is_empty { format!("SET STATEMENT foreign_key_checks=0 FOR {alter}") } else { alter };
            db.execute_unprepared(&sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for fk in manifest().into_iter().rev() {
            if fk.ref_table == "terminals" {
                continue;
            }
            let sql = format!("ALTER TABLE `{table}` DROP FOREIGN KEY `{name}`", table = fk.table, name = fk.name);
            // Best-effort: a fresh-DB down-to-zero test always succeeds in create order, so a drop
            // failure here would already indicate a real problem — propagate it rather than swallow.
            db.execute_unprepared(&sql).await?;
        }
        Ok(())
    }
}
