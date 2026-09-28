//! The ONE rounding rule (plan 21 decision D3), ported from
//! `src/modules/core/helpers/numbers.ts`'s `roundHalfAwayFromZero`/`round2`/`round4`: round half
//! **away from zero**, on the exact decimal value — `rust_decimal`'s `MidpointAwayFromZero`
//! strategy gives the identical result to the cent/qty-unit, since `Decimal` carries the exact
//! decimal value with no binary float noise to correct for (the TS side needs `toPrecision(15)` to
//! drop float noise; `Decimal` never has any).
//!
//! `verify:mocks`'s `rounding` area (`scripts/verify/rounding.ts`) pins the same cases against the
//! TS implementation — the two must always agree.

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Deserializer, Serializer};
use std::str::FromStr;

/// Money (2 decimals) — see module doc.
pub fn round2(n: Decimal) -> Decimal {
    round_dp(n, 2)
}

/// Unit cost, quantity and FX rate (4 decimals) — see module doc. Quantities also round with
/// `round2` per P2-19 (products.md §9); callers pick whichever is correct for the field.
pub fn round4(n: Decimal) -> Decimal {
    round_dp(n, 4)
}

/// Quantities round with `round2` (P2-19), not `round4` — kept as its own name so a call site
/// reads as a quantity rounding, not a money one, even though the digits happen to match.
pub fn round_qty(n: Decimal) -> Decimal {
    round2(n)
}

fn round_dp(n: Decimal, dp: u32) -> Decimal {
    let rounded = n.round_dp_with_strategy(dp, RoundingStrategy::MidpointAwayFromZero);
    normalize_negative_zero(rounded)
}

/// `-0` is a valid `Decimal` value (its sign bit can be set on a zero mantissa) but must never
/// leak out — TS's `Number` has the same footgun (`Object.is(-0, 0) === false`), which is exactly
/// why `roundHalfAwayFromZero` normalizes it. Mirrors that normalization here.
fn normalize_negative_zero(n: Decimal) -> Decimal {
    if n.is_zero() {
        Decimal::ZERO
    } else {
        n
    }
}

/// The normalized decimal text JS `String(n)` would print for this value — used inside Arabic
/// error messages (`القيد غير متوازن: المدين 100 ≠ الدائن 99.99`), where the mock interpolates a
/// plain JS number with no trailing zeros and no unnecessary sign.
pub fn js_number_string(d: Decimal) -> String {
    let normalized = d.normalize();
    // `Decimal::normalize()` strips trailing zeros but keeps "-0" as "0" already handled above;
    // `to_string()` on a normalized decimal matches JS's shortest round-trip formatting for the
    // finite decimal values this app ever produces (money/qty/rate), which never hit scientific
    // notation territory in either representation.
    normalized.to_string()
}

/// Serializes/deserializes a `Decimal` as a JSON number (P2-33) — never a string, never
/// serde_json's global `arbitrary_precision`. Deserializing accepts either a JSON number or a
/// numeric string (defensive; the wire contract only ever sends a number) using the shortest
/// round-trip text, and rejects NaN/±Infinity.
pub mod serde_number {
    use super::*;

    pub fn serialize<S: Serializer>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error> {
        let f: f64 = value
            .to_string()
            .parse()
            .map_err(|_| serde::ser::Error::custom("decimal did not fit in f64"))?;
        if !f.is_finite() {
            return Err(serde::ser::Error::custom("refusing to serialize a non-finite decimal"));
        }
        serializer.serialize_f64(f)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Decimal, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        decimal_from_json_value(&value).map_err(serde::de::Error::custom)
    }

    pub mod option {
        use super::*;

        pub fn serialize<S: Serializer>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error> {
            match value {
                Some(d) => super::serialize(d, serializer),
                None => serializer.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Decimal>, D::Error> {
            let value = Option::<serde_json::Value>::deserialize(deserializer)?;
            match value {
                None | Some(serde_json::Value::Null) => Ok(None),
                Some(v) => decimal_from_json_value(&v).map(Some).map_err(serde::de::Error::custom),
            }
        }
    }
}

fn decimal_from_json_value(value: &serde_json::Value) -> Result<Decimal, String> {
    match value {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if !f.is_finite() {
                    return Err("refusing to deserialize a non-finite number".to_string());
                }
                // Shortest round-trip text (`format!("{}", f64)`), same as JS `String(n)`.
                Decimal::from_str(&format!("{f}")).map_err(|e| e.to_string())
            } else {
                Err("number out of f64 range".to_string())
            }
        }
        serde_json::Value::String(s) => {
            let f: f64 = s.parse().map_err(|_| "not a numeric string".to_string())?;
            if !f.is_finite() {
                return Err("refusing to deserialize a non-finite number".to_string());
            }
            Decimal::from_str(s).map_err(|e| e.to_string())
        }
        _ => Err("expected a JSON number".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde::Serialize;

    #[test]
    fn round2_cases_match_scripts_verify_rounding() {
        // Mirrors scripts/verify/rounding.ts ROUND2_CASES exactly.
        let cases: [(Decimal, Decimal); 9] = [
            (dec!(1.005), dec!(1.01)),
            (dec!(1234.565), dec!(1234.57)),
            (dec!(-0.125), dec!(-0.13)),
            (dec!(-1.005), dec!(-1.01)),
            (dec!(-2.5), dec!(-2.5)),
            (dec!(0.285), dec!(0.29)),
            (dec!(3.45), dec!(3.45)), // 1.15 * 3 exactly, in decimal
            (dec!(0.3), dec!(0.3)),   // 0.1 + 0.2 exactly, in decimal
            (dec!(-0.004), dec!(0)),
        ];
        for (input, expected) in cases {
            let actual = round2(input);
            assert_eq!(actual, expected, "round2({input}) should be {expected}, got {actual}");
            assert!(!(actual.is_zero() && actual.is_sign_negative()), "round2({input}) produced a negative zero");
        }
        assert_eq!(round2(dec!(999999999.995)), dec!(1000000000.00));
    }

    #[test]
    fn round4_cases_match_scripts_verify_rounding() {
        assert_eq!(round4(dec!(1.00005)), dec!(1.0001));
        assert_eq!(round4(dec!(-0.00125)), dec!(-0.0013));
        assert_eq!(round4(dec!(12.34565)), dec!(12.3457));
        // 10 / 3 in decimal division, rounded to 4dp.
        let ten_thirds = dec!(10) / dec!(3);
        assert_eq!(round4(ten_thirds), dec!(3.3333));
    }

    #[test]
    fn serde_round_trip() {
        for text in ["0.1", "1234.57", "-0.13"] {
            let d = Decimal::from_str(text).unwrap();
            let json = serde_json::to_string(&Wrapper(d)).unwrap();
            let back: Wrapper = serde_json::from_str(&json).unwrap();
            assert_eq!(back.0, d);
        }
    }

    #[test]
    fn serde_rejects_nan_and_infinity() {
        let bad = ["NaN", "Infinity", "-Infinity"];
        for text in bad {
            let json = format!("{text}");
            let result: Result<Wrapper, _> = serde_json::from_str(&json);
            assert!(result.is_err(), "expected {text} to be rejected");
        }
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Wrapper(#[serde(with = "serde_number")] Decimal);
}
