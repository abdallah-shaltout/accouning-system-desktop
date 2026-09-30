//! One test binary for every integration suite in `tests/*.rs` (except the two bundled-server
//! suites, which own their own MariaDB processes and stay separate — see Cargo.toml). Linking the
//! app once instead of once per file is what keeps `cargo test` fast. Run one suite with a module
//! filter, e.g. `cargo test --manifest-path src-tauri/Cargo.toml --test all domain_invoices::`.

#[path = "../support/mod.rs"]
mod support;

#[path = "../architecture_rules.rs"]
mod architecture_rules;
#[path = "../db_deadlock_retry.rs"]
mod db_deadlock_retry;
#[path = "../db_entities_documents.rs"]
mod db_entities_documents;
#[path = "../db_entities_foundation.rs"]
mod db_entities_foundation;
#[path = "../db_entities_search.rs"]
mod db_entities_search;
#[path = "../db_foundation.rs"]
mod db_foundation;
#[path = "../domain_accounting.rs"]
mod domain_accounting;
#[path = "../domain_accounting_period.rs"]
mod domain_accounting_period;
#[path = "../domain_analytics.rs"]
mod domain_analytics;
#[path = "../domain_approvals.rs"]
mod domain_approvals;
#[path = "../domain_attachments.rs"]
mod domain_attachments;
#[path = "../domain_backup.rs"]
mod domain_backup;
#[path = "../domain_diagnostics.rs"]
mod domain_diagnostics;
#[path = "../domain_expenses.rs"]
mod domain_expenses;
#[path = "../domain_import.rs"]
mod domain_import;
#[path = "../domain_invoices.rs"]
mod domain_invoices;
#[path = "../domain_parties.rs"]
mod domain_parties;
#[path = "../domain_payments.rs"]
mod domain_payments;
#[path = "../domain_products.rs"]
mod domain_products;
#[path = "../domain_purchases.rs"]
mod domain_purchases;
#[path = "../domain_reports.rs"]
mod domain_reports;
#[path = "../domain_settings.rs"]
mod domain_settings;
#[path = "../domain_setup.rs"]
mod domain_setup;
#[path = "../domain_templates.rs"]
mod domain_templates;
#[path = "../domain_users.rs"]
mod domain_users;
#[path = "../domain_vouchers.rs"]
mod domain_vouchers;
#[path = "../shared_activity.rs"]
mod shared_activity;
#[path = "../shared_currency.rs"]
mod shared_currency;
#[path = "../shared_invariants.rs"]
mod shared_invariants;
#[path = "../shared_ledger.rs"]
mod shared_ledger;
#[path = "../shared_stock.rs"]
mod shared_stock;
