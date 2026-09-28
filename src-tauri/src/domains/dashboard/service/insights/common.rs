//! Shared helpers for the insight rules (14b-insights.md §3.0).

use rust_decimal::Decimal;

use crate::core::tx::TxResult;
use crate::entities::values::InsightThresholds as ThresholdsMap;
use crate::utils::format::{format_money as fmt_money_raw, format_number as fmt_num_raw, Numerals};

pub use super::super::common::{account_balance as acct_balance, day_of_key, days_between, trunc_days};

/// Evaluation context every rule closure receives (`InsightContext`, `insightTypes.ts:34-38`, minus
/// `role` — role filtering happens once, after every rule runs, in `engine.rs`).
pub struct InsightCtx {
    pub today: chrono::NaiveDate,
    pub now: chrono::DateTime<chrono::Utc>,
    pub thresholds: Thresholds,
    pub numerals: Numerals,
}

/// The text the mock prints for `formatNumber(-0)`/`formatMoney(-0)` — `Object.is(-0, 0) === false`
/// in JS, so `Intl.NumberFormat` prints a literal minus (G-36's `format_money`/`format_number`
/// already reproduce this for any negative `Decimal`, including a rounded-to-zero negative
/// magnitude — see `utils::format`'s module doc). Exposed here only for call sites that build the
/// text from a bare negative-zero `i64`/`Decimal::ZERO.neg()` rather than a computed value.
pub fn neg_zero_text(numerals: Numerals) -> &'static str {
    match numerals {
        Numerals::Latn => "-0",
        Numerals::Arab => "-٠",
    }
}

pub fn fmt_money(v: Decimal, numerals: Numerals) -> String {
    fmt_money_raw(v, numerals)
}

pub fn fmt_num(v: Decimal, max_frac: u32, numerals: Numerals) -> String {
    fmt_num_raw(v, max_frac, numerals)
}

/// `js_num` — plain `${}` interpolation of a raw JS number (never run through `formatNumber`).
pub fn js_num(v: Decimal) -> String {
    crate::utils::money::js_number_string(v)
}

/// The 14 configurable thresholds (`InsightThresholds`, `insightTypes.ts:47-76`), resolved from the
/// sparse DB overlay onto the hard-coded defaults (`DEFAULT_THRESHOLDS`, `insightTypes.ts:78-93`).
/// Unknown keys in the DB map are ignored (I-2/G-13 note); every field is `Decimal` (rule 5 — no
/// f32/f64), even the ones the mock treats as plain integers (day counts, multipliers) — the DB
/// column is a sparse `JsonDecimal` map, so a value that came from `setThresholds`/the settings UI
/// is a `Decimal` regardless.
#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    pub dead_stock_days: Decimal,
    pub dead_stock_value: Decimal,
    pub expiry_alert_days: Decimal,
    pub supplier_due_days: Decimal,
    pub vat_deadline_days: Decimal,
    pub cash_drawer_limit: Decimal,
    pub shift_open_hours: Decimal,
    pub unsettled_clearing_days: Decimal,
    pub min_margin_pct: Decimal,
    pub discount_leak_multiplier: Decimal,
    pub refund_spike_multiplier: Decimal,
    pub budget_near_pct: Decimal,
    pub backup_overdue_days: Decimal,
    pub year_end_days: Decimal,
}

impl Thresholds {
    /// `DEFAULT_THRESHOLDS` (`insightTypes.ts:78-93`).
    pub fn defaults() -> Self {
        Thresholds {
            dead_stock_days: Decimal::from(60),
            dead_stock_value: Decimal::from(500),
            expiry_alert_days: Decimal::from(30),
            supplier_due_days: Decimal::from(7),
            vat_deadline_days: Decimal::from(10),
            cash_drawer_limit: Decimal::from(10_000),
            shift_open_hours: Decimal::from(14),
            unsettled_clearing_days: Decimal::from(3),
            min_margin_pct: Decimal::from(5),
            discount_leak_multiplier: Decimal::from(2),
            refund_spike_multiplier: Decimal::from(2),
            budget_near_pct: Decimal::from(90),
            backup_overdue_days: Decimal::from(7),
            year_end_days: Decimal::from(30),
        }
    }

    /// `Thresholds::load(conn)` (14b §2): defaults overlaid by `settings.insight_thresholds`
    /// (sparse `BTreeMap<String, JsonDecimal>` — I-2, served by 01-settings).
    pub async fn load<C: sea_orm::ConnectionTrait>(conn: &C) -> TxResult<Self> {
        let row = crate::core::settings::load(conn).await?;
        let mut t = Self::defaults();
        if let Some(ThresholdsMap(map)) = row.insight_thresholds {
            for (key, value) in map {
                let v = value.0;
                match key.as_str() {
                    "deadStockDays" => t.dead_stock_days = v,
                    "deadStockValue" => t.dead_stock_value = v,
                    "expiryAlertDays" => t.expiry_alert_days = v,
                    "supplierDueDays" => t.supplier_due_days = v,
                    "vatDeadlineDays" => t.vat_deadline_days = v,
                    "cashDrawerLimit" => t.cash_drawer_limit = v,
                    "shiftOpenHours" => t.shift_open_hours = v,
                    "unsettledClearingDays" => t.unsettled_clearing_days = v,
                    "minMarginPct" => t.min_margin_pct = v,
                    "discountLeakMultiplier" => t.discount_leak_multiplier = v,
                    "refundSpikeMultiplier" => t.refund_spike_multiplier = v,
                    "budgetNearPct" => t.budget_near_pct = v,
                    "backupOverdueDays" => t.backup_overdue_days = v,
                    "yearEndDays" => t.year_end_days = v,
                    _ => {} // unknown key ignored (I-2).
                }
            }
        }
        Ok(t)
    }
}
