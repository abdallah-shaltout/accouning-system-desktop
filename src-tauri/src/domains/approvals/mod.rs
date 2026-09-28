//! `approvals` domain (03-domains/04-approvals.md) — the async manager-approval queue: submit a
//! request, list/count, approve/reject under a row lock so two managers on two terminals can't
//! both decide the same request. 5 IPC commands; no undo compensators (§5 — decisions are
//! terminal, not undoable via the registry).

pub mod commands;
pub mod dto;
pub mod service;

use crate::core::ipc::IpcSig;
use crate::ipc_sig;

/// §1's 5 commands, in table order.
pub fn ipc_signatures() -> Vec<IpcSig> {
    vec![
        ipc_sig!(approvals_submit_approval_request, dto::ApprovalsSubmitApprovalRequestArgs, dto::ApprovalRequest),
        ipc_sig!(approvals_get_approval_requests, dto::ApprovalsGetApprovalRequestsArgs, Vec<dto::ApprovalRequest>),
        ipc_sig!(approvals_get_pending_approval_count, (), u32),
        ipc_sig!(approvals_approve_request, dto::ApprovalsApproveRequestArgs, dto::ApprovalRequest),
        ipc_sig!(approvals_reject_request, dto::ApprovalsRejectRequestArgs, dto::ApprovalRequest),
    ]
}

/// G-8a: this domain's DTO exports (§2) — the manager calls this one line from the top-level
/// `domains::export_bindings` hook (`domains/mod.rs`) in the same commit that adds
/// `pub mod approvals;` there.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    dto::ApprovalKind::export_all(cfg).expect("export ApprovalKind");
    dto::ApprovalStatus::export_all(cfg).expect("export ApprovalStatus");
    dto::ApprovalRequest::export_all(cfg).expect("export ApprovalRequest");
    dto::ApprovalRequestInput::export_all(cfg).expect("export ApprovalRequestInput");
    dto::ApprovalDecisionInput::export_all(cfg).expect("export ApprovalDecisionInput");
    dto::ApprovalListFilter::export_all(cfg).expect("export ApprovalListFilter");
    dto::ApprovalsSubmitApprovalRequestArgs::export_all(cfg).expect("export ApprovalsSubmitApprovalRequestArgs");
    dto::ApprovalsGetApprovalRequestsArgs::export_all(cfg).expect("export ApprovalsGetApprovalRequestsArgs");
    dto::ApprovalsApproveRequestArgs::export_all(cfg).expect("export ApprovalsApproveRequestArgs");
    dto::ApprovalsRejectRequestArgs::export_all(cfg).expect("export ApprovalsRejectRequestArgs");
}
