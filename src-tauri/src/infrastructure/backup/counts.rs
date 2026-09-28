//! `counts.rs` (17-backup.md §3.6) — parity with `tableCounts` (`backupArchive.ts:63-70`): a fixed,
//! ordered list of table counts (the `MockDb` array keys in `blankDb()` order), each a real
//! `SELECT COUNT(*)` of **live** rows (soft-delete tables filtered by `deleted_at IS NULL`;
//! `customers`/`suppliers` both read the one `parties` table, split by `kind`).

use sea_orm::{ConnectionTrait, Statement};

use crate::core::tx::TxResult;

use super::dto::OrderedCounts;

/// The fixed key order (§3.6) — the `MockDb` array keys in `blankDb()` order, then `attachments: 0`.
pub const COUNT_KEYS: &[&str] = &[
    "users",
    "accounts",
    "journalEntries",
    "journalDrafts",
    "journalTemplates",
    "fiscalYears",
    "categories",
    "units",
    "priceLists",
    "products",
    "stockAdjustments",
    "stockMovements",
    "productBatches",
    "customFieldDefs",
    "stockCounts",
    "debitNoteDrafts",
    "customers",
    "suppliers",
    "partyGroups",
    "partyHistory",
    "invoices",
    "refunds",
    "quotations",
    "heldSales",
    "shifts",
    "purchaseOrders",
    "purchaseReturns",
    "payments",
    "taxes",
    "paymentMethods",
    "expenseCategories",
    "expenses",
    "recurringExpenses",
    "vouchers",
    "cardSettlements",
    "branches",
    "costCenters",
    "stockTransfers",
    "currencies",
    "exchangeRates",
    "approvalRequests",
    "activity",
    "audit",
];

/// The 12 soft-delete tables (`tests/architecture_rules.rs`'s `SOFT_DELETE_TABLES` list) — counted
/// with `WHERE deleted_at IS NULL` so a backup's counts preview matches what the UI actually has.
const SOFT_DELETE_TABLES: &[&str] = &[
    "accounts",
    "categories",
    "units",
    "price_lists",
    "custom_field_defs",
    "taxes",
    "payment_methods",
    "cost_centers",
    "expense_categories",
    "recurring_expenses",
    "journal_templates",
    "print_templates",
];

fn camel_to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        if c.is_ascii_uppercase() {
            out.push('_');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

async fn count_table<C: ConnectionTrait>(conn: &C, table: &str) -> Result<i64, sea_orm::DbErr> {
    let where_clause = if SOFT_DELETE_TABLES.contains(&table) { " WHERE `deleted_at` IS NULL" } else { "" };
    let stmt = Statement::from_string(conn.get_database_backend(), format!("SELECT COUNT(*) AS n FROM `{table}`{where_clause}"));
    let row = conn.query_one(stmt).await?;
    match row {
        Some(row) => row.try_get::<i64>("", "n"),
        None => Ok(0),
    }
}

async fn count_parties<C: ConnectionTrait>(conn: &C, kind: &str) -> Result<i64, sea_orm::DbErr> {
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT COUNT(*) AS n FROM `parties` WHERE `kind` = ? AND `deleted_at` IS NULL",
        [kind.into()],
    );
    let row = conn.query_one(stmt).await?;
    match row {
        Some(row) => row.try_get::<i64>("", "n"),
        None => Ok(0),
    }
}

/// `settings_preview_backup_counts` / the manifest's `counts` field: one `SELECT COUNT(*)` per fixed
/// key, in that exact order, then `attachments: 0` (D-5 — Rust archives never carry attachment blobs
/// while C-16 is open).
pub async fn table_counts<C: ConnectionTrait>(conn: &C) -> TxResult<OrderedCounts> {
    let mut counts = OrderedCounts::default();
    for key in COUNT_KEYS {
        let n = match *key {
            "customers" => count_parties(conn, "customer").await,
            "suppliers" => count_parties(conn, "supplier").await,
            other => count_table(conn, &camel_to_snake(other)).await,
        }?;
        counts.push(*key, n);
    }
    counts.push("attachments", 0);
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camel_to_snake_converts_every_fixed_key() {
        assert_eq!(camel_to_snake("journalEntries"), "journal_entries");
        assert_eq!(camel_to_snake("debitNoteDrafts"), "debit_note_drafts");
        assert_eq!(camel_to_snake("users"), "users");
        assert_eq!(camel_to_snake("costCenters"), "cost_centers");
    }

    #[test]
    fn count_keys_match_the_spec_order_and_length() {
        // 42 real keys (§3.6's list) + attachments pushed separately by table_counts.
        assert_eq!(COUNT_KEYS.len(), 42);
        assert_eq!(COUNT_KEYS.first(), Some(&"users"));
        assert_eq!(COUNT_KEYS.last(), Some(&"audit"));
    }
}
