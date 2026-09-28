//! `diagnostics` domain (plan 21 Part 03 §16). Slice A (W1): the 3 production `port` functions —
//! `getAuditEntries`, `getAuditEntities` (+ its `getAuditEntityKinds` alias, same command),
//! `exportSupportBundle` (data part only, D-1). Slice A2: the support-bundle DB snapshot (folded
//! into `service::support`, after 17-backup landed). Slice B: the 7 debug-build-only
//! accounting-debugger reads (`service::debugger`, after 12-accounting) — registered in every
//! build (D-4: each command's body refuses at runtime in a release build via `commands::debug_only`,
//! so `ipc_manifest_matches_handler` keeps one handler list with no build-profile branching).

pub mod commands;
pub mod dto;
pub mod service;

use crate::core::dto::AuditEntry;
use crate::core::ipc::IpcSig;
use crate::domains::accounting::dto::JournalEntry;
use crate::ipc_sig;

/// Slice A's 3 + slice B's 7 = 10 commands (spec §1/§9 checklist). No undo anywhere in this domain
/// (read-only — no `register_undo` export).
pub fn ipc_signatures() -> Vec<IpcSig> {
    vec![
        ipc_sig!(diagnostics_get_audit_entries, dto::DiagnosticsGetAuditEntriesArgs, Vec<AuditEntry>),
        ipc_sig!(diagnostics_get_audit_entities, (), Vec<String>),
        ipc_sig!(diagnostics_export_support_bundle, dto::DiagnosticsExportSupportBundleArgs, dto::SupportSnapshot),
        ipc_sig!(diagnostics_list_recent_documents, dto::DiagnosticsListRecentDocumentsArgs, Vec<dto::AccountingDocSummary>),
        ipc_sig!(diagnostics_get_posting_trace, dto::DiagnosticsGetPostingTraceArgs, Option<dto::PostingTraceDto>),
        ipc_sig!(diagnostics_get_journal_entry_raw, dto::DiagnosticsGetJournalEntryRawArgs, Option<JournalEntry>),
        ipc_sig!(diagnostics_get_balances_around, dto::DiagnosticsGetBalancesAroundArgs, Vec<dto::BalanceAround>),
        ipc_sig!(diagnostics_get_invariant_results, (), Vec<dto::InvariantResultDto>),
        ipc_sig!(diagnostics_get_drift_report, (), Vec<dto::DriftRow>),
        ipc_sig!(diagnostics_explain_account_balance, dto::DiagnosticsExplainAccountBalanceArgs, Vec<dto::ExplainLine>),
    ]
}

// NOTE for the manager: this domain's DTO exports are currently inlined directly in
// `domains/mod.rs`'s `export_bindings` (lines calling `diagnostics::dto::AuditFilter::export_all`,
// … `SupportSnapshot::export_all`) rather than through a per-domain `export_bindings(cfg)` hook —
// that file is manager-owned and out of this implementer's edit scope, so no such hook is added
// here (unlike `accounting`/`vouchers`/etc., which do have one). Slice B needs 10 more `export_all`
// lines added next to the existing 4, following the same inline convention: `AccountingDocSummary`,
// `BalanceAround`, `DriftRowKind`, `DriftRow`, `ExplainLine`, `InvariantDiffDto`,
// `InvariantResultDto`, `PostingTraceStepDto`, `TraceLineDto`, `TraceTotalsDto`, `PostingTraceDto`
// (11 total — `DriftRowKind` is also new). See this implementer's final report for the exact lines.
