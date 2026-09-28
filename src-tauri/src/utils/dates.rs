//! Date/time helpers ported from `src/mocks/utils.ts:95-109` (`localDateKey`/`inDateRange`) and the
//! `DocDate` design from cross-cutting.md §7 / phase-a-foundation.md A-3 (conflict C-09: the mock
//! sometimes writes an ISO instant into a "date" field and some code does `date.slice(0,10)`).

use chrono::{DateTime, Local, NaiveDate, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serializes/deserializes a `DateTime<Utc>` as exactly `%Y-%m-%dT%H:%M:%S%.3fZ` — the shape JS
/// `toISOString()` produces (always 3 fractional digits, always a literal `Z`).
pub mod iso_ms {
    use super::*;

    pub fn serialize<S: Serializer>(value: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format_iso_ms(*value))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<DateTime<Utc>, D::Error> {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&s)
            .map(|d| d.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}

pub fn format_iso_ms(value: DateTime<Utc>) -> String {
    value.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// The branch's business clock: the instant this transaction began, plus the branch timezone
/// (P2-08 — `SELECT UTC_TIMESTAMP(3)` once per transaction; `settings.timezone`, falling back to
/// the OS timezone when unset).
#[derive(Debug, Clone, Copy)]
pub struct BusinessClock {
    pub now: DateTime<Utc>,
    pub tz: Option<Tz>,
}

impl BusinessClock {
    pub fn new(now: DateTime<Utc>, tz: Option<Tz>) -> Self {
        Self { now, tz }
    }

    /// "Today" in the branch's business timezone (or the OS timezone when none is configured —
    /// matches `localDateKey`'s current browser-local behavior 1:1, cross-cutting §7).
    pub fn today(&self) -> NaiveDate {
        local_date_key(self.now, self.tz)
    }
}

/// Ports `localDateKey` (`utils.ts:95-101`): the calendar date of an instant, in the given
/// timezone, or the OS-local timezone when `tz` is `None`.
pub fn local_date_key(instant: DateTime<Utc>, tz: Option<Tz>) -> NaiveDate {
    match tz {
        Some(tz) => instant.with_timezone(&tz).date_naive(),
        None => instant.with_timezone(&Local).date_naive(),
    }
}

/// Ports `inDateRange` (`utils.ts:104-109`): inclusive comparison of `day`'s `YYYY-MM-DD` key
/// string against `from`/`to` bounds (also `YYYY-MM-DD` strings) — string comparison is safe here
/// because the format is fixed-width and zero-padded.
pub fn in_date_range(day: NaiveDate, from: Option<&str>, to: Option<&str>) -> bool {
    let key = day.format("%Y-%m-%d").to_string();
    if let Some(from) = from {
        if key.as_str() < from {
            return false;
        }
    }
    if let Some(to) = to {
        if key.as_str() > to {
            return false;
        }
    }
    true
}

/// The `DocDate` triple (P2-09 / C-09): a business day (`DATE` column) plus an optional exact
/// instant (`DATETIME(3) NULL`) when the source document actually carried one (the mock sometimes
/// writes an ISO instant into a `date` field: `sales.ts:241`, `core.ts:648`, `inventory.ts:275,371,412`,
/// `journal.ts:294`, `payments.ts:350`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocDate {
    pub day: NaiveDate,
    pub instant: Option<DateTime<Utc>>,
}

/// A plain business day (no instant) — the common case for UI-entered dates.
impl From<NaiveDate> for DocDate {
    fn from(day: NaiveDate) -> Self {
        Self { day, instant: None }
    }
}

impl DocDate {
    /// The mock's exact string for this value: the ISO instant when present, else the plain
    /// `YYYY-MM-DD` day — this is `x_key` from phase-a-foundation.md A-3.
    pub fn key(&self) -> String {
        match self.instant {
            Some(instant) => format_iso_ms(instant),
            None => self.day.format("%Y-%m-%d").to_string(),
        }
    }
}

impl Serialize for DocDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.key())
    }
}

/// The raw wire shape before it's resolved against a `BusinessClock`: a bare 10-char date string
/// deserializes as `Day`, anything else (an RFC 3339 instant) as `Instant`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawDocDate {
    Day(NaiveDate),
    Instant(DateTime<Utc>),
}

impl<'de> Deserialize<'de> for RawDocDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        RawDocDate::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl RawDocDate {
    pub fn parse(s: &str) -> Result<Self, String> {
        if s.len() == 10 {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(RawDocDate::Day)
                .map_err(|e| e.to_string())
        } else {
            DateTime::parse_from_rfc3339(s)
                .map(|d| RawDocDate::Instant(d.with_timezone(&Utc)))
                .map_err(|e| e.to_string())
        }
    }

    /// Resolves against the transaction's `BusinessClock` — a bare day needs no timezone
    /// conversion (it already *is* the business day), an instant is converted through the clock's
    /// timezone to derive its business day.
    pub fn resolve(&self, clock: &BusinessClock) -> DocDate {
        match self {
            RawDocDate::Day(day) => DocDate { day: *day, instant: None },
            RawDocDate::Instant(instant) => DocDate { day: local_date_key(*instant, clock.tz), instant: Some(*instant) },
        }
    }
}

/// Test-only convenience: builds a UTC `DateTime` from y/m/d/h/m/s without the `TimeZone` trait
/// ceremony at every call site.
#[cfg(test)]
fn utc_ymd_hms(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32, milli: u32) -> DateTime<Utc> {
    use chrono::TimeZone;
    Utc.with_ymd_and_hms(y, mo, d, h, mi, s).unwrap() + chrono::Duration::milliseconds(milli as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_ms_matches_js_tostring_shape() {
        let instant = utc_ymd_hms(2026, 9, 27, 10, 0, 0, 120);
        assert_eq!(format_iso_ms(instant), "2026-09-27T10:00:00.120Z");
    }

    #[test]
    fn doc_date_serializes_both_shapes() {
        let day_only = DocDate { day: NaiveDate::from_ymd_opt(2026, 9, 27).unwrap(), instant: None };
        assert_eq!(day_only.key(), "2026-09-27");

        let instant = utc_ymd_hms(2026, 9, 27, 10, 0, 0, 120);
        let with_instant = DocDate { day: NaiveDate::from_ymd_opt(2026, 9, 27).unwrap(), instant: Some(instant) };
        assert_eq!(with_instant.key(), "2026-09-27T10:00:00.120Z");
    }

    #[test]
    fn raw_doc_date_parses_both_shapes() {
        assert_eq!(
            RawDocDate::parse("2026-09-27").unwrap(),
            RawDocDate::Day(NaiveDate::from_ymd_opt(2026, 9, 27).unwrap())
        );
        assert!(matches!(RawDocDate::parse("2026-09-27T10:00:00.120Z").unwrap(), RawDocDate::Instant(_)));
    }

    #[test]
    fn local_date_key_across_midnight_in_riyadh() {
        let riyadh: Tz = "Asia/Riyadh".parse().unwrap();
        // 21:30 UTC on 2026-09-27 is 00:30 on 2026-09-28 in Riyadh (+03:00).
        let instant = utc_ymd_hms(2026, 9, 27, 21, 30, 0, 0);
        let key = local_date_key(instant, Some(riyadh));
        assert_eq!(key, NaiveDate::from_ymd_opt(2026, 9, 28).unwrap());
    }

    #[test]
    fn in_date_range_bounds() {
        let day = NaiveDate::from_ymd_opt(2026, 9, 15).unwrap();
        assert!(in_date_range(day, Some("2026-09-01"), Some("2026-09-30")));
        assert!(!in_date_range(day, Some("2026-09-16"), None));
        assert!(!in_date_range(day, None, Some("2026-09-14")));
        assert!(in_date_range(day, None, None));
    }
}
