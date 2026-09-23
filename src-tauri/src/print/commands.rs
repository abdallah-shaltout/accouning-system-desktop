//! Tauri commands for native thermal printing.
//!
//! `print_thermal_receipt` is async (`async fn` + returns immediately after
//! spawning the actual render+print work) so the calling Vue code never
//! blocks the POS on a slow/offline printer (docs/v2/12-documents-pdf-excel.md
//! §5: "printing is asynchronous and never blocks the next sale"). Progress
//! is reported back via a Tauri event (`print://receipt-result`) rather than
//! the command's return value, since the command itself returns before the
//! print finishes — `printService.ts` listens for that event to drive the
//! toast + reprint UX.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::escpos;
use super::payload::{ConnectionType, PrintReceiptRequest};
use super::printers::{self, PrinterInfo};
use super::render::render_receipt_bitmap;
use super::transport;

#[tauri::command]
pub fn list_printers() -> Result<Vec<PrinterInfo>, String> {
    printers::enumerate()
}

#[derive(Debug, Clone, Serialize)]
pub struct PrintReceiptResult {
    pub job_id: String,
    pub ok: bool,
    pub error: Option<String>,
}

/// Runs the full render → dither → ESC/POS → transport pipeline synchronously
/// (used by both the spawned async job and the smoke-test binary).
pub fn run_print_job(req: &PrintReceiptRequest) -> Result<(), String> {
    let bitmap = render_receipt_bitmap(&req.payload, req.printer.width, req.printer.dpi, &req.options)?;
    let job_bytes = escpos::build_receipt_job(&bitmap, req.printer.cut, req.printer.open_drawer, req.printer.copies);

    match req.printer.connection {
        ConnectionType::Windows => {
            let name = req
                .printer
                .printer_name
                .as_deref()
                .ok_or_else(|| "no Windows printer selected".to_string())?;
            transport::print_windows_raw(name, &job_bytes, "Receipt")
        }
        ConnectionType::Network => {
            let host = req.printer.host.as_deref().ok_or_else(|| "no printer IP configured".to_string())?;
            transport::print_network_raw(host, &job_bytes)
        }
    }
}

/// Fire-and-forget print: spawns the job on Tauri's async runtime and returns
/// a `job_id` immediately. Result (success or failure) is emitted later as a
/// `print://receipt-result` event carrying `{ job_id, ok, error }`, which
/// `printService.ts` listens for to show a toast (and, on failure, a
/// "إعادة الطباعة" reprint action + PDF fallback offer).
#[tauri::command]
pub async fn print_thermal_receipt(app: AppHandle, req: PrintReceiptRequest, job_id: String) -> Result<String, String> {
    let job_id_for_task = job_id.clone();
    tauri::async_runtime::spawn(async move {
        let result = run_print_job(&req);
        let payload = match result {
            Ok(()) => PrintReceiptResult { job_id: job_id_for_task.clone(), ok: true, error: None },
            Err(e) => PrintReceiptResult { job_id: job_id_for_task.clone(), ok: false, error: Some(e) },
        };
        let _ = app.emit("print://receipt-result", payload);
    });
    Ok(job_id)
}

/// Test print: same pipeline, but synchronous (the settings page's "اختبار
/// الطباعة" button wants an immediate ok/err rather than an event round trip,
/// since it's a deliberate one-off action, not something that must not block
/// the UI the way a POS sale is).
#[tauri::command]
pub fn print_test_receipt(req: PrintReceiptRequest) -> Result<(), String> {
    run_print_job(&req)
}
