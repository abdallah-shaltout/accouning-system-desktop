//! `domains::payments` (03-domains/09-payments.md): customer receipts / supplier payments, with
//! sub-ledger allocation to open invoices/POs and realized FX. A payment posts ONCE, for its full
//! amount, against the receivable/payable control account (plus a realized-FX line); allocations
//! never touch the GL again except the small FX-delta entry a later "allocate later" can post. 7
//! commands (§1); not undoable via the registry (§5 — a payment's posting is permanent).

pub mod commands;
pub mod dto;
pub mod service;

pub fn ipc_signatures() -> Vec<crate::core::ipc::IpcSig> {
    commands::ipc_signatures()
}

/// G-8a: this domain's DTO exports — the manager calls this one line from
/// `domains::export_bindings` (`domains/mod.rs`) in the same commit that adds `pub mod payments;`
/// there, per this checklist item in `03-domains/09-payments.md` §9.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    dto::PaymentTypeDto::export_all(cfg).expect("export PaymentTypeDto");
    dto::PartyKind::export_all(cfg).expect("export PartyKind");
    dto::PaymentTenderKind::export_all(cfg).expect("export PaymentTenderKind");
    dto::AllocationTargetKind::export_all(cfg).expect("export AllocationTargetKind");
    dto::AllocationInputTargetKind::export_all(cfg).expect("export AllocationInputTargetKind");
    dto::OpenDocumentKindDto::export_all(cfg).expect("export OpenDocumentKindDto");
    dto::AllocationStatus::export_all(cfg).expect("export AllocationStatus");
    dto::PaymentAllocation::export_all(cfg).expect("export PaymentAllocation");
    dto::Payment::export_all(cfg).expect("export Payment");
    dto::PaymentRow::export_all(cfg).expect("export PaymentRow");
    dto::PaymentAllocationInput::export_all(cfg).expect("export PaymentAllocationInput");
    dto::PaymentInput::export_all(cfg).expect("export PaymentInput");
    dto::PaymentFilter::export_all(cfg).expect("export PaymentFilter");
    dto::OpenDocument::export_all(cfg).expect("export OpenDocument");
    dto::PaymentsGetPaymentsArgs::export_all(cfg).expect("export PaymentsGetPaymentsArgs");
    dto::PaymentsGetPaymentsPagedArgs::export_all(cfg).expect("export PaymentsGetPaymentsPagedArgs");
    dto::PaymentsGetPaymentArgs::export_all(cfg).expect("export PaymentsGetPaymentArgs");
    dto::PaymentsCreatePaymentArgs::export_all(cfg).expect("export PaymentsCreatePaymentArgs");
    dto::PaymentsAllocateExistingPaymentArgs::export_all(cfg).expect("export PaymentsAllocateExistingPaymentArgs");
    dto::PaymentsRemoveAllocationArgs::export_all(cfg).expect("export PaymentsRemoveAllocationArgs");
    dto::PaymentsGetOpenDocumentsArgs::export_all(cfg).expect("export PaymentsGetOpenDocumentsArgs");
}
