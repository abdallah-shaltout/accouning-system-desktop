//! m0018_signed_small_ints (plan 21 DB test pass): every `SMALLINT UNSIGNED` / `TINYINT UNSIGNED`
//! column becomes signed.
//!
//! Why: the entities (`src/entities/**`) map these columns to `i16` / `i8` (and the ts-rs DTOs and
//! domain code build on those types), but sqlx's MySQL driver refuses to decode an `UNSIGNED` column
//! into a signed Rust integer — the first real DB run failed with `decoding column "sort_order":
//! ... Option<i16> ... not compatible with SQL type SMALLINT UNSIGNED` (setup's
//! `seed_company_shell`), and every `position` column had the same latent mismatch. Fixing the
//! schema side keeps the entity/DTO types (and the generated TS bindings) unchanged, the same choice
//! m0001 already made for `BIGINT` (`sqlx refuses UNSIGNED → i64`).
//!
//! The "never negative" guarantee `UNSIGNED` gave is kept as a named `CHECK (col >= 0)` per column
//! (`ck_<table>_<column>_non_negative`, the m0012 naming). `recurring_expenses.day` already has
//! `ck_recurring_expenses_day` (`BETWEEN 1 AND 28`), so it gets no extra check.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// `(table, column, signed type, null/default tail, add a non-negative CHECK)`.
const COLUMNS: &[(&str, &str, &str, &str, bool)] = &[
    ("payment_methods", "sort_order", "SMALLINT", "NOT NULL DEFAULT 0", true),
    ("custom_field_defs", "sort_order", "SMALLINT", "NOT NULL DEFAULT 0", true),
    ("stock_adjustment_lines", "position", "SMALLINT", "NOT NULL", true),
    ("stock_count_lines", "position", "SMALLINT", "NOT NULL", true),
    ("stock_transfer_lines", "position", "SMALLINT", "NOT NULL", true),
    ("party_phones", "position", "SMALLINT", "NOT NULL", true),
    ("invoice_lines", "position", "SMALLINT", "NOT NULL", true),
    ("invoice_tenders", "position", "SMALLINT", "NOT NULL", true),
    ("refund_lines", "position", "SMALLINT", "NOT NULL", true),
    ("quotation_lines", "position", "SMALLINT", "NOT NULL", true),
    ("shift_movements", "position", "SMALLINT", "NOT NULL", true),
    ("purchase_order_lines", "position", "SMALLINT", "NOT NULL", true),
    ("purchase_return_lines", "position", "SMALLINT", "NOT NULL", true),
    ("payment_allocations", "position", "SMALLINT", "NOT NULL", true),
    ("card_settlement_groups", "position", "SMALLINT", "NOT NULL", true),
    ("journal_lines", "position", "SMALLINT", "NOT NULL", true),
    ("journal_draft_lines", "position", "SMALLINT", "NOT NULL", true),
    ("recurring_expenses", "day", "TINYINT", "NOT NULL", false),
    ("journal_templates", "recurrence_day", "TINYINT", "NULL", true),
];

fn check_name(table: &str, column: &str) -> String {
    format!("ck_{table}_{column}_non_negative")
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for (table, column, ty, tail, check) in COLUMNS {
            let mut sql = format!("ALTER TABLE `{table}` MODIFY COLUMN `{column}` {ty} {tail}");
            if *check {
                sql.push_str(&format!(", ADD CONSTRAINT {} CHECK (`{column}` >= 0)", check_name(table, column)));
            }
            db.execute_unprepared(&sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for (table, column, ty, tail, check) in COLUMNS.iter().rev() {
            if *check {
                db.execute_unprepared(&format!("ALTER TABLE `{table}` DROP CONSTRAINT {}", check_name(table, column))).await?;
            }
            db.execute_unprepared(&format!("ALTER TABLE `{table}` MODIFY COLUMN `{column}` {ty} UNSIGNED {tail}")).await?;
        }
        Ok(())
    }
}
