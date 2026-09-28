//! Number/date-key formatting for the Arabic RTL UI (21.03 G-36) — a Rust port of the subset of
//! `src/modules/core/helpers/format.ts` that the reports/analytics domains need to embed inside a
//! server-built Arabic sentence (`trendInsight`, insight rule messages, …): `formatMoney`,
//! `formatNumber` and the `YYYY-MM-DD`/`DD/MM/YYYY` date-key half of `formatDate`. The rest of
//! `format.ts` (`formatHijri`, `formatRelative`, `formatDateWithHijri`, address formatting, …) stays
//! in TypeScript — those either need a live Hijri calendar / "now" or are pure display helpers no
//! backend command embeds text through.
//!
//! Locale: always `ar-SA-u-ca-gregory-nu-<numerals>` in the mock. Since the calendar is always
//! Gregorian and the region is fixed, the only two axes that change the *text* are:
//! - **`Numerals`**: `Latn` (`0-9`, `,` group, `.` decimal) or `Arab` (`٠-٩`, `٬` U+066C group, `٫`
//!   U+066B decimal) — `format.ts`'s `Numerals` type / `numeralSystem` setting.
//! - **`DateStyle`**: `Dmy` (`dd/mm/yyyy`) or `Ymd` (`yyyy-mm-dd`) — `format.ts`'s `dateFormatStyle`
//!   appearance setting (`formatDate`'s branch, `format.ts:103`).
//!
//! Rounding: `Intl.NumberFormat`'s ECMA-402 default `roundingMode` is `"halfExpand"` — round to the
//! nearest, ties away from zero — the same rule `rust_decimal`'s `MidpointAwayFromZero` implements,
//! so this reuses `Decimal::round_dp_with_strategy` rather than a second rounding routine (money/
//! qty/rate rounding is a different, already-settled rule — `round2`/`round4` in `utils::money` —
//! this module only rounds *for display*, at whatever `max_frac` the caller asks for, which is not
//! always 2).
//!
//! Both device display settings travel as command **args** (14-analytics.md decision A-2: they are
//! per-device appearance state, not something a report command can read off the DB), so every
//! function here takes `Numerals`/`DateStyle` explicitly rather than reading a global.

use chrono::NaiveDate;
use rust_decimal::{Decimal, RoundingStrategy};

/// `format.ts`'s `Numerals` type (`'latn' | 'arab'`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Numerals {
    Latn,
    Arab,
}

/// `format.ts`'s `dateFormatStyle` appearance setting (`'dmy' | 'ymd'`, `useAppearance.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateStyle {
    Dmy,
    Ymd,
}

const ARABIC_INDIC_DIGITS: [char; 10] = ['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩'];

/// Latin `0-9` → Arabic-Indic digits, digit by digit; every other character (including the group/
/// decimal separators this module already chose per `Numerals`) passes through unchanged.
fn to_numerals(s: &str, numerals: Numerals) -> String {
    if numerals == Numerals::Latn {
        return s.to_string();
    }
    s.chars()
        .map(|c| match c.to_digit(10) {
            Some(d) => ARABIC_INDIC_DIGITS[d as usize],
            None => c,
        })
        .collect()
}

/// Rounds `value` to `max_frac` fractional digits with ECMA-402's default `"halfExpand"` rounding
/// (ties away from zero), then renders it as `-`?`ddd(,ddd)*` + optional `.`/`٫` + fixed-width
/// fraction, with **no** trailing zeros trimmed beyond `max_frac` (matches `Intl.NumberFormat`'s
/// `maximumFractionDigits` alone, which drops trailing zeros down to `minimumFractionDigits`, here
/// always 0 unless the caller wants exactly `max_frac` — see `format_money` for the always-2-dp
/// case). `-0` prints as `-0`, per G-36 (Intl's own quirk: `Object.is(-0, 0) === false`, so
/// `Intl.NumberFormat` prints a literal minus for negative zero — the opposite of this crate's own
/// money rounding, `utils::money::round2`, which normalizes `-0` away because money must never
/// display a signed zero; display formatting for an already-computed value must reproduce Intl
/// byte-for-byte instead).
fn format_grouped(value: Decimal, min_frac: u32, max_frac: u32, numerals: Numerals) -> String {
    let rounded = value.round_dp_with_strategy(max_frac, RoundingStrategy::MidpointAwayFromZero);

    // Render at exactly `max_frac` decimals first (fixed-width), then trim trailing zeros down to
    // `min_frac` — this reproduces `minimumFractionDigits`/`maximumFractionDigits` together without
    // depending on `Decimal`'s own `scale()`, which does not always equal `max_frac` after rounding
    // (e.g. rounding 1.2 to 2dp keeps scale 1, not 2).
    let mut text = format!("{:.*}", max_frac as usize, rounded.abs());
    if max_frac > min_frac {
        if let Some(dot) = text.find('.') {
            let min_len = dot + 1 + min_frac as usize;
            let mut trimmed_len = text.len();
            while trimmed_len > min_len && text.as_bytes()[trimmed_len - 1] == b'0' {
                trimmed_len -= 1;
            }
            if trimmed_len == dot + 1 {
                // every fractional digit was a trailing zero and min_frac == 0: drop the dot too.
                trimmed_len = dot;
            }
            text.truncate(trimmed_len);
        }
    }

    let (int_part, frac_part) = match text.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (text.as_str(), None),
    };
    let group_sep = match numerals {
        Numerals::Latn => ',',
        Numerals::Arab => '٬',
    };
    let grouped_int = group_thousands(int_part, group_sep);

    let decimal_sep = match numerals {
        Numerals::Latn => '.',
        Numerals::Arab => '٫',
    };
    let mut out = String::new();
    // `rust_decimal` preserves the sign bit through rounding (it never normalizes a rounded result),
    // so a small negative magnitude that rounds to zero at `max_frac` still reports
    // `is_sign_negative() == true` here — exactly the `-0.00` Intl also prints for such a value.
    if rounded.is_sign_negative() {
        out.push('-');
    }
    out.push_str(&grouped_int);
    if let Some(f) = frac_part {
        out.push(decimal_sep);
        out.push_str(f);
    }
    // Only the ASCII digits get mapped to Arabic-Indic here — the group/decimal separators above
    // are already the right character for `numerals`, and `to_numerals` only ever touches `0-9`.
    to_numerals(&out, numerals)
}

/// Inserts `sep` every 3 digits from the right of a plain, sign-free ASCII digit string.
fn group_thousands(digits: &str, sep: char) -> String {
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(bytes.len() + bytes.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        let from_end = bytes.len() - i;
        if i > 0 && from_end % 3 == 0 {
            out.push(sep);
        }
        out.push(*b as char);
    }
    out
}

/// Ports `formatMoney` (`format.ts:72-74`): always exactly 2 decimals, grouped, in the given
/// numeral system. `None`/absent inputs are the caller's concern (the TS signature accepts
/// `number | undefined | null` and treats both as `0` — Rust callers pass `Decimal::ZERO` directly
/// instead of an `Option`, since every call site here already has a concrete amount).
pub fn format_money(value: Decimal, numerals: Numerals) -> String {
    format_grouped(value, 2, 2, numerals)
}

/// Ports `formatNumber` (`format.ts:76-78`): up to `max_frac` decimals (default 2 in the mock),
/// trailing zeros trimmed, grouped, in the given numeral system.
pub fn format_number(value: Decimal, max_frac: u32, numerals: Numerals) -> String {
    format_grouped(value, 0, max_frac, numerals)
}

/// Ports the `YYYY-MM-DD`/`DD/MM/YYYY` half of `formatDate` (`format.ts:96-105`) for a plain
/// business day (`NaiveDate` — this module never resolves a Hijri date or a time-of-day component;
/// `formatDateWithHijri`/`formatTime`/`formatRelative` stay client-side). Digits follow `numerals`;
/// the day/month are always zero-padded to 2 digits and the year to 4, matching
/// `Intl.DateTimeFormat`'s `2-digit`/`numeric` parts for a Gregorian year in this range.
pub fn format_date_key(day: NaiveDate, style: DateStyle, numerals: Numerals) -> String {
    use chrono::Datelike;
    let y = day.year();
    let m = day.month();
    let d = day.day();
    let out = match style {
        DateStyle::Ymd => format!("{y:04}-{m:02}-{d:02}"),
        DateStyle::Dmy => format!("{d:02}/{m:02}/{y:04}"),
    };
    to_numerals(&out, numerals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn format_money_always_two_decimals_latin() {
        assert_eq!(format_money(dec!(1234.5), Numerals::Latn), "1,234.50");
        assert_eq!(format_money(dec!(0), Numerals::Latn), "0.00");
        assert_eq!(format_money(dec!(-1234.567), Numerals::Latn), "-1,234.57");
    }

    #[test]
    fn format_money_arabic_numerals_and_separators() {
        // 1,234.50 -> ١٬٢٣٤٫٥٠
        assert_eq!(format_money(dec!(1234.5), Numerals::Arab), "١٬٢٣٤٫٥٠");
    }

    #[test]
    fn format_number_trims_trailing_zeros() {
        assert_eq!(format_number(dec!(1000000), 2, Numerals::Latn), "1,000,000");
        assert_eq!(format_number(dec!(12.345), 2, Numerals::Latn), "12.35"); // halfExpand
        assert_eq!(format_number(dec!(0.5), 2, Numerals::Latn), "0.5");
    }

    #[test]
    fn negative_zero_prints_with_a_minus_sign() {
        assert_eq!(format_money(dec!(-0.001), Numerals::Latn), "-0.00");
    }

    #[test]
    fn format_date_key_both_styles() {
        let day = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();
        assert_eq!(format_date_key(day, DateStyle::Ymd, Numerals::Latn), "2026-09-07");
        assert_eq!(format_date_key(day, DateStyle::Dmy, Numerals::Latn), "07/09/2026");
        assert_eq!(format_date_key(day, DateStyle::Ymd, Numerals::Arab), "٢٠٢٦-٠٩-٠٧");
    }
}
