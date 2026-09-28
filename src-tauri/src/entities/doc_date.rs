//! The `DocDate` triple bridge (21.02-B, owner B1, task B-4). Every entity with a business-day
//! field that can also carry an exact instant (P2-09 / C-09 — the mock sometimes writes an ISO
//! instant into a "date" field: `sales.ts:241`, `core.ts:648`, `inventory.ts:275,371,412`,
//! `journal.ts:294`, `payments.ts:350`) stores it as **three physical columns**:
//!
//! - `<field>_day DATE NOT NULL` — the business day (`utils::dates::DocDate::day`).
//! - `<field>_instant DATETIME(3) NULL` — the exact instant, when the source document carried one
//!   (`utils::dates::DocDate::instant`).
//! - `<field>_key VARCHAR(24) AS (COALESCE(<instant as ISO-ms text>, CAST(<day> AS CHAR)))
//!   STORED` — a generated column giving the exact string the mock would have stored (`DocDate::key()`),
//!   used only for the odd case where a `UNIQUE`/lookup needs to match the mock's literal string.
//!
//! **How B2 (and any table with a DocDate field) should use this:**
//! - In the migration: the `migration` crate has no dependency on this app crate (`lib.rs`'s own
//!   doc comment — the app depends on `migration`, never the reverse), so the actual
//!   `TableCreateStatement`-building helper can't live here; it is duplicated locally inside
//!   whichever `migration/src/m00NN_*.rs` file needs it (see `m0006_inventory.rs`'s private
//!   `doc_date_migration` module for the exact copy this owner wrote) — keep any change to the
//!   generated-column SQL (`generated_key_extra` below) in sync with that copy.
//! - In the entity: give the model two plain fields, `<field>_day: NaiveDate` and
//!   `<field>_instant: Option<DateTime<Utc>>` (the generated `_key` column is read-only — map it too
//!   if a query needs it, but no code ever sets it), then add a `fn <field>(&self) -> DocDate` method
//!   (via `doc_date::read`) and a setter (`doc_date::write`) that a controller uses instead of poking
//!   the two columns directly.

use chrono::{DateTime, NaiveDate, Utc};

use crate::utils::dates::DocDate;

/// Reads the `_day`/`_instant` pair back into the shared `DocDate` value type — every entity's
/// `fn <field>(&self) -> DocDate` accessor should be a one-line call to this.
pub fn read(day: NaiveDate, instant: Option<DateTime<Utc>>) -> DocDate {
    DocDate { day, instant }
}

/// Splits a `DocDate` back into the `(_day, _instant)` pair a setter writes onto the two physical
/// columns — the inverse of `read`.
pub fn write(value: DocDate) -> (NaiveDate, Option<DateTime<Utc>>) {
    (value.day, value.instant)
}

/// The exact `_key` string a generated column derived from `(_day, _instant)` would hold — kept
/// here (delegating to `DocDate::key`) so a migration's `CHECK`/comment and a test's expected value
/// can both cite one function instead of re-deriving the `COALESCE(...)` logic in prose.
pub fn key(day: NaiveDate, instant: Option<DateTime<Utc>>) -> String {
    read(day, instant).key()
}

pub fn day_column(field: &str) -> String {
    format!("{field}_day")
}

pub fn instant_column(field: &str) -> String {
    format!("{field}_instant")
}

pub fn key_column(field: &str) -> String {
    format!("{field}_key")
}

/// The raw `GENERATED ALWAYS AS (...) STORED` SQL fragment (without the column name/type prefix) —
/// used by a migration's `.extra(generated_key_extra(field))` on a
/// `ColumnDef::new(Alias::new(key_column(field))).string_len(24)` column. MariaDB forbids
/// `DATE_FORMAT()` in generated columns (locale-dependent, error 1901), so the text comes from
/// `CAST(... AS CHAR)`: a `DATETIME(3)` casts to `YYYY-MM-DD HH:MM:SS.mmm` (always 3 digits, like
/// the mock's `toISOString()`), `LEFT(…, 23)` guards against a wider column, and the space becomes
/// `T` plus a trailing `Z` — byte-for-byte `DocDate::key()`.
pub fn generated_key_extra(field: &str) -> String {
    let day = day_column(field);
    let instant = instant_column(field);
    format!(
        "GENERATED ALWAYS AS (COALESCE(\
            CONCAT(REPLACE(LEFT(CAST(`{instant}` AS CHAR), 23), ' ', 'T'), 'Z'), \
            CAST(`{day}` AS CHAR)\
        )) STORED"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn key_matches_docdate_key_day_only() {
        let day = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        assert_eq!(key(day, None), "2026-09-27");
    }

    #[test]
    fn key_matches_docdate_key_with_instant() {
        let day = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        let instant = Utc.with_ymd_and_hms(2026, 9, 27, 10, 0, 0).unwrap() + chrono::Duration::milliseconds(120);
        assert_eq!(key(day, Some(instant)), "2026-09-27T10:00:00.120Z");
    }

    #[test]
    fn read_write_round_trip() {
        let day = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let instant = Some(Utc.with_ymd_and_hms(2026, 1, 1, 3, 4, 5).unwrap());
        let doc = read(day, instant);
        assert_eq!(write(doc), (day, instant));
    }

    #[test]
    fn generated_key_extra_names_both_source_columns() {
        let extra = generated_key_extra("date");
        assert!(extra.contains("`date_instant`"));
        assert!(extra.contains("`date_day`"));
        assert!(extra.contains("STORED"));
    }
}
