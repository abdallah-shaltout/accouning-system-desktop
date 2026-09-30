//! One `insert_<table>` (or small group of them) per entity group, called by `run::import_snapshot`
//! in `order::IMPORT_ORDER` order (`03-domains/00-import.md` §3.2.6). Every function here takes the
//! already-parsed `MockDbV1` model, the shared `IdMap`, the resolved `BusinessClock`/`tz`, and an
//! `&mut i64` rounded-values counter it increments per §3.2.5 (every value checked against its
//! column's decimal scale).

pub mod catalog;
pub mod expenses;
pub mod inventory;
pub mod journal;
pub mod org;
pub mod parties;
pub mod payments;
pub mod platform;
pub mod purchases;
pub mod sales;

use chrono::{DateTime, NaiveDate, Utc};

use crate::infrastructure::import::model::parse_instant;

/// D-8: `created_at` = the row's own TS `createdAt` when present, else `import_base + i ms` (i =
/// array index) — so `ORDER BY created_at, id` reproduces array order for tables whose type has no
/// `createdAt` field at all (child rows, mostly).
pub fn synthetic_created_at(import_base: DateTime<Utc>, index: usize) -> DateTime<Utc> {
    import_base + chrono::Duration::milliseconds(index as i64)
}

/// Parses an optional ISO `createdAt` string, falling back to the synthetic D-8 timestamp when
/// absent or unparseable.
pub fn resolve_created_at(raw: Option<&str>, import_base: DateTime<Utc>, index: usize) -> DateTime<Utc> {
    raw.and_then(parse_instant).unwrap_or_else(|| synthetic_created_at(import_base, index))
}

/// Parses a required business-day/instant string (`DocDate`'s wire shape: a bare `YYYY-MM-DD` or a
/// full ISO instant) into `(day, instant)`, resolving an instant's day through the given timezone
/// (P2-09/step 3). A parse failure is the caller's job to turn into the `VALIDATION` message naming
/// the table (§3, step 3's "Unparseable → VALIDATION تاريخ غير صالح في <table>").
pub fn parse_doc_date(raw: &str, tz: Option<chrono_tz::Tz>) -> Option<(NaiveDate, Option<DateTime<Utc>>)> {
    if raw.len() == 10 {
        NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok().map(|d| (d, None))
    } else {
        DateTime::parse_from_rfc3339(raw).ok().map(|dt| {
            let instant = dt.with_timezone(&Utc);
            let day = crate::utils::dates::local_date_key(instant, tz);
            (day, Some(instant))
        })
    }
}

/// A typed, nullable FK column (§3.2.4, step 12): no old id (absent or empty) → `NULL`; a known old
/// id → its new id; an old id no row in this snapshot was assigned → a dangling reference, which
/// fails the whole import with `VALIDATION` naming `table` (the transaction rolls back) instead of
/// silently writing `NULL` and losing the link. For columns whose target the mock can hard-delete
/// (so a dangling id is legitimate legacy data) use [`lenient_ref`].
pub fn strict_ref(
    id_map: &crate::infrastructure::import::idmap::IdMap,
    old: Option<&str>,
    table: &str,
) -> crate::core::tx::TxResult<Option<crate::utils::id::Id>> {
    match old {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => id_map.resolve(s).map(Some).ok_or_else(|| dangling_ref(table, s)),
    }
}

/// A nullable FK whose target the mock hard-deletes while documents keep the old id
/// (`deleteCostCenter` checks only posted journal lines, `purchases.ts:703` drops a converted debit
/// note draft): a dangling id there becomes `NULL`, exactly what the reference means now.
pub fn lenient_ref(id_map: &crate::infrastructure::import::idmap::IdMap, old: Option<&str>) -> Option<crate::utils::id::Id> {
    old.filter(|s| !s.is_empty()).and_then(|s| id_map.resolve(s))
}

/// Step 12's `VALIDATION` text for a reference to a row the snapshot doesn't contain; the old id
/// goes to the diagnostics log only.
pub fn dangling_ref(table: &str, old: &str) -> crate::core::tx::TxError {
    log::error!(target: "import", "dangling reference in {table}: '{old}' is not a row of this snapshot");
    crate::core::tx::TxError::App(crate::core::error::AppError::validation(format!(
        "تعذر الاستيراد: مرجع غير موجود في {table} — أرسل ملف التشخيص للدعم"
    )))
}
