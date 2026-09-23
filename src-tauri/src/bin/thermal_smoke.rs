//! Phase 14 verification binary: exercises the full render -> dither ->
//! ESC/POS pipeline directly (mirrors `pdf_smoke`'s "call the Rust functions
//! the IPC layer calls, directly" approach, since there's no windowing system
//! here to drive `bun run tauri dev` interactively against a real printer).
//!
//! `cargo run --bin thermal_smoke` renders a realistic receipt at both 80mm
//! and 58mm, dithers it, packs it into an ESC/POS `GS v 0` raster job (with
//! cut + drawer kick), writes the raw byte stream to
//! `src-tauri/target/thermal_smoke_80mm.bin` / `..._58mm.bin`, and
//! structurally verifies:
//! - the `GS v 0` header's `xL/xH` (width in bytes) and `yL/yH` (height in
//!   dots) exactly match the actual bitmap dimensions that follow,
//! - the raster payload length equals `bytes_per_row * height`,
//! - a `GS V` cut command and an `ESC p` drawer-kick command are present,
//!   in that order, after the raster data.
//!
//! It also calls `list_printers` and reports whether this environment has
//! any real installed Windows printers (this is a CI/dev sandbox, so an
//! empty result there is expected and reported as such, not a failure).

use accounting_app_lib::print::commands::list_printers;
use accounting_app_lib::print::payload::{ConnectionType, PrintReceiptRequest, ReceiptWidth, ThermalPrinterConfig};
use accounting_app_lib::print::escpos;
use accounting_app_lib::print::render::render_receipt_bitmap;

fn realistic_receipt_payload() -> serde_json::Value {
    serde_json::json!({
        "document": {
            "kind": "invoice",
            "number": "INV-000512",
            "date": "2026-09-23 14:30",
            "titleAr": "فاتورة ضريبية مبسطة",
            "titleEn": "SIMPLIFIED TAX INVOICE",
        },
        "company": {
            "name": "مؤسسة الفاتورة النموذجية للتجارة",
            "address": "الرياض، حي العليا",
            "phone": "0112345678",
            "email": null, "website": null,
            "vatNumber": "311111111100003",
            "commercialRegister": "1010123456",
            "logo": null,
        },
        "party": null,
        "lines": [
            { "name": "جهاز تكييف هواء منزلي بقدرة ثمانية عشر ألف وحدة", "qty": "١", "price": "1,899.00", "net": "1899.00", "vatRate": 15, "vat": "284.85", "total": "2,183.85" },
            { "name": "Widget X200", "qty": "٥", "price": "45.00", "net": "215.00", "vatRate": 15, "vat": "32.25", "total": "247.25" },
        ],
        "totals": {
            "subtotal": "2,114.00", "discount": null, "vat": "317.10", "grand": "2,431.10",
            "paid": "2,431.10", "remaining": "0.00",
            "amountInWords": "فقط لا غير: ألفان وأربعمائة وواحد وثلاثون ريالاً سعودياً وعشرة هللات",
            "previousBalance": null, "currentBalance": null,
        },
        "qr": "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 21 21\" width=\"28mm\" height=\"28mm\"><rect width=\"21\" height=\"21\" fill=\"#fff\"/><path d=\"M0,0h1v1h-1zM2,2h1v1h-1z\" fill=\"#000\"/></svg>",
        "logo": null,
        "sample": true,
    })
}

fn check_escpos_structure(job: &[u8], expected_bytes_per_row: u32, expected_height: u32, expect_cut: bool, expect_drawer: bool) -> Result<(), String> {
    // init: ESC @ (0x1B 0x40)
    if job.len() < 2 || job[0] != 0x1B || job[1] != 0x40 {
        return Err("job does not start with ESC @ (init)".to_string());
    }
    // raster header starts right after init: GS v 0 m xL xH yL yH
    let hdr = &job[2..];
    if hdr.len() < 8 || hdr[0] != 0x1D || hdr[1] != b'v' || hdr[2] != b'0' {
        return Err("raster command header (GS v 0) not found where expected".to_string());
    }
    let x = u16::from_le_bytes([hdr[4], hdr[5]]) as u32;
    let y = u16::from_le_bytes([hdr[6], hdr[7]]) as u32;
    if x != expected_bytes_per_row {
        return Err(format!("raster width header = {x} bytes/row, expected {expected_bytes_per_row}"));
    }
    if y != expected_height {
        return Err(format!("raster height header = {y} dots, expected {expected_height}"));
    }
    let data_len = (x * y) as usize;
    let data_end = 2 + 8 + data_len;
    if job.len() < data_end {
        return Err(format!("job is {} bytes, too short for header-declared {data_len}-byte payload", job.len()));
    }

    let rest = &job[data_end..];
    let has_cut = rest.windows(4).any(|w| w == [0x1D, b'V', 66, 0]);
    let has_drawer = rest.windows(5).any(|w| w == [0x1B, b'p', 0x00, 25, 250]);
    if expect_cut && !has_cut {
        return Err("expected a GS V cut command after the raster data, none found".to_string());
    }
    if expect_drawer && !has_drawer {
        return Err("expected an ESC p drawer-kick command, none found".to_string());
    }
    if expect_cut && expect_drawer {
        let cut_pos = rest.windows(4).position(|w| w == [0x1D, b'V', 66, 0]).unwrap();
        let drawer_pos = rest.windows(5).position(|w| w == [0x1B, b'p', 0x00, 25, 250]).unwrap();
        if drawer_pos < cut_pos {
            return Err("drawer kick appears before cut; expected cut then drawer".to_string());
        }
    }
    Ok(())
}

fn run_width(width: ReceiptWidth, label: &str, failures: &mut u32) {
    println!("=== render+dither+ESC/POS: {label} ===");
    let payload = realistic_receipt_payload();
    let bitmap = match render_receipt_bitmap(&payload, width, 203, &serde_json::json!({})) {
        Ok(b) => b,
        Err(e) => {
            println!("  FAIL: render_receipt_bitmap: {e}");
            *failures += 1;
            return;
        }
    };
    println!("  bitmap: {}x{} px, {} bytes/row, {} total bytes", bitmap.width, bitmap.height, bitmap.bytes_per_row, bitmap.bits.len());

    let expected_dots = match width {
        ReceiptWidth::Mm80 => 576,
        ReceiptWidth::Mm58 => 384,
    };
    if bitmap.width != expected_dots {
        println!("  FAIL: bitmap width {} != expected {expected_dots} dots for {label}", bitmap.width);
        *failures += 1;
    } else {
        println!("  OK: bitmap width matches the printer's dot spec ({expected_dots} dots)");
    }

    // Dump a viewable PNG of the dithered bitmap too (not part of the
    // required byte-stream check, just a human sanity-check aid).
    {
        let mut img = image::GrayImage::new(bitmap.width, bitmap.height);
        for y in 0..bitmap.height {
            for x in 0..bitmap.width {
                let byte = bitmap.bits[(y * bitmap.bytes_per_row + x / 8) as usize];
                let bit = (byte >> (7 - (x % 8))) & 1;
                img.put_pixel(x, y, image::Luma([if bit == 1 { 0u8 } else { 255u8 }]));
            }
        }
        let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        let png_path = out_dir.join(format!("thermal_smoke_{label}_dithered.png"));
        img.save(&png_path).expect("write dithered png");
        println!("  wrote {} (visual check)", png_path.display());
    }

    let job = escpos::build_receipt_job(&bitmap, true, true, 1);
    match check_escpos_structure(&job, bitmap.bytes_per_row, bitmap.height, true, true) {
        Ok(()) => println!("  OK: ESC/POS structure valid (GS v 0 header matches bitmap dims, GS V cut + ESC p drawer kick present in order)"),
        Err(e) => {
            println!("  FAIL: {e}");
            *failures += 1;
        }
    }

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let _ = std::fs::create_dir_all(&out_dir);
    let out_path = out_dir.join(format!("thermal_smoke_{label}.bin"));
    std::fs::write(&out_path, &job).expect("write escpos job");
    println!("  wrote {} ({} bytes)", out_path.display(), job.len());
    println!();
}

fn main() {
    let mut failures = 0u32;

    run_width(ReceiptWidth::Mm80, "80mm", &mut failures);
    run_width(ReceiptWidth::Mm58, "58mm", &mut failures);

    println!("=== list_printers ===");
    match list_printers() {
        Ok(printers) if printers.is_empty() => {
            println!("  No printers enumerated in this environment (expected in a headless/CI/dev sandbox — this is not treated as a failure).");
        }
        Ok(printers) => {
            println!("  Found {} real installed printer(s):", printers.len());
            for p in &printers {
                println!("    - {}{}", p.name, if p.is_default { " (default)" } else { "" });
            }
        }
        Err(e) => {
            println!("  list_printers error: {e} (non-Windows target, or spooler unavailable — not a pipeline failure)");
        }
    }

    // Sanity-check that a Windows/network print of an unconfigured printer
    // fails cleanly with a message rather than panicking (no real printer
    // required for this check).
    println!("\n=== run_print_job error path (no printer configured) ===");
    let bad_req = PrintReceiptRequest {
        payload: realistic_receipt_payload(),
        printer: ThermalPrinterConfig {
            printer_name: None,
            connection: ConnectionType::Windows,
            host: None,
            width: ReceiptWidth::Mm80,
            dpi: 203,
            cut: true,
            open_drawer: false,
            copies: 1,
        },
        options: serde_json::json!({}),
    };
    match accounting_app_lib::print::commands::run_print_job(&bad_req) {
        Ok(()) => {
            println!("  FAIL: expected an error (no printer_name set)");
            failures += 1;
        }
        Err(e) => println!("  OK: cleanly rejected with: {e}"),
    }

    println!("\n=== thermal_smoke summary: {failures} failure(s) ===");
    if failures > 0 {
        std::process::exit(1);
    }
}
