//! Typst PDF spike (Phase 0 gate).
//!
//! This module proves out embedding Typst in Rust to render an invoice-like
//! document with Arabic shaping, a multi-page table with a repeating header,
//! mixed Arabic/English/digit content, a QR image and PDF/A-3b export.
//!
//! It is deliberately a spike, not the production PDF engine described in
//! docs/v2/12-documents-pdf-excel.md §2 (that is Phase 11a's job). Kept in
//! `pdf/` rather than `spike/` because this *is* the shape §2 describes
//! (`src-tauri/src/pdf/`, `render_pdf` command, `src-tauri/templates/*.typ`,
//! embedded fonts) — Phase 11a should be able to grow this file rather than
//! replace it.

pub mod fonts;
mod payload;
mod qr;
pub mod raster;
pub mod render;
mod world;

pub use payload::{DocumentMeta, DocumentPayload, PaperSize, RenderRequest, TemplateOptionsPeek};
pub use render::{CompileDiagnostic, RenderPdfResult, RenderPreviewResult};
pub use world::{RenderWorld, SpikeWorld};

// `raster` and `fonts` are `pub mod` (rather than `pub use`-only) so Phase
// 14's `src-tauri/src/print/` can reach `pdf::raster::rasterize_page` and
// `pdf::fonts::all_fonts` directly — the same embedded font set and PNG
// rasterization the PDF/preview pipeline uses, reused rather than duplicated
// for thermal rendering (docs/v2/12-documents-pdf-excel.md §5: "thermal
// printing renders through Typst too").

// NOTE: `render_pdf` / `render_preview` are referenced from `lib.rs` as
// `pdf::render::render_pdf` / `pdf::render::render_preview` (not re-exported
// here as plain functions) because `#[tauri::command]` generates sibling
// items (`__cmd__render_pdf` etc.) in the *defining* module that
// `tauri::generate_handler!` looks up relative to the path you give it — a
// `pub use` re-export doesn't bring those along.

use std::time::Instant;

use serde::{Deserialize, Serialize};
use typst_layout::PagedDocument;
use typst_pdf::{PdfOptions, PdfStandard, PdfStandards};

/// Sample line item for the spike's hardcoded `DocumentPayload`-like struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikeLine {
    pub name_ar: String,
    pub sku: String,
    pub qty: String,
    pub price: String,
    pub vat: String,
    pub total: String,
}

/// Sample `DocumentPayload`-like struct for the spike. Production Phase 11a
/// will replace this with the real DocumentPayload from §2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikeDocument {
    pub party_name: String,
    pub party_code: String,
    pub lines: Vec<SpikeLine>,
    pub subtotal: String,
    pub vat: String,
    pub grand: String,
}

impl Default for SpikeDocument {
    /// Sample data satisfying every spike requirement:
    /// - Arabic product name that is long enough to wrap within its cell.
    /// - English SKU + Latin/Arabic-Indic digits side by side in the same row.
    /// - 28 rows, enough that the table body itself spans onto a second page
    ///   (not just trailing content), exercising the repeating `table.header`.
    fn default() -> Self {
        let long_name =
            "جهاز تكييف هواء منزلي بقدرة ثمانية عشر ألف وحدة حرارية بريطانية مع تقنية التبريد السريع والفلتر الذكي المضاد للبكتيريا";
        let mut lines = vec![SpikeLine {
            name_ar: long_name.to_string(),
            sku: "AC-18000-BTU".to_string(),
            qty: "١".to_string(),
            price: "1,899.00".to_string(),
            vat: "284.85".to_string(),
            total: "2,183.85".to_string(),
        }];
        for i in 1..28 {
            lines.push(SpikeLine {
                name_ar: format!("منتج تجريبي رقم {}", arabic_indic(i + 1)),
                sku: format!("SKU-{:04}", 1000 + i),
                qty: arabic_indic((i % 5) + 1),
                price: format!("{}.00", 50 + i * 10),
                vat: format!("{:.2}", (50 + i * 10) as f64 * 0.15),
                total: format!("{:.2}", (50 + i * 10) as f64 * 1.15),
            });
        }
        Self {
            party_name: "مؤسسة الاختبار التجارية".to_string(),
            party_code: "CUST-0042".to_string(),
            lines,
            subtotal: "5,120.00".to_string(),
            vat: "768.00".to_string(),
            grand: "5,888.00".to_string(),
        }
    }
}

fn arabic_indic(n: u32) -> String {
    n.to_string()
        .chars()
        .map(|c| match c {
            '0'..='9' => {
                char::from_u32('٠' as u32 + (c as u32 - '0' as u32)).unwrap_or(c)
            }
            other => other,
        })
        .collect()
}

/// Result of a spike render: the PDF bytes plus timing/diagnostic info.
pub struct RenderResult {
    pub pdf_bytes: Vec<u8>,
    pub compile_ms: f64,
    pub export_ms: f64,
    pub total_ms: f64,
    pub achieved_standard: &'static str,
    pub warnings: Vec<String>,
}

/// Renders the spike invoice template with the given sample document.
///
/// Tries PDF/A-3b first (the ideal per docs/v2/12-documents-pdf-excel.md §1).
/// If that combination of standards is rejected by krilla/typst-pdf, falls
/// back to plain PDF 1.7 and reports which standard was actually achieved,
/// per the spike's instructions ("note that clearly rather than silently
/// skipping it").
pub fn render_spike(doc: &SpikeDocument) -> Result<RenderResult, String> {
    let total_start = Instant::now();

    let template_source =
        include_str!("../../templates/invoice_spike.typ").to_string();
    let cairo_bytes = fonts::cairo_bytes();
    let naskh_bytes = fonts::noto_naskh_arabic_bytes();
    let qr_png = qr::placeholder_qr_png(&format!(
        "SPIKE|{}|{}|{}",
        doc.party_code, doc.grand, doc.vat
    ))?;

    let data_json = serde_json::json!({
        "party": { "name": doc.party_name, "code": doc.party_code },
        "lines": doc.lines,
        "totals": { "subtotal": doc.subtotal, "vat": doc.vat, "grand": doc.grand },
    })
    .to_string();

    let world = SpikeWorld::new(
        template_source,
        data_json,
        qr_png,
        vec![cairo_bytes.to_vec(), naskh_bytes.to_vec()],
    )?;

    let compile_start = Instant::now();
    let warned = typst::compile::<PagedDocument>(&world);
    let compile_ms = compile_start.elapsed().as_secs_f64() * 1000.0;

    let document = warned
        .output
        .map_err(|diags| format_diagnostics(&world, &diags))?;

    let warnings: Vec<String> = warned
        .warnings
        .iter()
        .map(|w| w.message.to_string())
        .collect();

    // Try PDF/A-3b first, per the gate's ideal target.
    let export_start = Instant::now();
    let a3b_standards = PdfStandards::new(&[PdfStandard::A_3b])
        .map_err(|e| format!("could not build PDF/A-3b standard set: {e:?}"))?;
    let a3b_options = PdfOptions {
        standards: a3b_standards,
        ..Default::default()
    };

    let (pdf_bytes, achieved_standard) =
        match typst_pdf::pdf(&document, &a3b_options) {
            Ok(bytes) => (bytes, "PDF/A-3b"),
            Err(diags) => {
                // Document not compatible with the strict PDF/A-3b validator
                // (e.g. missing output-intent ICC profile, tagging gaps).
                // Fall back to plain PDF 1.7 and report clearly, per spike
                // instructions: never silently skip the requirement.
                eprintln!(
                    "[typst spike] PDF/A-3b export failed, falling back to plain PDF 1.7: {}",
                    format_diagnostics(&world, &diags)
                );
                let plain_options = PdfOptions::default();
                let bytes = typst_pdf::pdf(&document, &plain_options)
                    .map_err(|diags| format_diagnostics(&world, &diags))?;
                (bytes, "PDF 1.7 (PDF/A-3b export rejected, see stderr)")
            }
        };
    let export_ms = export_start.elapsed().as_secs_f64() * 1000.0;
    let total_ms = total_start.elapsed().as_secs_f64() * 1000.0;

    Ok(RenderResult {
        pdf_bytes,
        compile_ms,
        export_ms,
        total_ms,
        achieved_standard,
        warnings,
    })
}

fn format_diagnostics(
    world: &SpikeWorld,
    diags: &ecow::EcoVec<typst::diag::SourceDiagnostic>,
) -> String {
    let _ = world;
    diags
        .iter()
        .map(|d| d.message.to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Tauri command wrapping the spike render for manual invocation from the
/// frontend (dev-only). Returns raw PDF bytes to the caller.
#[tauri::command]
pub fn render_pdf_spike() -> Result<Vec<u8>, String> {
    let doc = SpikeDocument::default();
    let result = render_spike(&doc)?;
    eprintln!(
        "[typst spike] compile={:.2}ms export={:.2}ms total={:.2}ms standard={}",
        result.compile_ms, result.export_ms, result.total_ms, result.achieved_standard
    );
    for w in &result.warnings {
        eprintln!("[typst spike] warning: {w}");
    }
    Ok(result.pdf_bytes)
}
