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

use rust_decimal::Decimal;
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_version: Option<String>,
    pub schema: SchemaStatus,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<PageSort>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<F>,
}

/// `total: u32` and `totals: Record<string, number>` per core.md §2. G-14: `totals` values are
/// `Decimal` (rule 5, the one rounding rule) serialized as JSON numbers via `utils::money::
/// serde_number` — the original `f64` shape silently reintroduced binary-float rounding into a
/// value that a paged list often sums straight from posted money columns (page-total footers,
/// running balances), even though the field is "display-only": it is still a monetary aggregate
/// the accounting invariants can be checked against, not a UI-only rounding.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct PagedResult<R> {
    pub rows: Vec<R>,
    pub total: u32,
    #[ts(optional, type = "Record<string, number>")]
    #[serde(default, skip_serializing_if = "Option::is_none", with = "totals_map")]
    pub totals: Option<BTreeMap<String, Decimal>>,
}

/// G-14: `BTreeMap<String, Decimal>` serialized as `Record<string, number>` — each value goes
/// through the same `utils::money::serde_number` JSON-number rule the rest of the app's money
/// fields use (rule 5), rather than serde's default `Decimal` string representation. `pub` (21.04
/// phase A, A-3) so any other DTO with a `BTreeMap<String, Decimal>` field can reuse it via
/// `#[serde(with = "crate::core::dto::totals_map")]` (or `crate::core::dto::totals_map::required`
/// for a non-`Option` map, e.g. `InvoiceDetail`/`PurchaseDetail`'s `returned_qty`) instead of
/// hand-rolling the same map loop.
pub mod totals_map {
    use super::*;

    pub fn serialize<S: serde::Serializer>(value: &Option<BTreeMap<String, Decimal>>, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match value {
            None => serializer.serialize_none(),
            Some(map) => {
                let mut ser_map = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map {
                    let f: f64 = v
                        .to_string()
                        .parse()
                        .map_err(|_| serde::ser::Error::custom("decimal did not fit in f64"))?;
                    ser_map.serialize_entry(k, &f)?;
                }
                ser_map.end()
            }
        }
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<BTreeMap<String, Decimal>>, D::Error> {
        let raw = Option::<BTreeMap<String, serde_json::Value>>::deserialize(deserializer)?;
        match raw {
            None => Ok(None),
            Some(map) => {
                let mut out = BTreeMap::new();
                for (k, v) in map {
                    let d = crate::utils::money::decimal_from_json_value(&v).map_err(serde::de::Error::custom)?;
                    out.insert(k, d);
                }
                Ok(Some(out))
            }
        }
    }

    /// Same rule for a non-`Option` `BTreeMap<String, Decimal>` field (21.04 phase A, A-3 finding:
    /// `InvoiceDetail.returned_qty` / `PurchaseDetail.returned_qty` are always-present maps, never
    /// absent-vs-empty, so they don't need the `Option` wrapper the paged-result `totals` field does).
    pub mod required {
        use super::*;

        pub fn serialize<S: serde::Serializer>(value: &BTreeMap<String, Decimal>, serializer: S) -> Result<S::Ok, S::Error> {
            super::serialize(&Some(value.clone()), serializer)
        }

        pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<BTreeMap<String, Decimal>, D::Error> {
            Ok(super::deserialize(deserializer)?.unwrap_or_default())
        }
    }
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
    // G-8c: `@/...` (the project's `baseUrl`/`paths` alias to `src/`) resolves correctly from
    // every generated file's location, unlike a relative `../../` path which only happened to work
    // from this type's own `core/types/gen/` output directory and breaks for any other exported
    // type at a different depth (e.g. `AuditEntry` below, exported to `diagnostics/types/gen/`).
    #[ts(optional, type = "import('@/modules/core/types/route').AppRoute")]
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<serde_json::Value>,
    #[ts(optional, type = "unknown")]
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_label: Option<String>,
    pub action: AuditAction,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<Vec<AuditFieldDiff>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<Vec<AuditFieldDiff>>,
    #[ts(type = "string")]
    pub user_id: Id,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
    pub at: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub message: String,
    #[ts(optional, type = "import('@/modules/core/types/route').AppRoute")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<crate::utils::route::RouteRef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paged_result_totals_round_trip_as_json_numbers_not_strings() {
        use rust_decimal_macros::dec;
        let mut totals = BTreeMap::new();
        totals.insert("subtotal".to_string(), dec!(1234.57));
        totals.insert("vat".to_string(), dec!(0.3));
        let result = PagedResult::<i32> { rows: vec![], total: 2, totals: Some(totals.clone()) };

        let json = serde_json::to_value(&result).unwrap();
        let totals_json = json.get("totals").unwrap();
        assert_eq!(totals_json.get("subtotal").unwrap(), &serde_json::json!(1234.57));
        assert_eq!(totals_json.get("vat").unwrap(), &serde_json::json!(0.3));
        assert!(totals_json.get("subtotal").unwrap().is_number(), "totals values must be JSON numbers, not strings");

        let back: PagedResult<i32> = serde_json::from_value(json).unwrap();
        assert_eq!(back.totals, Some(totals));
    }

    #[test]
    fn paged_result_totals_omitted_when_none() {
        let result = PagedResult::<i32> { rows: vec![1], total: 1, totals: None };
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains("totals"), "totals must be absent on the wire when None, not null");
    }

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
