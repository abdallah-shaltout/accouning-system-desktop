//! Cross-cutting IPC DTOs with ts-rs bindings (21.02-F, F-2: paging, activity/audit, backend
//! status, error payload). Owned by phase F.
//!
//! Conventions (F-1): `#[serde(rename_all = "camelCase")]`; `Option` fields use
//! `#[serde_with::skip_serializing_none]` + `#[ts(optional)]` so they're *absent* on the wire, not
//! `null` (matching the TS `field?: T` shape exactly); `Decimal` fields use
//! `#[serde(with = "crate::utils::money::serde_number")]` + `#[ts(type = "number")]`; `Id` fields
//! use `#[ts(type = "string")]` (`utils::id::Id` has no `TS` impl of its own — this crate doesn't
//! own that file this wave, so every `Id` field overrides its binding individually rather than
//! adding one there); route fields use `crate::utils::route::RouteRef`, whose ts-rs binding is
//! overridden to import the real `AppRoute` type via `#[ts(type = "...")]` below so no separate
//! `RouteRef.ts` shim is generated. None of these types carry `#[ts(export)]` — `core/ipc.rs`'s
//! single `export_bindings` test drives every export itself via `TS::export_all`, so there is
//! exactly one hand-written test, not one ts-rs-generated test per type.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::core::error::AppError;
use crate::core::events::ChangeCategory as EventsChangeCategory;
pub use crate::core::events::ChangeCategory;
use crate::utils::id::Id;

// --- Error payload (mirrors AppError's serialized shape exactly) -------------------------------

/// The serialized shape of `AppError` (`core/error.rs`) — `{ code, message }`, matching
/// `core/types/backend.ts`'s `ApiErrorPayload` and `ApiErrorCode` exactly.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "core/types/gen/")]
pub enum ApiErrorCode {
    NotFound,
    Validation,
    Conflict,
    Forbidden,
    Unauthorized,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct ApiErrorPayload {
    pub code: ApiErrorCode,
    pub message: String,
}

impl From<&AppError> for ApiErrorPayload {
    fn from(err: &AppError) -> Self {
        let code = match err {
            AppError::NotFound { .. } => ApiErrorCode::NotFound,
            AppError::Validation { .. } => ApiErrorCode::Validation,
            AppError::Conflict { .. } => ApiErrorCode::Conflict,
            AppError::Forbidden { .. } => ApiErrorCode::Forbidden,
            AppError::Unauthorized { .. } => ApiErrorCode::Unauthorized,
            AppError::Internal { .. } => ApiErrorCode::Internal,
        };
        ApiErrorPayload { code, message: err.to_string() }
    }
}

impl From<AppError> for ApiErrorPayload {
    fn from(err: AppError) -> Self {
        ApiErrorPayload::from(&err)
    }
}

// --- backend:changed event payload --------------------------------------------------------------
// `ChangeCategory` itself derives `TS` in `core/events.rs` (F-2 note: "you own events.rs").

/// Payload of the `backend:changed` Tauri event (`core/events.rs::ChangedPayload`, aliased here
/// under the F-2-specified name so the generated TS type is called `BackendChangedPayload`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct BackendChangedPayload {
    pub categories: Vec<EventsChangeCategory>,
}

impl From<crate::core::events::ChangedPayload> for BackendChangedPayload {
    fn from(p: crate::core::events::ChangedPayload) -> Self {
        BackendChangedPayload { categories: p.categories }
    }
}

// --- Backend status (core_backend_status — command registered in a later wave) -----------------

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "core/types/gen/")]
pub enum BackendRole {
    Main,
    Terminal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "core/types/gen/")]
pub enum SchemaStatus {
    Ok,
    Behind,
    Ahead,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "core/types/gen/")]
pub enum ServerState {
    Provisioning,
    Starting,
    Upgrading,
    Running,
    Stopped,
    Failed,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct ServerFailure {
    pub code: String,
    pub message: String,
}

/// The managed-server sub-status (added by phase A2, C-22) — present only on a Main PC with a
/// managed server, built from `AppState.server.snapshot()` (not implemented in this wave).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct BackendServerStatus {
    pub state: ServerState,
    pub version: String,
    pub port: i32,
    pub lan_sharing: bool,
    #[ts(optional)]
    pub failure: Option<ServerFailure>,
}

/// Result of `core_backend_status` — the command itself is registered in a later wave (once the
/// bundled-DB server state exists); the DTO ships now so F-4's drift check has something to guard.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct BackendStatus {
    pub connected: bool,
    pub role: BackendRole,
    pub terminal_id: String,
    #[ts(optional)]
    pub server_version: Option<String>,
    pub schema: SchemaStatus,
    #[ts(optional)]
    pub error: Option<String>,
    #[ts(optional)]
    pub server: Option<BackendServerStatus>,
}

// --- Paging (mirrors core/types/paging.ts exactly) ----------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct PageSort {
    pub key: String,
    pub dir: SortDir,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "core/types/gen/")]
pub enum SortDir {
    Asc,
    Desc,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct PagedQuery<F> {
    pub page: i32,
    pub page_size: i32,
    #[ts(optional)]
    pub sort: Option<PageSort>,
    #[ts(optional)]
    pub filters: Option<F>,
}

/// `total: u32` and `totals: Record<string, number>` per core.md §2 — `totals` values are plain
/// `f64` (never `Decimal`) since they are display-only rounded aggregates, not accounting inputs.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct PagedResult<R> {
    pub rows: Vec<R>,
    pub total: u32,
    #[ts(optional)]
    pub totals: Option<BTreeMap<String, f64>>,
}

// --- Activity feed (mirrors core/types/index.ts exactly) -----------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "core/types/gen/")]
pub enum ActivityKind {
    Sale,
    Refund,
    Purchase,
    PurchaseReturn,
    Payment,
    Stock,
    Journal,
    Product,
    Party,
    User,
    Settings,
    Auth,
    Shift,
    Expense,
    Voucher,
    Approval,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct ActivityEntry {
    #[ts(type = "string")]
    pub id: Id,
    /// A business day key (`YYYY-MM-DD`) or ISO instant — matches the TS `string` field exactly
    /// (no `DocDate` triple here; the mock's `ActivityEntry.date` is already a plain string).
    pub date: String,
    #[ts(type = "string")]
    pub user_id: Id,
    pub kind: ActivityKind,
    pub message: String,
    #[ts(optional, type = "import('../../core/types/route').AppRoute")]
    pub link: Option<crate::utils::route::RouteRef>,
}

// --- Audit trail (mirrors diagnostics/types/index.ts's AuditEntry family exactly) ----------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub enum AuditAction {
    Create,
    Update,
    Post,
    Void,
    Reverse,
    Delete,
    Login,
    Settings,
}

/// `before`/`after` values are opaque per-field snapshots (never the whole document) — `unknown`
/// on the TS side, matched here via `#[ts(type = "unknown")]` (no concrete Rust type can describe
/// "any field's prior/new value" across every audited entity).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct AuditFieldDiff {
    pub field: String,
    #[ts(optional, type = "unknown")]
    pub before: Option<serde_json::Value>,
    #[ts(optional, type = "unknown")]
    pub after: Option<serde_json::Value>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct AuditEntry {
    #[ts(type = "string")]
    pub id: Id,
    pub entity: String,
    #[ts(type = "string")]
    pub entity_id: Id,
    #[ts(optional)]
    pub entity_label: Option<String>,
    pub action: AuditAction,
    #[ts(optional)]
    pub before: Option<Vec<AuditFieldDiff>>,
    #[ts(optional)]
    pub after: Option<Vec<AuditFieldDiff>>,
    #[ts(type = "string")]
    pub user_id: Id,
    #[ts(optional, type = "string")]
    pub branch_id: Option<Id>,
    pub at: String,
    #[ts(optional)]
    pub reason: Option<String>,
    pub message: String,
    #[ts(optional, type = "import('../../core/types/route').AppRoute")]
    pub link: Option<crate::utils::route::RouteRef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_error_payload_from_app_error_maps_every_variant() {
        let cases: [(AppError, &str); 6] = [
            (AppError::not_found("x"), "NOT_FOUND"),
            (AppError::validation("x"), "VALIDATION"),
            (AppError::conflict("x"), "CONFLICT"),
            (AppError::forbidden("x"), "FORBIDDEN"),
            (AppError::unauthorized("x"), "UNAUTHORIZED"),
            (AppError::internal("x", None), "INTERNAL"),
        ];
        for (err, expected_code) in cases {
            let payload: ApiErrorPayload = (&err).into();
            let json = serde_json::to_string(&payload).unwrap();
            assert!(json.contains(expected_code), "expected {expected_code} in {json}");
        }
    }
}
