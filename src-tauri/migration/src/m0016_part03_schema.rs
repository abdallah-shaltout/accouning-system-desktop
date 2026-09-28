//! m0016_part03_schema (21.03 Wave 0, manager gap fixes G-17/G-22/G-28/G-11): the schema gaps
//! discovered while planning Part 03's domain files, collected in
//! `plans/pending/21-rust-backend/03-domains/_part02-gaps.md`. Never an edit to m0001–m0015 (those
//! passed Part 02's tests already) — every fix here is a fresh `ALTER TABLE`/`ADD INDEX` on top of
//! the existing tables.
//!
//! Covers:
//! - **G-17** `accounts.code` has no unique index. 12-accounting needs one (`resolveAccount`-style
//!   lookups and the controller-level uniqueness check both assume it). Accounts is a soft-delete
//!   table (`org::Accounts` implements `SoftDelete`), so this follows the same `_live` generated
//!   column + unique pattern as `categories.name_live`/`products.sku_live` (m0002/m0005) rather than
//!   a plain unique on `code` — a soft-deleted account must free its code for reuse.
//! - **G-22** free-text invoice/quotation lines: `invoice_lines.product_id` and
//!   `quotation_lines.product_id` become nullable (both FKs, added in m0015, already tolerate NULL —
//!   only the column itself was `NOT NULL`, left over from before D-I2 was decided). `invoices.due_date`
//!   and `quotations.expiry_date` are full timestamps in the mock (`computeDueDate`/the desk form
//!   both store ISO instants — a plain `DATE` silently drops the time part and changes `isOverdue`'s
//!   string compare), so both become a `DocDate` triple (`<field>_day`/`<field>_instant`/generated
//!   `<field>_key`), following `entities/doc_date.rs`'s convention. The old single `DATE` columns are
//!   dropped; `_day` is backfilled from them first so no data is lost.
//! - **G-28** (06-products §7 G-P4): (a) `categories`/`units`/`price_lists` `name_live` uniques were
//!   built `utf8mb4_unicode_ci` (case-insensitive) in m0005, but P2-17 requires an **exact** (`bin`)
//!   compare for these three tables ("Food" and "food" must NOT conflict); (b) a generated
//!   `products.barcode_live` (live rows only, following the `sku_live` pattern) + a unique index, so
//!   two live products can no longer share a barcode at the DB level (today's plain, non-unique
//!   `ix_products_barcode` stays as a lookup index); (c) `purchase_orders.received_date` and
//!   `product_batches.received_date` become `DocDate` triples — both are set from ISO instants in the
//!   mock (`confirmPurchaseOrder`/opening or completion dates), and a plain `DATE` truncates them;
//!   (d) a `productCodesLock` row seeded into `document_counters`, backing a new
//!   `SequenceLock::ProductCodes` the manager adds to `shared::numbering` (not a schema change, so
//!   only the row is seeded here — see this file's header note below).
//! - **G-11** the two party `balance`/`unallocated_credit` columns are dropped: nothing in `src/`
//!   reads or writes `parties::Model.balance`/`unallocated_credit` (confirmed by search — every
//!   caller goes through `shared::balances::{customer_balance, supplier_balance, unallocated_credit_for}`,
//!   which derive the figure from the ledger on every call, exactly like the mock's own computed
//!   getters). Keeping two always-stale, never-written columns would be a foot-gun for a future
//!   direct read, so they are dropped rather than left in place.
//!
//! **G-27** (manager-PIN approval grants, 06b-inventory) is *not* a schema change: 06b's own D-I2/
//! G-P3 spec is `AppState.approval_grants` — an in-process, in-memory `Id -> Instant` map with a
//! 10-minute TTL, not a persisted table (a grant is deliberately per-terminal-process and must not
//! survive a restart). That lives in `core/state.rs` (manager-owned), so there is nothing to add
//! here; see this file's implementer report.
//!
//! **G-P4c** (`SequenceLock::ProductCodes`) is a `shared::numbering` enum addition (manager-owned
//! `src/shared/numbering.rs`), also not something this migration can add — but the lock *row* it
//! reads (`document_counters.kind = 'productCodesLock'`) is pure data, seeded below so the row exists
//! the moment the enum variant lands.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        g17_accounts_code_unique(db).await?;
        g22_invoice_quotation_lines_and_dates(db).await?;
        g28_catalog_name_live_collation_and_barcode(db).await?;
        g28d_received_date_doc_dates(db).await?;
        g28d_product_codes_lock_row(db).await?;
        g11_drop_unused_party_balance_columns(db).await?;
        g49_draft_line_fc_columns(db).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // --- G-49 (reverse) ---
        db.execute_unprepared("ALTER TABLE `journal_draft_lines` DROP COLUMN `rate`, DROP COLUMN `amount_fc`, DROP COLUMN `currency`").await?;

        // --- G-11 (reverse: restore the columns, zeroed — the mock never persisted real values
        // either, so there is nothing meaningful to backfill) ---
        db.execute_unprepared(
            "ALTER TABLE `parties` \
             ADD COLUMN `balance` DECIMAL(19,2) NOT NULL DEFAULT 0, \
             ADD COLUMN `unallocated_credit` DECIMAL(19,2) NULL",
        )
        .await?;

        // --- G-28d (reverse: drop the lock row and revert received_date to plain DATE) ---
        db.execute_unprepared("DELETE FROM `document_counters` WHERE `kind` = 'productCodesLock'").await?;
        revert_doc_date_to_plain_date(db, "purchase_orders", "received_date").await?;
        revert_doc_date_to_plain_date(db, "product_batches", "received_date").await?;

        // --- G-28 (reverse) ---
        db.execute_unprepared("ALTER TABLE `products` DROP INDEX `uq_products_barcode_live`, DROP COLUMN `barcode_live`").await?;
        db.execute_unprepared(
            "ALTER TABLE `categories` DROP INDEX `uq_categories_name_live`, DROP COLUMN `name_live`, \
             ADD COLUMN `name_live` VARCHAR(200) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci \
             GENERATED ALWAYS AS (CASE WHEN `deleted_at` IS NULL THEN `name` END) STORED, \
             ADD UNIQUE INDEX `uq_categories_name_live` (`name_live`)",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE `units` DROP INDEX `uq_units_name_live`, DROP COLUMN `name_live`, \
             ADD COLUMN `name_live` VARCHAR(200) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci \
             GENERATED ALWAYS AS (CASE WHEN `deleted_at` IS NULL THEN `name` END) STORED, \
             ADD UNIQUE INDEX `uq_units_name_live` (`name_live`)",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE `price_lists` DROP INDEX `uq_price_lists_name_live`, DROP COLUMN `name_live`, \
             ADD COLUMN `name_live` VARCHAR(200) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci \
             GENERATED ALWAYS AS (CASE WHEN `deleted_at` IS NULL THEN `name` END) STORED, \
             ADD UNIQUE INDEX `uq_price_lists_name_live` (`name_live`)",
        )
        .await?;

        // --- G-22 (reverse: back to plain DATE + NOT NULL product_id) ---
        revert_doc_date_to_plain_date(db, "invoices", "due_date").await?;
        revert_doc_date_to_plain_date(db, "quotations", "expiry_date").await?;
        db.execute_unprepared("ALTER TABLE `invoice_lines` MODIFY COLUMN `product_id` CHAR(36) NOT NULL").await?;
        db.execute_unprepared("ALTER TABLE `quotation_lines` MODIFY COLUMN `product_id` CHAR(36) NOT NULL").await?;

        // --- G-17 (reverse) ---
        db.execute_unprepared("ALTER TABLE `accounts` DROP INDEX `uq_accounts_code_live`, DROP COLUMN `code_live`").await?;

        Ok(())
    }
}

/// G-17: `accounts.code` unique while live. `accounts` is `utf8mb4_bin` already (m0003), so the
/// generated column inherits an exact (case-sensitive) compare with no explicit collation override —
/// matching the mock's plain `===` uniqueness check in the accounts controller.
async fn g17_accounts_code_unique<C: sea_orm::ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    db.execute_unprepared(
        "ALTER TABLE `accounts` \
         ADD COLUMN `code_live` VARCHAR(64) \
         GENERATED ALWAYS AS (CASE WHEN `deleted_at` IS NULL THEN `code` END) STORED, \
         ADD UNIQUE INDEX `uq_accounts_code_live` (`code_live`)",
    )
    .await?;
    Ok(())
}

/// G-22: free-text lines (`product_id` NULL) + `invoices.due_date`/`quotations.expiry_date` become
/// `DocDate` triples. Existing rows: `product_id` was `NOT NULL` so every existing row already has a
/// value (nothing to backfill there); `due_date`/`expiry_date` backfill their `_day` from the old
/// `DATE` column 1:1 (no existing row carries a time part, since the column was never wider than a
/// date) and leave `_instant` NULL, which is exactly what a plain `DATE` write would have meant.
async fn g22_invoice_quotation_lines_and_dates<C: sea_orm::ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    db.execute_unprepared("ALTER TABLE `invoice_lines` MODIFY COLUMN `product_id` CHAR(36) NULL").await?;
    db.execute_unprepared("ALTER TABLE `quotation_lines` MODIFY COLUMN `product_id` CHAR(36) NULL").await?;

    convert_plain_date_to_doc_date(db, "invoices", "due_date").await?;
    convert_plain_date_to_doc_date(db, "quotations", "expiry_date").await?;
    Ok(())
}

/// G-28 (a): `name_live` on the three catalog master-data tables switches from `utf8mb4_unicode_ci`
/// (case-insensitive — "Food" and "food" would collide) to an explicit `utf8mb4_bin` (exact) compare,
/// per P2-17. (b): `products.barcode_live` (mirrors `sku_live`'s "generated column over the live
/// rows, unique index on it" shape) plus its unique index; the existing plain `ix_products_barcode`
/// index is left in place as a non-unique lookup index for `findByCode`.
async fn g28_catalog_name_live_collation_and_barcode<C: sea_orm::ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    for table in ["categories", "units", "price_lists"] {
        let sql = format!(
            "ALTER TABLE `{table}` \
             DROP INDEX `uq_{table}_name_live`, \
             MODIFY COLUMN `name_live` VARCHAR(200) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin \
             GENERATED ALWAYS AS (CASE WHEN `deleted_at` IS NULL THEN `name` END) STORED, \
             ADD UNIQUE INDEX `uq_{table}_name_live` (`name_live`)"
        );
        db.execute_unprepared(&sql).await?;
    }

    // `products` is already `utf8mb4_bin` at the table level (m0005), so no explicit collation
    // override is needed on `barcode_live` (unlike `sku_live`, which deliberately overrides to `ci`).
    db.execute_unprepared(
        "ALTER TABLE `products` \
         ADD COLUMN `barcode_live` VARCHAR(64) \
         GENERATED ALWAYS AS (CASE WHEN `deleted_at` IS NULL THEN `barcode` END) STORED, \
         ADD UNIQUE INDEX `uq_products_barcode_live` (`barcode_live`)",
    )
    .await?;
    Ok(())
}

/// G-28 (c): `purchase_orders.received_date` and `product_batches.received_date` become `DocDate`
/// triples (both are written from an ISO instant in the mock). `product_batches.received_date` was
/// `NOT NULL`; the new `_day` column keeps that (backfilled from the old column, so no row is ever
/// without a business day), while `_instant` stays nullable like every other `DocDate` triple.
async fn g28d_received_date_doc_dates<C: sea_orm::ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    convert_plain_date_to_doc_date(db, "purchase_orders", "received_date").await?;
    convert_plain_date_to_doc_date_not_null(db, "product_batches", "received_date").await?;
    Ok(())
}

/// G-28 (d) / G-P4c: seed the `productCodesLock` counter row `shared::numbering::SequenceLock`
/// will grow a `ProductCodes` variant for (manager task — see this file's header). The row is
/// harmless before that variant exists (nothing reads it), and its presence means 06/06b's first
/// `numbering::lock(SequenceLock::ProductCodes)` call needs no separate seed migration.
async fn g28d_product_codes_lock_row<C: sea_orm::ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    db.execute_unprepared(
        "INSERT INTO `document_counters` (`kind`, `value`) VALUES ('productCodesLock', 0) \
         ON DUPLICATE KEY UPDATE `kind` = `kind`",
    )
    .await?;
    Ok(())
}

/// G-11: drop the two never-written, never-read party balance columns (confirmed by a full-repo
/// search: every reader goes through `shared::balances::{customer_balance, supplier_balance,
/// unallocated_credit_for}`, which derive the figure from the ledger on each call — there is no
/// `Set(...)` on either column anywhere in `src/`).
async fn g11_drop_unused_party_balance_columns<C: sea_orm::ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    db.execute_unprepared("ALTER TABLE `parties` DROP COLUMN `balance`, DROP COLUMN `unallocated_credit`").await?;
    Ok(())
}

/// Converts an existing nullable `<field> DATE` column into the `DocDate` triple
/// (`<field>_day DATE NOT NULL`, `<field>_instant DATETIME(3) NULL`, generated `<field>_key`),
/// backfilling `_day` from the old column (`NULL` stays `NULL` there is no equivalent "no date" state
/// for a `NOT NULL _day` column pointed at by an `Option<DocDate>` at the Rust layer — callers treat
/// a NULL `_day` row as "no DocDate", matching every other optional `DocDate` triple in this schema,
/// e.g. `shifts.closed_at_day`). Order: add the two new columns, copy, add the generated column, drop
/// the old one — so the copy always happens before the source column disappears.
async fn convert_plain_date_to_doc_date<C: sea_orm::ConnectionTrait>(db: &C, table: &str, field: &str) -> Result<(), DbErr> {
    db.execute_unprepared(&format!(
        "ALTER TABLE `{table}` \
         ADD COLUMN `{field}_day` DATE NULL, \
         ADD COLUMN `{field}_instant` DATETIME(3) NULL"
    ))
    .await?;
    db.execute_unprepared(&format!("UPDATE `{table}` SET `{field}_day` = `{field}`")).await?;
    db.execute_unprepared(&format!(
        "ALTER TABLE `{table}` \
         ADD COLUMN `{field}_key` VARCHAR(24) AS (COALESCE(\
            CONCAT(REPLACE(LEFT(CAST(`{field}_instant` AS CHAR), 23), ' ', 'T'), 'Z'), \
            CAST(`{field}_day` AS CHAR)\
         )) STORED, \
         DROP COLUMN `{field}`"
    ))
    .await?;
    Ok(())
}

/// Same as `convert_plain_date_to_doc_date`, but for a source column that was `NOT NULL` — the new
/// `_day` column keeps that guarantee (backfilled before the `NOT NULL` is applied, so the
/// intermediate state is never queried).
async fn convert_plain_date_to_doc_date_not_null<C: sea_orm::ConnectionTrait>(db: &C, table: &str, field: &str) -> Result<(), DbErr> {
    db.execute_unprepared(&format!(
        "ALTER TABLE `{table}` \
         ADD COLUMN `{field}_day` DATE NULL, \
         ADD COLUMN `{field}_instant` DATETIME(3) NULL"
    ))
    .await?;
    db.execute_unprepared(&format!("UPDATE `{table}` SET `{field}_day` = `{field}`")).await?;
    db.execute_unprepared(&format!("ALTER TABLE `{table}` MODIFY COLUMN `{field}_day` DATE NOT NULL")).await?;
    db.execute_unprepared(&format!(
        "ALTER TABLE `{table}` \
         ADD COLUMN `{field}_key` VARCHAR(24) AS (COALESCE(\
            CONCAT(REPLACE(LEFT(CAST(`{field}_instant` AS CHAR), 23), ' ', 'T'), 'Z'), \
            CAST(`{field}_day` AS CHAR)\
         )) STORED, \
         DROP COLUMN `{field}`"
    ))
    .await?;
    Ok(())
}

/// The `down()` inverse of `convert_plain_date_to_doc_date{,_not_null}`: collapses the triple back to
/// a plain `<field> DATE`, backfilled from `_day` (the `_instant` half, if any, is lost on the way
/// down — acceptable for a migration rollback, which is a dev/ops escape hatch, not a supported
/// forward-compatible downgrade path).
async fn revert_doc_date_to_plain_date<C: sea_orm::ConnectionTrait>(db: &C, table: &str, field: &str) -> Result<(), DbErr> {
    db.execute_unprepared(&format!("ALTER TABLE `{table}` DROP COLUMN `{field}_key`, ADD COLUMN `{field}` DATE NULL")).await?;
    db.execute_unprepared(&format!("UPDATE `{table}` SET `{field}` = `{field}_day`")).await?;
    db.execute_unprepared(&format!("ALTER TABLE `{table}` DROP COLUMN `{field}_day`, DROP COLUMN `{field}_instant`")).await?;
    Ok(())
}

/// G-49: `journal_draft_lines` mirrors `journal_lines`' FC columns (m0012 lines 78-80), so a draft
/// carries its currency/FC amount/rate into `post_draft` like the mock's `journalDrafts` do.
async fn g49_draft_line_fc_columns<C: sea_orm::ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    db.execute_unprepared(
        "ALTER TABLE `journal_draft_lines`          ADD COLUMN `currency` CHAR(3) NULL,          ADD COLUMN `amount_fc` DECIMAL(19,2) NULL,          ADD COLUMN `rate` DECIMAL(19,6) NULL",
    )
    .await?;
    Ok(())
}
