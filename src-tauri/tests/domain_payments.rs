//! DB-backed tests for the `payments` domain (03-domains/09-payments.md §8a). Written now, run in
//! the deferred time-boxed test pass (per-implementer hard rule: never run cargo from this agent).
//! Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.

use crate::support;

use std::sync::Arc;

use chrono::Datelike;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::error::AppError;
use accounting_app_lib::core::tx::{with_tx, BoxFuture, TxCtx, TxOpts};
use accounting_app_lib::entities::journal::journal_entries::JournalEntryType;
use accounting_app_lib::entities::journal::journal_lines::PartyKind as JournalPartyKind;
use accounting_app_lib::shared::ledger::accounts::SystemRole;
use accounting_app_lib::shared::ledger::{post, AccountRef, PartyRef, PostJournal, PostingLine, SourceRef};
use accounting_app_lib::domains::payments::dto::{
    AllocationInputTargetKind, AllocationStatus, PartyKind, PaymentAllocationInput, PaymentInput, PaymentTenderKind, PaymentTypeDto,
};
use accounting_app_lib::domains::payments::service;
use accounting_app_lib::entities::org::{accounts, branches, fiscal_years, settings};
use accounting_app_lib::entities::parties::parties;
use accounting_app_lib::entities::sales::invoices;
use accounting_app_lib::entities::purchases::purchase_orders;
use accounting_app_lib::entities::values::{AccountingPolicy, PrinterMode, PrinterSettings};
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, Set};
use support::TestDb;

/// Logs in an admin session whose `users` row exists (FK target of `journal_entries.created_by`,
/// `audit.user_id`, documents' `created_by`/`cashier_id`, …).
async fn log_in(test_db: &TestDb) -> Id {
    let user_id = Id::new();
    let user = AuthenticatedUser {
        id: user_id,
        username: "test".to_string(),
        role: Role::Admin,
        home_branch_id: Id::new(),
        allowed_branches: vec![],
        price_list_id: None,
        max_discount: None,
    };
    let connection = test_db.state.db.read().unwrap().as_ref().expect("test db connected").connection.clone();
    support::seed_user(&connection, user_id, "admin").await;
    *test_db.state.session.write().unwrap() = Some(user);
    user_id
}

/// Fixture: one branch + settings row (base currency SAR), an open fiscal year covering today, the
/// role accounts `receivable`/`payable`/`cash`/`bank`/`fxGain`/`fxLoss`, and one customer + one
/// supplier party.
struct Fixture {
    pub branch_id: Id,
    pub customer_id: Id,
    pub supplier_id: Id,
}

async fn seed_role_account<C: ConnectionTrait>(conn: &C, code: &str, role: &str, currency: Option<&str>) -> Id {
    let id = Id::new();
    let now = chrono::Utc::now();
    let account = accounts::ActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(id),
        code: Set(code.to_string()),
        name: Set(format!("حساب {code}")),
        name_en: Set(None),
        parent_id: Set(None),
        is_group: Set(false),
        kind: Set("ASSET".to_string()),
        subtype: Set("cash".to_string()),
        normal_side: Set("DEBIT".to_string()),
        system_role: Set(Some(role.to_string())),
        currency: Set(currency.map(|c| c.to_string())),
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
    account.insert(conn).await.unwrap();
    id
}

async fn seed_fixture(conn: &DatabaseTransaction) -> Fixture {
    let now = chrono::Utc::now();
    let branch_id = Id::new();
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
    branch.insert(conn).await.unwrap();

    support::seed_currency(conn, "SAR").await;
    support::seed_currency(conn, "USD").await; // the FC tests tag invoices/receipts in USD
    let settings_row = settings::ActiveModel {
        id: Set(Id::new()),
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
        printer: Set(PrinterSettings {
            mode: PrinterMode::A4,
            thermal_width_mm: 80,
            thermal: None,
            a4_printer_name: None,
            label_printer_name: None,
            a4_template: None,
            image_template: None,
        }),
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
    settings_row.insert(conn).await.unwrap();

    let today = chrono::Utc::now().date_naive();
    let fiscal_year = fiscal_years::ActiveModel {
        id: Set(Id::new()),
        name: Set("2026".to_string()),
        start_date: Set(today.with_day(1).unwrap().with_month(1).unwrap()),
        end_date: Set(today.with_day(31).unwrap_or(today).with_month(12).unwrap_or(today)),
        is_closed: Set(false),
        closing_entry_id: Set(None),
        closed_at: Set(None),
        closed_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    fiscal_year.insert(conn).await.unwrap();

    let _receivable_id = seed_role_account(conn, "1200", "receivable", None).await;
    let _payable_id = seed_role_account(conn, "2100", "payable", None).await;
    let _cash_id = seed_role_account(conn, "1110", "cash", None).await;
    let _bank_id = seed_role_account(conn, "1120", "bank", None).await;
    let _fx_gain_id = seed_role_account(conn, "4900", "fxGain", None).await;
    let _fx_loss_id = seed_role_account(conn, "5900", "fxLoss", None).await;
    // The seeded invoices/POs post their own AR/AP entries (see `post_document_entry`) against these.
    let _sales_id = seed_role_account(conn, "4100", "sales", None).await;
    let _freight_in_id = seed_role_account(conn, "5200", "freightIn", None).await;

    let customer_id = Id::new();
    let customer = parties::ActiveModel {
        id: Set(customer_id),
        kind: Set("customer".to_string()),
        r#type: Set("individual".to_string()),
        name: Set("عميل تجريبي".to_string()),
        name_en: Set(None),
        code: Set("C-0001".to_string()),
        group_id: Set(None),
        tags: Set(None),
        active: Set(true),
        phone: Set(None),
        email: Set(None),
        contacts: Set(None),
        address: Set(None),
        national_address: Set(None),
        structured_address: Set(None),
        vat_number: Set(None),
        cr_number: Set(None),
        national_id: Set(None),
        currency: Set(None),
        price_list_id: Set(None),
        payment_terms_days: Set(None),
        salesperson_id: Set(None),
        branch_id: Set(None),
        bank: Set(None),
        opening_balance: Set(None),
        notes: Set(None),
        linked_party_id: Set(None),
        credit_limit: Set(None),
        contact_person: Set(None),
        default_expense_account_id: Set(None),
        search_normalized: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    customer.insert(conn).await.unwrap();

    let supplier_id = Id::new();
    let supplier = parties::ActiveModel {
        id: Set(supplier_id),
        kind: Set("supplier".to_string()),
        r#type: Set("company".to_string()),
        name: Set("مورد تجريبي".to_string()),
        name_en: Set(None),
        code: Set("S-0001".to_string()),
        group_id: Set(None),
        tags: Set(None),
        active: Set(true),
        phone: Set(None),
        email: Set(None),
        contacts: Set(None),
        address: Set(None),
        national_address: Set(None),
        structured_address: Set(None),
        vat_number: Set(None),
        cr_number: Set(None),
        national_id: Set(None),
        currency: Set(None),
        price_list_id: Set(None),
        payment_terms_days: Set(None),
        salesperson_id: Set(None),
        branch_id: Set(None),
        bank: Set(None),
        opening_balance: Set(None),
        notes: Set(None),
        linked_party_id: Set(None),
        credit_limit: Set(None),
        contact_person: Set(None),
        default_expense_account_id: Set(None),
        search_normalized: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    supplier.insert(conn).await.unwrap();

    Fixture { branch_id, customer_id, supplier_id }
}

#[allow(clippy::too_many_arguments)]
async fn seed_invoice<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    customer_id: Id,
    number: &str,
    grand_total: Decimal,
    currency: Option<&str>,
    exchange_rate: Option<Decimal>,
) -> Id {
    // `invoices.cashier_id` FKs to `users`.
    let cashier_id = Id::new();
    support::seed_user(conn, cashier_id, "cashier").await;
    let id = Id::new();
    let now = chrono::Utc::now();
    let invoice = invoices::ActiveModel {
        id: Set(id),
        number: Set(number.to_string()),
        date_day: Set(now.date_naive()),
        date_instant: Set(None),
        customer_id: Set(Some(customer_id)),
        cashier_id: Set(cashier_id),
        status: Set(invoices::InvoiceStatus::Completed),
        payment_status: Set(invoices::PaymentStatus::Unpaid),
        sub_total: Set(grand_total),
        discount_rate: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        tax_rate: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        grand_total: Set(grand_total),
        payment_method: Set(invoices::SalePaymentMethod::Cash),
        paid_amount: Set(Decimal::ZERO),
        refunded_amount: Set(Decimal::ZERO),
        tendered_amount: Set(None),
        due_date_day: Set(None),
        due_date_instant: Set(None),
        note: Set(None),
        source: Set(Some(invoices::InvoiceSource::Desk)),
        shift_id: Set(None),
        branch_id: Set(None),
        invoice_type: Set(Some(invoices::InvoiceType::Standard)),
        po_reference: Set(None),
        terms: Set(None),
        attachment_ids: Set(None),
        currency: Set(currency.map(|c| c.to_string())),
        exchange_rate: Set(exchange_rate),
        cost_center_id: Set(None),
        search_normalized: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(invoices::SyncStatus::Local),
    };
    invoice.insert(conn).await.unwrap();
    // Dr receivable (customer, FC-tagged at the invoice's rate) / Cr sales: without the invoice's own
    // posting the ledger would not carry the receivable its outstanding claims (`customer-allocation`).
    let fc = currency.zip(exchange_rate).map(|(c, rate)| (c.to_string(), grand_total, rate));
    let base = fc.as_ref().map(|(_, amount, rate)| accounting_app_lib::utils::money::round2(*amount * *rate)).unwrap_or(grand_total);
    post_document_entry(conn, cx, "invoice", id, number, SystemRole::Receivable, SystemRole::Sales, base, (JournalPartyKind::Customer, customer_id), fc).await;
    id
}

/// Posts a directly-seeded document's own entry: the party line on `party_role` (AR debit / AP
/// credit), the other side on `other_role`, FC-tagged when `fc = (currency, amount_fc, rate)`.
#[allow(clippy::too_many_arguments)]
async fn post_document_entry<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    source_kind: &str,
    source_id: Id,
    number: &str,
    party_role: SystemRole,
    other_role: SystemRole,
    base: Decimal,
    party: (JournalPartyKind, Id),
    fc: Option<(String, Decimal, Decimal)>,
) {
    let is_receivable = party_role == SystemRole::Receivable;
    let mut party_line =
        if is_receivable { PostingLine::debit(AccountRef::Role(party_role), base) } else { PostingLine::credit(AccountRef::Role(party_role), base) };
    party_line.party = Some(PartyRef { kind: party.0, id: party.1 });
    if let Some((currency, amount_fc, rate)) = fc {
        party_line.currency = Some(currency);
        party_line.amount_fc = Some(amount_fc);
        party_line.rate = Some(rate);
    }
    let other_line =
        if is_receivable { PostingLine::credit(AccountRef::Role(other_role), base) } else { PostingLine::debit(AccountRef::Role(other_role), base) };
    let mut req = PostJournal::new(cx.clock.today(), format!("{source_kind} {number}"), JournalEntryType::System, vec![party_line, other_line]);
    req.source = Some(SourceRef { kind: source_kind.to_string(), id: source_id, number: Some(number.to_string()) });
    post(conn, cx, req).await.unwrap();
}

async fn seed_purchase_order<C: ConnectionTrait>(conn: &C, cx: &TxCtx, supplier_id: Id, number: &str, grand_total: Decimal) -> Id {
    let id = Id::new();
    let now = chrono::Utc::now();
    let po = purchase_orders::ActiveModel {
        id: Set(id),
        number: Set(number.to_string()),
        supplier_id: Set(supplier_id),
        date_day: Set(now.date_naive()),
        date_instant: Set(None),
        status: Set(purchase_orders::PurchaseStatus::Received),
        sub_total: Set(grand_total),
        tax_rate: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        grand_total: Set(grand_total),
        payment_status: Set(purchase_orders::PaymentStatus::Unpaid),
        paid_amount: Set(Decimal::ZERO),
        returned_amount: Set(Decimal::ZERO),
        note: Set(None),
        invoice_discount_pct: Set(None),
        invoice_discount_amount: Set(None),
        landed_costs: Set(None),
        supplier_invoice_no: Set(None),
        supplier_invoice_date: Set(None),
        vat_not_recoverable: Set(None),
        sent_at: Set(None),
        backorder_of_id: Set(None),
        received_date_day: Set(Some(now.date_naive())),
        received_date_instant: Set(Some(now)),
        attachment_ids: Set(None),
        cost_center_id: Set(None),
        branch_id: Set(None),
        currency: Set(None),
        exchange_rate: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(purchase_orders::SyncStatus::Local),
    };
    po.insert(conn).await.unwrap();
    // Cr payable (supplier) / Dr freightIn as a stand-in for the received goods: the fixture PO has
    // no product lines, so debiting inventory would break `inventory-gl`.
    post_document_entry(conn, cx, "purchaseOrder", id, number, SystemRole::Payable, SystemRole::FreightIn, grand_total, (JournalPartyKind::Supplier, supplier_id), None).await;
    id
}

#[tokio::test]
async fn create_payment_receipt_no_allocation_posts_unallocated() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let (fixture_customer_id, payment) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Received,
                target_type: PartyKind::Customer,
                target_id: fixture.customer_id,
                amount: dec!(500),
                method: PaymentTenderKind::Cash,
                note: None,
                allocations: None,
                branch_id: Some(fixture.branch_id),
                currency: None,
                amount_fc: None,
                rate: None,
            };
            let payment = service::create::create_payment(tx, cx, &undo, input).await?;
            Ok((fixture.customer_id, payment))
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<(Id, accounting_app_lib::domains::payments::dto::Payment)>>
    })
    .await
    .unwrap();

    assert_eq!(payment.amount, dec!(500));
    assert!(payment.allocations.is_empty());
    assert_eq!(payment.target_id, fixture_customer_id);

    let row = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move { service::read::get_payment(tx, payment.id).await }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::payments::dto::PaymentRow>>
    })
    .await
    .unwrap();
    assert_eq!(row.unallocated, dec!(500));
    assert_eq!(row.allocation_status, AllocationStatus::Unallocated);

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let report = invariants::run_all(tx).await.unwrap();
            assert!(report.iter().all(|r| r.passed), "invariants failed: {report:?}");
            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn create_payment_receipt_allocated_to_two_invoices_updates_paid_amount() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let (invoice1, invoice2, payment) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let invoice1 = seed_invoice(tx, cx, fixture.customer_id, "INV-0001", dec!(300), None, None).await;
            let invoice2 = seed_invoice(tx, cx, fixture.customer_id, "INV-0002", dec!(200), None, None).await;

            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Received,
                target_type: PartyKind::Customer,
                target_id: fixture.customer_id,
                amount: dec!(500),
                method: PaymentTenderKind::Cash,
                note: None,
                allocations: Some(vec![
                    PaymentAllocationInput { target_kind: AllocationInputTargetKind::Invoice, target_id: invoice1, amount: dec!(300) },
                    PaymentAllocationInput { target_kind: AllocationInputTargetKind::Invoice, target_id: invoice2, amount: dec!(200) },
                ]),
                branch_id: Some(fixture.branch_id),
                currency: None,
                amount_fc: None,
                rate: None,
            };
            let payment = service::create::create_payment(tx, cx, &undo, input).await?;
            Ok((invoice1, invoice2, payment))
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<(Id, Id, accounting_app_lib::domains::payments::dto::Payment)>>
    })
    .await
    .unwrap();

    assert_eq!(payment.allocations.len(), 2);
    // Two allocations -> target_ref stays unset (only a single allocation sets it).
    assert!(payment.target_ref.is_none());

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let inv1 = invoices::Entity::find_by_id(invoice1).one(tx).await.unwrap().unwrap();
            let inv2 = invoices::Entity::find_by_id(invoice2).one(tx).await.unwrap().unwrap();
            assert_eq!(inv1.paid_amount, dec!(300));
            assert_eq!(inv1.payment_status, invoices::PaymentStatus::Paid);
            assert_eq!(inv2.paid_amount, dec!(200));
            assert_eq!(inv2.payment_status, invoices::PaymentStatus::Paid);
            let report = invariants::run_all(tx).await.unwrap();
            assert!(report.iter().all(|r| r.passed), "invariants failed: {report:?}");
            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn create_payment_supplier_against_received_po() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, payment) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let po_id = seed_purchase_order(tx, cx, fixture.supplier_id, "PO-0001", dec!(400)).await;

            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Paid,
                target_type: PartyKind::Supplier,
                target_id: fixture.supplier_id,
                amount: dec!(400),
                method: PaymentTenderKind::BankTransfer,
                note: None,
                allocations: Some(vec![PaymentAllocationInput { target_kind: AllocationInputTargetKind::PurchaseOrder, target_id: po_id, amount: dec!(400) }]),
                branch_id: Some(fixture.branch_id),
                currency: None,
                amount_fc: None,
                rate: None,
            };
            let payment = service::create::create_payment(tx, cx, &undo, input).await?;
            Ok((po_id, payment))
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<(Id, accounting_app_lib::domains::payments::dto::Payment)>>
    })
    .await
    .unwrap();

    assert_eq!(payment.target_ref, Some(po_id));

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let po = purchase_orders::Entity::find_by_id(po_id).one(tx).await.unwrap().unwrap();
            assert_eq!(po.paid_amount, dec!(400));
            assert_eq!(po.payment_status, purchase_orders::PaymentStatus::Paid);
            let report = invariants::run_all(tx).await.unwrap();
            assert!(report.iter().all(|r| r.passed), "invariants failed: {report:?}");
            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn create_payment_fx_gain_on_usd_invoice() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    // Invoice: 100 USD at 3.75 -> grand_total (FC) = 100, outstanding base = 375.
    // Receipt: 100 USD at 3.80 -> cash base = 380, AR posts at 375 (invoice's own rate) -> FX gain 5.
    let (invoice_id, payment) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let invoice_id = seed_invoice(tx, cx, fixture.customer_id, "INV-0003", dec!(100), Some("USD"), Some(dec!(3.75))).await;

            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Received,
                target_type: PartyKind::Customer,
                target_id: fixture.customer_id,
                amount: dec!(380),
                method: PaymentTenderKind::Cash,
                note: None,
                allocations: Some(vec![PaymentAllocationInput { target_kind: AllocationInputTargetKind::Invoice, target_id: invoice_id, amount: dec!(380) }]),
                branch_id: Some(fixture.branch_id),
                currency: Some("USD".to_string()),
                amount_fc: Some(dec!(100)),
                rate: Some(dec!(3.80)),
            };
            let payment = service::create::create_payment(tx, cx, &undo, input).await?;
            Ok((invoice_id, payment))
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<(Id, accounting_app_lib::domains::payments::dto::Payment)>>
    })
    .await
    .unwrap();

    assert_eq!(payment.fx_gain_loss, Some(dec!(5)));
    assert_eq!(payment.allocations[0].amount, dec!(375)); // AR at the invoice's own rate.
    assert_eq!(payment.allocations[0].fx_gain_loss, Some(dec!(5)));

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let inv = invoices::Entity::find_by_id(invoice_id).one(tx).await.unwrap().unwrap();
            assert_eq!(inv.paid_amount, dec!(100)); // settled in the invoice's own FC.
            assert_eq!(inv.payment_status, invoices::PaymentStatus::Paid);
            let report = invariants::run_all(tx).await.unwrap();
            assert!(report.iter().all(|r| r.passed), "invariants failed: {report:?}");
            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn create_payment_base_currency_partial_against_fc_invoice_is_refused() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let result = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let invoice_id = seed_invoice(tx, cx, fixture.customer_id, "INV-0004", dec!(100), Some("USD"), Some(dec!(3.75))).await;

            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Received,
                target_type: PartyKind::Customer,
                target_id: fixture.customer_id,
                amount: dec!(100),
                method: PaymentTenderKind::Cash,
                note: None,
                allocations: Some(vec![PaymentAllocationInput { target_kind: AllocationInputTargetKind::Invoice, target_id: invoice_id, amount: dec!(100) }]),
                branch_id: Some(fixture.branch_id),
                currency: None,
                amount_fc: None,
                rate: None,
            };
            service::create::create_payment(tx, cx, &undo, input).await
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::payments::dto::Payment>>
    })
    .await;

    let err = result.unwrap_err();
    match err {
        AppError::Validation { message } => assert!(message.contains("التخصيص الجزئي بالعملة الأساسية"), "unexpected message: {message}"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }
}

#[tokio::test]
async fn allocate_later_then_remove_allocation_round_trips() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let undo1 = undo.clone();
    let (invoice_id, payment_id, allocation_id) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo1.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let invoice_id = seed_invoice(tx, cx, fixture.customer_id, "INV-0005", dec!(500), None, None).await;

            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Received,
                target_type: PartyKind::Customer,
                target_id: fixture.customer_id,
                amount: dec!(500),
                method: PaymentTenderKind::Cash,
                note: None,
                allocations: None,
                branch_id: Some(fixture.branch_id),
                currency: None,
                amount_fc: None,
                rate: None,
            };
            let payment = service::create::create_payment(tx, cx, &undo, input).await?;

            let allocated = service::allocate::allocate_existing_payment(
                tx,
                cx,
                &undo,
                payment.id,
                vec![PaymentAllocationInput { target_kind: AllocationInputTargetKind::Invoice, target_id: invoice_id, amount: dec!(500) }],
            )
            .await?;
            let allocation_id = allocated.allocations[0].id;
            Ok((invoice_id, payment.id, allocation_id))
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<(Id, Id, Id)>>
    })
    .await
    .unwrap();

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let inv = invoices::Entity::find_by_id(invoice_id).one(tx).await.unwrap().unwrap();
            assert_eq!(inv.paid_amount, dec!(500));
            assert_eq!(inv.payment_status, invoices::PaymentStatus::Paid);
            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .unwrap();

    // Remove the allocation (no FX involved, so it's allowed) -> the invoice's paid_amount reverts.
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move { service::allocate::remove_allocation(tx, cx, &undo, payment_id, allocation_id).await }) as BoxFuture<
            '_,
            accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::payments::dto::Payment>,
        >
    })
    .await
    .unwrap();

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let inv = invoices::Entity::find_by_id(invoice_id).one(tx).await.unwrap().unwrap();
            assert_eq!(inv.paid_amount, Decimal::ZERO);
            assert_eq!(inv.payment_status, invoices::PaymentStatus::Unpaid);
            let report = invariants::run_all(tx).await.unwrap();
            assert!(report.iter().all(|r| r.passed), "invariants failed: {report:?}");
            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn remove_allocation_with_realized_fx_is_refused() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let undo1 = undo.clone();
    let (payment_id, allocation_id) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo1.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let invoice_id = seed_invoice(tx, cx, fixture.customer_id, "INV-0006", dec!(100), Some("USD"), Some(dec!(3.75))).await;

            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Received,
                target_type: PartyKind::Customer,
                target_id: fixture.customer_id,
                amount: dec!(380),
                method: PaymentTenderKind::Cash,
                note: None,
                allocations: Some(vec![PaymentAllocationInput { target_kind: AllocationInputTargetKind::Invoice, target_id: invoice_id, amount: dec!(380) }]),
                branch_id: Some(fixture.branch_id),
                currency: Some("USD".to_string()),
                amount_fc: Some(dec!(100)),
                rate: Some(dec!(3.80)),
            };
            let payment = service::create::create_payment(tx, cx, &undo, input).await?;
            let allocation_id = payment.allocations[0].id;
            Ok((payment.id, allocation_id))
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<(Id, Id)>>
    })
    .await
    .unwrap();

    let result = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move { service::allocate::remove_allocation(tx, cx, &undo, payment_id, allocation_id).await }) as BoxFuture<
            '_,
            accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::payments::dto::Payment>,
        >
    })
    .await;

    let err = result.unwrap_err();
    match err {
        AppError::Validation { message } => assert!(message.contains("فرق عملة"), "unexpected message: {message}"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }
}

#[tokio::test]
async fn get_payment_unknown_id_is_not_found() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;

    let result = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move { service::read::get_payment(tx, Id::new()).await }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::payments::dto::PaymentRow>>
    })
    .await;

    let err = result.unwrap_err();
    match err {
        AppError::NotFound { message } => assert_eq!(message, "السند غير موجود"),
        other => panic!("expected NOT_FOUND, got {other:?}"),
    }
}

#[tokio::test]
async fn over_allocation_beyond_payment_amount_is_refused() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let result = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let invoice_id = seed_invoice(tx, cx, fixture.customer_id, "INV-0007", dec!(1000), None, None).await;

            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Received,
                target_type: PartyKind::Customer,
                target_id: fixture.customer_id,
                amount: dec!(100),
                method: PaymentTenderKind::Cash,
                note: None,
                allocations: Some(vec![PaymentAllocationInput { target_kind: AllocationInputTargetKind::Invoice, target_id: invoice_id, amount: dec!(200) }]),
                branch_id: Some(fixture.branch_id),
                currency: None,
                amount_fc: None,
                rate: None,
            };
            service::create::create_payment(tx, cx, &undo, input).await
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::payments::dto::Payment>>
    })
    .await;

    let err = result.unwrap_err();
    match err {
        AppError::Validation { message } => assert_eq!(message, "إجمالي التخصيص أكبر من مبلغ السند"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }
}

#[tokio::test]
async fn list_payments_filters_and_search() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let input1 = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Received,
                target_type: PartyKind::Customer,
                target_id: fixture.customer_id,
                amount: dec!(200),
                method: PaymentTenderKind::Cash,
                note: None,
                allocations: None,
                branch_id: Some(fixture.branch_id),
                currency: None,
                amount_fc: None,
                rate: None,
            };
            service::create::create_payment(tx, cx, &undo, input1).await?;

            let input2 = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Paid,
                target_type: PartyKind::Supplier,
                target_id: fixture.supplier_id,
                amount: dec!(150),
                method: PaymentTenderKind::BankTransfer,
                note: None,
                allocations: None,
                branch_id: Some(fixture.branch_id),
                currency: None,
                amount_fc: None,
                rate: None,
            };
            service::create::create_payment(tx, cx, &undo, input2).await?;
            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .unwrap();

    let received_only = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            service::read::get_payments(
                tx,
                accounting_app_lib::domains::payments::dto::PaymentFilter { r#type: Some(PaymentTypeDto::Received), ..Default::default() },
            )
            .await
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<Vec<accounting_app_lib::domains::payments::dto::PaymentRow>>>
    })
    .await
    .unwrap();
    assert_eq!(received_only.len(), 1);
    assert_eq!(received_only[0].payment.amount, dec!(200));

    let party_search = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            service::read::get_payments(tx, accounting_app_lib::domains::payments::dto::PaymentFilter { search: Some("مورد".to_string()), ..Default::default() }).await
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<Vec<accounting_app_lib::domains::payments::dto::PaymentRow>>>
    })
    .await
    .unwrap();
    assert_eq!(party_search.len(), 1);
    assert_eq!(party_search[0].payment.amount, dec!(150));
}

#[tokio::test]
async fn payments_for_invoice_returns_only_received_allocated_ones() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let invoice_id = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let invoice_id = seed_invoice(tx, cx, fixture.customer_id, "INV-0008", dec!(500), None, None).await;
            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Received,
                target_type: PartyKind::Customer,
                target_id: fixture.customer_id,
                amount: dec!(500),
                method: PaymentTenderKind::Cash,
                note: None,
                allocations: Some(vec![PaymentAllocationInput { target_kind: AllocationInputTargetKind::Invoice, target_id: invoice_id, amount: dec!(500) }]),
                branch_id: Some(fixture.branch_id),
                currency: None,
                amount_fc: None,
                rate: None,
            };
            service::create::create_payment(tx, cx, &undo, input).await?;
            Ok(invoice_id)
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<Id>>
    })
    .await
    .unwrap();

    let payments = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move { service::read::payments_for_invoice(tx, invoice_id).await }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<Vec<accounting_app_lib::domains::payments::dto::Payment>>>
    })
    .await
    .unwrap();
    assert_eq!(payments.len(), 1);
    assert_eq!(payments[0].amount, dec!(500));
}

#[tokio::test]
async fn create_payment_period_locked_refuses_with_forbidden() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let result = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;

            // Lock the period at a date after today (`assert_open_period` refuses any date at or
            // before `lock_date`) — nothing should be written once `create_payment` hits the lock.
            let lock_date = cx.clock.today() + chrono::Duration::days(1);
            let settings_row = accounting_app_lib::entities::org::settings::Entity::find().one(tx).await.unwrap().unwrap();
            let mut model: accounting_app_lib::entities::org::settings::ActiveModel = settings_row.into();
            model.accounting = Set(Some(AccountingPolicy { lock_date: Some(lock_date), default_purchase_account_id: None }));
            model.update(tx).await.unwrap();

            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Received,
                target_type: PartyKind::Customer,
                target_id: fixture.customer_id,
                amount: dec!(100),
                method: PaymentTenderKind::Cash,
                note: None,
                allocations: None,
                branch_id: Some(fixture.branch_id),
                currency: None,
                amount_fc: None,
                rate: None,
            };
            service::create::create_payment(tx, cx, &undo, input).await
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::payments::dto::Payment>>
    })
    .await;

    let err = result.unwrap_err();
    assert!(matches!(err, AppError::Forbidden { .. }), "expected FORBIDDEN, got {err:?}");

    // Nothing was written: no `payments` row exists at all.
    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            use sea_orm::PaginatorTrait;
            let count = accounting_app_lib::entities::payments::payments::Entity::find().count(tx).await.unwrap();
            assert_eq!(count, 0, "a period-locked create_payment must write nothing");
            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .unwrap();
}

/// ACC-0015 helper: the payment's journal lines, per entry (ordered by entry number, then position),
/// as `(system_role, debit, credit, amount_fc, rate)`.
async fn payment_entry_lines<C: ConnectionTrait>(conn: &C, payment_id: Id) -> Vec<Vec<(String, Decimal, Decimal, Option<Decimal>, Option<Decimal>)>> {
    use accounting_app_lib::entities::journal::{journal_entries, journal_lines};
    use sea_orm::{ColumnTrait, QueryFilter, QueryOrder};
    let entries = journal_entries::Entity::find()
        .filter(journal_entries::Column::SourceKind.eq("payment"))
        .filter(journal_entries::Column::SourceId.eq(payment_id))
        .order_by_asc(journal_entries::Column::Number)
        .all(conn)
        .await
        .unwrap();
    let mut out = Vec::new();
    for e in entries {
        let lines = journal_lines::Entity::find()
            .filter(journal_lines::Column::JournalEntryId.eq(e.id))
            .order_by_asc(journal_lines::Column::Position)
            .all(conn)
            .await
            .unwrap();
        let mut rows = Vec::new();
        for l in lines {
            let role = accounts::Entity::find_by_id(l.account_id).one(conn).await.unwrap().unwrap().system_role.unwrap_or_default();
            rows.push((role, l.debit, l.credit, l.amount_fc, l.rate));
        }
        out.push(rows);
    }
    out
}

fn invariant_passed(report: &[invariants::InvariantResult], key: &str) -> bool {
    report.iter().filter(|r| r.key == key).all(|r| r.passed)
}

/// ACC-0015: a USD receipt recorded unallocated, then allocated to a USD invoice booked at a lower
/// rate. The allocate-later FX entry releases the untagged cash (Dr AR 380), settles the invoice at
/// ITS rate FC-tagged (Cr AR 375 = 100 USD × 3.75) and books the gain (Cr FX gain 5) — not one
/// 5.00 AR line tagged with the whole 100 USD. `fx-conversion` and `one-active-entry` both hold.
#[tokio::test]
async fn allocate_later_fx_entry_tags_the_settled_fc_at_the_invoice_rate() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let payment_id = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let invoice_id = seed_invoice(tx, cx, fixture.customer_id, "INV-0015", dec!(100), Some("USD"), Some(dec!(3.75))).await;
            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Received,
                target_type: PartyKind::Customer,
                target_id: fixture.customer_id,
                amount: dec!(380),
                method: PaymentTenderKind::BankTransfer,
                note: None,
                allocations: None,
                branch_id: Some(fixture.branch_id),
                currency: Some("USD".to_string()),
                amount_fc: Some(dec!(100)),
                rate: Some(dec!(3.80)),
            };
            let payment = service::create::create_payment(tx, cx, &undo, input).await?;
            let allocated = service::allocate::allocate_existing_payment(
                tx,
                cx,
                &undo,
                payment.id,
                vec![PaymentAllocationInput { target_kind: AllocationInputTargetKind::Invoice, target_id: invoice_id, amount: dec!(380) }],
            )
            .await?;
            assert_eq!(allocated.fx_gain_loss, Some(dec!(5)));
            Ok(payment.id)
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<Id>>
    })
    .await
    .unwrap();

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let entries = payment_entry_lines(tx, payment_id).await;
            assert_eq!(entries.len(), 2, "the receipt + one allocate-later FX entry");
            assert_eq!(
                entries[1],
                vec![
                    ("receivable".to_string(), dec!(380), dec!(0), None, None),
                    ("receivable".to_string(), dec!(0), dec!(375), Some(dec!(100)), Some(dec!(3.75))),
                    ("fxGain".to_string(), dec!(0), dec!(5), None, None),
                ]
            );
            let report = invariants::run_all(tx).await.unwrap();
            assert!(invariant_passed(&report, "fx-conversion"), "fx-conversion failed: {report:?}");
            assert!(invariant_passed(&report, "one-active-entry"), "one-active-entry failed: {report:?}");
            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .unwrap();
}

/// ACC-0015 (PAID side): paying a USD PO booked at 3.75 with USD bought at 3.80 is a LOSS (more base
/// cash went out than the payable carried). `fx_gain_loss` is −5 and the allocate-later entry
/// balances: Cr AP 380 (release) / Dr AP 375 (100 USD at 3.75) / Dr FX loss 5. Before the fix the
/// sign was `cash − ar` (+5, "gain") and the entry could not balance.
#[tokio::test]
async fn allocate_later_supplier_payment_books_fx_loss() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let payment_id = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let po_id = seed_purchase_order(tx, cx, fixture.supplier_id, "PO-0015", dec!(100)).await;
            let po = purchase_orders::Entity::find_by_id(po_id).one(tx).await.unwrap().unwrap();
            let mut po_model: purchase_orders::ActiveModel = po.into();
            po_model.currency = Set(Some("USD".to_string()));
            po_model.exchange_rate = Set(Some(dec!(3.75)));
            po_model.update(tx).await.unwrap();

            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Paid,
                target_type: PartyKind::Supplier,
                target_id: fixture.supplier_id,
                amount: dec!(380),
                method: PaymentTenderKind::BankTransfer,
                note: None,
                allocations: None,
                branch_id: Some(fixture.branch_id),
                currency: Some("USD".to_string()),
                amount_fc: Some(dec!(100)),
                rate: Some(dec!(3.80)),
            };
            let payment = service::create::create_payment(tx, cx, &undo, input).await?;
            let allocated = service::allocate::allocate_existing_payment(
                tx,
                cx,
                &undo,
                payment.id,
                vec![PaymentAllocationInput { target_kind: AllocationInputTargetKind::PurchaseOrder, target_id: po_id, amount: dec!(380) }],
            )
            .await?;
            assert_eq!(allocated.fx_gain_loss, Some(dec!(-5)));
            Ok(payment.id)
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<Id>>
    })
    .await
    .unwrap();

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let entries = payment_entry_lines(tx, payment_id).await;
            assert_eq!(entries.len(), 2);
            assert_eq!(
                entries[1],
                vec![
                    ("payable".to_string(), dec!(0), dec!(380), None, None),
                    ("payable".to_string(), dec!(375), dec!(0), Some(dec!(100)), Some(dec!(3.75))),
                    ("fxLoss".to_string(), dec!(5), dec!(0), None, None),
                ]
            );
            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .unwrap();
}

/// ACC-0014: a payment refused by the lock date (committed in an earlier transaction, like a real
/// user's settings) leaves no trace — no payment row, the invoice's paid amount untouched, and the
/// next accepted payment still gets the first number.
#[tokio::test]
async fn refused_payment_consumes_no_number_and_touches_no_invoice() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let (fixture_customer, fixture_branch, invoice_id) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let invoice_id = seed_invoice(tx, cx, fixture.customer_id, "INV-0014", dec!(200), None, None).await;
            Ok((fixture.customer_id, fixture.branch_id, invoice_id))
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<(Id, Id, Id)>>
    })
    .await
    .unwrap();

    let receipt = move |date: String| PaymentInput {
        date,
        r#type: PaymentTypeDto::Received,
        target_type: PartyKind::Customer,
        target_id: fixture_customer,
        amount: dec!(150),
        method: PaymentTenderKind::Cash,
        note: None,
        allocations: Some(vec![PaymentAllocationInput { target_kind: AllocationInputTargetKind::Invoice, target_id: invoice_id, amount: dec!(150) }]),
        branch_id: Some(fixture_branch),
        currency: None,
        amount_fc: None,
        rate: None,
    };

    // Lock everything up to tomorrow, in its own committed transaction.
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let settings_row = settings::Entity::find().one(tx).await.unwrap().unwrap();
            let mut model: settings::ActiveModel = settings_row.into();
            model.accounting = Set(Some(AccountingPolicy { lock_date: Some(cx.clock.today() + chrono::Duration::days(1)), default_purchase_account_id: None }));
            model.update(tx).await.unwrap();
            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .unwrap();

    let undo1 = undo.clone();
    let refused = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo1.clone();
        let input = receipt(cx.clock.today().format("%Y-%m-%d").to_string());
        Box::pin(async move { service::create::create_payment(tx, cx, &undo, input).await })
            as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::payments::dto::Payment>>
    })
    .await;
    assert!(matches!(refused, Err(AppError::Forbidden { .. })), "expected FORBIDDEN, got {refused:?}");

    // Unlock and post the same receipt: it gets the FIRST number, and it is the only payment.
    let accepted = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        let input = receipt(cx.clock.today().format("%Y-%m-%d").to_string());
        Box::pin(async move {
            let inv = invoices::Entity::find_by_id(invoice_id).one(tx).await.unwrap().unwrap();
            assert_eq!(inv.paid_amount, Decimal::ZERO, "the refused payment must not touch the invoice");
            let settings_row = settings::Entity::find().one(tx).await.unwrap().unwrap();
            let mut model: settings::ActiveModel = settings_row.into();
            model.accounting = Set(Some(AccountingPolicy { lock_date: None, default_purchase_account_id: None }));
            model.update(tx).await.unwrap();
            service::create::create_payment(tx, cx, &undo, input).await
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::payments::dto::Payment>>
    })
    .await
    .unwrap();

    assert!(accepted.number.ends_with("000001"), "the refused payment consumed a number: {}", accepted.number);
    let accepted_number = accepted.number.clone();
    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        let accepted_number = accepted_number.clone();
        Box::pin(async move {
            use sea_orm::PaginatorTrait;
            let count = accounting_app_lib::entities::payments::payments::Entity::find().count(tx).await.unwrap();
            assert_eq!(count, 1, "only the accepted payment exists");
            let first = accounting_app_lib::entities::payments::payments::Entity::find().one(tx).await.unwrap().unwrap();
            assert_eq!(first.number, accepted_number);
            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .unwrap();
}
