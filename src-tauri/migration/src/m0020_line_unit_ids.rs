//! m0020_line_unit_ids (plan 21 Part 04 Wave 2, parity lane L2): `unit_id` on the four document
//! line tables stops being a `UUID` FK to `units` and becomes the free `VARCHAR(64)` string the UI
//! actually sends.
//!
//! Why: a line's `unitId` (`InvoiceLine` / `PurchaseLine` / `QuotationLine` / `StockTransferLine`
//! in the TS types) is the product's own `ProductUnit.id` — the `u.id` of an entry in
//! `Product.units`, which lives as JSON on `products.units` (B-1) and is a free string on both
//! backends (`pu-panadol-box` in the fixtures, `new-<seq>-<ts>` from `UnitsEditor.vue`). It is not
//! a `units` row id (that is `ProductUnit.unitId`). `PurchaseFormPage.unitOptionsFor` and
//! `usePosStore` (`l.unit?.id`) both put `u.id` on the line, so the old `fk_*_unit_id` FK refused
//! every line in a non-base unit (purchase, POS sale, quotation, transfer) and a non-UUID value
//! would have decoded to a hashed stand-in anyway. Stock never reads it — conversion uses the
//! line's snapshotted `unit_factor` — so it is a display/round-trip label, exactly as on the mock.
//!
//! Existing values are kept (`UUID` → text is lossless). The FK's backing index (named after the
//! FK) is left in place: harmless, and `down` re-uses it. `down` refuses (1292) rather than drop
//! data if a non-UUID unit id was stored meanwhile.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// `(table, fk name)` — the four `unit_id` FKs m0015 added on line tables.
const LINE_TABLES: &[(&str, &str)] = &[
    ("stock_transfer_lines", "fk_stock_transfer_lines_unit_id"),
    ("invoice_lines", "fk_invoice_lines_unit_id"),
    ("quotation_lines", "fk_quotation_lines_unit_id"),
    ("purchase_order_lines", "fk_purchase_order_lines_unit_id"),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for (table, fk) in LINE_TABLES {
            db.execute_unprepared(&format!("ALTER TABLE `{table}` DROP FOREIGN KEY IF EXISTS `{fk}`")).await?;
            db.execute_unprepared(&format!("ALTER TABLE `{table}` MODIFY COLUMN `unit_id` VARCHAR(64) NULL")).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for (table, fk) in LINE_TABLES.iter().rev() {
            db.execute_unprepared(&format!("ALTER TABLE `{table}` MODIFY COLUMN `unit_id` UUID NULL")).await?;
            db.execute_unprepared(&format!(
                "ALTER TABLE `{table}` ADD CONSTRAINT `{fk}` FOREIGN KEY (`unit_id`) REFERENCES `units` (`id`)"
            ))
            .await?;
        }
        Ok(())
    }
}
