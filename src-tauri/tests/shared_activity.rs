//! `shared::activity` DB-backed tests (21.02-E "Tests"): `log`/`record` write both rows correctly,
//! and the undo mechanism's full state machine, using a test-only `Compensator` that reverses a
//! manual journal entry through `shared::ledger::reverse`. Needs `EQUAL_TEST_DATABASE_URL` (see
//! `tests/support/mod.rs`) — never skipped when absent.

use crate::support;

use std::sync::Arc;

use accounting_app_lib::core::auth::{Area, AuthenticatedUser, Role};
use accounting_app_lib::core::error::AppError;
use accounting_app_lib::core::tx::{with_tx, BoxFuture, TxCtx, TxError, TxOpts, TxResult};
use accounting_app_lib::entities::journal::journal_entries::JournalEntryType;
use accounting_app_lib::entities::platform::activity::ActivityKind;
use accounting_app_lib::entities::platform::audit::{self, AuditAction};
use accounting_app_lib::shared::activity::{log, log_undoable, record, undo, AuditInput, Compensator, UndoRegistry, UndoRequest, UndoSpec};
use accounting_app_lib::shared::ledger::accounts::SystemRole;
use accounting_app_lib::shared::ledger::{self, AccountRef, MirrorDims, PostJournal, PostingLine, ReversalReason, ReverseRequest};
use accounting_app_lib::utils::id::Id;
use accounting_app_lib::utils::route::RouteRef;
use async_trait::async_trait;
use chrono::{Datelike, NaiveDate};
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set};
use support::TestDb;

/// Stamps a logged-in session directly into `AppState` (same pattern as `db_foundation.rs`), with
/// a caller-chosen role so the closed-year admin/non-admin case can be exercised.
async fn log_in(test_db: &TestDb, role: Role) -> Id {
    let user_id = Id::new();
    {
        // The session user must exist: `audit.user_id`/`activity.user_id` FK to `users`.
        let role_str = serde_json::to_value(role).unwrap().as_str().unwrap().to_string();
        let db_guard = test_db.state.db.read().unwrap();
        support::seed_user(&db_guard.as_ref().unwrap().connection, user_id, &role_str).await;
    }
    let user = AuthenticatedUser {
        id: user_id,
        username: "test".to_string(),
        role,
        home_branch_id: Id::new(),
        allowed_branches: vec![],
        price_list_id: None,
        max_discount: None,
    };
    *test_db.state.session.write().unwrap() = Some(user);
    user_id
}

/// A test-only compensator that reverses a manual journal entry (the `UndoSpec.payload` carries
/// the journal entry id to reverse) through `ledger::reverse`. Mirrors the real shape a domain's
/// Part-03 compensator would take: it reads the original document from the payload, calls the
/// existing reversal operation, and returns the audit id that operation's own `record`/`log_undoable`
/// call would have produced — here it just re-records through `log_undoable`-less `record` so the
/// test also exercises `record`'s ordinary (non-undoable) path for the compensation row.
struct ManualEntryCompensator;

#[async_trait]
impl Compensator for ManualEntryCompensator {
    fn action_type(&self) -> &'static str {
        "test.manualJournalEntry"
    }

    fn area(&self) -> Area {
        Area::Accounting
    }

    async fn compensate(
        &self,
        tx: &DatabaseTransaction,
        cx: &TxCtx,
        registry: &UndoRegistry,
        original: &audit::Model,
        req: &UndoRequest,
    ) -> Result<Id, AppError> {
        let payload = original.payload.as_ref().ok_or_else(|| AppError::internal("no payload", None))?;
        let journal_id: Id = payload
            .get("journalEntryId")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| AppError::internal("payload missing journalEntryId", None))?;

        let is_admin = cx.actor.as_ref().map(|a| a.role == Role::Admin).unwrap_or(false);

        let reversed = ledger::reverse(
            tx,
            cx,
            ReverseRequest {
                original_id: journal_id,
                date: req.date.as_ref().map(|d| d.resolve(&cx.clock)).unwrap_or_else(|| cx.clock.today().into()),
                description: format!("عكس: {}", req.reason),
                entry_type: JournalEntryType::Manual,
                allow_closed_period: is_admin,
                reason: Some(ReversalReason { text: req.reason.clone(), stamp_original: true }),
                dims: MirrorDims::Keep,
            },
        )
        .await
        .map_err(TxError::into_app_error)?;

        // G-19: uses the REAL registry `undo()` was called with (previously a fresh, disconnected
        // `UndoRegistry::new()` — harmless only because this call passes `undo: None`; a
        // compensator that itself needs to record an undoable action couldn't have worked before).
        let comp_id = record(
            tx,
            cx,
            registry,
            AuditInput {
                entity: "journal".to_string(),
                entity_id: reversed.id,
                entity_label: None,
                action: AuditAction::Reverse,
                before: None,
                after: None,
                user_id: None,
                branch_id: None,
                at: None,
                reason: Some(req.reason.clone()),
                message: "تراجع عن قيد يدوي".to_string(),
                link: Some(RouteRef::detail("journal-entry", reversed.id.to_string())),
                activity_kind: Some(ActivityKind::Journal),
                undo: None,
            },
        )
        .await
        .map_err(TxError::into_app_error)?;

        Ok(comp_id)
    }
}

/// A compensator that always fails — proves a failing compensation rolls everything back inside
/// the caller's single transaction (no links, no mirror entry, nothing committed).
struct FailingCompensator;

#[async_trait]
impl Compensator for FailingCompensator {
    fn action_type(&self) -> &'static str {
        "test.alwaysFails"
    }
    fn area(&self) -> Area {
        Area::Accounting
    }
    async fn compensate(
        &self,
        _tx: &DatabaseTransaction,
        _cx: &TxCtx,
        _registry: &UndoRegistry,
        _original: &audit::Model,
        _req: &UndoRequest,
    ) -> Result<Id, AppError> {
        Err(AppError::validation("فشل متعمد للاختبار"))
    }
}

fn test_registry() -> Arc<UndoRegistry> {
    let mut registry = UndoRegistry::new();
    registry.register(Arc::new(ManualEntryCompensator));
    registry.register(Arc::new(FailingCompensator));
    Arc::new(registry)
}

/// Seeds a minimal chart of accounts (cash + sales, both system-role-tagged, matching
/// `resolve_account`'s expectations) via direct inserts — Part 02's fixture-building convention
/// for a DB test that needs `shared::ledger::post` to resolve `SystemRole::Cash`/`SystemRole::Sales`
/// without going through the (Part 03) accounts-setup domain code.
async fn seed_accounts(tx: &DatabaseTransaction) {
    // `ledger::post` resolves the base currency / default branch from the singleton settings row.
    support::seed_minimal_settings(tx, "SAR").await;
    use accounting_app_lib::entities::org::accounts::ActiveModel as AccountActiveModel;
    let now = chrono::Utc::now();
    for (role, code, name, kind, subtype, normal_side) in [
        (SystemRole::Cash, "1000", "الصندوق", "ASSET", "cash", "DEBIT"),
        (SystemRole::Sales, "4000", "مبيعات البضائع", "REVENUE", "revenue", "CREDIT"),
    ]
    {
        let model = AccountActiveModel {
            code_live: sea_orm::ActiveValue::NotSet,
            id: Set(Id::new()),
            code: Set(code.to_string()),
            name: Set(name.to_string()),
            name_en: Set(None),
            parent_id: Set(None),
            is_group: Set(false),
            kind: Set(kind.to_string()),
            subtype: Set(subtype.to_string()),
            normal_side: Set(normal_side.to_string()),
            system_role: Set(Some(role.as_str().to_string())),
            currency: Set(None),
            branch_id: Set(None),
            requires_party: Set(Some(false)),
            allow_manual: Set(true),
            requires_cost_center: Set(Some(false)),
            active: Set(true),
            can_delete: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
            sync_status: Set("synced".to_string()),
        };
        model.insert(tx).await.expect("seeding a fixture account must succeed");
    }
}

/// Posts a simple balanced manual journal entry (cash debit / revenue credit) directly through
/// `ledger::post`, for tests to then undo. Returns its id.
async fn post_manual_entry(tx: &DatabaseTransaction, cx: &TxCtx) -> Id {
    seed_accounts(tx).await;
    let entry = ledger::post(
        tx,
        cx,
        PostJournal {
            date: cx.clock.today().into(),
            description: "قيد يدوي للاختبار".to_string(),
            entry_type: JournalEntryType::Manual,
            source: None,
            lines: vec![
                PostingLine::debit(AccountRef::Role(SystemRole::Cash), dec!(100)),
                PostingLine::credit(AccountRef::Role(SystemRole::Sales), dec!(100)),
            ],
            allow_closed_period: false,
            attachment_ids: vec![],
            template_id: None,
        },
    )
    .await
    .expect("posting a balanced manual entry must succeed");
    entry.id
}

#[tokio::test]
async fn log_writes_audit_and_activity_rows() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let registry = test_registry();

    let journal_id = Id::new();
    let link = RouteRef::detail("journal-entry", journal_id.to_string());

    let audit_id: Id = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        let link = link.clone();
        Box::pin(async move {
            log(txn, ctx, &registry, ActivityKind::Journal, "قيد جديد", None, Some(link)).await
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let audit_row = audit::Entity::find_by_id(audit_id).one(conn).await.unwrap().unwrap();
    assert_eq!(audit_row.entity, "journal");
    assert_eq!(audit_row.entity_id, journal_id);
    assert_eq!(audit_row.action, AuditAction::Create);

    use accounting_app_lib::entities::platform::activity;
    let activity_row = activity::Entity::find().filter(activity::Column::AuditId.eq(audit_id)).one(conn).await.unwrap().unwrap();
    assert_eq!(activity_row.kind, ActivityKind::Journal);
    assert_eq!(activity_row.message, "قيد جديد");
    assert_eq!(activity_row.audit_id, Some(audit_id));
}

#[tokio::test]
async fn log_falls_back_on_a_list_link() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let registry = test_registry();

    let audit_id: Id = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move { log(txn, ctx, &registry, ActivityKind::Approval, "طلب موافقة", None, Some(RouteRef::list("approvals"))).await })
            as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let audit_row = audit::Entity::find_by_id(audit_id).one(conn).await.unwrap().unwrap();
    assert_eq!(audit_row.entity, "approval", "a list link has no id param, so it must fall back to (kind, new id)");
}

#[tokio::test]
async fn auth_kind_defaults_to_login_action() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let registry = test_registry();

    let audit_id: Id = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move { log(txn, ctx, &registry, ActivityKind::Auth, "تسجيل دخول", None, None).await }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let audit_row = audit::Entity::find_by_id(audit_id).one(conn).await.unwrap().unwrap();
    assert_eq!(audit_row.action, AuditAction::Login);
}

#[tokio::test]
async fn undo_happy_path_links_both_rows() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let registry = test_registry();

    let original_audit_id: Id = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move {
            let journal_id = post_manual_entry(txn, ctx).await;
            log_undoable(
                txn,
                ctx,
                &registry,
                ActivityKind::Journal,
                "قيد يدوي",
                None,
                Some(RouteRef::detail("journal-entry", journal_id.to_string())),
                UndoSpec { action_type: "test.manualJournalEntry", payload: serde_json::json!({ "journalEntryId": journal_id.to_string() }) },
            )
            .await
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let comp_id: Id = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move {
            undo(txn, ctx, &registry, original_audit_id, UndoRequest { reason: "خطأ في الإدخال".to_string(), date: None }).await
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let original = audit::Entity::find_by_id(original_audit_id).one(conn).await.unwrap().unwrap();
    let comp = audit::Entity::find_by_id(comp_id).one(conn).await.unwrap().unwrap();
    assert_eq!(original.undone_by, Some(comp_id));
    assert_eq!(comp.undo_of, Some(original_audit_id));
}

#[tokio::test]
async fn undo_twice_gives_conflict() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let registry = test_registry();

    let original_audit_id: Id = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move {
            let journal_id = post_manual_entry(txn, ctx).await;
            log_undoable(
                txn,
                ctx,
                &registry,
                ActivityKind::Journal,
                "قيد يدوي",
                None,
                Some(RouteRef::detail("journal-entry", journal_id.to_string())),
                UndoSpec { action_type: "test.manualJournalEntry", payload: serde_json::json!({ "journalEntryId": journal_id.to_string() }) },
            )
            .await
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let first: Result<Id, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move { undo(txn, ctx, &registry, original_audit_id, UndoRequest { reason: "أول تراجع".to_string(), date: None }).await })
            as BoxFuture<'_, TxResult<Id>>
    })
    .await;
    assert!(first.is_ok());

    let second: Result<Id, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move { undo(txn, ctx, &registry, original_audit_id, UndoRequest { reason: "تراجع ثاني".to_string(), date: None }).await })
            as BoxFuture<'_, TxResult<Id>>
    })
    .await;
    assert!(matches!(second, Err(AppError::Conflict { .. })), "undoing an already-undone row must give CONFLICT");
}

#[tokio::test]
async fn undo_of_a_non_undoable_row_gives_validation() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let registry = test_registry();

    let audit_id: Id = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move { log(txn, ctx, &registry, ActivityKind::Sale, "بيع", None, None).await }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let result: Result<Id, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move { undo(txn, ctx, &registry, audit_id, UndoRequest { reason: "سبب".to_string(), date: None }).await })
            as BoxFuture<'_, TxResult<Id>>
    })
    .await;
    assert!(matches!(result, Err(AppError::Validation { .. })), "a non-undoable row must give VALIDATION, not silently succeed");
}

#[tokio::test]
async fn undo_with_empty_reason_gives_validation() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let registry = test_registry();

    let audit_id: Id = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move {
            let journal_id = post_manual_entry(txn, ctx).await;
            log_undoable(
                txn,
                ctx,
                &registry,
                ActivityKind::Journal,
                "قيد يدوي",
                None,
                Some(RouteRef::detail("journal-entry", journal_id.to_string())),
                UndoSpec { action_type: "test.manualJournalEntry", payload: serde_json::json!({ "journalEntryId": journal_id.to_string() }) },
            )
            .await
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let result: Result<Id, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move { undo(txn, ctx, &registry, audit_id, UndoRequest { reason: "   ".to_string(), date: None }).await })
            as BoxFuture<'_, TxResult<Id>>
    })
    .await;
    assert!(matches!(result, Err(AppError::Validation { .. })), "a blank/whitespace-only reason must give VALIDATION");
}

#[tokio::test]
async fn undo_unregistered_action_type_gives_internal() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let registry = test_registry();

    // Constructed with an action_type the registry doesn't know, bypassing record()'s own
    // fail-fast check by writing the audit row directly — simulates data that predates a
    // compensator being un-registered, or a bug where the registry passed to undo() differs
    // from the one used at record time.
    let audit_id: Id = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        Box::pin(async move {
            let id = Id::new();
            let user_id = ctx.actor.as_ref().unwrap().id;
            let model = audit::ActiveModel {
                id: Set(id),
                entity: Set("journal".to_string()),
                entity_id: Set(Id::new()),
                entity_label: Set(None),
                action: Set(AuditAction::Create),
                before: Set(None),
                after: Set(None),
                user_id: Set(user_id),
                branch_id: Set(None),
                at_day: Set(ctx.clock.today()),
                at_instant: Set(Some(ctx.clock.now)),
                reason: Set(None),
                message: Set("test".to_string()),
                link: Set(None),
                action_type: Set(Some("no.such.compensator".to_string())),
                payload: Set(Some(serde_json::json!({}))),
                is_undoable: Set(true),
                undo_of: Set(None),
                undone_by: Set(None),
                terminal_id: Set(Some(ctx.terminal_id)),
                created_at: Set(ctx.clock.now),
            };
            model.insert(txn).await.map_err(TxError::from)?;
            Ok(id)
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let result: Result<Id, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move { undo(txn, ctx, &registry, audit_id, UndoRequest { reason: "سبب".to_string(), date: None }).await })
            as BoxFuture<'_, TxResult<Id>>
    })
    .await;
    assert!(matches!(result, Err(AppError::Internal { .. })), "an unregistered action_type must fail fast with INTERNAL");
}

/// Seeds a fiscal year covering `day`, already `is_closed = true` — this test only needs a closed
/// period to exist for `assert_open_period`/the compensator's `allow_closed_period` gate to react
/// to; actually closing a year (computing the closing entry, etc.) is Part 03 domain logic, not
/// anything Part 02's `shared::ledger` exposes.
async fn seed_closed_fiscal_year(tx: &DatabaseTransaction, day: NaiveDate) {
    use accounting_app_lib::entities::org::fiscal_years::ActiveModel as FiscalYearActiveModel;
    let now = chrono::Utc::now();
    let model = FiscalYearActiveModel {
        id: Set(Id::new()),
        name: Set(format!("{}", day.format("%Y"))),
        start_date: Set(NaiveDate::from_ymd_opt(day.year(), 1, 1).unwrap()),
        end_date: Set(NaiveDate::from_ymd_opt(day.year(), 12, 31).unwrap()),
        is_closed: Set(true),
        closing_entry_id: Set(None),
        closed_at: Set(Some(now)),
        closed_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    model.insert(tx).await.expect("seeding a fixture closed fiscal year must succeed");
}

#[tokio::test]
async fn closed_year_non_admin_forbidden_admin_succeeds() {
    let test_db = TestDb::fresh().await;

    // Post the entry and its undoable audit row as admin (setup) *before* the year is closed (a
    // real post would itself be refused in a closed year without allow_closed_period), then seed
    // the year as already closed for the undo attempts below.
    log_in(&test_db, Role::Admin).await;
    let registry = test_registry();

    let (audit_id, entry_date) = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move {
            let journal_id = post_manual_entry(txn, ctx).await;
            let audit_id = log_undoable(
                txn,
                ctx,
                &registry,
                ActivityKind::Journal,
                "قيد يدوي",
                None,
                Some(RouteRef::detail("journal-entry", journal_id.to_string())),
                UndoSpec { action_type: "test.manualJournalEntry", payload: serde_json::json!({ "journalEntryId": journal_id.to_string() }) },
            )
            .await?;
            Ok((audit_id, ctx.clock.today()))
        }) as BoxFuture<'_, TxResult<(Id, chrono::NaiveDate)>>
    })
    .await
    .unwrap();

    with_tx(&test_db.state, TxOpts::default(), |txn, _ctx| {
        Box::pin(async move {
            seed_closed_fiscal_year(txn, entry_date).await;
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    // Non-admin: FORBIDDEN.
    log_in(&test_db, Role::Accountant).await;
    let non_admin_result: Result<Id, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move { undo(txn, ctx, &registry, audit_id, UndoRequest { reason: "محاولة تراجع".to_string(), date: None }).await })
            as BoxFuture<'_, TxResult<Id>>
    })
    .await;
    assert!(matches!(non_admin_result, Err(AppError::Forbidden { .. })), "undoing in a closed year as non-admin must give FORBIDDEN (D7)");

    // Admin: succeeds.
    log_in(&test_db, Role::Admin).await;
    let admin_result: Result<Id, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move { undo(txn, ctx, &registry, audit_id, UndoRequest { reason: "تراجع الأدمن".to_string(), date: None }).await })
            as BoxFuture<'_, TxResult<Id>>
    })
    .await;
    assert!(admin_result.is_ok(), "an admin must be able to undo even in a closed year (D7)");
}

#[tokio::test]
async fn a_failing_compensator_rolls_everything_back() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let registry = test_registry();

    let audit_id: Id = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move {
            log_undoable(
                txn,
                ctx,
                &registry,
                ActivityKind::Journal,
                "عملية ستفشل",
                None,
                None,
                UndoSpec { action_type: "test.alwaysFails", payload: serde_json::json!({}) },
            )
            .await
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let result: Result<Id, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        let registry = registry.clone();
        Box::pin(async move { undo(txn, ctx, &registry, audit_id, UndoRequest { reason: "محاولة".to_string(), date: None }).await })
            as BoxFuture<'_, TxResult<Id>>
    })
    .await;
    assert!(result.is_err());

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let original = audit::Entity::find_by_id(audit_id).one(conn).await.unwrap().unwrap();
    assert_eq!(original.undone_by, None, "a failed compensation must leave no link on the original row");

    use accounting_app_lib::entities::platform::audit::Column;
    let leftover = audit::Entity::find().filter(Column::UndoOf.eq(audit_id)).all(conn).await.unwrap();
    assert!(leftover.is_empty(), "a failed compensation must leave no compensation row behind");
}
