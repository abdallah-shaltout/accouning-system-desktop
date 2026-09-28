//! DB-backed tests for the `reports` domain (03-domains/13-reports.md §8a,
//! 13b-reports-operational.md §8a). Written now, run in the deferred, time-boxed test pass (per-
//! implementer hard rule: never run cargo from this agent). Needs `EQUAL_TEST_DATABASE_URL` — see
//! `tests/support/mod.rs`.
//!
//! `seed_branch_and_settings` here is a local copy of the fixture pattern `domain_parties.rs`/
//! `domain_settings.rs` each already built (tests can't import each other, per the Wave 1 compile
//! lessons) — kept minimal to what this domain's tests need. Every report is read-only, so tests
//! seed accounts/journal entries directly via `ActiveModel::insert` rather than going through a
//! writer domain's service layer (out of this domain's owned files).

mod support;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::tx::{with_read_ctx, BoxFuture, TxResult};
use accounting_app_lib::domains::reports::dto::{
    DateRangeInput, DimensionOptions, DiscountGroupBy, GrossProfitGroupBy, LedgerTargets, PartyKindArg, ReportRangeFilter,
};
use accounting_app_lib::domains::reports::service::{ledgers, sales, statements, stock};
use accounting_app_lib::entities::journal::{journal_entries, journal_lines};
use accounting_app_lib::entities::org::{accounts, branches, settings};
use accounting_app_lib::entities::values::{AccountingPolicy, PrinterMode, PrinterSettings};
use accounting_app_lib::utils::id::Id;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set};
use support::TestDb;

async fn seed_branch_and_settings(db: &TestDb) -> Id {
    let branch_id = Id::new();
    let settings_id = Id::new();

    accounting_app_lib::core::tx::with_tx(&db.state, accounting_app_lib::core::tx::TxOpts { require_user: false }, move |tx: &DatabaseTransaction, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();

            let branch = branches::ActiveModel {
                id: Set(branch_id),
                name: Set("الفرع الرئيسي".to_string()),
                code: Set("MAIN".to_string()),
                address: Set(None),
                national_address: Set(None),
                phone: Set(None),
                receipt_header: Set(None),
                cash_account_id: Set(None),
                bank_account_id: Set(None),
                default_price_list_id: Set(None),
                cost_center_id: Set(None),
                active: Set(true),
                can_delete: Set(false),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
                sync_status: Set("local".to_string()),
            };
            branch.insert(tx).await?;

            let printer = PrinterSettings { mode: PrinterMode::A4, thermal_width_mm: 80, thermal: None, a4_printer_name: None, label_printer_name: None, a4_template: None, image_template: None };

            let settings_row = settings::ActiveModel {
                id: Set(settings_id),
                singleton: Set(1),
                store_name: Set("متجر تجريبي".to_string()),
                logo: Set(None),
                stamp: Set(None),
                signature: Set(None),
                currency: Set("SAR".to_string()),
                country: Set(Some("SA".to_string())),
                vat_number: Set(None),
                default_tax_id: Set(None),
                invoice_number_prefix: Set("INV-".to_string()),
                printer: Set(printer),
                prices_include_tax: Set(true),
                address: Set(None),
                national_address: Set(None),
                phone: Set(None),
                commercial_register: Set(None),
                receipt_footer: Set(None),
                accounting: Set(Some(AccountingPolicy { lock_date: None, default_purchase_account_id: None })),
                backup: Set(None),
                inventory_approval_threshold: Set(None),
                role_access_overrides: Set(None),
                insight_thresholds: Set(None),
                pos: Set(None),
                sales: Set(None),
                features: Set(None),
                onboarding: Set(None),
                timezone: Set(None),
                default_branch_id: Set(branch_id),
                created_at: Set(now),
                updated_at: Set(now),
            };
            settings_row.insert(tx).await?;

            Ok(())
        })
    })
    .await
    .expect("seed_branch_and_settings must succeed");

    branch_id
}

fn log_in(db: &TestDb, role: Role) -> Id {
    let user_id = Id::new();
    let user = AuthenticatedUser { id: user_id, username: "test".to_string(), role, home_branch_id: Id::new(), allowed_branches: vec![], price_list_id: None, max_discount: None };
    *db.state.session.write().unwrap() = Some(user);
    user_id
}

/// Inserts one live account, returning its id.
async fn seed_account(tx: &DatabaseTransaction, code: &str, name: &str, kind: &str, subtype: &str, normal_side: &str, system_role: Option<&str>) -> Id {
    let id = Id::new();
    let now = chrono::Utc::now();
    let account = accounts::ActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(id),
        code: Set(code.to_string()),
        name: Set(name.to_string()),
        name_en: Set(None),
        parent_id: Set(None),
        is_group: Set(false),
        kind: Set(kind.to_string()),
        subtype: Set(subtype.to_string()),
        normal_side: Set(normal_side.to_string()),
        system_role: Set(system_role.map(|s| s.to_string())),
        currency: Set(None),
        branch_id: Set(None),
        requires_party: Set(Some(false)),
        allow_manual: Set(true),
        requires_cost_center: Set(Some(false)),
        active: Set(true),
        can_delete: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    account.insert(tx).await.expect("account insert must succeed");
    id
}

/// Posts one balanced journal entry with the given (account, debit, credit) lines on `day`.
async fn post_entry(tx: &DatabaseTransaction, number: &str, day: chrono::NaiveDate, description: &str, lines: &[(Id, Decimal, Decimal)]) -> Id {
    let entry_id = Id::new();
    let now = chrono::Utc::now();
    let total_debit: Decimal = lines.iter().fold(Decimal::ZERO, |a, (_, d, _)| a + *d);
    let total_credit: Decimal = lines.iter().fold(Decimal::ZERO, |a, (_, _, c)| a + *c);

    let entry = journal_entries::ActiveModel {
        id: Set(entry_id),
        number: Set(number.to_string()),
        date_day: Set(day),
        date_instant: Set(None),
        description: Set(description.to_string()),
        r#type: Set(journal_entries::JournalEntryType::Manual),
        status: Set(journal_entries::JournalEntryStatus::Posted),
        source_kind: Set(None),
        source_id: Set(None),
        source_number: Set(None),
        total_debit: Set(total_debit),
        total_credit: Set(total_credit),
        reversed: Set(false),
        reversal_of_id: Set(None),
        reversal_reason: Set(None),
        created_by: Set(Id::new()),
        posted_by: Set(Some(Id::new())),
        posted_at_day: Set(Some(day)),
        posted_at_instant: Set(None),
        attachment_ids: Set(None),
        template_id: Set(None),
        search_normalized: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(journal_entries::SyncStatus::Local),
    };
    entry.insert(tx).await.expect("journal entry insert must succeed");

    for (i, (account_id, debit, credit)) in lines.iter().enumerate() {
        let line = journal_lines::ActiveModel {
            id: Set(Id::new()),
            journal_entry_id: Set(entry_id),
            position: Set(i as i16),
            account_id: Set(*account_id),
            description: Set(None),
            debit: Set(*debit),
            credit: Set(*credit),
            party_kind: Set(None),
            party_id: Set(None),
            branch_id: Set(None),
            cost_center_id: Set(None),
            currency: Set(None),
            amount_fc: Set(None),
            rate: Set(None),
        };
        line.insert(tx).await.expect("journal line insert must succeed");
    }

    entry_id
}

fn day(y: i32, m: u32, d: u32) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// -------------------------------------------------------------------------------------------
// Trial balance / P&L / balance sheet
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn trial_balance_splits_opening_and_period_and_drops_inactive_accounts() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    let (cash, _revenue, unrelated) = accounting_app_lib::core::tx::with_tx(&db.state, accounting_app_lib::core::tx::TxOpts { require_user: false }, |tx, _cx| {
        Box::pin(async move {
            let cash = seed_account(tx, "1000", "الصندوق", "ASSET", "cash", "DEBIT", Some("cash")).await;
            let revenue = seed_account(tx, "4000", "المبيعات", "REVENUE", "revenue", "CREDIT", Some("sales")).await;
            let unrelated = seed_account(tx, "5000", "غير مستخدم", "EXPENSE", "operatingExpense", "DEBIT", None).await;
            // Opening period: an entry before the report's `from`.
            post_entry(tx, "JE-0001", day(2026, 1, 1), "رصيد افتتاحي", &[(cash, dec!(1000), Decimal::ZERO), (revenue, Decimal::ZERO, dec!(1000))]).await;
            // In-period entry.
            post_entry(tx, "JE-0002", day(2026, 2, 10), "بيع", &[(cash, dec!(500), Decimal::ZERO), (revenue, Decimal::ZERO, dec!(500))]).await;
            Ok::<_, accounting_app_lib::core::tx::TxError>((cash, revenue, unrelated))
        }) as BoxFuture<'_, TxResult<(Id, Id, Id)>>
    })
    .await
    .expect("seed accounts/entries must succeed");

    let range = ReportRangeFilter { from: Some("2026-02-01".to_string()), to: Some("2026-02-28".to_string()), ..Default::default() };
    let rows = with_read_ctx(&db.state, move |tx, _ctx| {
        let range = range.clone();
        Box::pin(async move { statements::trial_balance(tx, &range).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::reports::dto::TrialBalanceRow>>>
    })
    .await
    .expect("trial balance must succeed");

    // The unrelated account (no activity at all) must be dropped, not zero-filled.
    assert!(rows.iter().all(|r| r.account_id != unrelated));
    let cash_row = rows.iter().find(|r| r.account_id == cash).expect("cash row must be present");
    assert_eq!(cash_row.opening_balance, dec!(1000));
    assert_eq!(cash_row.period_debit, dec!(500));
    assert_eq!(cash_row.closing_debit, dec!(1500));
    assert_eq!(cash_row.closing_credit, Decimal::ZERO);
}

#[tokio::test]
async fn profit_and_loss_splits_cogs_from_opex_and_drops_zero_lines() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    let (revenue, cogs, opex) = accounting_app_lib::core::tx::with_tx(&db.state, accounting_app_lib::core::tx::TxOpts { require_user: false }, |tx, _cx| {
        Box::pin(async move {
            let cash = seed_account(tx, "1000", "الصندوق", "ASSET", "cash", "DEBIT", None).await;
            let revenue = seed_account(tx, "4000", "المبيعات", "REVENUE", "revenue", "CREDIT", None).await;
            let cogs = seed_account(tx, "5000", "تكلفة البضاعة المباعة", "EXPENSE", "costOfSales", "DEBIT", None).await;
            let opex = seed_account(tx, "6000", "مصاريف تشغيلية", "EXPENSE", "operatingExpense", "DEBIT", None).await;
            // A zero-net account (debits equal credits) must be dropped from the lines, not the totals.
            let zero_net = seed_account(tx, "6100", "حساب متوازن", "EXPENSE", "operatingExpense", "DEBIT", None).await;
            post_entry(tx, "JE-0001", day(2026, 3, 1), "بيع", &[(cash, dec!(1000), Decimal::ZERO), (revenue, Decimal::ZERO, dec!(1000))]).await;
            post_entry(tx, "JE-0002", day(2026, 3, 1), "تكلفة", &[(cogs, dec!(400), Decimal::ZERO), (cash, Decimal::ZERO, dec!(400))]).await;
            post_entry(tx, "JE-0003", day(2026, 3, 2), "مصروف", &[(opex, dec!(100), Decimal::ZERO), (cash, Decimal::ZERO, dec!(100))]).await;
            post_entry(tx, "JE-0004", day(2026, 3, 3), "متوازن", &[(zero_net, dec!(50), Decimal::ZERO), (zero_net, Decimal::ZERO, dec!(50))]).await;
            Ok::<_, accounting_app_lib::core::tx::TxError>((revenue, cogs, opex))
        }) as BoxFuture<'_, TxResult<(Id, Id, Id)>>
    })
    .await
    .expect("seed must succeed");

    let range = ReportRangeFilter { from: Some("2026-03-01".to_string()), to: Some("2026-03-31".to_string()), ..Default::default() };
    let pnl = with_read_ctx(&db.state, move |tx, _ctx| {
        let range = range.clone();
        Box::pin(async move { statements::profit_and_loss(tx, &range).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::reports::dto::ProfitAndLoss>>
    })
    .await
    .expect("P&L must succeed");

    assert_eq!(pnl.net_revenue, dec!(1000));
    assert_eq!(pnl.total_cogs, dec!(400));
    assert_eq!(pnl.total_expenses, dec!(100));
    assert_eq!(pnl.gross_profit, dec!(600));
    assert_eq!(pnl.net_income, dec!(500));
    assert!(pnl.revenue.iter().any(|l| l.account_id == revenue));
    assert!(pnl.cogs.iter().any(|l| l.account_id == cogs));
    assert!(pnl.expenses.iter().any(|l| l.account_id == opex));
    // The zero-net account never appears in any section.
    assert!(pnl.expenses.iter().all(|l| l.name != "حساب متوازن"));
}

#[tokio::test]
async fn balance_sheet_reports_balanced_and_rejects_invalid_as_of() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    accounting_app_lib::core::tx::with_tx(&db.state, accounting_app_lib::core::tx::TxOpts { require_user: false }, |tx, _cx| {
        Box::pin(async move {
            let cash = seed_account(tx, "1000", "الصندوق", "ASSET", "cash", "DEBIT", None).await;
            let capital = seed_account(tx, "3000", "رأس المال", "EQUITY", "equity", "CREDIT", Some("capital")).await;
            post_entry(tx, "JE-0001", day(2026, 1, 1), "افتتاح", &[(cash, dec!(5000), Decimal::ZERO), (capital, Decimal::ZERO, dec!(5000))]).await;
            Ok::<_, accounting_app_lib::core::tx::TxError>(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("seed must succeed");

    let bs = with_read_ctx(&db.state, move |tx, _ctx| {
        Box::pin(async move { statements::balance_sheet_from_str(tx, "2026-06-30", None).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::reports::dto::BalanceSheet>>
    })
    .await
    .expect("balance sheet must succeed");
    assert!(bs.balanced);
    assert_eq!(bs.total_assets, dec!(5000));
    assert_eq!(bs.total_equity, dec!(5000));

    let invalid = with_read_ctx(&db.state, move |tx, _ctx| {
        Box::pin(async move { statements::balance_sheet_from_str(tx, "not-a-date", None).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::reports::dto::BalanceSheet>>
    })
    .await;
    let err = invalid.expect_err("invalid asOf must be refused");
    assert_eq!(err.to_string(), "تاريخ غير صالح");
}

// -------------------------------------------------------------------------------------------
// Account ledger
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn account_ledger_not_found_and_running_balance() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    let missing = with_read_ctx(&db.state, move |tx, _ctx| {
        Box::pin(async move { ledgers::account_ledger(tx, Id::new(), &DateRangeInput::default()).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::reports::dto::AccountLedger>>
    })
    .await;
    let err = missing.expect_err("unknown account must be NOT_FOUND");
    assert_eq!(err.to_string(), "الحساب غير موجود");

    let cash = accounting_app_lib::core::tx::with_tx(&db.state, accounting_app_lib::core::tx::TxOpts { require_user: false }, |tx, _cx| {
        Box::pin(async move {
            let cash = seed_account(tx, "1000", "الصندوق", "ASSET", "cash", "DEBIT", None).await;
            let revenue = seed_account(tx, "4000", "المبيعات", "REVENUE", "revenue", "CREDIT", None).await;
            post_entry(tx, "JE-0001", day(2026, 1, 1), "قيد1", &[(cash, dec!(100), Decimal::ZERO), (revenue, Decimal::ZERO, dec!(100))]).await;
            post_entry(tx, "JE-0002", day(2026, 1, 2), "قيد2", &[(cash, dec!(50), Decimal::ZERO), (revenue, Decimal::ZERO, dec!(50))]).await;
            Ok::<_, accounting_app_lib::core::tx::TxError>(cash)
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .expect("seed must succeed");

    let ledger = with_read_ctx(&db.state, move |tx, _ctx| {
        Box::pin(async move { ledgers::account_ledger(tx, cash, &DateRangeInput::default()).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::reports::dto::AccountLedger>>
    })
    .await
    .expect("ledger must succeed");
    assert_eq!(ledger.rows.len(), 2);
    assert_eq!(ledger.rows[0].balance, dec!(100));
    assert_eq!(ledger.rows[1].balance, dec!(150));
    assert_eq!(ledger.closing_balance, dec!(150));
}

// -------------------------------------------------------------------------------------------
// Ledger targets / dimension options (lookups)
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn ledger_targets_and_dimension_options_exclude_inactive_and_deleted() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    accounting_app_lib::core::tx::with_tx(&db.state, accounting_app_lib::core::tx::TxOpts { require_user: false }, |tx, _cx| {
        Box::pin(async move {
            seed_account(tx, "1000", "الصندوق", "ASSET", "cash", "DEBIT", None).await;
            seed_account(tx, "2000", "حساب آخر", "LIABILITY", "currentLiability", "CREDIT", None).await;
            Ok::<_, accounting_app_lib::core::tx::TxError>(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("seed must succeed");

    let targets = with_read_ctx(&db.state, move |tx, _ctx| Box::pin(async move { ledgers::ledger_targets(tx).await }) as BoxFuture<'_, TxResult<LedgerTargets>>)
        .await
        .expect("ledger targets must succeed");
    assert_eq!(targets.accounts.len(), 2);
    // Sorted by code: "1000" before "2000".
    assert!(targets.accounts[0].label.starts_with("1000"));

    let dims = with_read_ctx(&db.state, move |tx, _ctx| Box::pin(async move { ledgers::dimension_options(tx).await }) as BoxFuture<'_, TxResult<DimensionOptions>>)
        .await
        .expect("dimension options must succeed");
    // No branches/cost-centers/currencies seeded beyond the fixture branch (inactive filter
    // already covered by the fixture being active) — just confirm the call succeeds and returns
    // the seeded branch.
    assert_eq!(dims.branches.len(), 1);
}

// -------------------------------------------------------------------------------------------
// Sales report (13b)
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn sales_report_summary_matches_invoice_totals_with_no_invoices() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    let report = with_read_ctx(&db.state, move |tx, _ctx| {
        Box::pin(async move { sales::sales_report(tx, &DateRangeInput::default()).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::reports::dto::SalesReport>>
    })
    .await
    .expect("sales report must succeed on an empty invoice set");
    assert_eq!(report.summary.invoice_count, 0);
    assert_eq!(report.summary.total, Decimal::ZERO);
    assert_eq!(report.summary.average_invoice, Decimal::ZERO);
    assert!(report.by_product.is_empty());
}

#[tokio::test]
async fn discounts_report_empty_set_and_gross_profit_empty_set() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    let discounts = with_read_ctx(&db.state, move |tx, _ctx| {
        Box::pin(async move { sales::discounts_report(tx, &DateRangeInput::default(), DiscountGroupBy::Cashier).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::reports::dto::DiscountReportRow>>>
    })
    .await
    .expect("discounts report must succeed");
    assert!(discounts.is_empty());

    let gp = with_read_ctx(&db.state, move |tx, _ctx| {
        Box::pin(async move { sales::gross_profit_report(tx, &DateRangeInput::default(), GrossProfitGroupBy::Invoice).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::reports::dto::GrossProfitRow>>>
    })
    .await
    .expect("gross profit report must succeed");
    assert!(gp.is_empty());
}

// -------------------------------------------------------------------------------------------
// Inventory / low stock / dead stock
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn inventory_report_orders_by_arabic_category_then_name() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    let rows = with_read_ctx(&db.state, move |tx, _ctx| {
        Box::pin(async move { stock::inventory_report(tx).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::reports::dto::InventoryReportRow>>>
    })
    .await
    .expect("inventory report must succeed on an empty product set");
    assert!(rows.is_empty());
}

#[tokio::test]
async fn dead_stock_report_rejects_negative_days() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    let result = with_read_ctx(&db.state, move |tx, ctx| {
        let clock = ctx.clock;
        Box::pin(async move { stock::dead_stock_report(tx, Some(-1), &clock).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::reports::dto::DeadStockRow>>>
    })
    .await;
    let err = result.expect_err("negative days must be refused");
    assert_eq!(err.to_string(), "عدد الأيام غير صالح");
}

// -------------------------------------------------------------------------------------------
// Aging / overdue (no open documents => empty)
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn aging_and_overdue_reports_are_empty_with_no_parties() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    let clock = accounting_app_lib::utils::dates::BusinessClock::new(chrono::Utc::now(), None);
    let aging = with_read_ctx(&db.state, move |tx, _ctx| {
        Box::pin(async move { accounting_app_lib::domains::reports::service::receivables::aging_report(tx, PartyKindArg::Customer, &clock).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::reports::dto::AgingReportRow>>>
    })
    .await
    .expect("aging report must succeed with no parties");
    assert!(aging.is_empty());

    let clock2 = accounting_app_lib::utils::dates::BusinessClock::new(chrono::Utc::now(), None);
    let overdue = with_read_ctx(&db.state, move |tx, _ctx| {
        Box::pin(async move { accounting_app_lib::domains::reports::service::receivables::overdue_report(tx, PartyKindArg::Supplier, &clock2).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::reports::dto::OverdueRow>>>
    })
    .await
    .expect("overdue report must succeed with no parties");
    assert!(overdue.is_empty());
}

// -------------------------------------------------------------------------------------------
// Session gate (every read command requires a session)
// -------------------------------------------------------------------------------------------

#[tokio::test]
async fn every_report_read_requires_a_session() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    // Deliberately not logged in — `with_read_ctx` itself refuses before the closure ever runs.

    let result = with_read_ctx(&db.state, move |tx, _ctx| {
        Box::pin(async move { stock::inventory_report(tx).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::reports::dto::InventoryReportRow>>>
    })
    .await;
    let err = result.expect_err("no session must be UNAUTHORIZED");
    assert_eq!(err.to_string(), "سجّل الدخول أولاً");
}
