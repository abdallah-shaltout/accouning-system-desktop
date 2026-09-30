//! Business domains (plan 21 Part 03, master plan §4): one module per Vue module, each with
//! `commands.rs` (thin IPC layer) · `service.rs` (logic) · `dto.rs` (ts-rs DTOs = the TS types).
//! Manager-owned: implementers add their domain's `pub mod` line and hook lines through the
//! manager, never directly (entry file 03-DOMAINS-IMPLEMENTATION.md §3.7).

pub mod accounting;
pub mod analytics;
pub mod approvals;
pub mod attachments;
pub mod dashboard;
pub mod diagnostics;
pub mod expenses;
pub mod invoices;
pub mod parties;
pub mod payments;
pub mod products;
pub mod purchases;
pub mod reports;
pub mod settings;
pub mod setup;
pub mod templates;
pub mod users;
pub mod vouchers;

use crate::core::ipc::IpcSig;
use crate::shared::activity::undo::UndoRegistry;

/// Every domain command's `ipc_sig!` line, concatenated — `core::ipc::all_signatures()` appends
/// this so `ipc_manifest_matches_handler` sees each new command next to `generate_handler!`.
pub fn all_ipc_signatures() -> Vec<IpcSig> {
    let mut sigs = Vec::new();
    sigs.extend(crate::infrastructure::import::commands::ipc_signatures());
    sigs.extend(crate::infrastructure::backup::commands::ipc_signatures());
    sigs.extend(settings::ipc_signatures());
    sigs.extend(users::ipc_signatures());
    sigs.extend(templates::ipc_signatures());
    sigs.extend(diagnostics::ipc_signatures());
    sigs.extend(approvals::ipc_signatures());
    sigs.extend(expenses::ipc_signatures());
    sigs.extend(parties::ipc_signatures());
    sigs.extend(products::ipc_signatures());
    sigs.extend(setup::ipc_signatures());
    sigs.extend(vouchers::ipc_signatures());
    sigs.extend(purchases::ipc_signatures());
    sigs.extend(payments::ipc_signatures());
    sigs.extend(invoices::ipc_signatures());
    sigs.extend(accounting::ipc_signatures());
    sigs.extend(reports::ipc_signatures());
    sigs.extend(analytics::ipc_signatures());
    sigs.extend(dashboard::ipc_signatures());
    sigs.extend(attachments::ipc_signatures());
    sigs
}

/// Registers every domain's undo compensators (phase-e E-5: accounting ×4, setup ×1) into the
/// registry `AppState` holds — "scale by adding": one line per domain that has compensators.
pub fn register_undo(registry: &mut UndoRegistry) {
    setup::register_undo(registry);
    accounting::register_undo(registry);
}

/// G-8a: the hook `core::ipc`'s `export_bindings` test calls so each domain's DTO module can add
/// its own `SomeDto::export_all(cfg).expect(...)` line here without ever editing `core/ipc.rs`
/// itself — "scale by adding, not by branching" (CLAUDE.md). Panics (via `.expect`) on failure,
/// same as every existing export line in `core/ipc.rs`'s test — this only runs inside that test.
/// Empty until the first domain lands; each domain's checklist item is exactly one
/// `crate::domains::<name>::dto::SomeDto::export_all(cfg).expect("export SomeDto");` line appended
/// below, in the same commit that adds its `pub mod` line.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    // Shared types several domains' DTOs reference (G-50).
    crate::shared::ledger::accounts::SystemRole::export_all(cfg).expect("export SystemRole");
    // W1 (2026-09-28): the DTOs each module's contract.check.ts imports from its types/gen/.
    crate::infrastructure::import::dto::ImportMode::export_all(cfg).expect("export ImportMode");
    crate::infrastructure::import::dto::LegacySnapshotSummary::export_all(cfg).expect("export LegacySnapshotSummary");
    crate::infrastructure::import::dto::ImportSnapshotResult::export_all(cfg).expect("export ImportSnapshotResult");
    crate::infrastructure::import::dto::SetupInspectLegacySnapshotArgs::export_all(cfg).expect("export SetupInspectLegacySnapshotArgs");
    crate::infrastructure::import::dto::SetupImportSnapshotArgs::export_all(cfg).expect("export SetupImportSnapshotArgs");
    settings::dto::StoreSettings::export_all(cfg).expect("export StoreSettings");
    settings::dto::Tax::export_all(cfg).expect("export Tax");
    settings::dto::PaymentMethod::export_all(cfg).expect("export PaymentMethod");
    settings::dto::PaymentMethodInput::export_all(cfg).expect("export PaymentMethodInput");
    settings::dto::Branch::export_all(cfg).expect("export Branch");
    settings::dto::BranchInput::export_all(cfg).expect("export BranchInput");
    settings::dto::CostCenter::export_all(cfg).expect("export CostCenter");
    settings::dto::CostCenterInput::export_all(cfg).expect("export CostCenterInput");
    settings::dto::CostCenterBudget::export_all(cfg).expect("export CostCenterBudget");
    settings::dto::Currency::export_all(cfg).expect("export Currency");
    settings::dto::ExchangeRate::export_all(cfg).expect("export ExchangeRate");
    settings::dto::ExchangeRateInput::export_all(cfg).expect("export ExchangeRateInput");
    settings::dto::FcBalanceRow::export_all(cfg).expect("export FcBalanceRow");
    settings::dto::RevaluationResult::export_all(cfg).expect("export RevaluationResult");
    settings::dto::PairingInfo::export_all(cfg).expect("export PairingInfo");
    settings::dto::LanSharingStatus::export_all(cfg).expect("export LanSharingStatus");
    // The `Settings*Args`/`Setup*Args`/`Templates*Args`/`Diagnostics*Args` command-argument DTOs
    // below are declared next to their `#[tauri::command]` fn in `commands.rs` (not `dto.rs`), and
    // were missing from this hook — `ipc.gen.ts`'s `IpcCommands` referenced them by name with no
    // `.ts` file ever written, so every command using one failed to type-check (G-8b).
    settings::commands::SettingsUpdateSettingsArgs::export_all(cfg).expect("export SettingsUpdateSettingsArgs");
    settings::commands::SettingsSaveTaxArgs::export_all(cfg).expect("export SettingsSaveTaxArgs");
    settings::commands::SettingsDeleteTaxArgs::export_all(cfg).expect("export SettingsDeleteTaxArgs");
    settings::commands::SettingsSavePaymentMethodArgs::export_all(cfg).expect("export SettingsSavePaymentMethodArgs");
    settings::commands::SettingsReorderPaymentMethodsArgs::export_all(cfg).expect("export SettingsReorderPaymentMethodsArgs");
    settings::commands::SettingsDeletePaymentMethodArgs::export_all(cfg).expect("export SettingsDeletePaymentMethodArgs");
    settings::commands::SettingsCreateBranchArgs::export_all(cfg).expect("export SettingsCreateBranchArgs");
    settings::commands::SettingsUpdateBranchArgs::export_all(cfg).expect("export SettingsUpdateBranchArgs");
    settings::commands::SettingsDeactivateBranchArgs::export_all(cfg).expect("export SettingsDeactivateBranchArgs");
    settings::commands::SettingsReactivateBranchArgs::export_all(cfg).expect("export SettingsReactivateBranchArgs");
    settings::commands::SettingsCreateCostCenterArgs::export_all(cfg).expect("export SettingsCreateCostCenterArgs");
    settings::commands::SettingsUpdateCostCenterArgs::export_all(cfg).expect("export SettingsUpdateCostCenterArgs");
    settings::commands::SettingsDeleteCostCenterArgs::export_all(cfg).expect("export SettingsDeleteCostCenterArgs");
    settings::commands::SettingsGetExchangeRatesArgs::export_all(cfg).expect("export SettingsGetExchangeRatesArgs");
    settings::commands::SettingsCreateCurrencyArgs::export_all(cfg).expect("export SettingsCreateCurrencyArgs");
    settings::commands::SettingsUpdateCurrencyArgs::export_all(cfg).expect("export SettingsUpdateCurrencyArgs");
    settings::commands::SettingsSaveExchangeRateArgs::export_all(cfg).expect("export SettingsSaveExchangeRateArgs");
    settings::commands::SettingsSetBaseCurrencyArgs::export_all(cfg).expect("export SettingsSetBaseCurrencyArgs");
    settings::commands::SettingsGetRevaluationPreviewArgs::export_all(cfg).expect("export SettingsGetRevaluationPreviewArgs");
    settings::commands::SettingsPostRevaluationArgs::export_all(cfg).expect("export SettingsPostRevaluationArgs");
    settings::commands::SettingsDisableLanSharingArgs::export_all(cfg).expect("export SettingsDisableLanSharingArgs");
    users::export_bindings(cfg);
    approvals::export_bindings(cfg);
    expenses::export_bindings(cfg);
    parties::export_bindings(cfg);
    products::export_bindings(cfg);
    crate::infrastructure::backup::dto::BackupManifest::export_all(cfg).expect("export BackupManifest");
    crate::infrastructure::backup::dto::BackupKind::export_all(cfg).expect("export BackupKind");
    crate::infrastructure::backup::dto::BackupSettings::export_all(cfg).expect("export BackupSettings");
    crate::infrastructure::backup::dto::BackupSettingsPatch::export_all(cfg).expect("export BackupSettingsPatch");
    crate::infrastructure::backup::dto::RestorePreview::export_all(cfg).expect("export RestorePreview");
    crate::infrastructure::backup::dto::BackupArchive::export_all(cfg).expect("export BackupArchive");
    crate::infrastructure::backup::dto::AutoBackupOutcome::export_all(cfg).expect("export AutoBackupOutcome");
    crate::infrastructure::backup::dto::AutoBackupTrigger::export_all(cfg).expect("export AutoBackupTrigger");
    crate::infrastructure::backup::dto::AutoBackupSkipReason::export_all(cfg).expect("export AutoBackupSkipReason");
    crate::infrastructure::backup::dto::SettingsSaveBackupSettingsArgs::export_all(cfg).expect("export SettingsSaveBackupSettingsArgs");
    crate::infrastructure::backup::dto::SettingsBuildBackupArchiveArgs::export_all(cfg).expect("export SettingsBuildBackupArchiveArgs");
    crate::infrastructure::backup::dto::SettingsRecordBackupSavedArgs::export_all(cfg).expect("export SettingsRecordBackupSavedArgs");
    crate::infrastructure::backup::dto::SettingsRunAutoBackupIfDueArgs::export_all(cfg).expect("export SettingsRunAutoBackupIfDueArgs");
    crate::infrastructure::backup::dto::SettingsPreviewRestoreArgs::export_all(cfg).expect("export SettingsPreviewRestoreArgs");
    crate::infrastructure::backup::dto::SettingsRestoreFromArchiveArgs::export_all(cfg).expect("export SettingsRestoreFromArchiveArgs");
    setup::dto::OnboardingProgress::export_all(cfg).expect("export OnboardingProgress");
    setup::dto::OnboardingProgressPatch::export_all(cfg).expect("export OnboardingProgressPatch");
    setup::dto::CountryTaxInput::export_all(cfg).expect("export CountryTaxInput");
    setup::dto::PostOpeningBalancesResult::export_all(cfg).expect("export PostOpeningBalancesResult");
    setup::dto::DeviceSetupState::export_all(cfg).expect("export DeviceSetupState");
    setup::dto::PairTerminalInput::export_all(cfg).expect("export PairTerminalInput");
    setup::commands::SetupPairTerminalArgs::export_all(cfg).expect("export SetupPairTerminalArgs");
    setup::commands::SetupSaveOnboardingProgressArgs::export_all(cfg).expect("export SetupSaveOnboardingProgressArgs");
    setup::commands::SetupMarkStepDoneArgs::export_all(cfg).expect("export SetupMarkStepDoneArgs");
    setup::commands::SetupMarkStepSkippedArgs::export_all(cfg).expect("export SetupMarkStepSkippedArgs");
    setup::commands::SetupApplyBusinessTypeDefaultsArgs::export_all(cfg).expect("export SetupApplyBusinessTypeDefaultsArgs");
    setup::commands::SetupApplyCountryTaxArgs::export_all(cfg).expect("export SetupApplyCountryTaxArgs");
    setup::commands::SetupApplyFiscalYearArgs::export_all(cfg).expect("export SetupApplyFiscalYearArgs");
    setup::commands::SetupApplyBranchesArgs::export_all(cfg).expect("export SetupApplyBranchesArgs");
    setup::commands::SetupApplyCoaTemplateArgs::export_all(cfg).expect("export SetupApplyCoaTemplateArgs");
    setup::commands::SetupApplyPaymentMethodsArgs::export_all(cfg).expect("export SetupApplyPaymentMethodsArgs");
    setup::commands::SetupPostOpeningBalancesArgs::export_all(cfg).expect("export SetupPostOpeningBalancesArgs");
    setup::commands::SetupPostOpeningStockArgs::export_all(cfg).expect("export SetupPostOpeningStockArgs");
    setup::commands::SetupRecloseOpeningBalanceEquityArgs::export_all(cfg).expect("export SetupRecloseOpeningBalanceEquityArgs");
    setup::commands::SetupPostPartyOpeningArgs::export_all(cfg).expect("export SetupPostPartyOpeningArgs");
    setup::commands::SetupReversePartyOpeningArgs::export_all(cfg).expect("export SetupReversePartyOpeningArgs");
    templates::dto::DocumentKind::export_all(cfg).expect("export DocumentKind");
    templates::dto::BaseTemplateId::export_all(cfg).expect("export BaseTemplateId");
    templates::dto::PdfTemplate::export_all(cfg).expect("export PdfTemplate");
    templates::dto::TemplatesImportTemplateArgs::export_all(cfg).expect("export TemplatesImportTemplateArgs");
    templates::dto::TemplatesListTemplatesArgs::export_all(cfg).expect("export TemplatesListTemplatesArgs");
    templates::dto::TemplatesGetTemplateArgs::export_all(cfg).expect("export TemplatesGetTemplateArgs");
    templates::dto::TemplatesGetDefaultTemplateArgs::export_all(cfg).expect("export TemplatesGetDefaultTemplateArgs");
    templates::dto::TemplatesSaveTemplateArgs::export_all(cfg).expect("export TemplatesSaveTemplateArgs");
    templates::dto::TemplatesSetAsDefaultArgs::export_all(cfg).expect("export TemplatesSetAsDefaultArgs");
    templates::dto::TemplatesDuplicateTemplateArgs::export_all(cfg).expect("export TemplatesDuplicateTemplateArgs");
    templates::dto::TemplatesDeleteTemplateArgs::export_all(cfg).expect("export TemplatesDeleteTemplateArgs");
    templates::dto::TemplatesResetTemplateToDefaultsArgs::export_all(cfg).expect("export TemplatesResetTemplateToDefaultsArgs");
    templates::dto::TemplatesCreateTemplateArgs::export_all(cfg).expect("export TemplatesCreateTemplateArgs");
    diagnostics::dto::AuditFilter::export_all(cfg).expect("export AuditFilter");
    diagnostics::dto::DiagnosticsGetAuditEntriesArgs::export_all(cfg).expect("export DiagnosticsGetAuditEntriesArgs");
    diagnostics::dto::DiagnosticsExportSupportBundleArgs::export_all(cfg).expect("export DiagnosticsExportSupportBundleArgs");
    diagnostics::dto::ServerDiagnosticsDto::export_all(cfg).expect("export ServerDiagnosticsDto");
    diagnostics::dto::SupportSnapshot::export_all(cfg).expect("export SupportSnapshot");
    diagnostics::dto::AccountingDocSummary::export_all(cfg).expect("export AccountingDocSummary");
    diagnostics::dto::BalanceAround::export_all(cfg).expect("export BalanceAround");
    diagnostics::dto::DriftRowKind::export_all(cfg).expect("export DriftRowKind");
    diagnostics::dto::DriftRow::export_all(cfg).expect("export DriftRow");
    diagnostics::dto::ExplainLine::export_all(cfg).expect("export ExplainLine");
    diagnostics::dto::InvariantDiffDto::export_all(cfg).expect("export InvariantDiffDto");
    diagnostics::dto::InvariantResultDto::export_all(cfg).expect("export InvariantResultDto");
    diagnostics::dto::PostingTraceStepDto::export_all(cfg).expect("export PostingTraceStepDto");
    diagnostics::dto::TraceLineDto::export_all(cfg).expect("export TraceLineDto");
    diagnostics::dto::TraceTotalsDto::export_all(cfg).expect("export TraceTotalsDto");
    diagnostics::dto::PostingTraceDto::export_all(cfg).expect("export PostingTraceDto");
    diagnostics::dto::DiagnosticsListRecentDocumentsArgs::export_all(cfg).expect("export DiagnosticsListRecentDocumentsArgs");
    diagnostics::dto::DiagnosticsGetPostingTraceArgs::export_all(cfg).expect("export DiagnosticsGetPostingTraceArgs");
    diagnostics::dto::DiagnosticsGetJournalEntryRawArgs::export_all(cfg).expect("export DiagnosticsGetJournalEntryRawArgs");
    diagnostics::dto::DiagnosticsGetBalancesAroundArgs::export_all(cfg).expect("export DiagnosticsGetBalancesAroundArgs");
    diagnostics::dto::DiagnosticsExplainAccountBalanceArgs::export_all(cfg).expect("export DiagnosticsExplainAccountBalanceArgs");
    vouchers::export_bindings(cfg);
    purchases::export_bindings(cfg);
    payments::export_bindings(cfg);
    invoices::export_bindings(cfg);
    accounting::export_bindings(cfg);
    reports::export_bindings(cfg);
    analytics::export_bindings(cfg);
    dashboard::export_bindings(cfg);
    attachments::export_bindings(cfg);
}
