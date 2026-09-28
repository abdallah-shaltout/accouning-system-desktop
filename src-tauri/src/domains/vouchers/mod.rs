//! `vouchers` domain (03-domains/10-vouchers.md) — four "fast journal" vouchers (receipt, payment,
//! transfer, owner) that each post one balanced entry, plus the card/wallet settlement that clears
//! `cardClearing`/`walletClearing`. 11 IPC commands (§1); no undo compensators (§5 — vouchers and
//! settlements have no reversal in the mock; corrections are manual journal entries).
//!
//! `service::general::record_transfer_voucher` is also called directly (not through IPC) by the
//! invoices domain's shift-close cash-drop (08b, wave W4/next).

pub mod commands;
pub mod dto;
pub mod service;

use crate::core::ipc::IpcSig;
use crate::ipc_sig;

/// §1's 11 commands, in table order.
pub fn ipc_signatures() -> Vec<IpcSig> {
    vec![
        ipc_sig!(vouchers_create_receipt_voucher, dto::VouchersCreateReceiptVoucherArgs, dto::Voucher),
        ipc_sig!(vouchers_create_payment_voucher, dto::VouchersCreatePaymentVoucherArgs, dto::Voucher),
        ipc_sig!(vouchers_create_transfer_voucher, dto::VouchersCreateTransferVoucherArgs, dto::Voucher),
        ipc_sig!(vouchers_create_owner_voucher, dto::VouchersCreateOwnerVoucherArgs, dto::Voucher),
        ipc_sig!(vouchers_get_vouchers, dto::VouchersGetVouchersArgs, Vec<dto::Voucher>),
        ipc_sig!(vouchers_get_voucher, dto::VouchersGetVoucherArgs, dto::Voucher),
        ipc_sig!(vouchers_get_unsettled_tender_groups, (), Vec<dto::UnsettledTenderGroup>),
        ipc_sig!(vouchers_estimate_settlement_fee, dto::VouchersEstimateSettlementFeeArgs, dto::FeeEstimate),
        ipc_sig!(vouchers_create_card_settlement, dto::VouchersCreateCardSettlementArgs, dto::CardSettlement),
        ipc_sig!(vouchers_get_card_settlements, (), Vec<dto::CardSettlement>),
        ipc_sig!(vouchers_get_card_settlement, dto::VouchersGetCardSettlementArgs, dto::CardSettlement),
    ]
}

/// G-8a: this domain's DTO exports (§2) — the manager calls this one line from the top-level
/// `domains::export_bindings` hook (`domains/mod.rs`, manager-owned) in the same commit that adds
/// `pub mod vouchers;` there.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    dto::VoucherKind::export_all(cfg).expect("export VoucherKind");
    dto::OwnerDirection::export_all(cfg).expect("export OwnerDirection");
    dto::Voucher::export_all(cfg).expect("export Voucher");
    dto::ReceiptVoucherInput::export_all(cfg).expect("export ReceiptVoucherInput");
    dto::PaymentVoucherInput::export_all(cfg).expect("export PaymentVoucherInput");
    dto::TransferVoucherInput::export_all(cfg).expect("export TransferVoucherInput");
    dto::OwnerVoucherInput::export_all(cfg).expect("export OwnerVoucherInput");
    dto::VoucherFilter::export_all(cfg).expect("export VoucherFilter");
    dto::ClearingRole::export_all(cfg).expect("export ClearingRole");
    dto::UnsettledTenderGroup::export_all(cfg).expect("export UnsettledTenderGroup");
    dto::CardSettlementGroupRef::export_all(cfg).expect("export CardSettlementGroupRef");
    dto::CardSettlementInput::export_all(cfg).expect("export CardSettlementInput");
    dto::CardSettlementGroup::export_all(cfg).expect("export CardSettlementGroup");
    dto::CardSettlement::export_all(cfg).expect("export CardSettlement");
    dto::FeeEstimate::export_all(cfg).expect("export FeeEstimate");
    dto::VouchersCreateReceiptVoucherArgs::export_all(cfg).expect("export VouchersCreateReceiptVoucherArgs");
    dto::VouchersCreatePaymentVoucherArgs::export_all(cfg).expect("export VouchersCreatePaymentVoucherArgs");
    dto::VouchersCreateTransferVoucherArgs::export_all(cfg).expect("export VouchersCreateTransferVoucherArgs");
    dto::VouchersCreateOwnerVoucherArgs::export_all(cfg).expect("export VouchersCreateOwnerVoucherArgs");
    dto::VouchersGetVouchersArgs::export_all(cfg).expect("export VouchersGetVouchersArgs");
    dto::VouchersGetVoucherArgs::export_all(cfg).expect("export VouchersGetVoucherArgs");
    dto::VouchersEstimateSettlementFeeArgs::export_all(cfg).expect("export VouchersEstimateSettlementFeeArgs");
    dto::VouchersCreateCardSettlementArgs::export_all(cfg).expect("export VouchersCreateCardSettlementArgs");
    dto::VouchersGetCardSettlementArgs::export_all(cfg).expect("export VouchersGetCardSettlementArgs");
}
