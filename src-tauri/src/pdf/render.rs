//! Production render commands (Phase 11a): `render_pdf` and `render_preview`,
//! per docs/v2/12-documents-pdf-excel.md §2's architecture diagram.
//!
//! Both take the same `RenderRequest` (payload JSON + options JSON + either
//! `template_id` or raw `template_source`), build a `RenderWorld`, compile
//! once, then either export a PDF (`render_pdf`) or one SVG string per page
//! (`render_preview`, for `PdfPreview.vue`'s live preview).
//!
//! Compile errors are returned as structured `{ line, column, message }`
//! entries (1-based line/column, matching how editors and the advanced
//! source tab want to display them) rather than a flattened string, so the
//! designer's advanced tab can show inline errors at the right line.

use ecow::EcoVec;
use serde::Serialize;
use typst::diag::SourceDiagnostic;
use typst::foundations::Smart;
use typst::World;
use typst_layout::PagedDocument;
use typst_library::WorldExt;
use typst_pdf::{PdfOptions, PdfStandard, PdfStandards};

/// `/Creator` metadata on every exported PDF (docs/v2/16-equal-rebrand-and-ui-kit.md Phase B).
/// `/Producer` isn't user-settable — `typst-pdf` always writes its own "Typst $version" there.
const PDF_CREATOR: &str = "Equal Accounting";

use super::fonts;
use super::payload::{PaperSize, RenderRequest, TemplateOptionsPeek};
use super::world::RenderWorld;

const LIB_TYP: &str = include_str!("../../templates/lib.typ");
const INVOICE_STANDARD_TYP: &str = include_str!("../../templates/invoice_standard.typ");
const INVOICE_SIMPLIFIED_TYP: &str = include_str!("../../templates/invoice_simplified.typ");
// Phase 11b additions (docs/v2/12-documents-pdf-excel.md §3 document kinds table). Each is a new
// template file wired in additively here — `build_world`/`render_pdf`/`render_preview` below are
// unchanged in shape, just fed more `template_id` values.
const QUOTATION_TYP: &str = include_str!("../../templates/quotation.typ");
const CREDIT_NOTE_TYP: &str = include_str!("../../templates/credit_note.typ");
const DEBIT_NOTE_TYP: &str = include_str!("../../templates/debit_note.typ");
const PURCHASE_ORDER_TYP: &str = include_str!("../../templates/purchase_order.typ");
const VOUCHER_TYP: &str = include_str!("../../templates/voucher.typ");
const STATEMENT_TYP: &str = include_str!("../../templates/statement.typ");
const Z_REPORT_TYP: &str = include_str!("../../templates/z_report.typ");
const TRANSFER_NOTE_TYP: &str = include_str!("../../templates/transfer_note.typ");
const GENERIC_REPORT_TYP: &str = include_str!("../../templates/generic_report.typ");
const LABEL_SHEET_TYP: &str = include_str!("../../templates/label_sheet.typ");
const LABEL_THERMAL_TYP: &str = include_str!("../../templates/label_thermal.typ");
// Official report layout (letterhead, meta strip, KPI bar, sectioned tables, signatures) — renders
// `payload.report`, the same `ReportDocument` model the web print/preview renders as HTML.
const REPORT_TYP: &str = include_str!("../../templates/report.typ");

/// One structured compile diagnostic, positioned in the main template source.
#[derive(Debug, Clone, Serialize)]
pub struct CompileDiagnostic {
    /// 1-based line number in the main template source, or `null` when the
    /// diagnostic isn't tied to a specific location (e.g. it points into
    /// `lib.typ` or a virtual data file instead of `main.typ`).
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RenderPdfResult {
    /// Base64-encoded PDF bytes (Tauri IPC JSON-serializes `Vec<u8>` as an
    /// array of numbers otherwise, which is far heavier over the wire).
    pub pdf_base64: String,
    pub achieved_standard: String,
    pub warnings: Vec<CompileDiagnostic>,
}

#[derive(Debug, Serialize)]
pub struct RenderPreviewResult {
    /// One SVG string per page, in order.
    pub pages: Vec<String>,
    pub warnings: Vec<CompileDiagnostic>,
}

fn builtin_template_source(template_id: &str) -> Result<&'static str, String> {
    match template_id {
        "invoice_standard" => Ok(INVOICE_STANDARD_TYP),
        "invoice_simplified" => Ok(INVOICE_SIMPLIFIED_TYP),
        "quotation" => Ok(QUOTATION_TYP),
        "credit_note" => Ok(CREDIT_NOTE_TYP),
        "debit_note" => Ok(DEBIT_NOTE_TYP),
        "purchase_order" => Ok(PURCHASE_ORDER_TYP),
        "voucher" => Ok(VOUCHER_TYP),
        "statement" => Ok(STATEMENT_TYP),
        "z_report" => Ok(Z_REPORT_TYP),
        "transfer_note" => Ok(TRANSFER_NOTE_TYP),
        "generic_report" => Ok(GENERIC_REPORT_TYP),
        "report" => Ok(REPORT_TYP),
        "label_sheet" => Ok(LABEL_SHEET_TYP),
        "label_thermal" => Ok(LABEL_THERMAL_TYP),
        other => Err(format!("unknown built-in template id '{other}'")),
    }
}

fn decode_data_url_or_base64(s: &str) -> Result<Vec<u8>, String> {
    let b64 = s.split_once(",").map(|(_, data)| data).unwrap_or(s);
    base64_decode(b64.trim())
}

/// Minimal base64 decoder (standard alphabet, with or without padding) so we
/// don't need to pull in an extra crate just for logo data: URLs — the qr/PDF
/// paths already need a base64 encoder for the response, added alongside.
mod b64 {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(data: &[u8]) -> String {
        let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
        for chunk in data.chunks(3) {
            let b0 = chunk[0];
            let b1 = *chunk.get(1).unwrap_or(&0);
            let b2 = *chunk.get(2).unwrap_or(&0);
            let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
            out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
            out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
            out.push(if chunk.len() > 1 { ALPHABET[((n >> 6) & 0x3f) as usize] as char } else { '=' });
            out.push(if chunk.len() > 2 { ALPHABET[(n & 0x3f) as usize] as char } else { '=' });
        }
        out
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        let mut table = [255u8; 256];
        for (i, &c) in ALPHABET.iter().enumerate() {
            table[c as usize] = i as u8;
        }
        let clean: Vec<u8> = s.bytes().filter(|&b| b != b'=' && !b.is_ascii_whitespace()).collect();
        let mut out = Vec::with_capacity(clean.len() / 4 * 3);
        for chunk in clean.chunks(4) {
            let mut vals = [0u8; 4];
            for (i, &b) in chunk.iter().enumerate() {
                let v = table[b as usize];
                if v == 255 {
                    return Err(format!("invalid base64 byte: {}", b as char));
                }
                vals[i] = v;
            }
            let n = ((vals[0] as u32) << 18) | ((vals[1] as u32) << 12) | ((vals[2] as u32) << 6) | (vals[3] as u32);
            out.push((n >> 16) as u8);
            if chunk.len() > 2 {
                out.push((n >> 8) as u8);
            }
            if chunk.len() > 3 {
                out.push(n as u8);
            }
        }
        Ok(out)
    }
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    b64::decode(s)
}

/// Builds the `RenderWorld` shared by `render_pdf` and `render_preview`.
fn build_world(req: &RenderRequest) -> Result<RenderWorld, String> {
    let main_source = match (&req.template_source, &req.template_id) {
        (Some(src), _) => src.clone(),
        (None, Some(id)) => builtin_template_source(id)?.to_string(),
        (None, None) => INVOICE_STANDARD_TYP.to_string(),
    };

    let data_json = serde_json::to_string(&req.payload).map_err(|e| format!("invalid payload JSON: {e}"))?;
    let opts_json = serde_json::to_string(&req.options).map_err(|e| format!("invalid options JSON: {e}"))?;

    // Logo: payload.logo may be a data: URL (data:image/png;base64,...) or
    // absent. Typst's `image()` needs a real raster/vector format; PNG only
    // for now (matches the spike's qr.png precedent) — SVG logos are a
    // TODO(phase 11b) since they'd need format sniffing.
    let logo_png = match req.payload.get("logo").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => Some(decode_data_url_or_base64(s)?),
        _ => None,
    };

    let qr_svg = req.payload.get("qr").and_then(|v| v.as_str()).map(|s| s.to_string());

    let font_specs = fonts::all_fonts();

    RenderWorld::new(main_source, LIB_TYP.to_string(), data_json, opts_json, logo_png, qr_svg, font_specs)
}

fn diagnostics_to_structured(world: &RenderWorld, diags: &EcoVec<SourceDiagnostic>, severity: &str) -> Vec<CompileDiagnostic> {
    diags
        .iter()
        .map(|d| {
            let (line, column) = d
                .span
                .id()
                .filter(|id| *id == world.main_id())
                .and_then(|_| world.range(d.span))
                .and_then(|range| {
                    let source = world.source(world.main_id()).ok()?;
                    source.lines().byte_to_line_column(range.start)
                })
                .map(|(l, c)| (Some(l + 1), Some(c + 1)))
                .unwrap_or((None, None));
            CompileDiagnostic { line, column, severity: severity.to_string(), message: d.message.to_string() }
        })
        .collect()
}

fn compile_document(world: &RenderWorld) -> Result<(PagedDocument, Vec<CompileDiagnostic>), Vec<CompileDiagnostic>> {
    let warned = typst::compile::<PagedDocument>(world);
    let warnings = diagnostics_to_structured(world, &warned.warnings, "warning");
    match warned.output {
        Ok(doc) => Ok((doc, warnings)),
        Err(errors) => Err(diagnostics_to_structured(world, &errors, "error")),
    }
}

fn paper_size_from_options(options: &serde_json::Value) -> PaperSize {
    serde_json::from_value::<TemplateOptionsPeek>(options.clone()).map(|p| p.paper).unwrap_or_default()
}

/// Renders a PDF from a `DocumentPayload` + `TemplateOptions` + template
/// (built-in id or custom source). Returns base64 PDF bytes so the frontend
/// can `atob` → `Uint8Array` → save/attach without a lossy JSON number array.
///
/// Tries PDF/A-3b first; on rejection (e.g. missing ICC output intent for a
/// non-invoice template that hasn't set `document(date: ...)`), falls back to
/// plain PDF 1.7, matching the spike's fallback behavior — this is never
/// silent, `achieved_standard` in the result always says which was used.
#[tauri::command]
pub fn render_pdf(req: RenderRequest) -> Result<RenderPdfResult, Vec<CompileDiagnostic>> {
    let _paper = paper_size_from_options(&req.options);
    let world = build_world(&req).map_err(|e| vec![CompileDiagnostic { line: None, column: None, severity: "error".into(), message: e }])?;

    let (document, mut warnings) = compile_document(&world)?;

    let a3b_standards = PdfStandards::new(&[PdfStandard::A_3b]).map_err(|e| {
        vec![CompileDiagnostic { line: None, column: None, severity: "error".into(), message: format!("could not build PDF/A-3b standard set: {e:?}") }]
    })?;
    let a3b_options = PdfOptions { standards: a3b_standards, creator: Smart::Custom(Some(PDF_CREATOR.into())), ..Default::default() };

    let (pdf_bytes, achieved_standard) = match typst_pdf::pdf(&document, &a3b_options) {
        Ok(bytes) => (bytes, "PDF/A-3b".to_string()),
        Err(diags) => {
            let fallback_diags = diagnostics_to_structured(&world, &diags, "warning");
            warnings.extend(fallback_diags);
            let plain_options = PdfOptions { creator: Smart::Custom(Some(PDF_CREATOR.into())), ..Default::default() };
            let bytes = typst_pdf::pdf(&document, &plain_options)
                .map_err(|diags| diagnostics_to_structured(&world, &diags, "error"))?;
            (bytes, "PDF 1.7 (PDF/A-3b export rejected, see warnings)".to_string())
        }
    };

    Ok(RenderPdfResult { pdf_base64: b64::encode(&pdf_bytes), achieved_standard, warnings })
}

/// Renders each page of the document as an SVG string, for `PdfPreview.vue`'s
/// live preview.
#[tauri::command]
pub fn render_preview(req: RenderRequest) -> Result<RenderPreviewResult, Vec<CompileDiagnostic>> {
    let world = build_world(&req).map_err(|e| vec![CompileDiagnostic { line: None, column: None, severity: "error".into(), message: e }])?;
    let (document, warnings) = compile_document(&world)?;

    let svg_opts = typst_svg::SvgOptions::default();
    let pages: Vec<String> = document.pages().iter().map(|page| typst_svg::svg(page, &svg_opts)).collect();

    Ok(RenderPreviewResult { pages, warnings })
}
