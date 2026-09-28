//! Shared PNG rasterization helper (Phase 14, docs/v2/12-documents-pdf-excel.md
//! §5: "Typst → PNG at the printer's resolution ... → 1-bit dithering →
//! ESC/POS raster").
//!
//! Added as a **new file** rather than editing `render.rs`/`world.rs`, per
//! Phase 14's scope constraints (another agent is concurrently touching other
//! parts of the tree; this module is additive only). It's used by
//! `src-tauri/src/print/render.rs` to turn a compiled Typst page into raw
//! RGBA pixels at a specific DPI, which the thermal pipeline then dithers to
//! 1-bit and packs into ESC/POS raster bytes.
//!
//! **Why SVG → resvg → tiny-skia instead of a hypothetical `typst_render`
//! crate:** this codebase's Typst version (0.15.1) exposes `typst-svg` for
//! vector export (already used by `render::render_preview`) but not a raster
//! crate. `resvg` + `tiny-skia` are already present in `Cargo.lock` (pulled
//! in transitively), so rendering the same SVG output through them adds no
//! new supply-chain surface — just promotes an existing transitive
//! dependency to a direct one.

use typst_layout::PagedDocument;

/// A rasterized page: raw RGBA8 pixels, width/height in pixels.
pub struct RasterPage {
    pub width: u32,
    pub height: u32,
    /// Tightly packed RGBA8, row-major, top-to-bottom.
    pub rgba: Vec<u8>,
}

/// Renders one page of a compiled Typst document to RGBA8 pixels at the given
/// DPI (thermal printers commonly use 203 dpi; Typst's own unit is points,
/// 72pt = 1in).
pub fn rasterize_page(document: &PagedDocument, page_index: usize, dpi: f32) -> Result<RasterPage, String> {
    let page = document
        .pages()
        .get(page_index)
        .ok_or_else(|| format!("page index {page_index} out of range ({} pages)", document.pages().len()))?;

    let svg_opts = typst_svg::SvgOptions::default();
    let svg = typst_svg::svg(page, &svg_opts);

    let scale = dpi / 72.0; // Typst page dimensions are in points (1pt = 1/72in).
    render_svg_to_rgba(&svg, scale)
}

/// Renders arbitrary SVG markup (used for pages and, in principle, any other
/// SVG the print pipeline needs) to RGBA8 pixels at the given scale factor.
fn render_svg_to_rgba(svg: &str, scale: f32) -> Result<RasterPage, String> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| format!("SVG parse failed: {e}"))?;

    let size = tree.size();
    let px_width = (size.width() * scale).round().max(1.0) as u32;
    let px_height = (size.height() * scale).round().max(1.0) as u32;

    let mut pixmap = tiny_skia::Pixmap::new(px_width, px_height)
        .ok_or_else(|| format!("could not allocate {px_width}x{px_height} pixmap"))?;
    // White background — thermal receipts print on white paper and the
    // dithering step downstream expects a filled canvas, not transparency.
    pixmap.fill(tiny_skia::Color::WHITE);

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Ok(RasterPage { width: px_width, height: px_height, rgba: pixmap.data().to_vec() })
}
