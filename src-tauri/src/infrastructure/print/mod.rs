//! Native thermal printing (Phase 14, docs/v2/12-documents-pdf-excel.md §5).
//!
//! Pipeline: a receipt `DocumentPayload` (Phase 11a's shape, reused as-is) is
//! rendered through the same Typst world as the PDF engine, using the new
//! `receipt_thermal.typ` template sized for 80mm/576px or 58mm/384px at
//! 203dpi → rasterized to RGBA via `pdf::raster` → dithered to 1-bit
//! (Floyd–Steinberg) → packed into ESC/POS `GS v 0` raster command bytes →
//! optionally followed by a paper-cut (`GS V`) and/or cash-drawer kick
//! (`ESC p`) → sent over one of two transports: a Windows printer queue in
//! raw mode, or a raw TCP socket to `IP:9100`.
//!
//! This module only touches `src-tauri/src/print/` (new) and adds one new
//! file to `src-tauri/src/pdf/` (`raster.rs`) — see that file's doc comment.
//! It does not modify any existing `pdf/` file.

pub mod dither;
pub mod escpos;
pub mod payload;
pub mod printers;
pub mod render;
mod transport;

pub mod commands;

pub use commands::{list_printers, print_test_receipt, print_thermal_receipt};
