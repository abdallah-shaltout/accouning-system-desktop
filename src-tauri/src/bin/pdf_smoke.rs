//! Phase 11a verification binary: calls the production `render_pdf` and
//! `render_preview` Rust commands directly with realistic invoice JSON,
//! exactly the shape `pdfService.ts` builds (`DocumentPayload` +
//! `TemplateOptions`). This is the closest this environment can get to a
//! real Tauri IPC round trip without a running desktop shell (no windowing
//! system to drive `bun run tauri dev` interactively here) — it exercises
//! the exact same Rust functions the IPC layer calls, just invoked directly
//! instead of through `invoke()`.
//!
//! `cargo run --bin pdf_smoke` — renders a realistic standard invoice + a
//! realistic simplified invoice through `render_pdf` (PDF bytes, sanity
//! checked structurally with `lopdf` like the spike did), then renders the
//! standard invoice through `render_preview` (SVG pages), then deliberately
//! breaks the Typst source and confirms `render_pdf` returns a structured
//! compile error with a real line number, and confirms it recovers once the
//! syntax is fixed again.

use accounting_app_lib::pdf::render::{render_pdf, render_preview};
use accounting_app_lib::pdf::RenderRequest;

/// Minimal local base64 decoder (mirrors `pdf/render.rs`'s private one) just
/// to decode the smoke test's own `render_pdf` output back to bytes — kept
/// self-contained rather than adding an external base64 crate dependency
/// for one test binary.
mod base64 {
    pub struct Std;
    impl Std {
        pub fn decode(&self, s: &str) -> Vec<u8> {
            let table: std::collections::HashMap<u8, u8> =
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
                    .bytes()
                    .enumerate()
                    .map(|(i, c)| (c, i as u8))
                    .collect();
            let clean: Vec<u8> = s.bytes().filter(|b| *b != b'=' && !b.is_ascii_whitespace()).collect();
            let mut out = Vec::new();
            for chunk in clean.chunks(4) {
                let vals: Vec<u32> = chunk.iter().map(|b| *table.get(b).unwrap_or(&0) as u32).collect();
                let n = vals.iter().enumerate().fold(0u32, |acc, (i, v)| acc | (v << (18 - 6 * i)));
                out.push((n >> 16) as u8);
                if chunk.len() > 2 {
                    out.push((n >> 8) as u8);
                }
                if chunk.len() > 3 {
                    out.push(n as u8);
                }
            }
            out
        }
    }
}

fn realistic_payload(simplified: bool) -> serde_json::Value {
    serde_json::json!({
        "document": {
            "kind": "invoice",
            "number": "INV-000512",
            "date": "2026-09-23 14:30",
            "titleAr": if simplified { "فاتورة ضريبية مبسطة" } else { "فاتورة ضريبية" },
            "titleEn": if simplified { "SIMPLIFIED TAX INVOICE" } else { "TAX INVOICE" },
        },
        "company": {
            "name": "مؤسسة الفاتورة النموذجية للتجارة",
            "address": "الرياض، حي العليا، طريق الملك فهد",
            "phone": "0112345678",
            "email": "info@example.com",
            "website": "example.com",
            "vatNumber": "311111111100003",
            "commercialRegister": "1010123456",
            "logo": null,
        },
        "party": if simplified { serde_json::Value::Null } else {
            serde_json::json!({
                "name": "شركة العميل النموذجي المحدودة",
                "vatNumber": "311222222200003",
                "address": "جدة، حي الروضة، شارع الأمير سلطان",
                "phone": "0509876543",
            })
        },
        "lines": [
            { "name": "جهاز تكييف هواء منزلي بقدرة ثمانية عشر ألف وحدة حرارية بريطانية", "sku": "AC-18000-BTU", "qty": "١", "price": "1,899.00", "discount": "0.00", "net": "1899.00", "vatRate": 15, "vat": "284.85", "total": "2,183.85" },
            { "name": "منتج تجريبي بالإنجليزية Widget X200", "sku": "SKU-1042", "qty": "٥", "price": "45.00", "discount": "10.00", "net": "215.00", "vatRate": 15, "vat": "32.25", "total": "247.25" },
            { "name": "خدمة تركيب وصيانة", "sku": "SVC-01", "qty": "١", "price": "300.00", "discount": "0.00", "net": "300.00", "vatRate": 15, "vat": "45.00", "total": "345.00" },
        ],
        "totals": {
            "subtotal": "2,414.00",
            "discount": "10.00",
            "vat": "362.10",
            "grand": "2,776.10",
            "paid": "2,776.10",
            "remaining": "0.00",
            "amountInWords": "فقط لا غير: ألفان وسبعمائة وستة وسبعون ريالاً سعودياً وعشرة هللات",
            "previousBalance": null,
            "currentBalance": null,
        },
        "qr": "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 21 21\" width=\"30mm\" height=\"30mm\"><rect width=\"21\" height=\"21\" fill=\"#fff\"/><path d=\"M0,0h1v1h-1zM2,2h1v1h-1z\" fill=\"#000\"/></svg>",
        "logo": null,
    })
}

fn default_options() -> serde_json::Value {
    serde_json::json!({
        "accentColor": "#4f46e5",
        "logoPosition": "start",
        "logoSize": "m",
        "fontFamily": "Cairo",
        "fontSize": 10,
        "header": {
            "showCompanyName": true, "showAddress": true, "showVatNumber": true,
            "showCommercialRegister": true, "showPhone": true, "showEmail": false, "showWebsite": false,
            "title": "فاتورة ضريبية", "titleEn": "TAX INVOICE",
        },
        "columns": [
            { "key": "index", "label": "#", "visible": true },
            { "key": "name", "label": "الصنف", "visible": true },
            { "key": "qty", "label": "الكمية", "visible": true },
            { "key": "price", "label": "السعر", "visible": true },
            { "key": "vatRate", "label": "الضريبة %", "visible": true },
            { "key": "vat", "label": "الضريبة", "visible": true },
            { "key": "total", "label": "الإجمالي", "visible": true },
        ],
        "totals": { "showAmountInWords": true, "showBalance": false },
        "footer": {
            "terms": "البضاعة المباعة لا ترد ولا تستبدل بعد 7 أيام.",
            "bankDetails": "SA0000000000000000000000",
            "showSignatureLines": true,
            "thankYouLine": "شكراً لتعاملكم معنا",
            "showPageNumbers": true,
        },
        "qr": { "position": "center", "size": "3cm" },
        "paper": "a4",
    })
}

fn sanity_check_pdf(bytes: &[u8]) -> Result<usize, String> {
    let doc = lopdf::Document::load_mem(bytes).map_err(|e| format!("lopdf failed to parse: {e}"))?;
    let pages = doc.get_pages();
    if pages.is_empty() {
        return Err("lopdf parsed the document but found zero pages".to_string());
    }
    Ok(pages.len())
}

fn main() {
    let mut failures = 0;

    // --- 1. Standard invoice: render_pdf -----------------------------------------------------
    println!("=== 1. render_pdf: invoice_standard, realistic payload ===");
    let req = RenderRequest {
        template_source: None,
        template_id: Some("invoice_standard".to_string()),
        payload: realistic_payload(false),
        options: default_options(),
    };
    match render_pdf(req) {
        Ok(result) => {
            let bytes = base64::Std.decode(&result.pdf_base64);
            match sanity_check_pdf(&bytes) {
                Ok(pages) => println!("  OK: {} bytes, standard={}, {} page(s) (lopdf-verified)", bytes.len(), result.achieved_standard, pages),
                Err(e) => {
                    println!("  FAIL: {e}");
                    failures += 1;
                }
            }
            let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
            let _ = std::fs::create_dir_all(&out_dir);
            std::fs::write(out_dir.join("pdf_smoke_standard.pdf"), &bytes).expect("write pdf");
            println!("  wrote {}", out_dir.join("pdf_smoke_standard.pdf").display());
        }
        Err(diags) => {
            println!("  FAIL: compile/export errors: {diags:?}");
            failures += 1;
        }
    }

    // --- 2. Simplified invoice: render_pdf ----------------------------------------------------
    println!("\n=== 2. render_pdf: invoice_simplified, realistic payload (no party VAT) ===");
    let req = RenderRequest {
        template_source: None,
        template_id: Some("invoice_simplified".to_string()),
        payload: realistic_payload(true),
        options: default_options(),
    };
    match render_pdf(req) {
        Ok(result) => {
            let bytes = base64::Std.decode(&result.pdf_base64);
            match sanity_check_pdf(&bytes) {
                Ok(pages) => println!("  OK: {} bytes, standard={}, {} page(s) (lopdf-verified)", bytes.len(), result.achieved_standard, pages),
                Err(e) => {
                    println!("  FAIL: {e}");
                    failures += 1;
                }
            }
        }
        Err(diags) => {
            println!("  FAIL: compile/export errors: {diags:?}");
            failures += 1;
        }
    }

    // --- 3. render_preview: SVG pages ----------------------------------------------------------
    println!("\n=== 3. render_preview: invoice_standard, realistic payload ===");
    let req = RenderRequest {
        template_source: None,
        template_id: Some("invoice_standard".to_string()),
        payload: realistic_payload(false),
        options: default_options(),
    };
    match render_preview(req) {
        Ok(result) => {
            if result.pages.is_empty() || !result.pages[0].trim_start().starts_with("<svg") {
                println!("  FAIL: expected >=1 SVG page starting with <svg, got {} page(s)", result.pages.len());
                failures += 1;
            } else {
                println!("  OK: {} SVG page(s), first page {} bytes, starts with <svg", result.pages.len(), result.pages[0].len());
            }
        }
        Err(diags) => {
            println!("  FAIL: {diags:?}");
            failures += 1;
        }
    }

    // --- 4. Deliberately broken custom Typst source: structured compile error -----------------
    println!("\n=== 4. render_pdf: deliberately broken custom template source ===");
    let broken_source = r#"#import "lib.typ": invoice-document

#let data = json("data.json")
#let opts = json("opts.json")

#invoice-document(data, opts, simplified: false
"#; // missing closing paren — a real, common Typst syntax error
    let req = RenderRequest {
        template_source: Some(broken_source.to_string()),
        template_id: None,
        payload: realistic_payload(false),
        options: default_options(),
    };
    match render_pdf(req) {
        Ok(_) => {
            println!("  FAIL: expected a compile error, got a successful render");
            failures += 1;
        }
        Err(diags) => {
            if diags.is_empty() {
                println!("  FAIL: error path returned zero diagnostics");
                failures += 1;
            } else {
                println!("  OK: {} diagnostic(s) returned:", diags.len());
                for d in &diags {
                    println!(
                        "    [{}] line={:?} column={:?}: {}",
                        d.severity, d.line, d.column, d.message
                    );
                }
                let has_line = diags.iter().any(|d| d.line.is_some());
                if !has_line {
                    println!("  FAIL: no diagnostic carried a line number");
                    failures += 1;
                } else {
                    println!("  OK: at least one diagnostic carries a real line number");
                }
            }
        }
    }

    // --- 5. Recovery: fix the syntax, confirm it compiles again -------------------------------
    println!("\n=== 5. render_pdf: same custom template, syntax fixed ===");
    let fixed_source = r#"#import "lib.typ": invoice-document

#let data = json("data.json")
#let opts = json("opts.json")

#invoice-document(data, opts, simplified: false)
"#;
    let req = RenderRequest {
        template_source: Some(fixed_source.to_string()),
        template_id: None,
        payload: realistic_payload(false),
        options: default_options(),
    };
    match render_pdf(req) {
        Ok(result) => {
            let bytes = base64::Std.decode(&result.pdf_base64);
            match sanity_check_pdf(&bytes) {
                Ok(pages) => println!("  OK: recovered, {} bytes, {} page(s)", bytes.len(), pages),
                Err(e) => {
                    println!("  FAIL: {e}");
                    failures += 1;
                }
            }
        }
        Err(diags) => {
            println!("  FAIL: expected recovery, got errors: {diags:?}");
            failures += 1;
        }
    }

    println!("\n=== pdf_smoke summary: {} failure(s) ===", failures);
    if failures > 0 {
        std::process::exit(1);
    }
}
