//! Embedded fonts for the production PDF engine (Phase 11a).
//!
//! Extends the spike's Cairo + Noto Naskh Arabic with IBM Plex Sans Arabic
//! and Tajawal, matching the 4 font families the appearance settings (Phase 0,
//! `src/modules/core/controllers/useAppearance.ts`) already offer for the web
//! UI as `@fontsource*` npm packages.
//!
//! **Vendoring approach (same as the spike):** the web UI's fontsource
//! packages only ship woff/woff2, which is fine for browsers but risky for
//! Typst's font loader (`ttf-parser`) — the spike's notes flagged variable
//! woff2 as unreliable there. IBM Plex Sans Arabic and Tajawal aren't even
//! shipped as variable fonts by fontsource (static per-weight woff/woff2
//! only), so rather than transcode woff2 → ttf, Regular + Bold **static TTF**
//! builds of both were pulled fresh from the same upstream Google Fonts OFL
//! repository the spike used for Cairo/Noto Naskh Arabic (same typefaces,
//! same OFL license, different container format). Cairo and Noto Naskh
//! Arabic keep the spike's variable TTFs unchanged.
//!
//! Each font family exposes `{family}_regular_bytes()` / `{family}_bold_bytes()`
//! (or a single variable-font accessor for Cairo/Noto Naskh) plus a
//! `FontSpec` list consumed by `World::new` to build the font book.

/// One embedded font face: the Typst family name the template's `set text(font: ...)`
/// can reference, and its raw font bytes.
pub struct FontSpec {
    pub family: &'static str,
    pub bytes: &'static [u8],
}

pub fn cairo_bytes() -> &'static [u8] {
    include_bytes!("../../assets/fonts/Cairo[slnt,wght].ttf")
}

pub fn noto_naskh_arabic_bytes() -> &'static [u8] {
    include_bytes!("../../assets/fonts/NotoNaskhArabic[wght].ttf")
}

pub fn ibm_plex_sans_arabic_regular_bytes() -> &'static [u8] {
    include_bytes!("../../assets/fonts/IBMPlexSansArabic-Regular.ttf")
}

pub fn ibm_plex_sans_arabic_bold_bytes() -> &'static [u8] {
    include_bytes!("../../assets/fonts/IBMPlexSansArabic-Bold.ttf")
}

pub fn tajawal_regular_bytes() -> &'static [u8] {
    include_bytes!("../../assets/fonts/Tajawal-Regular.ttf")
}

pub fn tajawal_bold_bytes() -> &'static [u8] {
    include_bytes!("../../assets/fonts/Tajawal-Bold.ttf")
}

/// All fonts embedded in the binary and available to every render, by Typst
/// family name (what a template's `set text(font: "...")` / `opts.fontFamily`
/// should reference). Variable fonts (Cairo, Noto Naskh Arabic) contribute
/// their full weight/slant range from one buffer; static families contribute
/// one buffer per weight, all under the same family name so Typst's own
/// weight matching picks the closest face.
pub fn all_fonts() -> Vec<FontSpec> {
    vec![
        FontSpec { family: "Cairo", bytes: cairo_bytes() },
        FontSpec { family: "Noto Naskh Arabic", bytes: noto_naskh_arabic_bytes() },
        FontSpec { family: "IBM Plex Sans Arabic", bytes: ibm_plex_sans_arabic_regular_bytes() },
        FontSpec { family: "IBM Plex Sans Arabic", bytes: ibm_plex_sans_arabic_bold_bytes() },
        FontSpec { family: "Tajawal", bytes: tajawal_regular_bytes() },
        FontSpec { family: "Tajawal", bytes: tajawal_bold_bytes() },
    ]
}
