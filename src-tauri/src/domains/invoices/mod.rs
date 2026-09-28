//! `invoices` domain (03-domains/08-invoices.md, 08b-pos-shifts.md) — sales, refunds, quotations,
//! invoice reads/print data (13 commands) + POS shifts, cash in/out, X/Z report, held sales
//! (12 commands). 25 commands total, one domain.
//!
//! **Needs from manager** (blocking `cargo check` for this module — see this wave's final report):
//! `pub mod invoices;` + `domains::invoices::ipc_signatures()`/`export_bindings()` hooks in
//! `domains/mod.rs`, the matching `domains::invoices::commands::*` lines in `lib.rs`'s
//! `generate_handler!`. Depends on `domains::payments` (`Payment` DTO + `payments_for_invoice`,
//! already wired), `domains::parties` (`Customer` DTO + `get_customer`, already wired),
//! `domains::vouchers::service::general::record_transfer_voucher` (already wired), and
//! `domains::settings::service::store::get_settings` (already wired) — all present in this wave.

pub mod commands;
pub mod dto;
pub mod service;

use crate::core::ipc::IpcSig;
use crate::ipc_sig;

/// 25 commands, 08 §1 (13) + 08b §1 (12), table order.
pub fn ipc_signatures() -> Vec<IpcSig> {
    use dto as d;

    vec![
        // 08 reads/writes
        ipc_sig!(invoices_get_invoices, d::InvoicesGetInvoicesArgs, Vec<d::InvoiceRow>),
        ipc_sig!(invoices_get_invoices_paged, d::InvoicesGetInvoicesPagedArgs, crate::core::dto::PagedResult<d::InvoiceRow>),
        ipc_sig!(invoices_get_invoice, d::InvoicesGetInvoiceArgs, d::InvoiceDetail),
        ipc_sig!(invoices_preview_sale, d::InvoicesPreviewSaleArgs, Vec<d::JournalPreviewLine>),
        ipc_sig!(invoices_create_sale, d::InvoicesCreateSaleArgs, d::Invoice),
        ipc_sig!(invoices_create_refund, d::InvoicesCreateRefundArgs, d::Refund),
        ipc_sig!(invoices_get_refund, d::InvoicesGetRefundArgs, d::Refund),
        ipc_sig!(invoices_get_invoice_print_data, d::InvoicesGetInvoicePrintDataArgs, d::PrintData),
        ipc_sig!(invoices_get_quotations, d::InvoicesGetQuotationsArgs, Vec<d::QuotationRow>),
        ipc_sig!(invoices_get_quotation, d::InvoicesGetQuotationArgs, d::QuotationRow),
        ipc_sig!(invoices_save_quotation, d::InvoicesSaveQuotationArgs, d::Quotation),
        ipc_sig!(invoices_set_quotation_status, d::InvoicesSetQuotationStatusArgs, d::Quotation),
        ipc_sig!(invoices_convert_quotation_to_invoice, d::InvoicesConvertQuotationToInvoiceArgs, d::Invoice),
        // 08b shifts/held sales
        ipc_sig!(invoices_get_current_shift, (), Option<d::ShiftRow>),
        ipc_sig!(invoices_get_shifts, d::InvoicesGetShiftsArgs, Vec<d::ShiftRow>),
        ipc_sig!(invoices_get_shift, d::InvoicesGetShiftArgs, d::ShiftRow),
        ipc_sig!(invoices_open_pos_shift, d::InvoicesOpenPosShiftArgs, d::Shift),
        ipc_sig!(invoices_get_x_report, d::InvoicesGetXReportArgs, d::ShiftRow),
        ipc_sig!(invoices_close_pos_shift, d::InvoicesClosePosShiftArgs, d::Shift),
        ipc_sig!(invoices_force_close_pos_shift, d::InvoicesForceClosePosShiftArgs, d::Shift),
        ipc_sig!(invoices_record_cash_in_out, d::InvoicesRecordCashInOutArgs, ()),
        ipc_sig!(invoices_get_held_sales, (), Vec<d::HeldSale>),
        ipc_sig!(invoices_hold_sale, d::InvoicesHoldSaleArgs, d::HeldSale),
        ipc_sig!(invoices_resume_held_sale, d::InvoicesResumeHeldSaleArgs, d::HeldSale),
        ipc_sig!(invoices_discard_held_sale, d::InvoicesDiscardHeldSaleArgs, ()),
    ]
}

/// This domain's DTO exports (08 §2, 08b §2) — one line the manager appends to
/// `domains::export_bindings` (`domains/mod.rs`) in the same commit that adds `pub mod invoices;`
/// there, mirroring `purchases::export_bindings`'s doc comment.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    use dto as d;

    d::InvoiceStatus::export_all(cfg).expect("export InvoiceStatus");
    d::PaymentStatus::export_all(cfg).expect("export invoices::PaymentStatus");
    d::SalePaymentMethod::export_all(cfg).expect("export SalePaymentMethod");
    d::InvoiceSource::export_all(cfg).expect("export InvoiceSource");
    d::InvoiceType::export_all(cfg).expect("export InvoiceType");
    d::RefundMethod::export_all(cfg).expect("export invoices::RefundMethod");
    d::QuotationStatus::export_all(cfg).expect("export QuotationStatus");
    d::InvoiceLine::export_all(cfg).expect("export InvoiceLine");
    d::Tender::export_all(cfg).expect("export Tender");
    d::Invoice::export_all(cfg).expect("export Invoice");
    d::InvoiceRow::export_all(cfg).expect("export InvoiceRow");
    d::JournalRef::export_all(cfg).expect("export invoices::JournalRef");
    d::InvoiceDetail::export_all(cfg).expect("export InvoiceDetail");
    d::JournalPreviewLine::export_all(cfg).expect("export JournalPreviewLine");
    d::RefundLine::export_all(cfg).expect("export RefundLine");
    d::Refund::export_all(cfg).expect("export Refund");
    d::SaleInputLine::export_all(cfg).expect("export SaleInputLine");
    d::SaleInput::export_all(cfg).expect("export SaleInput");
    d::RefundInput::export_all(cfg).expect("export RefundInput");
    d::InvoiceListFilter::export_all(cfg).expect("export InvoiceListFilter");
    d::Quotation::export_all(cfg).expect("export Quotation");
    d::QuotationRow::export_all(cfg).expect("export QuotationRow");
    d::QuotationInput::export_all(cfg).expect("export QuotationInput");
    d::QuotationFilter::export_all(cfg).expect("export QuotationFilter");
    d::ConvertPayment::export_all(cfg).expect("export ConvertPayment");
    d::PrintData::export_all(cfg).expect("export PrintData");
    d::ShiftStatus::export_all(cfg).expect("export ShiftStatus");
    d::HandoverMode::export_all(cfg).expect("export HandoverMode");
    d::ShiftMovementKind::export_all(cfg).expect("export ShiftMovementKind");
    d::CashMovementKind::export_all(cfg).expect("export CashMovementKind");
    d::DenominationCount::export_all(cfg).expect("export DenominationCount");
    d::ShiftMovement::export_all(cfg).expect("export ShiftMovement");
    d::Shift::export_all(cfg).expect("export Shift");
    d::SalesByMethod::export_all(cfg).expect("export SalesByMethod");
    d::ShiftSummary::export_all(cfg).expect("export ShiftSummary");
    d::ShiftRow::export_all(cfg).expect("export ShiftRow");
    d::OpenShiftInput::export_all(cfg).expect("export OpenShiftInput");
    d::CloseShiftInput::export_all(cfg).expect("export CloseShiftInput");
    d::ShiftFilter::export_all(cfg).expect("export ShiftFilter");
    d::HeldSaleLine::export_all(cfg).expect("export HeldSaleLine");
    d::HeldSale::export_all(cfg).expect("export HeldSale");
    d::HeldSaleInput::export_all(cfg).expect("export HeldSaleInput");
    d::InvoicesGetInvoicesArgs::export_all(cfg).expect("export InvoicesGetInvoicesArgs");
    d::InvoicesGetInvoicesPagedArgs::export_all(cfg).expect("export InvoicesGetInvoicesPagedArgs");
    d::InvoicesGetInvoiceArgs::export_all(cfg).expect("export InvoicesGetInvoiceArgs");
    d::InvoicesPreviewSaleArgs::export_all(cfg).expect("export InvoicesPreviewSaleArgs");
    d::InvoicesCreateSaleArgs::export_all(cfg).expect("export InvoicesCreateSaleArgs");
    d::InvoicesCreateRefundArgs::export_all(cfg).expect("export InvoicesCreateRefundArgs");
    d::InvoicesGetRefundArgs::export_all(cfg).expect("export InvoicesGetRefundArgs");
    d::InvoicesGetInvoicePrintDataArgs::export_all(cfg).expect("export InvoicesGetInvoicePrintDataArgs");
    d::InvoicesGetQuotationsArgs::export_all(cfg).expect("export InvoicesGetQuotationsArgs");
    d::InvoicesGetQuotationArgs::export_all(cfg).expect("export InvoicesGetQuotationArgs");
    d::InvoicesSaveQuotationArgs::export_all(cfg).expect("export InvoicesSaveQuotationArgs");
    d::InvoicesSetQuotationStatusArgs::export_all(cfg).expect("export InvoicesSetQuotationStatusArgs");
    d::InvoicesConvertQuotationToInvoiceArgs::export_all(cfg).expect("export InvoicesConvertQuotationToInvoiceArgs");
    d::InvoicesGetShiftsArgs::export_all(cfg).expect("export InvoicesGetShiftsArgs");
    d::InvoicesGetShiftArgs::export_all(cfg).expect("export InvoicesGetShiftArgs");
    d::InvoicesOpenPosShiftArgs::export_all(cfg).expect("export InvoicesOpenPosShiftArgs");
    d::InvoicesGetXReportArgs::export_all(cfg).expect("export InvoicesGetXReportArgs");
    d::InvoicesClosePosShiftArgs::export_all(cfg).expect("export InvoicesClosePosShiftArgs");
    d::InvoicesForceClosePosShiftArgs::export_all(cfg).expect("export InvoicesForceClosePosShiftArgs");
    d::InvoicesRecordCashInOutArgs::export_all(cfg).expect("export InvoicesRecordCashInOutArgs");
    d::InvoicesHoldSaleArgs::export_all(cfg).expect("export InvoicesHoldSaleArgs");
    d::InvoicesResumeHeldSaleArgs::export_all(cfg).expect("export InvoicesResumeHeldSaleArgs");
    d::InvoicesDiscardHeldSaleArgs::export_all(cfg).expect("export InvoicesDiscardHeldSaleArgs");
}
