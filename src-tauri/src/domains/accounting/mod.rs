//! `accounting` domain (03-domains/12-accounting.md + 12b-period-close.md) — chart of accounts,
//! manual journal, drafts, reversal, journal templates/recurring (12), fiscal years, lock date,
//! year close/reopen, VAT settlement (12b). 30 IPC commands (19 + 11); four undo compensators
//! (phase-e E-5): `accounting.createJournalEntry` (post path), `accounting.postJournalDraft`,
//! `accounting.postRecurringTemplate` (`undo.rs`, 12-owned) and `accounting.closeYear`
//! (`undo_period.rs`, 12b-owned).
//!
//! H-3 note: `dto::{FiscalYear, Account, AccountKind, AccountSubtype, NormalSide}` were created in
//! Part 03 W2 by 02-setup (`setup_apply_fiscal_year`/`setup_apply_coa_template` return them) and
//! are kept in `dto/mod.rs` verbatim — `domains::setup` imports them from here by the same names.

pub mod commands;
pub mod dto;
pub mod service;
pub mod undo;
pub mod undo_period;

use crate::core::ipc::IpcSig;
use crate::ipc_sig;
use crate::shared::activity::undo::UndoRegistry;

/// 12's 19 commands (§1) + 12b's 11 commands (`commands::period::ipc_signatures()`) = 30 total.
pub fn ipc_signatures() -> Vec<IpcSig> {
    let mut sigs = vec![
        ipc_sig!(accounting_get_accounts, dto::AccountingGetAccountsArgs, Vec<dto::AccountWithBalance>),
        ipc_sig!(accounting_save_account, dto::AccountingSaveAccountArgs, dto::Account),
        ipc_sig!(accounting_delete_account, dto::AccountingDeleteAccountArgs, ()),
        ipc_sig!(accounting_reparent_account, dto::AccountingReparentAccountArgs, dto::Account),
        ipc_sig!(accounting_get_journal_entries, dto::AccountingGetJournalEntriesArgs, Vec<dto::JournalRow>),
        ipc_sig!(accounting_get_journal_entries_for_source, dto::AccountingGetJournalEntriesForSourceArgs, Vec<dto::LinkedJournalEntry>),
        ipc_sig!(accounting_get_journal_entries_paged, dto::AccountingGetJournalEntriesPagedArgs, crate::core::dto::PagedResult<dto::JournalRow>),
        ipc_sig!(accounting_get_journal_entry, dto::AccountingGetJournalEntryArgs, dto::JournalEntryDetail),
        ipc_sig!(accounting_create_journal_entry, dto::AccountingCreateJournalEntryArgs, dto::JournalEntry),
        ipc_sig!(accounting_update_journal_draft, dto::AccountingUpdateJournalDraftArgs, dto::JournalEntry),
        ipc_sig!(accounting_post_journal_draft, dto::AccountingPostJournalDraftArgs, dto::JournalEntry),
        ipc_sig!(accounting_delete_journal_draft, dto::AccountingDeleteJournalDraftArgs, ()),
        ipc_sig!(accounting_reverse_journal_entry, dto::AccountingReverseJournalEntryArgs, dto::JournalEntry),
        ipc_sig!(accounting_get_journal_templates, (), Vec<dto::JournalTemplate>),
        ipc_sig!(accounting_get_journal_template, dto::AccountingGetJournalTemplateArgs, dto::JournalTemplate),
        ipc_sig!(accounting_create_or_update_journal_template, dto::AccountingCreateOrUpdateJournalTemplateArgs, dto::JournalTemplate),
        ipc_sig!(accounting_remove_journal_template, dto::AccountingRemoveJournalTemplateArgs, ()),
        ipc_sig!(accounting_load_template_into_entry, dto::AccountingLoadTemplateIntoEntryArgs, dto::JournalTemplate),
        ipc_sig!(accounting_post_recurring_template, dto::AccountingPostRecurringTemplateArgs, dto::JournalEntry),
    ];
    sigs.extend(commands::period::ipc_signatures());
    sigs
}

/// G-8a hook: this domain's DTO exports, called from `domains::export_bindings`
/// (`domains/mod.rs`, manager-owned) in the same commit that added `pub mod accounting;` there.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    dto::FiscalYear::export_all(cfg).expect("export FiscalYear");
    dto::AccountKind::export_all(cfg).expect("export AccountKind");
    dto::AccountSubtype::export_all(cfg).expect("export AccountSubtype");
    dto::NormalSide::export_all(cfg).expect("export NormalSide");
    dto::Account::export_all(cfg).expect("export Account");
    dto::AccountInput::export_all(cfg).expect("export AccountInput");
    dto::AccountWithBalance::export_all(cfg).expect("export AccountWithBalance");
    dto::DateRange::export_all(cfg).expect("export DateRange");
    dto::JournalEntryType::export_all(cfg).expect("export JournalEntryType");
    dto::JournalEntryStatus::export_all(cfg).expect("export JournalEntryStatus");
    dto::JournalSourceKind::export_all(cfg).expect("export JournalSourceKind");
    dto::JournalSourceRef::export_all(cfg).expect("export JournalSourceRef");
    dto::JournalLine::export_all(cfg).expect("export JournalLine");
    dto::JournalEntry::export_all(cfg).expect("export JournalEntry");
    dto::JournalEntryInputLine::export_all(cfg).expect("export JournalEntryInputLine");
    dto::JournalEntryInput::export_all(cfg).expect("export JournalEntryInput");
    dto::JournalFilter::export_all(cfg).expect("export JournalFilter");
    dto::JournalRow::export_all(cfg).expect("export JournalRow");
    dto::JournalEntryDetail::export_all(cfg).expect("export JournalEntryDetail");
    dto::LinkedJournalEntry::export_all(cfg).expect("export LinkedJournalEntry");
    dto::JournalTemplateLine::export_all(cfg).expect("export JournalTemplateLine");
    dto::RecurrenceEvery::export_all(cfg).expect("export RecurrenceEvery");
    dto::JournalTemplateRecurrence::export_all(cfg).expect("export JournalTemplateRecurrence");
    dto::JournalTemplate::export_all(cfg).expect("export JournalTemplate");
    dto::JournalTemplateInput::export_all(cfg).expect("export JournalTemplateInput");
    dto::FiscalYearInput::export_all(cfg).expect("export FiscalYearInput");
    dto::CloseYearPreCheckKey::export_all(cfg).expect("export CloseYearPreCheckKey");
    dto::CloseYearPreCheck::export_all(cfg).expect("export CloseYearPreCheck");
    dto::CloseYearResult::export_all(cfg).expect("export CloseYearResult");
    dto::VatPeriodTotals::export_all(cfg).expect("export VatPeriodTotals");
    dto::AccountingGetAccountsArgs::export_all(cfg).expect("export AccountingGetAccountsArgs");
    dto::AccountingSaveAccountArgs::export_all(cfg).expect("export AccountingSaveAccountArgs");
    dto::AccountingDeleteAccountArgs::export_all(cfg).expect("export AccountingDeleteAccountArgs");
    dto::AccountingReparentAccountArgs::export_all(cfg).expect("export AccountingReparentAccountArgs");
    dto::AccountingGetJournalEntriesArgs::export_all(cfg).expect("export AccountingGetJournalEntriesArgs");
    dto::AccountingGetJournalEntriesForSourceArgs::export_all(cfg).expect("export AccountingGetJournalEntriesForSourceArgs");
    dto::AccountingGetJournalEntriesPagedArgs::export_all(cfg).expect("export AccountingGetJournalEntriesPagedArgs");
    dto::AccountingGetJournalEntryArgs::export_all(cfg).expect("export AccountingGetJournalEntryArgs");
    dto::AccountingCreateJournalEntryArgs::export_all(cfg).expect("export AccountingCreateJournalEntryArgs");
    dto::AccountingUpdateJournalDraftArgs::export_all(cfg).expect("export AccountingUpdateJournalDraftArgs");
    dto::AccountingPostJournalDraftArgs::export_all(cfg).expect("export AccountingPostJournalDraftArgs");
    dto::AccountingDeleteJournalDraftArgs::export_all(cfg).expect("export AccountingDeleteJournalDraftArgs");
    dto::AccountingReverseJournalEntryArgs::export_all(cfg).expect("export AccountingReverseJournalEntryArgs");
    dto::AccountingGetJournalTemplateArgs::export_all(cfg).expect("export AccountingGetJournalTemplateArgs");
    dto::AccountingCreateOrUpdateJournalTemplateArgs::export_all(cfg).expect("export AccountingCreateOrUpdateJournalTemplateArgs");
    dto::AccountingRemoveJournalTemplateArgs::export_all(cfg).expect("export AccountingRemoveJournalTemplateArgs");
    dto::AccountingLoadTemplateIntoEntryArgs::export_all(cfg).expect("export AccountingLoadTemplateIntoEntryArgs");
    dto::AccountingPostRecurringTemplateArgs::export_all(cfg).expect("export AccountingPostRecurringTemplateArgs");
    dto::AccountingSaveFiscalYearArgs::export_all(cfg).expect("export AccountingSaveFiscalYearArgs");
    dto::AccountingSaveLockDateArgs::export_all(cfg).expect("export AccountingSaveLockDateArgs");
    dto::AccountingGetCloseYearPreChecksArgs::export_all(cfg).expect("export AccountingGetCloseYearPreChecksArgs");
    dto::AccountingCloseYearArgs::export_all(cfg).expect("export AccountingCloseYearArgs");
    dto::AccountingReopenYearArgs::export_all(cfg).expect("export AccountingReopenYearArgs");
    dto::AccountingGetVatPeriodTotalsArgs::export_all(cfg).expect("export AccountingGetVatPeriodTotalsArgs");
    dto::AccountingSubmitVatSettlementArgs::export_all(cfg).expect("export AccountingSubmitVatSettlementArgs");
    dto::AccountingPayVatSettlementNowArgs::export_all(cfg).expect("export AccountingPayVatSettlementNowArgs");
}

/// Registers all four accounting undo compensators (phase-e E-5) — called by `domains::register_undo`.
pub fn register_undo(r: &mut UndoRegistry) {
    undo::register_undo(r);
}
