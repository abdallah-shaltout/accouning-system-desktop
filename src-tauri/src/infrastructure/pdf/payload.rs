//! `DocumentPayload` and `TemplateOptions`: the JSON contracts between
//! `pdfService.ts` and the Rust render commands, per
//! docs/v2/12-documents-pdf-excel.md §2 ("Data in: the template reads
//! `json("data.json")`. Everything is prepared in TS ... The template only
//! lays out.") and §3 (`TemplateOptions`).
//!
//! These are intentionally loose (`serde_json::Value` for the free-form bits,
//! `#[serde(default)]` everywhere) because:
//! - The TS side is the source of truth for shape; Rust just needs to get the
//!   JSON to the Typst world and back out again.
//! - The advanced Typst source editor lets a template reference fields this
//!   struct doesn't know about, so round-tripping through a typed struct
//!   would silently drop them. `render_pdf`/`render_preview` pass the raw
//!   JSON bytes straight through as virtual files rather than re-serializing
//!   a Rust struct — these types exist mainly for documentation and for the
//!   few fields Rust itself needs to read (template kind, paper size).

use serde::{Deserialize, Serialize};

/// Only the fields Rust needs to read directly. Everything else in the
/// payload/options JSON passes through opaquely to the Typst `data.json` /
/// `opts.json` virtual files.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DocumentPayload {
    pub document: DocumentMeta,
    #[serde(default)]
    pub company: serde_json::Value,
    #[serde(default)]
    pub party: serde_json::Value,
    #[serde(default)]
    pub lines: Vec<serde_json::Value>,
    #[serde(default)]
    pub totals: serde_json::Value,
    /// Pre-rendered QR SVG markup (built in TS from `zatcaQr.ts` + `uqr`), or
    /// null when the template has no QR (e.g. a quotation).
    #[serde(default)]
    pub qr: Option<String>,
    /// data: URL or raw base64 PNG/SVG for the company logo, or null.
    #[serde(default)]
    pub logo: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DocumentMeta {
    pub kind: String,
    pub number: String,
    pub date: String,
    #[serde(rename = "titleAr")]
    pub title_ar: String,
    #[serde(rename = "titleEn")]
    pub title_en: String,
}

/// Paper size subset proved in Phase 11a (docs/v2/12-documents-pdf-excel.md
/// §3 "Paper: A4 / A5 / Letter / 80mm / 58mm / custom label size").
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PaperSize {
    A4,
    A5,
    Letter,
    #[serde(rename = "80mm")]
    Mm80,
    #[serde(rename = "58mm")]
    Mm58,
}

impl Default for PaperSize {
    fn default() -> Self {
        PaperSize::A4
    }
}

/// The subset of `TemplateOptions` (docs/v2/12-documents-pdf-excel.md §3)
/// Rust needs to read (paper size, for picking the Typst page size). The
/// full options object is passed through to `opts.json` verbatim — this
/// struct is only used to peek at `paper` before compiling.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TemplateOptionsPeek {
    #[serde(default)]
    pub paper: PaperSize,
}

/// Request shape for both `render_pdf` and `render_preview`.
#[derive(Debug, Clone, Deserialize)]
pub struct RenderRequest {
    /// Raw Typst source (advanced tab / custom template) — mutually
    /// exclusive with `template_id`. When both are omitted, `template_id`
    /// defaults to `invoice_standard`.
    #[serde(default)]
    pub template_source: Option<String>,
    #[serde(default)]
    pub template_id: Option<String>,
    /// The `DocumentPayload` JSON, as a raw value so unknown fields survive.
    pub payload: serde_json::Value,
    /// The `TemplateOptions` JSON, as a raw value so unknown fields survive.
    #[serde(default)]
    pub options: serde_json::Value,
}
