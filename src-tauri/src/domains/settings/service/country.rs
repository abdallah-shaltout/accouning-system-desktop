//! `country.rs` — the Rust mirror of `src/modules/core/helpers/countryProfiles.ts` (01-settings.md
//! §3). `pub` so `00-import` and `02-setup` can reuse `country_profile`/`country_timezone` without
//! re-deriving the EG/SA facts. No `regex` crate dependency — `tax_id_valid` is hand-written per
//! country, matching each profile's own `RegExp` exactly.

/// `CountryProfile` (Rust-only mirror — the frontend's own richer `CountryProfile` type is not
/// duplicated here; this carries only the fields the settings/setup domains need server-side).
pub struct CountryProfile {
    pub code: &'static str,
    pub currency_code: &'static str,
    pub vat_rate: u8,
    pub vat_label: &'static str,
    pub prices_include_tax_default: bool,
    pub tax_id_label: &'static str,
    pub tax_id_hint: &'static str,
    pub tax_id_valid: fn(&str) -> bool,
    pub timezone: &'static str,
}

fn eg_tax_id_valid(s: &str) -> bool {
    s.len() == 9 && s.chars().all(|c| c.is_ascii_digit())
}

fn sa_tax_id_valid(s: &str) -> bool {
    // ^3\d{13}3$ — 15 digits, starts and ends with 3.
    let bytes = s.as_bytes();
    bytes.len() == 15 && bytes[0] == b'3' && bytes[14] == b'3' && s.chars().all(|c| c.is_ascii_digit())
}

const EG: CountryProfile = CountryProfile {
    code: "EG",
    currency_code: "EGP",
    vat_rate: 14,
    vat_label: "ضريبة القيمة المضافة 14%",
    prices_include_tax_default: true,
    tax_id_label: "رقم التسجيل الضريبي",
    tax_id_hint: "9 أرقام",
    tax_id_valid: eg_tax_id_valid,
    timezone: "Africa/Cairo",
};

const SA: CountryProfile = CountryProfile {
    code: "SA",
    currency_code: "SAR",
    vat_rate: 15,
    vat_label: "ضريبة القيمة المضافة 15%",
    prices_include_tax_default: true,
    tax_id_label: "الرقم الضريبي",
    tax_id_hint: "15 رقماً يبدأ وينتهي بالرقم 3",
    tax_id_valid: sa_tax_id_valid,
    timezone: "Asia/Riyadh",
};

/// `countryProfile(code)` (`countryProfiles.ts:152-154`): falls back to `EG` for any unrecognized
/// or absent code — matches the mock exactly (no error, ever).
pub fn country_profile(code: Option<&str>) -> &'static CountryProfile {
    match code {
        Some("SA") => &SA,
        _ => &EG,
    }
}

/// `settings.timezone` value for a given country (D-5) — `None` only if `country_profile` couldn't
/// resolve (never happens in practice, `country_profile` always returns a real profile).
pub fn country_timezone(code: Option<&str>) -> Option<&'static str> {
    Some(country_profile(code).timezone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eg_tax_id_validation() {
        assert!((EG.tax_id_valid)("123456789"));
        assert!(!(EG.tax_id_valid)("12345678"));
        assert!(!(EG.tax_id_valid)("12345678a"));
    }

    #[test]
    fn sa_tax_id_validation() {
        assert!((SA.tax_id_valid)("310000000000003"));
        assert!(!(SA.tax_id_valid)("12345678"));
        assert!(!(SA.tax_id_valid)("410000000000003"));
        assert!(!(SA.tax_id_valid)("310000000000004"));
    }

    #[test]
    fn country_profile_falls_back_to_eg() {
        assert_eq!(country_profile(None).code, "EG");
        assert_eq!(country_profile(Some("XX")).code, "EG");
        assert_eq!(country_profile(Some("SA")).code, "SA");
    }

    #[test]
    fn country_timezone_matches_profile() {
        assert_eq!(country_timezone(Some("EG")), Some("Africa/Cairo"));
        assert_eq!(country_timezone(Some("SA")), Some("Asia/Riyadh"));
    }
}
