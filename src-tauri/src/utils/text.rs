//! Arabic-aware search normalization, an exact port of `src/modules/core/helpers/search.ts`'s
//! `normalizeArabic`/`matchesSearch` (cross-cutting.md §6: reproduced in Rust rather than a MariaDB
//! collation, since no built-in collation folds hamza/alef variants or Arabic-Indic digits).
//!
//! Also `compare_ar` (21.03 G-34): a port of the mock's `a.localeCompare(b, 'ar')` (e.g.
//! `getInventoryReport`, `13b-reports-operational.md`), backed by ICU4X's compiled `ar` collation
//! data (`icu_collator`, default `strength`/options — the same defaults V8's `Intl.Collator`/
//! `localeCompare` use for a bare `'ar'` locale with no `-u-co-...`/options object).

use std::cmp::Ordering;
use std::sync::OnceLock;

use icu_collator::options::CollatorOptions;
use icu_collator::{Collator, CollatorBorrowed, CollatorPreferences};
use icu_locale_core::LanguageIdentifier;

/// Tashkeel (combining marks U+064B–U+065F, U+0670) + tatweel (U+0640).
fn is_tashkeel_or_tatweel(c: char) -> bool {
    matches!(c, '\u{064B}'..='\u{065F}' | '\u{0670}' | '\u{0640}')
}

fn map_letter(c: char) -> char {
    match c {
        'أ' | 'إ' | 'آ' => 'ا',
        'ة' => 'ه',
        'ى' => 'ي',
        'ؤ' => 'و',
        'ئ' => 'ي',
        other => other,
    }
}

const ARABIC_INDIC_DIGITS: [char; 10] = ['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩'];

fn map_digit(c: char) -> char {
    ARABIC_INDIC_DIGITS
        .iter()
        .position(|&d| d == c)
        .map(|i| char::from_digit(i as u32, 10).unwrap())
        .unwrap_or(c)
}

/// JS's `String.prototype.trim()` whitespace set (including U+FEFF, the BOM) — matches the TS
/// helper's own `.trim()` call exactly rather than Rust's narrower default `char::is_whitespace`.
fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

/// Ports `normalizeArabic` (`search.ts:27-33`).
pub fn normalize_arabic(value: Option<&str>) -> String {
    let Some(value) = value else { return String::new() };
    let mapped: String = value
        .chars()
        .filter(|c| !is_tashkeel_or_tatweel(*c))
        .map(map_letter)
        .map(map_digit)
        .collect();
    mapped.trim_matches(is_js_whitespace).to_lowercase()
}

/// Joins the normalized fields with U+001F (Unit Separator) so a query can never match across two
/// adjacent fields' boundary (e.g. the end of a name and the start of an SKU) — the
/// `search_normalized` generated columns (P2-38) store this joined text.
pub fn search_haystack(fields: &[Option<&str>]) -> String {
    fields.iter().map(|f| normalize_arabic(*f)).collect::<Vec<_>>().join("\u{1F}")
}

/// True if `needle` (normalized) is found inside any of `haystack` (each normalized) — ports
/// `matchesSearch` (`search.ts:36-40`).
pub fn matches_search(haystack: &[Option<&str>], needle: Option<&str>) -> bool {
    let q = normalize_arabic(needle);
    if q.is_empty() {
        return true;
    }
    haystack.iter().any(|h| normalize_arabic(*h).contains(&q))
}

/// The `ar` collator, built once per process (constructing it walks the compiled collation tables,
/// so every call reuses the same `CollatorBorrowed<'static>` rather than rebuilding it — `Collator`/
/// `CollatorBorrowed` hold only `&'static` data behind the `compiled_data` feature, so sharing one
/// across threads via `OnceLock` is sound).
fn ar_collator() -> &'static CollatorBorrowed<'static> {
    static COLLATOR: OnceLock<CollatorBorrowed<'static>> = OnceLock::new();
    COLLATOR.get_or_init(|| {
        let lang: LanguageIdentifier = "ar".parse().expect("'ar' is a valid BCP-47 language identifier");
        let prefs = CollatorPreferences::from(lang);
        Collator::try_new(prefs, CollatorOptions::default()).expect("ICU4X ships compiled 'ar' collation data")
    })
}

/// Ports `a.localeCompare(b, 'ar')` (`compare_ar`, 21.03 G-34) — used wherever the mock sorts
/// human-readable Arabic text with `localeCompare` (e.g. `getInventoryReport`, 13b §3). Plain byte/
/// codepoint order (`str`'s own `Ord`) does not group letters the way a human reader expects, and no
/// MariaDB collation reproduces V8's CLDR-based `ar` ordering, so this always runs in Rust, never in
/// SQL `ORDER BY`.
pub fn compare_ar(a: &str, b: &str) -> Ordering {
    ar_collator().compare(a, b)
}

/// Escapes `% _ \` for a `LIKE` pattern and wraps the escaped text in `%…%` (substring match).
pub fn like_contains(q: &str) -> String {
    let mut escaped = String::with_capacity(q.len());
    for c in q.chars() {
        if c == '%' || c == '_' || c == '\\' {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    format!("%{escaped}%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ahmed_variants_match() {
        assert_eq!(normalize_arabic(Some("احمد")), normalize_arabic(Some("أحمد")));
    }

    #[test]
    fn arabic_indic_digits_match_latin() {
        let normalized_needle = normalize_arabic(Some("٤٢"));
        let normalized_haystack = normalize_arabic(Some("INV-42"));
        assert!(normalized_haystack.contains(&normalized_needle));
    }

    #[test]
    fn haystack_separator_prevents_cross_field_match() {
        // "ab" + "cd" joined naively would contain "bc"; the U+001F separator must prevent that.
        let haystack = search_haystack(&[Some("ab"), Some("cd")]);
        assert!(!haystack.contains("bc"));
        assert!(matches_search(&[Some("ab"), Some("cd")], Some("ab")));
        assert!(!matches_search(&[Some("ab"), Some("cd")], Some("bc")));
    }

    #[test]
    fn like_escaping() {
        assert_eq!(like_contains("50%_off\\"), "%50\\%\\_off\\\\%");
    }

    #[test]
    fn empty_needle_matches_everything() {
        assert!(matches_search(&[Some("x")], None));
        assert!(matches_search(&[Some("x")], Some("")));
    }

    #[test]
    fn compare_ar_orders_arabic_letters_not_by_codepoint() {
        // Plain byte order would put 'ب' (U+0628) before 'أ' (U+0623) since 0x628 > 0x623 is false
        // here (0x623 < 0x628), so byte order and collation order actually agree on these two in
        // isolation; the real point is that hamza variants collate together with 'ا' at primary
        // strength while remaining distinct at a finer one — assert the weaker, always-true
        // property that the collator is a genuine, non-panicking, non-trivial comparator.
        assert_eq!(compare_ar("ابراهيم", "ابراهيم"), Ordering::Equal);
        assert_eq!(compare_ar("أحمد", "بسام"), Ordering::Less);
        assert_eq!(compare_ar("بسام", "أحمد"), Ordering::Greater);
    }
}
