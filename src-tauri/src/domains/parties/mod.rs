//! `domains::parties` (03-domains/05-parties.md): customers/suppliers over the single `parties`
//! table (P2-15) — codes, groups, statements/balances/aging (always computed on read via
//! `shared::balances`, never stored), history, "both roles" linking. 15 commands (§1); not
//! undoable via the registry (§5).

pub mod commands;
pub mod dto;
pub mod service;

pub fn ipc_signatures() -> Vec<crate::core::ipc::IpcSig> {
    commands::ipc_signatures()
}

/// G-8a: this domain's DTO exports — the manager calls this one line from
/// `domains::export_bindings` (`domains/mod.rs`) in the same commit that adds `pub mod parties;`
/// there, per this checklist item in `03-domains/05-parties.md` §9.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    dto::PartyKind::export_all(cfg).expect("export PartyKind");
    dto::PartyPhone::export_all(cfg).expect("export PartyPhone");
    dto::PartyContact::export_all(cfg).expect("export PartyContact");
    dto::PartyBankInfo::export_all(cfg).expect("export PartyBankInfo");
    dto::OpeningBalanceStub::export_all(cfg).expect("export OpeningBalanceStub");
    dto::NationalAddressDto::export_all(cfg).expect("export NationalAddressDto");
    dto::Customer::export_all(cfg).expect("export Customer");
    dto::Supplier::export_all(cfg).expect("export Supplier");
    dto::CustomerInput::export_all(cfg).expect("export CustomerInput");
    dto::SupplierInput::export_all(cfg).expect("export SupplierInput");
    dto::PartyPhoneInput::export_all(cfg).expect("export PartyPhoneInput");
    dto::PartyGroup::export_all(cfg).expect("export PartyGroup");
    dto::PartyStatementRow::export_all(cfg).expect("export PartyStatementRow");
    dto::AgingBucketKey::export_all(cfg).expect("export AgingBucketKey");
    dto::AgingDocument::export_all(cfg).expect("export AgingDocument");
    dto::AgingBucket::export_all(cfg).expect("export AgingBucket");
    dto::PartyHistoryEntry::export_all(cfg).expect("export PartyHistoryEntry");
    dto::PartyFilter::export_all(cfg).expect("export PartyFilter");
    dto::DuplicateWarning::export_all(cfg).expect("export DuplicateWarning");
    dto::DuplicateCheckInput::export_all(cfg).expect("export DuplicateCheckInput");
    dto::PartiesCheckDuplicatesArgs::export_all(cfg).expect("export PartiesCheckDuplicatesArgs");
    dto::PartiesGetPartyGroupsArgs::export_all(cfg).expect("export PartiesGetPartyGroupsArgs");
    dto::PartiesGetCustomersArgs::export_all(cfg).expect("export PartiesGetCustomersArgs");
    dto::PartiesGetCustomerArgs::export_all(cfg).expect("export PartiesGetCustomerArgs");
    dto::PartiesSaveCustomerArgs::export_all(cfg).expect("export PartiesSaveCustomerArgs");
    dto::PartiesGetCustomerStatementArgs::export_all(cfg).expect("export PartiesGetCustomerStatementArgs");
    dto::PartiesGetSuppliersArgs::export_all(cfg).expect("export PartiesGetSuppliersArgs");
    dto::PartiesGetSupplierArgs::export_all(cfg).expect("export PartiesGetSupplierArgs");
    dto::PartiesSaveSupplierArgs::export_all(cfg).expect("export PartiesSaveSupplierArgs");
    dto::PartiesGetSupplierStatementArgs::export_all(cfg).expect("export PartiesGetSupplierStatementArgs");
    dto::PartiesLinkPartyRecordsArgs::export_all(cfg).expect("export PartiesLinkPartyRecordsArgs");
    dto::PartiesUnlinkPartyRecordArgs::export_all(cfg).expect("export PartiesUnlinkPartyRecordArgs");
    dto::PartiesGetLinkedNetBalanceArgs::export_all(cfg).expect("export PartiesGetLinkedNetBalanceArgs");
    dto::PartiesGetPartyHistoryArgs::export_all(cfg).expect("export PartiesGetPartyHistoryArgs");
    dto::PartiesGetPartyAgingArgs::export_all(cfg).expect("export PartiesGetPartyAgingArgs");
    // `OptionalMoney` is a manual `TS` impl with no `.export_all()` (no `decl`/dependencies to
    // write) — nothing to call here; its shape (`number | null`) is inlined wherever it's used.
}
