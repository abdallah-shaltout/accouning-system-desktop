//! `diagnostics` domain DTOs (plan 21 Part 03 §16). Slice A (W1): `AuditEntry`, `AuditFieldDiff`,
//! `AuditAction` are **reused** from `crate::core::dto` (already exported + checked in
//! `diagnostics/types/contract.check.ts`) — not redeclared here; this file carries the audit filter
//! and support-bundle snapshot shapes new to this module (spec §2). Slice B (this file's bottom
//! half, after 12-accounting): the 7 debug-build accounting-debugger DTOs — `AccountingDocSummary`,
//! `BalanceAround`, `DriftRow`, `ExplainLine`, and the `InvariantResultDto`/`PostingTraceDto`
//! mirrors of `shared::invariants::InvariantResult`/`shared::ledger::trace::PostingTrace` (neither
//! upstream type derives `TS` or serializes `Decimal` as a JSON number, so a `From<&…>` mirror DTO
//! is built here rather than adding a dependency the other direction — spec §2).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::utils::id::Id;
use crate::utils::money::serde_number;

/// Args shape of `diagnostics_get_audit_entries` — mirrors `auditService.ts:10-17`'s `AuditFilter`
/// exactly (all fields optional, compared as plain strings by the mock — spec §1/§3).
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct AuditFilter {
    #[ts(optional, type = "string")]
    pub user_id: Option<crate::utils::id::Id>,
    #[ts(optional)]
    pub entity: Option<String>,
    #[ts(optional)]
    pub action: Option<crate::core::dto::AuditAction>,
    /// `YYYY-MM-DD`, compared against the UTC slice of `AuditEntry.at` (Q-1, kept quirk).
    #[ts(optional)]
    pub from: Option<String>,
    #[ts(optional)]
    pub to: Option<String>,
    #[ts(optional)]
    pub search: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct DiagnosticsGetAuditEntriesArgs {
    #[ts(optional)]
    pub filter: Option<AuditFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct DiagnosticsExportSupportBundleArgs {
    #[ts(optional)]
    pub include_db_snapshot: Option<bool>,
}

/// camelCase mirror of `infrastructure::database::errors::ServerDiagnostics` (spec §2) — a plain
/// DTO copy rather than deriving `TS` on the infra struct directly, since that struct lives outside
/// this domain's owned files and is not itself part of the IPC contract today.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct ServerDiagnosticsDto {
    pub state: String,
    #[ts(optional)]
    pub version: Option<String>,
    #[ts(optional)]
    pub port: Option<u16>,
    pub lan_sharing: bool,
    #[ts(optional)]
    pub last_failure: Option<String>,
    #[ts(optional)]
    pub error_log_tail: Option<String>,
}

impl From<crate::infrastructure::database::errors::ServerDiagnostics> for ServerDiagnosticsDto {
    fn from(s: crate::infrastructure::database::errors::ServerDiagnostics) -> Self {
        ServerDiagnosticsDto {
            state: s.state,
            version: s.version,
            port: s.port,
            lan_sharing: s.lan_sharing,
            last_failure: s.last_failure,
            error_log_tail: s.error_log_tail,
        }
    }
}

/// Result of `diagnostics_export_support_bundle` (spec §2, D-1): the Rust-supplied part of the
/// support bundle only — settings (redacted) + server diagnostics (+ DB snapshot in slice A2). The
/// frontend still owns log-file collection, zipping and the native save dialog (D-1).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct SupportSnapshot {
    #[ts(type = "unknown")]
    pub settings_redacted: serde_json::Value,
    /// Slice A2: `Some(...)` only when the caller passed `includeDbSnapshot: true` (and holds
    /// Settings:Write) — every table via `infrastructure::backup::dataset`, excluding `credentials`,
    /// redacted (spec §3 step 5, D-3).
    #[ts(optional, type = "unknown")]
    pub db_snapshot: Option<serde_json::Value>,
    #[ts(optional)]
    pub server: Option<ServerDiagnosticsDto>,
}

// ---------------------------------------------------------------------------------------------
// Slice B — debug-build-only accounting-debugger DTOs (spec §2, §9 slice B checklist).
// ---------------------------------------------------------------------------------------------

/// `AccountingDocSummary` (`accountingDebugService.ts:24-34`) — one row of the debugger's document
/// picker.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct AccountingDocSummary {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub date: String,
    pub description: String,
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub kind: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_credit: Decimal,
    #[ts(optional)]
    pub source_kind: Option<String>,
    pub has_trace: bool,
}

/// `BalanceAround` (`accountingDebugService.ts:87-97`, inline return of `getBalancesAround`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct BalanceAround {
    #[ts(type = "string")]
    pub account_id: Id,
    pub account_name: String,
    #[ts(optional)]
    pub account_code: Option<String>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub before: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub after: Decimal,
}

/// `DriftRow['kind']` (`accountingDebugService.ts:107`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub enum DriftRowKind {
    Customer,
    Supplier,
    Product,
}

/// `DriftRow` (`accountingDebugService.ts:106-115`) — subledger-vs-GL drift, display-only (pass/fail
/// stays in `shared::invariants::run_all`, spec §3).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct DriftRow {
    pub kind: DriftRowKind,
    #[ts(type = "string")]
    pub id: Id,
    pub label: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub subledger: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub gl: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub diff: Decimal,
    #[ts(optional, type = "string")]
    pub first_diverging_doc_id: Option<Id>,
    #[ts(optional)]
    pub first_diverging_doc_number: Option<String>,
}

/// `ExplainLine` (`accountingDebugService.ts:205-213`) — "اشرح هذا الرقم" rows.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct ExplainLine {
    #[ts(type = "string")]
    pub doc_id: Id,
    pub doc_number: String,
    pub doc_date: String,
    #[ts(optional)]
    pub source_kind: Option<String>,
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub credit: Decimal,
}

/// Mirror of `shared::invariants::InvariantDiff` with a ts-rs derive and `serde_number` money
/// (the upstream type derives only `Serialize`, not `TS`, and serializes `Decimal` via
/// `rust_decimal`'s default string-encoding `serde` impl rather than a JSON number — spec §2).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct InvariantDiffDto {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub expected: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub actual: Decimal,
    #[ts(type = "number")]
    pub delta_minor_units: i64,
}

impl From<&crate::shared::invariants::InvariantDiff> for InvariantDiffDto {
    fn from(d: &crate::shared::invariants::InvariantDiff) -> Self {
        InvariantDiffDto { expected: d.expected, actual: d.actual, delta_minor_units: d.delta_minor_units }
    }
}

/// Mirror of `shared::invariants::InvariantResult` (`#[ts(rename = "InvariantResult")]` per spec
/// §2 — the TS name the frontend already imports as `InvariantResult` from `@/mocks`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "InvariantResult")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct InvariantResultDto {
    pub key: String,
    pub doc: String,
    pub passed: bool,
    pub message: String,
    #[ts(optional)]
    pub diff: Option<InvariantDiffDto>,
}

impl From<&crate::shared::invariants::InvariantResult> for InvariantResultDto {
    fn from(r: &crate::shared::invariants::InvariantResult) -> Self {
        InvariantResultDto { key: r.key.clone(), doc: r.doc.clone(), passed: r.passed, message: r.message.clone(), diff: r.diff.as_ref().map(Into::into) }
    }
}

/// Mirror of `shared::ledger::trace::VatTrace`/`CostTrace`/`FxTrace` — carried as-is inside a
/// `PostingTraceStepDto.detail` (the mock stores these as opaque `unknown` JSON on the trace step,
/// never as a typed field elsewhere — spec §2 keeps that shape rather than inventing a stricter one).
pub type PostingTraceStepDetail = serde_json::Value;

/// Mirror of `shared::ledger::trace::PostingTraceStep`.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct PostingTraceStepDto {
    pub kind: String,
    pub label: String,
    #[ts(type = "unknown")]
    pub detail: PostingTraceStepDetail,
}

impl From<&crate::shared::ledger::trace::PostingTraceStep> for PostingTraceStepDto {
    fn from(s: &crate::shared::ledger::trace::PostingTraceStep) -> Self {
        PostingTraceStepDto { kind: s.kind.to_string(), label: s.label.clone(), detail: s.detail.clone() }
    }
}

/// Mirror of `shared::ledger::trace::TraceLine`, with `serde_number` money fields.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct TraceLineDto {
    #[ts(type = "string")]
    pub account_id: Id,
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub credit: Decimal,
    #[ts(optional, type = "'customer' | 'supplier'")]
    pub party_kind: Option<String>,
    #[ts(optional, type = "string")]
    pub party_id: Option<Id>,
    #[ts(type = "string")]
    pub branch_id: Id,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
    pub currency: String,
    #[serde(default, with = "serde_number::option")]
    #[ts(optional, type = "number")]
    pub amount_fc: Option<Decimal>,
    #[serde(default, with = "serde_number::option")]
    #[ts(optional, type = "number")]
    pub rate: Option<Decimal>,
}

impl From<&crate::shared::ledger::trace::TraceLine> for TraceLineDto {
    fn from(l: &crate::shared::ledger::trace::TraceLine) -> Self {
        TraceLineDto {
            account_id: l.account_id,
            description: l.description.clone(),
            debit: l.debit,
            credit: l.credit,
            party_kind: l.party_kind.map(|k| k.to_string()),
            party_id: l.party_id,
            branch_id: l.branch_id,
            cost_center_id: l.cost_center_id,
            currency: l.currency.clone(),
            amount_fc: l.amount_fc,
            rate: l.rate,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct TraceTotalsDto {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub dr: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cr: Decimal,
}

impl From<&crate::shared::ledger::trace::Totals> for TraceTotalsDto {
    fn from(t: &crate::shared::ledger::trace::Totals) -> Self {
        TraceTotalsDto { dr: t.dr, cr: t.cr }
    }
}

/// Mirror of `shared::ledger::trace::PostingTrace` (`#[ts(rename = "PostingTrace")]` — the TS name
/// the frontend already imports from `@/mocks`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "PostingTrace")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct PostingTraceDto {
    pub doc_type: String,
    #[ts(type = "string")]
    pub doc_id: Id,
    #[ts(optional, type = "string")]
    pub correlation_id: Option<Id>,
    pub steps: Vec<PostingTraceStepDto>,
    pub lines: Vec<TraceLineDto>,
    pub totals: TraceTotalsDto,
    pub balanced: bool,
    #[ts(type = "string")]
    pub at: String,
}

impl From<&crate::shared::ledger::trace::PostingTrace> for PostingTraceDto {
    fn from(t: &crate::shared::ledger::trace::PostingTrace) -> Self {
        PostingTraceDto {
            doc_type: t.doc_type.clone(),
            doc_id: t.doc_id,
            correlation_id: t.correlation_id,
            steps: t.steps.iter().map(Into::into).collect(),
            lines: t.lines.iter().map(Into::into).collect(),
            totals: (&t.totals).into(),
            balanced: t.balanced,
            at: crate::utils::dates::format_iso_ms(t.at),
        }
    }
}

// --- Command args ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct DiagnosticsListRecentDocumentsArgs {
    #[ts(optional)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct DiagnosticsGetPostingTraceArgs {
    #[ts(type = "string")]
    pub entry_id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct DiagnosticsGetJournalEntryRawArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct DiagnosticsGetBalancesAroundArgs {
    #[ts(type = "string")]
    pub entry_id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "diagnostics/types/gen/")]
pub struct DiagnosticsExplainAccountBalanceArgs {
    #[ts(type = "string")]
    pub account_id: Id,
    #[ts(optional, type = "string")]
    pub party_id: Option<Id>,
}
