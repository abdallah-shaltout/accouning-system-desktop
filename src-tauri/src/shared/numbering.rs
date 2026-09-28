//! `shared::numbering` — gapless document numbers (21.02-C, P2-21). Ports `nextNumber`
//! (`src/mocks/db.ts:232-254`) against the `document_counters` table (seeded by migration
//! `m0001_infrastructure`, 17 rows: the 15 `DocumentKind` values plus the two `MAX+1`-code locks).
//! `document_counters` has no SeaORM entity (B-1/C-11: a short natural key, no UUID PK, never
//! synced) — this module talks to it directly via `Statement`, which is also what keeps this the
//! **only** place in the backend that touches it (C-8's architecture rule).

use sea_orm::{ConnectionTrait, EntityTrait, Statement};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::entities::org::settings::Entity as SettingsEntity;

/// One `document_counters.kind` row — every kind `nextNumber` (`db.ts:126-141`) can be called
/// with, plus the two `MAX+1`-code lock rows (`SequenceLock`, below). The exact string is the
/// primary key value stored in the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    Invoice,
    Refund,
    PurchaseOrder,
    PurchaseReturn,
    Payment,
    Journal,
    Adjustment,
    StockCount,
    DebitNoteDraft,
    Quotation,
    Shift,
    Expense,
    Voucher,
    CardSettlement,
    StockTransfer,
}

impl DocumentKind {
    fn kind_key(self) -> &'static str {
        match self {
            DocumentKind::Invoice => "invoice",
            DocumentKind::Refund => "refund",
            DocumentKind::PurchaseOrder => "purchaseOrder",
            DocumentKind::PurchaseReturn => "purchaseReturn",
            DocumentKind::Payment => "payment",
            DocumentKind::Journal => "journal",
            DocumentKind::Adjustment => "adjustment",
            DocumentKind::StockCount => "stockCount",
            DocumentKind::DebitNoteDraft => "debitNoteDraft",
            DocumentKind::Quotation => "quotation",
            DocumentKind::Shift => "shift",
            DocumentKind::Expense => "expense",
            DocumentKind::Voucher => "voucher",
            DocumentKind::CardSettlement => "cardSettlement",
            DocumentKind::StockTransfer => "stockTransfer",
        }
    }

    /// The fixed prefix map (`db.ts:220-234`'s `PREFIX`) — `invoice` is the one exception, whose
    /// prefix comes from `settings.invoice_number_prefix` instead of this table.
    fn fixed_prefix(self) -> Option<&'static str> {
        match self {
            DocumentKind::Invoice => None,
            DocumentKind::Refund => Some("RET-"),
            DocumentKind::PurchaseOrder => Some("PO-"),
            DocumentKind::PurchaseReturn => Some("PR-"),
            DocumentKind::Payment => Some("PAY-"),
            DocumentKind::Journal => Some("JE-"),
            DocumentKind::Quotation => Some("QUO-"),
            DocumentKind::Shift => Some("SH-"),
            DocumentKind::Adjustment => Some("ADJ-"),
            DocumentKind::StockCount => Some("CNT-"),
            DocumentKind::DebitNoteDraft => Some("DN-"),
            DocumentKind::Expense => Some("EXP-"),
            DocumentKind::Voucher => Some("VCH-"),
            DocumentKind::CardSettlement => Some("STL-"),
            DocumentKind::StockTransfer => Some("TRF-"),
        }
    }
}

/// The two `MAX+1`-code lock rows (`parties.md` §5's `nextCode`, P2-21) — not document numbers
/// themselves, just serialization points `Part 03`'s party-code assignment takes before computing
/// `MAX(code) + 1` over the live `parties` rows, so two concurrent creates can't compute the same
/// next code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceLock {
    CustomerCode,
    SupplierCode,
}

impl SequenceLock {
    fn kind_key(self) -> &'static str {
        match self {
            SequenceLock::CustomerCode => "customerCodeLock",
            SequenceLock::SupplierCode => "supplierCodeLock",
        }
    }
}

/// `nextNumber` (`db.ts:232-254`): `UPDATE document_counters SET value = value + 1 WHERE kind = ?`
/// (the row stays X-locked by the update until commit — gapless, since a rollback releases the
/// number no one else could take in the meantime), then reads `value` back. Zero-padded to 6
/// digits with the kind's prefix (the fixed map, or `settings.invoice_number_prefix` for
/// `invoice`).
pub async fn next_number<C: ConnectionTrait>(conn: &C, kind: DocumentKind) -> TxResult<String> {
    let key = kind.kind_key();

    let update = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "UPDATE document_counters SET value = value + 1 WHERE kind = ?",
        [key.into()],
    );
    conn.execute(update).await.map_err(TxError::from)?;

    let select = Statement::from_sql_and_values(conn.get_database_backend(), "SELECT value FROM document_counters WHERE kind = ?", [key.into()]);
    let row = conn.query_one(select).await.map_err(TxError::from)?;
    let value: i64 = match row {
        Some(row) => row.try_get::<i64>("", "value").map_err(TxError::from)?,
        None => {
            return Err(TxError::App(AppError::internal(
                "لم يتم العثور على عداد المستندات — البيانات الأساسية غير مكتملة",
                Some(format!("document_counters row missing for kind={key}")),
            )));
        }
    };

    let prefix = match kind.fixed_prefix() {
        Some(p) => p.to_string(),
        None => {
            let settings = SettingsEntity::find()
                .one(conn)
                .await
                .map_err(TxError::from)?
                .ok_or_else(|| AppError::internal("لم يتم العثور على صف الإعدادات — يجب تشغيل معالج الإعداد الأولي أولاً", None))?;
            settings.invoice_number_prefix
        }
    };

    Ok(format!("{prefix}{:06}", value))
}

/// `lock(SequenceLock::{CustomerCode, SupplierCode})`: `SELECT ... FOR UPDATE` on the lock row, so
/// Part 03's party-code assignment can safely run the mock's `MAX+1` code rule
/// (`partyService.ts:55-63`) — this doesn't bump `value` itself (only `next_number` does), it's a
/// pure serialization point. Not named after `crate::core::lock` (the shared row-lock helper
/// module) to avoid a confusing shadow; that helper isn't reused directly here because
/// `document_counters`'s primary key column is `kind`, not `id`.
pub async fn lock<C: ConnectionTrait>(conn: &C, which: SequenceLock) -> TxResult<()> {
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT kind FROM document_counters WHERE kind = ? FOR UPDATE",
        [which.kind_key().into()],
    );
    conn.query_all(stmt).await.map_err(TxError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_keys_match_the_mock_exactly() {
        assert_eq!(DocumentKind::Invoice.kind_key(), "invoice");
        assert_eq!(DocumentKind::PurchaseOrder.kind_key(), "purchaseOrder");
        assert_eq!(DocumentKind::CardSettlement.kind_key(), "cardSettlement");
        assert_eq!(SequenceLock::CustomerCode.kind_key(), "customerCodeLock");
        assert_eq!(SequenceLock::SupplierCode.kind_key(), "supplierCodeLock");
    }

    #[test]
    fn fixed_prefixes_match_db_ts() {
        assert_eq!(DocumentKind::Refund.fixed_prefix(), Some("RET-"));
        assert_eq!(DocumentKind::Journal.fixed_prefix(), Some("JE-"));
        assert_eq!(DocumentKind::Invoice.fixed_prefix(), None);
    }
}

