//! Request/settings shapes for the thermal print commands. Mirrors
//! `pdf::payload`'s "loose on purpose" approach: the receipt `DocumentPayload`
//! itself is the *same* JSON shape Phase 11a's `pdfService.ts` already builds
//! (`document`, `company`, `party`, `lines`, `totals`, `qr`, `logo`), so it's
//! passed through as a raw `serde_json::Value` rather than re-declaring a
//! parallel struct — the Typst template is the single source of truth for
//! which fields it reads.

use serde::{Deserialize, Serialize};

/// 58mm → 384px, 80mm → 576px at 203dpi, per docs/v2/12-documents-pdf-excel.md
/// §5. These are the printable-area dot counts real thermal printers use
/// (not the raw paper width): an "80mm" roll's printable width is ~72mm
/// (576 / 203 * 25.4 ≈ 72mm), and a "58mm" roll's is ~48mm — the rest is
/// unprintable margin the print head physically can't reach. The Typst page
/// width is set to that same printable width (see `receipt_thermal.typ`) so
/// the rendered pixel width always lands exactly on these dot counts, with
/// no rounding mismatch between the render step and the ESC/POS header.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReceiptWidth {
    #[serde(rename = "58mm")]
    Mm58,
    #[serde(rename = "80mm")]
    Mm80,
}

impl ReceiptWidth {
    /// Nominal paper roll width in mm (for the settings UI label only).
    pub fn mm(self) -> f32 {
        match self {
            ReceiptWidth::Mm58 => 58.0,
            ReceiptWidth::Mm80 => 80.0,
        }
    }

    /// Printable width in mm — what the Typst page is actually sized to.
    pub fn printable_mm(self) -> f32 {
        match self {
            ReceiptWidth::Mm58 => 48.0,
            ReceiptWidth::Mm80 => 72.0,
        }
    }

    /// Dot width at 203dpi (the standard thermal-head resolution): fixed at
    /// 384/576 per §5, rather than derived, so it's exact regardless of
    /// floating-point mm->dot rounding.
    pub fn dots_203dpi(self) -> u32 {
        match self {
            ReceiptWidth::Mm58 => 384,
            ReceiptWidth::Mm80 => 576,
        }
    }

    /// Dot width at an arbitrary DPI, scaled from the 203dpi reference and
    /// rounded to a multiple of 8 as ESC/POS `GS v 0` requires (width is sent
    /// in whole bytes).
    pub fn dots_at_dpi(self, dpi: u32) -> u32 {
        let raw = self.dots_203dpi() as f32 * dpi as f32 / 203.0;
        ((raw / 8.0).round() as u32).max(1) * 8
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionType {
    /// Windows printer queue (USB or driver-installed), raw-mode spooling.
    Windows,
    /// Raw TCP to `host:9100`.
    Network,
}

/// Settings → الطابعات (printer settings page) persisted shape, mirrored on
/// the TS side by `modules/settings/types`'s `PrinterSettings`. Kept here
/// (rather than in `pdf::payload`) since it's specific to the thermal
/// transport, not the render pipeline.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThermalPrinterConfig {
    /// Windows queue name (from `list_printers`) — used when `connection ==
    /// Windows`.
    #[serde(default)]
    pub printer_name: Option<String>,
    pub connection: ConnectionType,
    /// Network host/IP — used when `connection == Network`. Port is always
    /// 9100 per §5 ("a network printer (IP:9100)").
    #[serde(default)]
    pub host: Option<String>,
    pub width: ReceiptWidth,
    #[serde(default = "default_dpi")]
    pub dpi: u32,
    #[serde(default = "default_true")]
    pub cut: bool,
    #[serde(default)]
    pub open_drawer: bool,
    #[serde(default = "default_copies")]
    pub copies: u32,
}

fn default_dpi() -> u32 {
    203
}
fn default_true() -> bool {
    true
}
fn default_copies() -> u32 {
    1
}

/// Request body for `print_thermal_receipt` / `print_test_receipt`.
#[derive(Debug, Clone, Deserialize)]
pub struct PrintReceiptRequest {
    /// The `DocumentPayload` JSON (same shape as `pdf::payload::DocumentPayload`).
    pub payload: serde_json::Value,
    pub printer: ThermalPrinterConfig,
    /// Optional extra options passed to the Typst template's `opts.json`
    /// (currently just `receiptFooter`); merged with the paper size derived
    /// from `printer.width`.
    #[serde(default)]
    pub options: serde_json::Value,
}
