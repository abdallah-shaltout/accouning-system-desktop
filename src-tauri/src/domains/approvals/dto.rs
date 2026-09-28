//! `approvals` DTOs (03-domains/04-approvals.md §2) — mirror `src/modules/approvals/types/index.ts`
//! exactly. `ApprovalKind`/`ApprovalStatus` are the TS-facing shapes of the SeaORM entity enums
//! (`entities::platform::approval_requests::{ApprovalKind, ApprovalStatus}`), converted with
//! `From`/`Into` rather than reusing the entity enum directly, so a schema-level rename never
//! silently changes the wire contract.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::entities::platform::approval_requests::{self, Model as ApprovalRequestModel};
use crate::utils::id::Id;
use crate::utils::money::serde_number;
use crate::utils::route::RouteRef;

/// `ApprovalKind` (`types/index.ts:20`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "approvals/types/gen/")]
pub enum ApprovalKind {
    Discount,
    WriteOff,
    BelowCost,
}

impl From<approval_requests::ApprovalKind> for ApprovalKind {
    fn from(k: approval_requests::ApprovalKind) -> Self {
        match k {
            approval_requests::ApprovalKind::Discount => ApprovalKind::Discount,
            approval_requests::ApprovalKind::WriteOff => ApprovalKind::WriteOff,
            approval_requests::ApprovalKind::BelowCost => ApprovalKind::BelowCost,
        }
    }
}

impl From<ApprovalKind> for approval_requests::ApprovalKind {
    fn from(k: ApprovalKind) -> Self {
        match k {
            ApprovalKind::Discount => approval_requests::ApprovalKind::Discount,
            ApprovalKind::WriteOff => approval_requests::ApprovalKind::WriteOff,
            ApprovalKind::BelowCost => approval_requests::ApprovalKind::BelowCost,
        }
    }
}

/// `ApprovalStatus` (`types/index.ts:22`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "approvals/types/gen/")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

impl From<approval_requests::ApprovalStatus> for ApprovalStatus {
    fn from(s: approval_requests::ApprovalStatus) -> Self {
        match s {
            approval_requests::ApprovalStatus::Pending => ApprovalStatus::Pending,
            approval_requests::ApprovalStatus::Approved => ApprovalStatus::Approved,
            approval_requests::ApprovalStatus::Rejected => ApprovalStatus::Rejected,
        }
    }
}

impl From<ApprovalStatus> for approval_requests::ApprovalStatus {
    fn from(s: ApprovalStatus) -> Self {
        match s {
            ApprovalStatus::Pending => approval_requests::ApprovalStatus::Pending,
            ApprovalStatus::Approved => approval_requests::ApprovalStatus::Approved,
            ApprovalStatus::Rejected => approval_requests::ApprovalStatus::Rejected,
        }
    }
}

/// `ApprovalRequest` (`types/index.ts:24-44`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "approvals/types/gen/")]
pub struct ApprovalRequest {
    #[ts(type = "string")]
    pub id: Id,
    pub kind: ApprovalKind,
    pub summary: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value: Decimal,
    #[ts(optional)]
    pub request_note: Option<String>,
    #[ts(type = "string")]
    pub requested_by: Id,
    pub requested_by_name: String,
    pub requested_at: String,
    pub status: ApprovalStatus,
    #[ts(optional, type = "string")]
    pub decided_by: Option<Id>,
    #[ts(optional)]
    pub decided_by_name: Option<String>,
    #[ts(optional)]
    pub decided_at: Option<String>,
    #[ts(optional)]
    pub decision_comment: Option<String>,
    #[ts(optional, type = "import('@/modules/core/types/route').AppRoute")]
    pub link: Option<RouteRef>,
}

/// Builds the response DTO from the persisted row (`requested_at`/`decided_at` = `DocDate::key()`,
/// D-5 — depends on the `_instant` columns being `DATETIME(3)`, already true in `m0013_platform.rs`).
pub fn to_dto(m: &ApprovalRequestModel) -> ApprovalRequest {
    ApprovalRequest {
        id: m.id,
        kind: m.kind.clone().into(),
        summary: m.summary.clone(),
        value: m.value,
        request_note: m.request_note.clone(),
        requested_by: m.requested_by,
        requested_by_name: m.requested_by_name.clone(),
        requested_at: m.requested_at().key(),
        status: m.status.clone().into(),
        decided_by: m.decided_by,
        decided_by_name: m.decided_by_name.clone(),
        decided_at: m.decided_at().map(|d| d.key()),
        decision_comment: m.decision_comment.clone(),
        link: m.link.clone().map(|v| v.into()),
    }
}

/// `ApprovalRequestInput` (`types/index.ts:46-52`) — request-only.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "approvals/types/gen/")]
pub struct ApprovalRequestInput {
    pub kind: ApprovalKind,
    pub summary: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value: Decimal,
    #[ts(optional)]
    pub request_note: Option<String>,
    #[ts(optional, type = "import('@/modules/core/types/route').AppRoute")]
    pub link: Option<RouteRef>,
}

/// `ApprovalDecisionInput` (`types/index.ts:54-56`).
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "approvals/types/gen/")]
pub struct ApprovalDecisionInput {
    #[ts(optional)]
    pub comment: Option<String>,
}

/// The inline `{ status?: 'pending' | 'approved' | 'rejected' }` filter (`approvalService.ts:18`).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "approvals/types/gen/")]
pub struct ApprovalListFilter {
    #[ts(optional)]
    pub status: Option<ApprovalStatus>,
}

// --- Command args (§3.2 convention: one struct per command, camelCase) --------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "approvals/types/gen/")]
pub struct ApprovalsSubmitApprovalRequestArgs {
    pub input: ApprovalRequestInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "approvals/types/gen/")]
pub struct ApprovalsGetApprovalRequestsArgs {
    #[ts(optional)]
    pub filter: Option<ApprovalListFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "approvals/types/gen/")]
pub struct ApprovalsApproveRequestArgs {
    pub id: String,
    #[ts(optional)]
    pub input: Option<ApprovalDecisionInput>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "approvals/types/gen/")]
pub struct ApprovalsRejectRequestArgs {
    pub id: String,
    pub input: ApprovalDecisionInput,
}
