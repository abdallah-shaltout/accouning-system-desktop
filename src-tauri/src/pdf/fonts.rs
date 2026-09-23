//! Embedded fonts for the Typst PDF spike.
//!
//! Both fonts are vendored as variable TTFs under `src-tauri/assets/fonts/`
//! and baked into the binary with `include_bytes!`, per the architecture in
//! docs/v2/12-documents-pdf-excel.md §2 ("Fonts embedded with
//! `include_bytes!`").
//!
//! Font provenance:
//! - Cairo: the project already vendors Cairo as a variable **woff2** via the
//!   `@fontsource-variable/cairo` npm package (used by the web UI), but
//!   Typst's font loader (via `ttf-parser`) wants ttf/otf/woff for reliable
//!   parsing of variable axes, so for the spike a variable **TTF** build of
//!   Cairo was pulled from the same upstream source (Google Fonts' OFL
//!   repository) instead of transcoding the woff2. Same typeface, same
//!   license (OFL), different container format — safe for a spike; Phase 11a
//!   should decide whether to standardize on one font pipeline for both web
//!   and Typst.
//! - Noto Naskh Arabic: not vendored anywhere in the repo (checked
//!   `node_modules/@fontsource*` — absent), so it was downloaded fresh from
//!   Google Fonts' OFL repository (variable TTF, weight axis).
pub fn cairo_bytes() -> Vec<u8> {
    include_bytes!("../../assets/fonts/Cairo[slnt,wght].ttf").to_vec()
}

pub fn noto_naskh_arabic_bytes() -> Vec<u8> {
    include_bytes!("../../assets/fonts/NotoNaskhArabic[wght].ttf").to_vec()
}
