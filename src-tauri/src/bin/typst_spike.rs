//! Standalone runner for the Typst PDF spike (Phase 0 gate).
//!
//! `cargo run --bin typst_spike` — renders the sample invoice document N
//! times (to get a stable timing measurement), writes the last PDF to
//! `src-tauri/target/typst_spike_output.pdf`, and prints:
//! - per-run wall-clock timings (compile / export / total)
//! - which PDF standard was actually achieved (PDF/A-3b, or the fallback)
//! - a structural sanity check on the produced PDF bytes (page count via the
//!   `%%EOF`/`/Type /Page` scan — see `sanity_check` below)

use accounting_app_lib::pdf::{render_spike, SpikeDocument};

fn main() {
    let doc = SpikeDocument::default();
    let runs = 5;
    let mut totals = Vec::with_capacity(runs);
    let mut last_pdf: Vec<u8> = Vec::new();
    let mut achieved = "";
    let mut warnings: Vec<String> = Vec::new();

    for i in 0..runs {
        match render_spike(&doc) {
            Ok(result) => {
                println!(
                    "run {:>2}: compile={:>7.2}ms  export={:>7.2}ms  total={:>7.2}ms  standard={}",
                    i + 1,
                    result.compile_ms,
                    result.export_ms,
                    result.total_ms,
                    result.achieved_standard
                );
                totals.push(result.total_ms);
                last_pdf = result.pdf_bytes;
                achieved = result.achieved_standard;
                warnings = result.warnings;
            }
            Err(e) => {
                eprintln!("render failed on run {}: {e}", i + 1);
                std::process::exit(1);
            }
        }
    }

    let avg = totals.iter().sum::<f64>() / totals.len() as f64;
    let min = totals.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = totals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    println!(
        "\n{} runs: avg={:.2}ms min={:.2}ms max={:.2}ms  (<200ms target)",
        runs, avg, min, max
    );
    println!("Achieved PDF standard: {achieved}");
    if !warnings.is_empty() {
        println!("Compiler warnings ({}):", warnings.len());
        for w in &warnings {
            println!("  - {w}");
        }
    }

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let _ = std::fs::create_dir_all(&out_dir);
    let out_path = out_dir.join("typst_spike_output.pdf");
    std::fs::write(&out_path, &last_pdf).expect("failed to write output PDF");
    println!("\nWrote {} bytes to {}", last_pdf.len(), out_path.display());

    match sanity_check(&last_pdf) {
        Ok(pages) => println!(
            "Structural sanity check (lopdf parse): PDF opened successfully, {pages} page(s) found."
        ),
        Err(e) => {
            eprintln!("Structural sanity check FAILED: {e}");
            std::process::exit(1);
        }
    }
}

/// A real structural check using `lopdf` (an independent PDF parser, not
/// Typst's own writer): loads the document from bytes and counts its pages
/// via the page tree. This is the closest this environment can get to "opens
/// in a PDF viewer" without a scriptable Acrobat/Edge available — it proves
/// the byte stream is a well-formed PDF with a valid xref/trailer/page tree,
/// which is what a viewer needs to render it.
fn sanity_check(bytes: &[u8]) -> Result<usize, String> {
    let doc = lopdf::Document::load_mem(bytes).map_err(|e| format!("lopdf failed to parse: {e}"))?;
    let pages = doc.get_pages();
    if pages.is_empty() {
        return Err("lopdf parsed the document but found zero pages".to_string());
    }
    Ok(pages.len())
}
