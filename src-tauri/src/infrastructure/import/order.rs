//! `IMPORT_ORDER`/`DEFERRED` (`03-domains/00-import.md` §3.2.6, step 6): the FK-safe insert order
//! derived from `migration/src/fk_manifest_b1.md`, and the columns whose FK is set in a second pass
//! ("phase B", step 11) after every row exists, because they reference a row inserted later in this
//! same order (a self-reference, or a genuine forward reference).

/// Every table this importer writes, in FK-safe order. A table's FK targets (other than the ones
/// listed in [`DEFERRED`]) must all appear **earlier** in this list.
pub const IMPORT_ORDER: &[&str] = &[
    "currencies",
    "exchange_rates",
    "branches",
    "accounts",
    "cost_centers",
    "fiscal_years",
    "cost_center_budgets",
    "taxes",
    "payment_methods",
    "users",
    "credentials",
    "units",
    "categories",
    "price_lists",
    "party_groups",
    "parties",
    "party_phones",
    "custom_field_defs",
    "products",
    "product_prices",
    "product_branch_stock",
    "product_batches",
    "settings",
    "journal_entries",
    "journal_lines",
    "journal_drafts",
    "journal_draft_lines",
    "journal_templates",
    "invoices",
    "invoice_lines",
    "invoice_tenders",
    "refunds",
    "refund_lines",
    "quotations",
    "quotation_lines",
    "held_sales",
    "shifts",
    "shift_movements",
    "purchase_orders",
    "purchase_order_lines",
    "purchase_returns",
    "purchase_return_lines",
    "payments",
    "payment_allocations",
    "vouchers",
    "card_settlements",
    "card_settlement_groups",
    "expense_categories",
    "expenses",
    "recurring_expenses",
    "stock_adjustments",
    "stock_adjustment_lines",
    "stock_counts",
    "stock_count_lines",
    "stock_transfers",
    "stock_transfer_lines",
    "stock_movements",
    "debit_note_drafts",
    "party_history",
    "approval_requests",
    "print_templates",
    "audit",
    "activity",
];

/// `(table, column)` pairs inserted `NULL` in phase A (the main insert loop, in `IMPORT_ORDER`) and
/// set by one `UPDATE ... WHERE id = ?` per row in phase B, because the column's FK target is
/// either the table's own self-reference or a table that comes **later** in [`IMPORT_ORDER`]. Every
/// column here must be nullable in the schema (self-references and forward references both are).
pub const DEFERRED: &[(&str, &str)] = &[
    ("accounts", "parent_id"),
    ("categories", "parent_id"),
    ("cost_centers", "parent_id"),
    ("audit", "undo_of"),
    ("audit", "undone_by"),
    ("branches", "cash_account_id"),
    ("branches", "bank_account_id"),
    ("branches", "cost_center_id"),
    ("branches", "default_price_list_id"),
    ("cost_centers", "manager_user_id"),
    ("fiscal_years", "closing_entry_id"),
    ("fiscal_years", "closed_by"),
    ("users", "price_list_id"),
    ("journal_entries", "reversal_of_id"),
];

/// True if `(table, column)` is one of the [`DEFERRED`] pairs — phase A must write `NULL` for it
/// and phase B fills it in afterward, rather than resolving it inline during the main insert loop.
pub fn is_deferred(table: &str, column: &str) -> bool {
    DEFERRED.iter().any(|&(t, c)| t == table && c == column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn import_order_has_no_duplicate_tables() {
        let set: HashSet<&str> = IMPORT_ORDER.iter().copied().collect();
        assert_eq!(set.len(), IMPORT_ORDER.len(), "IMPORT_ORDER must not list a table twice");
    }

    #[test]
    fn is_deferred_matches_the_list_exactly() {
        assert!(is_deferred("accounts", "parent_id"));
        assert!(is_deferred("journal_entries", "reversal_of_id"));
        assert!(!is_deferred("accounts", "code"));
        assert!(!is_deferred("invoices", "customer_id"));
    }

    #[test]
    fn deferred_tables_all_appear_in_import_order() {
        for &(table, _) in DEFERRED {
            assert!(IMPORT_ORDER.contains(&table), "DEFERRED table '{table}' must also be in IMPORT_ORDER");
        }
    }
}
