//! `shared::validation` (G-10) — the shared tax-id validation rule, a behaviour-exact port of
//! `countryProfile(...).taxId` (`src/modules/core/helpers/countryProfiles.ts:100-148`). Used by
//! both `05-parties` (`saveCustomer`/`saveSupplier`'s VAT-number check) and `01-settings`
//! (`updateSettings`'s own VAT-number check) — one copy, not two, since both call sites need the
//! exact same regex/label/hint per country.
//!
//! **Placement note for the manager:** `03-domains/05-parties.md` §7 (G-10 text) names this
//! `utils::country::tax_id_rule` — a new `utils/country.rs` module. `utils/**` is outside this
//! wave's owned files (`shared/**` + `tests/shared_*.rs` only), so it's implemented here instead,
//! in `shared::validation`, per the gap register's own alternate wording ("`shared/validation.rs`
//! or where 05-parties says"). If the manager prefers the exact `utils::country::tax_id_rule` path
//! the domain plan names, this module's contents can be moved verbatim — the API below is designed
//! to make that a pure file move (no logic depends on living under `shared`).

/// `CountryProfile.taxId` (`countryProfiles.ts:50-54`) pared to what validation needs: a label for
/// the error message, a human hint, and a pure `matches` predicate (no `regex` crate dependency —
/// both rules are simple enough to check by hand, see `matches_eg`/`matches_sa` below).
pub struct TaxIdRule {
    pub label: &'static str,
    pub hint: &'static str,
    matches: fn(&str) -> bool,
}

impl TaxIdRule {
    pub fn matches(&self, value: &str) -> bool {
        (self.matches)(value)
    }

    /// The exact Arabic validation message `saveCustomer`/`saveSupplier`/`updateSettings` throw:
    /// `` `${label} يجب أن يكون ${hint}` `` (`partyService.ts:34-35`).
    pub fn error_message(&self) -> String {
        format!("{} يجب أن يكون {}", self.label, self.hint)
    }
}

/// `^\d{9}$` (`countryProfiles.ts:103`) — exactly 9 ASCII digits, no more, no less.
fn matches_eg(value: &str) -> bool {
    value.len() == 9 && value.bytes().all(|b| b.is_ascii_digit())
}

/// `^3\d{13}3$` (`countryProfiles.ts:137`) — exactly 15 ASCII digits, starting and ending with
/// `3`.
fn matches_sa(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 15 && bytes.iter().all(|b| b.is_ascii_digit()) && bytes[0] == b'3' && bytes[14] == b'3'
}

const EG: TaxIdRule = TaxIdRule { label: "رقم التسجيل الضريبي", hint: "9 أرقام", matches: matches_eg };
const SA: TaxIdRule = TaxIdRule { label: "الرقم الضريبي", hint: "15 رقماً يبدأ وينتهي بالرقم 3", matches: matches_sa };

/// `countryProfile(code).taxId` (`countryProfiles.ts:152-154`): looks up the tax-id rule for a
/// settings country code, falling back to `DEFAULT_COUNTRY` (`'EG'`, `countryProfiles.ts:150`) for
/// `None`/unrecognized codes — exactly like `countryProfile`'s own
/// `(code as CountryCode) in COUNTRY_PROFILES ? code : DEFAULT_COUNTRY` fallback. Only `EG`/`SA`
/// are modeled (the two profiles `COUNTRY_PROFILES` defines today); an unrecognized code is not a
/// panic, it is simply treated as `EG`, matching the mock exactly.
pub fn tax_id_rule(country: Option<&str>) -> &'static TaxIdRule {
    match country {
        Some("SA") => &SA,
        _ => &EG,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eg_rule_accepts_exactly_9_digits() {
        let rule = tax_id_rule(Some("EG"));
        assert!(rule.matches("123456789"));
        assert!(!rule.matches("12345678"));
        assert!(!rule.matches("1234567890"));
        assert!(!rule.matches("12345678a"));
        assert!(!rule.matches(""));
    }

    #[test]
    fn sa_rule_accepts_15_digits_starting_and_ending_with_3() {
        let rule = tax_id_rule(Some("SA"));
        assert!(rule.matches("312345678901233"));
        assert!(!rule.matches("312345678901234")); // doesn't end with 3
        assert!(!rule.matches("212345678901233")); // doesn't start with 3
        assert!(!rule.matches("31234567890123")); // 14 digits
        assert!(!rule.matches("3123456789012333")); // 16 digits
    }

    #[test]
    fn unknown_or_missing_country_falls_back_to_eg() {
        let default_rule = tax_id_rule(None);
        let unknown_rule = tax_id_rule(Some("XX"));
        assert!(default_rule.matches("123456789"));
        assert!(unknown_rule.matches("123456789"));
        assert_eq!(default_rule.label, "رقم التسجيل الضريبي");
    }

    #[test]
    fn error_message_matches_the_mock_format() {
        let rule = tax_id_rule(Some("SA"));
        assert_eq!(rule.error_message(), "الرقم الضريبي يجب أن يكون 15 رقماً يبدأ وينتهي بالرقم 3");
        let eg_rule = tax_id_rule(Some("EG"));
        assert_eq!(eg_rule.error_message(), "رقم التسجيل الضريبي يجب أن يكون 9 أرقام");
    }

    #[test]
    fn arabic_indic_digits_are_not_ascii_digits() {
        // The mock's regex is a plain JS `\d`, which (with no `u` flag / Unicode property escape)
        // matches only ASCII 0-9 — an Arabic-Indic numeral string must be rejected exactly like
        // the mock would reject it.
        let rule = tax_id_rule(Some("EG"));
        assert!(!rule.matches("١٢٣٤٥٦٧٨٩"));
    }
}
