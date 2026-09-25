//! Official report template smoke test: renders `templates/report.typ` with a realistic
//! `ReportDocument` (the shape `src/modules/reports/print/types.ts` builds) through the production
//! `render_pdf` + `render_preview` commands, and writes the PDF plus one SVG per page to
//! `target/report_smoke*` so the layout can be eyeballed.
//!
//! `cargo run --bin report_smoke`

use accounting_app_lib::pdf::render::{render_pdf, render_preview};
use accounting_app_lib::pdf::RenderRequest;
use serde_json::{json, Value};

fn row(cells: &[&str], kind: &str) -> Value {
    json!({ "cells": cells, "kind": kind })
}

fn company() -> Value {
    json!({ "name": "متجر النموذج للتجارة", "address": "الرياض، حي العليا، طريق الملك فهد", "phone": "0112345678", "vatNumber": "311111111100003", "commercialRegister": "1010123456" })
}

fn meta() -> Value {
    json!([
        { "label": "الفترة من", "value": "01/01/2026" },
        { "label": "الفترة إلى", "value": "24/09/2026" },
        { "label": "تاريخ الإصدار", "value": "24/09/2026 10:30" },
        { "label": "العملة", "value": "SAR" },
        { "label": "أُعدّ بواسطة", "value": "مدير النظام" },
    ])
}

fn trial_balance(orientation: &str) -> Value {
    let mut rows = vec![];
    let accounts = [
        ("1110", "الصندوق الرئيسي"), ("1120", "البنك الأهلي"), ("1130", "نقاط البيع - مدى"), ("1210", "العملاء"),
        ("1310", "المخزون"), ("1410", "ضريبة المدخلات"), ("2110", "الموردون"), ("2210", "ضريبة المخرجات"),
        ("3100", "رأس المال"), ("4100", "المبيعات"), ("4200", "مرتجعات المبيعات"), ("5100", "تكلفة البضاعة المباعة"),
        ("6100", "الرواتب والأجور"), ("6200", "الإيجار"), ("6300", "الكهرباء والمياه"),
    ];
    for rep in 0..4 {
        for (code, name) in accounts.iter() {
            let label = if rep == 0 { name.to_string() } else { format!("{name} ({rep})") };
            rows.push(json!({ "cells": [code, label, "12,500.00", "3,450.75", "—", "15,950.75", "—"], "kind": "normal" }));
        }
    }
    rows.push(row(&["", "الإجمالي", "650,728.51", "210,450.00", "210,450.00", "650,728.51", "650,728.51"], "total"));
    json!({
        "title": "ميزان المراجعة",
        "subtitle": "أرصدة الحسابات في نهاية الفترة — يجب أن يتساوى المدين والدائن",
        "badge": "تقرير رسمي",
        "company": company(),
        "meta": meta(),
        "orientation": orientation,
        "issuedAt": "24/09/2026 10:30",
        "issuedIso": "2026-09-24T10:30:00.000Z",
        "footerNote": "هذا التقرير صادر من النظام آلياً ومخصص للاستخدام الرسمي",
        "signatures": ["المحاسب", "المدير المالي", "المدير العام"],
        "blocks": [
            { "type": "kpis", "items": [
                { "label": "عدد الحسابات ذات الرصيد", "value": "60" },
                { "label": "إجمالي المدين", "value": "650,728.51", "emphasis": true },
                { "label": "إجمالي الدائن", "value": "650,728.51" },
                { "label": "الفرق", "value": "-0.00" },
            ]},
            { "type": "note", "text": "الميزان متوازن — إجمالي المدين يساوي إجمالي الدائن", "tone": "ok" },
            { "type": "table", "columns": [
                { "label": "الرمز", "dim": true, "width": 0.7 },
                { "label": "الحساب", "width": 2.6 },
                { "label": "رصيد أول المدة", "numeric": true, "width": 1.2 },
                { "label": "حركة مدينة", "numeric": true, "width": 1.2 },
                { "label": "حركة دائنة", "numeric": true, "width": 1.2 },
                { "label": "رصيد مدين", "numeric": true, "width": 1.2 },
                { "label": "رصيد دائن", "numeric": true, "width": 1.2 },
            ], "rows": rows },
        ],
    })
}

fn balance_sheet() -> Value {
    let cols = json!([
        { "label": "الرمز", "dim": true, "width": 0.7 },
        { "label": "الحساب", "width": 3 },
        { "label": "المبلغ", "numeric": true, "width": 1.3 },
    ]);
    json!({
        "title": "الميزانية العمومية",
        "subtitle": "المركز المالي في تاريخ محدد",
        "badge": "تقرير رسمي",
        "company": company(),
        "meta": [{ "label": "كما في", "value": "24/09/2026" }, { "label": "تاريخ الإصدار", "value": "24/09/2026 10:30" }, { "label": "العملة", "value": "SAR" }, { "label": "الميزان", "value": "متوازن" }],
        "orientation": "portrait",
        "issuedAt": "24/09/2026 10:30",
        "issuedIso": "2026-09-24T10:30:00.000Z",
        "footerNote": "هذا التقرير صادر من النظام آلياً ومخصص للاستخدام الرسمي",
        "signatures": ["المحاسب", "المدير المالي"],
        "blocks": [
            { "type": "boxes", "items": [
                { "title": "إجمالي الأصول", "value": "410,495.68" },
                { "title": "إجمالي الالتزامات", "value": "72,076.95" },
                { "title": "حقوق الملكية", "value": "338,418.73", "sub": "شاملة صافي ربح الفترة", "emphasis": true },
            ]},
            { "type": "columns", "columns": [
                [{ "type": "table", "columns": cols, "rows": [
                    row(&["الأصول"], "section"),
                    row(&["1110", "الصندوق الرئيسي", "25,340.00"], "normal"),
                    row(&["1120", "البنك الأهلي", "180,200.50"], "normal"),
                    row(&["1210", "العملاء", "45,000.00"], "normal"),
                    row(&["1310", "المخزون", "159,955.18"], "normal"),
                    row(&["", "إجمالي الأصول", "410,495.68"], "grand"),
                ]}],
                [{ "type": "table", "columns": cols, "rows": [
                    row(&["الالتزامات"], "section"),
                    row(&["2110", "الموردون", "52,076.95"], "normal"),
                    row(&["2210", "ضريبة المخرجات", "20,000.00"], "normal"),
                    row(&["", "إجمالي الالتزامات", "72,076.95"], "subtotal"),
                    row(&["حقوق الملكية"], "section"),
                    row(&["3100", "رأس المال", "300,000.00"], "normal"),
                    row(&["3300", "صافي ربح الفترة (غير مُقفل)", "-1,581.27"], "opening"),
                    row(&["", "إجمالي حقوق الملكية", "338,418.73"], "subtotal"),
                    row(&["", "الالتزامات + حقوق الملكية", "410,495.68"], "grand"),
                ]}],
            ]},
            { "type": "heading", "text": "بيانات إضافية" },
            { "type": "banner", "title": "1110 — الصندوق الرئيسي", "subtitle": "حساب نقدية", "tag": "الجانب الطبيعي: مدين" },
            { "type": "note", "text": "الميزانية غير متوازنة — راجع القيود", "tone": "warn" },
            { "type": "table", "columns": cols, "rows": [], "emptyText": "لا توجد بيانات لهذه الفترة" },
        ],
    })
}

fn render(label: &str, report: Value) -> usize {
    let payload = json!({
        "document": { "kind": "report", "number": "", "date": "24/09/2026", "titleAr": report["title"], "titleEn": "" },
        "company": company(), "party": null, "lines": [], "totals": {}, "qr": null, "logo": null,
        "report": report,
    });
    let options = json!({ "paper": "a4" });
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let _ = std::fs::create_dir_all(&out_dir);

    println!("=== {label} ===");
    let req = RenderRequest { template_source: None, template_id: Some("report".into()), payload: payload.clone(), options: options.clone() };
    let mut failures = 0;
    match render_pdf(req) {
        Ok(r) => {
            println!("  OK pdf: standard={}, warnings={}", r.achieved_standard, r.warnings.len());
            for w in &r.warnings {
                println!("    warn: {}", w.message);
            }
        }
        Err(d) => {
            println!("  FAIL pdf: {d:?}");
            failures += 1;
        }
    }
    let req = RenderRequest { template_source: None, template_id: Some("report".into()), payload, options };
    match render_preview(req) {
        Ok(r) => {
            for (i, svg) in r.pages.iter().enumerate() {
                let p = out_dir.join(format!("report_smoke_{label}_{}.svg", i + 1));
                std::fs::write(&p, svg).expect("write svg");
                println!("  wrote {}", p.display());
            }
        }
        Err(d) => {
            println!("  FAIL preview: {d:?}");
            failures += 1;
        }
    }
    failures
}

fn main() {
    let mut failures = 0;
    // `cargo run --bin report_smoke -- a.json b.json` renders real `ReportDocument`s captured from
    // the app (e.g. by the e2e print check) instead of the built-in samples.
    let files: Vec<String> = std::env::args().skip(1).collect();
    if !files.is_empty() {
        for f in files {
            let text = std::fs::read_to_string(&f).expect("read report json");
            let report: Value = serde_json::from_str(&text).expect("parse report json");
            let label = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
            failures += render(&label, report);
        }
        println!("\n=== report_smoke: {failures} failure(s) ===");
        if failures > 0 {
            std::process::exit(1);
        }
        return;
    }
    failures += render("trial_balance", trial_balance("portrait"));
    failures += render("trial_balance_landscape", trial_balance("landscape"));
    failures += render("balance_sheet", balance_sheet());
    println!("\n=== report_smoke: {failures} failure(s) ===");
    if failures > 0 {
        std::process::exit(1);
    }
}
