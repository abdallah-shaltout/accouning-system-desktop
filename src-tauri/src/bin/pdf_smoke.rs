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
//!
//! Phase 11b appends one `render_pdf` smoke test per new document kind added
//! in this phase (docs/v2/12-documents-pdf-excel.md §3's document-kinds
//! table): quotation, credit note, debit note, purchase order, voucher,
//! party statement, Z-report, transfer note, the generic report template,
//! and both label layouts (sheet grid + one-per-page thermal) — each with a
//! realistic payload shaped the way that kind's `pdfService.ts` builder
//! (`src/modules/core/services/pdfService.ts`) actually produces it, sanity
//! checked the same structural way with `lopdf`.

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

    // --- 6+. Phase 11b: one render_pdf smoke test per new document kind -----------------------
    failures += run_new_kind_tests();

    println!("\n=== pdf_smoke summary: {} failure(s) ===", failures);
    if failures > 0 {
        std::process::exit(1);
    }
}

/// Renders one realistic payload through `render_pdf` for the given built-in template id and
/// sanity-checks the PDF structurally with `lopdf`, printing OK/FAIL like the numbered tests
/// above. Returns 1 on failure, 0 on success, so callers can just sum results.
fn smoke_one(label: &str, template_id: &str, payload: serde_json::Value, options: serde_json::Value) -> usize {
    println!("\n=== {label}: render_pdf, template_id={template_id} ===");
    let req = RenderRequest { template_source: None, template_id: Some(template_id.to_string()), payload, options };
    match render_pdf(req) {
        Ok(result) => {
            let bytes = base64::Std.decode(&result.pdf_base64);
            match sanity_check_pdf(&bytes) {
                Ok(pages) => {
                    println!("  OK: {} bytes, standard={}, {} page(s) (lopdf-verified)", bytes.len(), result.achieved_standard, pages);
                    0
                }
                Err(e) => {
                    println!("  FAIL: {e}");
                    1
                }
            }
        }
        Err(diags) => {
            println!("  FAIL: compile/export errors: {diags:?}");
            1
        }
    }
}

fn run_new_kind_tests() -> usize {
    let mut failures = 0;
    let opts = default_options();

    let document_meta = |kind: &str, title_ar: &str, title_en: &str| {
        serde_json::json!({ "kind": kind, "number": "DOC-000123", "date": "2026-09-24 10:00", "titleAr": title_ar, "titleEn": title_en })
    };
    let company = || serde_json::json!({ "name": "مؤسسة الفاتورة النموذجية للتجارة", "address": "الرياض", "phone": "0112345678", "vatNumber": "311111111100003", "commercialRegister": "1010123456", "logo": null });
    let party = || serde_json::json!({ "name": "شركة العميل النموذجي المحدودة", "vatNumber": null, "address": "جدة", "phone": "0509876543" });
    let lines = || {
        serde_json::json!([
            { "name": "منتج تجريبي أول", "sku": "SKU-1", "qty": "٢", "price": "100.00", "discount": "0.00", "net": "200.00", "vatRate": 15, "vat": "30.00", "total": "230.00" },
            { "name": "منتج تجريبي ثانٍ", "sku": "SKU-2", "qty": "١", "price": "50.00", "discount": "0.00", "net": "50.00", "vatRate": 15, "vat": "7.50", "total": "57.50" },
        ])
    };
    let totals = || serde_json::json!({ "subtotal": "250.00", "discount": null, "vat": "37.50", "grand": "287.50", "paid": null, "remaining": null, "amountInWords": "فقط لا غير: مئتان وسبعة وثمانون ريالاً سعودياً وخمسون هللة", "previousBalance": null, "currentBalance": null });

    // quotation
    {
        let mut doc = document_meta("quotation", "عرض سعر", "QUOTATION");
        doc["validUntil"] = serde_json::json!("2026-10-24");
        let payload = serde_json::json!({ "document": doc, "company": company(), "party": party(), "lines": lines(), "totals": totals(), "qr": null, "logo": null });
        failures += smoke_one("quotation", "quotation", payload, opts.clone());
    }

    // credit note
    {
        let mut doc = document_meta("creditNote", "إشعار دائن", "CREDIT NOTE");
        doc["refNumber"] = serde_json::json!("INV-000512");
        doc["refDate"] = serde_json::json!("2026-09-20");
        doc["reason"] = serde_json::json!("رجوع بضاعة تالفة");
        let payload = serde_json::json!({ "document": doc, "company": company(), "party": party(), "lines": lines(), "totals": totals(), "qr": null, "logo": null });
        failures += smoke_one("credit_note", "credit_note", payload, opts.clone());
    }

    // debit note
    {
        let mut doc = document_meta("debitNote", "إشعار مدين", "DEBIT NOTE");
        doc["refNumber"] = serde_json::json!("PO-000077");
        doc["refDate"] = serde_json::json!("2026-09-18");
        doc["reason"] = serde_json::json!("بضاعة منتهية الصلاحية");
        let payload = serde_json::json!({ "document": doc, "company": company(), "party": party(), "lines": lines(), "totals": totals(), "qr": null, "logo": null });
        failures += smoke_one("debit_note", "debit_note", payload, opts.clone());
    }

    // purchase order
    {
        let doc = document_meta("purchaseOrder", "أمر شراء", "PURCHASE ORDER");
        let payload = serde_json::json!({ "document": doc, "company": company(), "party": party(), "lines": lines(), "totals": totals(), "qr": null, "logo": null });
        failures += smoke_one("purchase_order", "purchase_order", payload, opts.clone());
    }

    // voucher
    {
        let mut doc = document_meta("voucher", "سند قبض", "RECEIPT VOUCHER");
        doc["description"] = serde_json::json!("تحصيل دفعة نقدية من العميل");
        doc["accountsLine"] = serde_json::json!("من: الصندوق — إلى: العملاء");
        doc["note"] = serde_json::json!(null);
        let payload = serde_json::json!({
            "document": doc, "company": company(), "party": null, "lines": [],
            "totals": { "subtotal": null, "discount": null, "vat": null, "grand": "1,000.00", "paid": null, "remaining": null, "amountInWords": "فقط لا غير: ألف ريال سعودي", "previousBalance": null, "currentBalance": null },
            "qr": null, "logo": null,
        });
        failures += smoke_one("voucher", "voucher", payload, opts.clone());
    }

    // party statement
    {
        let doc = document_meta("statement", "كشف حساب", "STATEMENT OF ACCOUNT");
        let statement_lines = serde_json::json!([
            { "date": "2026-09-01", "description": "رصيد افتتاحي", "number": "OPEN-1", "debit": "", "credit": "", "balance": "0.00" },
            { "date": "2026-09-10", "description": "فاتورة مبيعات", "number": "INV-000512", "debit": "287.50", "credit": "", "balance": "287.50" },
            { "date": "2026-09-20", "description": "سند قبض", "number": "RCV-000031", "debit": "", "credit": "200.00", "balance": "87.50" },
        ]);
        let payload = serde_json::json!({
            "document": doc, "company": company(),
            "party": { "name": "شركة العميل النموذجي المحدودة", "code": "CUST-0042", "vatNumber": null, "address": "جدة", "phone": "0509876543" },
            "lines": statement_lines,
            "totals": { "subtotal": null, "discount": null, "vat": null, "grand": "87.50", "paid": null, "remaining": null, "amountInWords": null, "previousBalance": null, "currentBalance": null },
            "qr": null, "logo": null,
        });
        failures += smoke_one("statement", "statement", payload, opts.clone());
    }

    // Z-report
    {
        let mut doc = document_meta("zReport", "تقرير إغلاق الوردية (Z)", "Z-REPORT");
        doc["cashierName"] = serde_json::json!("محمد الكاشير");
        doc["openedAt"] = serde_json::json!("2026-09-24 08:00");
        doc["closedAt"] = serde_json::json!("2026-09-24 16:00");
        let movements = serde_json::json!([
            { "time": "2026-09-24 09:15", "kind": "SALE_CASH", "ref": "INV-000510", "amount": "230.00" },
            { "time": "2026-09-24 12:40", "kind": "PAY_OUT", "ref": "", "amount": "50.00" },
        ]);
        let payload = serde_json::json!({
            "document": doc, "company": company(), "party": null, "lines": movements,
            "totals": {
                "openingFloat": "500.00", "cashSales": "230.00", "cashRefunds": "0.00", "payIns": "0.00",
                "payOuts": "50.00", "bankDrops": "0.00", "expectedCash": "680.00", "countedCash": "678.00",
                "grand": "-2.00",
            },
            "qr": null, "logo": null,
        });
        failures += smoke_one("z_report", "z_report", payload, opts.clone());
    }

    // transfer note
    {
        let mut doc = document_meta("transferNote", "إذن تحويل مخزون", "STOCK TRANSFER NOTE");
        doc["fromBranch"] = serde_json::json!("الفرع الرئيسي");
        doc["toBranch"] = serde_json::json!("فرع جدة");
        doc["note"] = serde_json::json!(null);
        let transfer_lines = serde_json::json!([
            { "name": "منتج تجريبي أول", "qty": "٥", "price": "", "discount": "", "net": "", "vatRate": 0, "vat": "", "total": "٥" },
            { "name": "منتج تجريبي ثانٍ", "qty": "٣", "price": "", "discount": "", "net": "", "vatRate": 0, "vat": "", "total": "٣" },
        ]);
        let payload = serde_json::json!({
            "document": doc, "company": company(), "party": null, "lines": transfer_lines,
            "totals": { "subtotal": null, "discount": null, "vat": null, "grand": null, "paid": null, "remaining": null, "amountInWords": null, "previousBalance": null, "currentBalance": null },
            "qr": null, "logo": null,
        });
        failures += smoke_one("transfer_note", "transfer_note", payload, opts.clone());
    }

    // generic report
    {
        let doc = document_meta("report", "تقرير المبيعات اليومي", "DAILY SALES REPORT");
        let report_opts = serde_json::json!({
            "accentColor": "#4f46e5", "fontFamily": "Cairo", "fontSize": 10, "paper": "a4",
            "columns": [
                { "key": "number", "label": "الرقم", "visible": true },
                { "key": "customer", "label": "العميل", "visible": true },
                { "key": "total", "label": "الإجمالي", "visible": true },
            ],
        });
        let report_rows = serde_json::json!([
            { "number": "INV-000510", "customer": "عميل نقدي", "total": "230.00" },
            { "number": "INV-000511", "customer": "شركة العميل النموذجي", "total": "287.50" },
        ]);
        let mut payload_doc = doc.clone();
        payload_doc["filterLine"] = serde_json::json!("من 2026-09-24 إلى 2026-09-24");
        let payload = serde_json::json!({
            "document": payload_doc, "company": company(), "party": null, "lines": report_rows,
            "totals": { "grand": "517.50" },
            "qr": null, "logo": null,
        });
        failures += smoke_one("generic_report", "generic_report", payload, report_opts);
    }

    // label sheet (A4 grid)
    {
        let label_opts = serde_json::json!({
            "accentColor": "#4f46e5", "fontFamily": "Cairo", "paper": "a4",
            "label": { "widthMm": 70.0, "heightMm": 37.0, "cols": 3, "rows": 8, "marginTopMm": 10.0, "marginLeftMm": 8.0, "gutterXMm": 2.0, "gutterYMm": 0.0, "startCell": 1, "showStoreName": true, "showPrice": true, "showSku": false, "showBatch": false, "showBarcode": false, "showQr": false },
        });
        let label_items = serde_json::json!([
            { "name": "منتج تجريبي أول", "priceText": "100.00", "sku": "SKU-1" },
            { "name": "منتج تجريبي ثانٍ", "priceText": "50.00", "sku": "SKU-2" },
        ]);
        let payload = serde_json::json!({
            "document": { "kind": "label", "number": "", "date": "2026-09-24", "titleAr": "ملصقات", "titleEn": "LABELS" },
            "company": { "name": "مؤسسة الفاتورة النموذجية للتجارة", "logo": null },
            "party": null, "lines": [], "totals": {}, "qr": null, "logo": null,
            "labels": label_items,
        });
        failures += smoke_one("label_sheet", "label_sheet", payload, label_opts);
    }

    // label thermal (one per page)
    {
        let label_opts = serde_json::json!({
            "accentColor": "#4f46e5", "fontFamily": "Cairo", "paper": "58mm",
            "label": { "widthMm": 50.0, "heightMm": 30.0, "cols": 1, "rows": 1, "marginTopMm": 2.0, "marginLeftMm": 2.0, "gutterXMm": 0.0, "gutterYMm": 0.0, "startCell": 1, "showStoreName": true, "showPrice": true, "showSku": true, "showBatch": true, "showBarcode": false, "showQr": false },
        });
        let label_items = serde_json::json!([
            { "name": "منتج تجريبي أول", "priceText": "100.00", "sku": "SKU-1", "batchNo": "B-2026-01", "expiryText": "2027-01-01" },
        ]);
        let payload = serde_json::json!({
            "document": { "kind": "label", "number": "", "date": "2026-09-24", "titleAr": "ملصق", "titleEn": "LABEL" },
            "company": { "name": "مؤسسة الفاتورة النموذجية للتجارة", "logo": null },
            "party": null, "lines": [], "totals": {}, "qr": null, "logo": null,
            "labels": label_items,
        });
        failures += smoke_one("label_thermal", "label_thermal", payload, label_opts);
    }

    failures
}
