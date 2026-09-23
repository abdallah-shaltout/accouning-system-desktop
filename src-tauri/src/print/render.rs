//! Renders a receipt `DocumentPayload` through Typst → PNG pixels → 1-bit
//! dithered bitmap, reusing Phase 11a's `RenderWorld` and embedded fonts
//! (docs/v2/12-documents-pdf-excel.md §5: "thermal printing renders through
//! Typst too").

use typst_layout::PagedDocument;

use crate::pdf::{fonts, raster, RenderWorld};

use super::dither::{dither_floyd_steinberg, Bitmap1Bit};
use super::payload::ReceiptWidth;

const LIB_TYP: &str = include_str!("../../templates/lib.typ");
const RECEIPT_THERMAL_TYP: &str = include_str!("../../templates/receipt_thermal.typ");

/// Builds the Typst world for a receipt render. `payload` is the raw
/// `DocumentPayload` JSON (same shape `pdfService.ts` builds); `extra_options`
/// merges into `opts.json` (currently just `receiptFooter`), with `paper` set
/// from `width` so `lib.typ`'s `page-typst-size`/`page-margin` pick the right
/// dot-for-dot page size.
fn build_world(payload: &serde_json::Value, width: ReceiptWidth, extra_options: &serde_json::Value) -> Result<RenderWorld, String> {
    let mut opts = extra_options.clone();
    if !opts.is_object() {
        opts = serde_json::json!({});
    }
    opts["paper"] = serde_json::Value::String(match width {
        ReceiptWidth::Mm58 => "58mm".to_string(),
        ReceiptWidth::Mm80 => "80mm".to_string(),
    });
    // The Typst page is sized to the printable width (not the nominal paper
    // width) so the rasterized pixel width exactly matches `dots_203dpi()`
    // (see `ReceiptWidth`'s doc comment) — no rounding mismatch between this
    // render step and the ESC/POS raster header.
    opts["printableWidthMm"] = serde_json::json!(width.printable_mm());
    // Receipts use a fixed compact size regardless of the A4 template's
    // fontSize option — the thermal template hardcodes its own sizes, but
    // `lib.typ::font-family` still reads `opts.fontFamily`.
    if opts.get("fontFamily").is_none() {
        opts["fontFamily"] = serde_json::Value::String("Cairo".to_string());
    }

    let data_json = serde_json::to_string(payload).map_err(|e| format!("invalid payload JSON: {e}"))?;
    let opts_json = serde_json::to_string(&opts).map_err(|e| format!("invalid options JSON: {e}"))?;

    let logo_png = match payload.get("logo").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => Some(decode_data_url_or_base64(s)?),
        _ => None,
    };
    let qr_svg = payload.get("qr").and_then(|v| v.as_str()).map(|s| s.to_string());

    RenderWorld::new(
        RECEIPT_THERMAL_TYP.to_string(),
        LIB_TYP.to_string(),
        data_json,
        opts_json,
        logo_png,
        qr_svg,
        fonts::all_fonts(),
    )
}

fn decode_data_url_or_base64(s: &str) -> Result<Vec<u8>, String> {
    let b64 = s.split_once(",").map(|(_, data)| data).unwrap_or(s);
    let clean: Vec<u8> = b64.trim().bytes().filter(|&b| b != b'=' && !b.is_ascii_whitespace()).collect();
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut table = [255u8; 256];
    for (i, &c) in ALPHABET.iter().enumerate() {
        table[c as usize] = i as u8;
    }
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

/// Compiles the receipt and dithers the first page to a 1-bit bitmap at the
/// given DPI, clamped/padded to the printer's dot width (58mm=384px,
/// 80mm=576px at 203dpi — other DPIs scale proportionally).
///
/// Only the first page is used: a receipt is always laid out with
/// `height: auto` (see `lib.typ::page-typst-size`), so a well-formed receipt
/// template produces exactly one (tall) page. If a caller's template somehow
/// produces more, only the first is printed rather than silently
/// concatenating pages, which would risk printing garbage past the dot-width
/// mismatch of later pages.
pub fn render_receipt_bitmap(
    payload: &serde_json::Value,
    width: ReceiptWidth,
    dpi: u32,
    extra_options: &serde_json::Value,
) -> Result<Bitmap1Bit, String> {
    let world = build_world(payload, width, extra_options)?;

    let warned = typst::compile::<PagedDocument>(&world);
    let document = warned.output.map_err(|diags| {
        diags.iter().map(|d| d.message.to_string()).collect::<Vec<_>>().join("; ")
    })?;

    if document.pages().is_empty() {
        return Err("receipt template produced no pages".to_string());
    }

    let page = raster::rasterize_page(&document, 0, dpi as f32)?;

    // The rasterizer scales the Typst page (already sized to `width` in
    // `lib.typ`) to `dpi`; round the width to the expected dot count so
    // rounding error from the mm→px conversion can't desync the printer's
    // per-line byte count from what the paper roll actually is.
    let expected_dots = target_dots(width, dpi);
    let final_bitmap = if page.width == expected_dots {
        dither_floyd_steinberg(&page.rgba, page.width, page.height)
    } else {
        // Re-pad/crop to the expected width so `GS v 0`'s xL/xH always
        // matches a byte-aligned printer-native width.
        resample_width(&page, expected_dots)
    };

    Ok(final_bitmap)
}

fn target_dots(width: ReceiptWidth, dpi: u32) -> u32 {
    width.dots_at_dpi(dpi)
}

/// Pads or crops a rasterized page's RGBA buffer to `target_width` (keeping
/// height and left/right content position) before dithering, for the rare
/// case where DPI-scaling rounding doesn't land exactly on the expected
/// byte-aligned dot width.
fn resample_width(page: &raster::RasterPage, target_width: u32) -> Bitmap1Bit {
    let src_w = page.width as usize;
    let dst_w = target_width as usize;
    let h = page.height as usize;
    let mut out = vec![255u8; dst_w * h * 4];
    let copy_w = src_w.min(dst_w);
    for y in 0..h {
        let src_row = &page.rgba[y * src_w * 4..y * src_w * 4 + copy_w * 4];
        let dst_start = y * dst_w * 4;
        out[dst_start..dst_start + copy_w * 4].copy_from_slice(src_row);
    }
    dither_floyd_steinberg(&out, target_width, page.height)
}
