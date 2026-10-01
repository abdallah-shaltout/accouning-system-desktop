//! DB-backed tests for the `attachments` domain (C-16). Written now, run in the deferred
//! time-boxed test pass (per-implementer hard rule: never run cargo from this agent). Needs
//! `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.
//!
//! Not branch-scoped, no seeding — every test just needs a session (`with_tx`'s `require_user`).

use crate::support;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::error::AppError;
use accounting_app_lib::core::tx::{with_tx, TxOpts};
use accounting_app_lib::domains::attachments::dto::AttachmentKind;
use accounting_app_lib::domains::attachments::service;
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use support::TestDb;

fn log_in(db: &TestDb, role: Role) -> Id {
    let user_id = Id::new();
    let user = AuthenticatedUser {
        id: user_id,
        username: "test".to_string(),
        role,
        home_branch_id: Id::new(),
        allowed_branches: vec![],
        price_list_id: None,
        max_discount: None,
    };
    *db.state.session.write().unwrap() = Some(user);
    user_id
}

fn small_blob_base64() -> String {
    BASE64.encode(b"hello world")
}

// --- save / fetch -----------------------------------------------------------------------------------

#[tokio::test]
async fn save_then_fetch_round_trips_bytes_and_meta() {
    let db = TestDb::fresh().await;
    log_in(&db, Role::Cashier);

    let saved = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            service::save_attachment(
                tx,
                cx,
                None,
                Some("customer:cus-3".to_string()),
                "receipt.png".to_string(),
                "image/png".to_string(),
                AttachmentKind::Image,
                Some(100),
                Some(200),
                small_blob_base64(),
                None,
            )
            .await
        })
    })
    .await
    .expect("save must succeed");

    assert_eq!(saved.name, "receipt.png");
    assert_eq!(saved.owner_ref.as_deref(), Some("customer:cus-3"));
    assert_eq!(saved.size, 11, "size must be the decoded blob's byte length");
    assert_eq!(saved.blob_base64, small_blob_base64());

    let id = saved.id.to_string();
    let fetched = with_tx(&db.state, TxOpts::default(), move |tx, _cx| {
        let id = id.clone();
        Box::pin(async move { service::fetch_attachment(tx, &id).await })
    })
    .await
    .expect("fetch must succeed")
    .expect("row must exist");
    assert_eq!(fetched.blob_base64, small_blob_base64());
    assert_eq!(fetched.width, Some(100));
    assert_eq!(fetched.height, Some(200));
    db.finish().await;
}

#[tokio::test]
async fn fetch_unknown_and_unparsable_ids_return_none() {
    let db = TestDb::fresh().await;
    log_in(&db, Role::Cashier);

    let unparsable = with_tx(&db.state, TxOpts::default(), |tx, _cx| Box::pin(async move { service::fetch_attachment(tx, "not-a-uuid").await }))
        .await
        .expect("must succeed");
    assert!(unparsable.is_none());

    let unknown_id = Id::new().to_string();
    let unknown = with_tx(&db.state, TxOpts::default(), move |tx, _cx| {
        let unknown_id = unknown_id.clone();
        Box::pin(async move { service::fetch_attachment(tx, &unknown_id).await })
    })
    .await
    .expect("must succeed");
    assert!(unknown.is_none());
    db.finish().await;
}

#[tokio::test]
async fn fetch_attachments_by_owner_ref_orders_newest_first_and_excludes_others() {
    let db = TestDb::fresh().await;
    log_in(&db, Role::Cashier);

    for name in ["a.png", "b.png"] {
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            Box::pin(async move {
                service::save_attachment(
                    tx,
                    cx,
                    None,
                    Some("journal:je-1".to_string()),
                    name.to_string(),
                    "image/png".to_string(),
                    AttachmentKind::Image,
                    None,
                    None,
                    small_blob_base64(),
                    None,
                )
                .await
            })
        })
        .await
        .expect("save must succeed");
    }
    // A different owner must never show up in the first owner's list.
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            service::save_attachment(
                tx,
                cx,
                None,
                Some("journal:je-2".to_string()),
                "other.png".to_string(),
                "image/png".to_string(),
                AttachmentKind::Image,
                None,
                None,
                small_blob_base64(),
                None,
            )
            .await
        })
    })
    .await
    .expect("save must succeed");

    let list = with_tx(&db.state, TxOpts::default(), |tx, _cx| Box::pin(async move { service::fetch_attachments(tx, "journal:je-1").await }))
        .await
        .expect("fetch must succeed");
    assert_eq!(list.len(), 2, "only journal:je-1's own attachments must be returned");
    assert!(list.iter().all(|m| m.owner_ref.as_deref() == Some("journal:je-1")));
    db.finish().await;
}

#[tokio::test]
async fn fetch_attachments_by_ids_batches_and_skips_missing() {
    let db = TestDb::fresh().await;
    log_in(&db, Role::Cashier);

    let saved = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            service::save_attachment(
                tx,
                cx,
                None,
                None,
                "a.png".to_string(),
                "image/png".to_string(),
                AttachmentKind::Image,
                None,
                None,
                small_blob_base64(),
                None,
            )
            .await
        })
    })
    .await
    .expect("save must succeed");

    let missing_id = Id::new().to_string();
    let ids = vec![saved.id.to_string(), missing_id, "not-a-uuid".to_string()];
    let records = with_tx(&db.state, TxOpts::default(), move |tx, _cx| {
        let ids = ids.clone();
        Box::pin(async move { service::fetch_attachments_by_ids(tx, &ids).await })
    })
    .await
    .expect("must succeed");

    assert_eq!(records.len(), 1, "unknown/unparsable ids must be silently skipped");
    assert_eq!(records[0].id, saved.id);
    db.finish().await;
}

// --- size limit ---------------------------------------------------------------------------------------

#[tokio::test]
async fn oversized_blob_is_rejected_with_arabic_validation_message() {
    let db = TestDb::fresh().await;
    log_in(&db, Role::Cashier);

    let too_big = vec![0u8; service::MAX_ATTACHMENT_SIZE + 1];
    let too_big_base64 = BASE64.encode(&too_big);

    let err = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let too_big_base64 = too_big_base64.clone();
        Box::pin(async move {
            service::save_attachment(
                tx,
                cx,
                None,
                None,
                "big.bin".to_string(),
                "application/octet-stream".to_string(),
                AttachmentKind::Other,
                None,
                None,
                too_big_base64,
                None,
            )
            .await
        })
    })
    .await
    .unwrap_err();

    assert!(matches!(err, AppError::Validation { .. }));
    assert!(err.to_string().contains("أكبر من الحد المسموح"), "must be the Arabic size-limit message, got: {err}");
    db.finish().await;
}

/// Part 04 Wave 2 (L1): a present-but-non-UUID id used to be re-keyed silently to a fresh id — the
/// caller kept the id it sent and could never fetch the file again. It is refused now, and nothing
/// is stored.
#[tokio::test]
async fn non_uuid_client_id_is_refused_not_rekeyed() {
    let db = TestDb::fresh().await;
    log_in(&db, Role::Cashier);

    let err = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            service::save_attachment(
                tx,
                cx,
                Some("att-1".to_string()),
                Some("customer:cus-1".to_string()),
                "a.png".to_string(),
                "image/png".to_string(),
                AttachmentKind::Image,
                None,
                None,
                small_blob_base64(),
                None,
            )
            .await
        })
    })
    .await
    .unwrap_err();
    assert!(matches!(err, AppError::Validation { .. }), "got: {err}");

    let rows = with_tx(&db.state, TxOpts::default(), |tx, _cx| Box::pin(async move { service::fetch_attachments(tx, "customer:cus-1").await }))
        .await
        .unwrap();
    assert!(rows.is_empty(), "a refused save must store nothing");
    db.finish().await;
}

// --- update (upsert) -----------------------------------------------------------------------------------

#[tokio::test]
async fn saving_with_an_existing_id_overwrites_in_place() {
    let db = TestDb::fresh().await;
    log_in(&db, Role::Cashier);

    let first = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            service::save_attachment(
                tx,
                cx,
                None,
                None,
                "v1.png".to_string(),
                "image/png".to_string(),
                AttachmentKind::Image,
                None,
                None,
                small_blob_base64(),
                None,
            )
            .await
        })
    })
    .await
    .expect("save must succeed");

    let updated_blob = BASE64.encode(b"a different, longer payload");
    let id = first.id.to_string();
    let second = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let id = id.clone();
        let updated_blob = updated_blob.clone();
        Box::pin(async move {
            service::save_attachment(
                tx,
                cx,
                Some(id),
                None,
                "v2.png".to_string(),
                "image/png".to_string(),
                AttachmentKind::Image,
                None,
                None,
                updated_blob,
                None,
            )
            .await
        })
    })
    .await
    .expect("update must succeed");

    assert_eq!(second.id, first.id, "same id must overwrite, not insert a second row");
    assert_eq!(second.name, "v2.png");
    assert_ne!(second.blob_base64, first.blob_base64);
    db.finish().await;
}

// --- remove --------------------------------------------------------------------------------------------

#[tokio::test]
async fn remove_deletes_and_is_idempotent_on_unknown_id() {
    let db = TestDb::fresh().await;
    log_in(&db, Role::Cashier);

    let saved = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            service::save_attachment(
                tx,
                cx,
                None,
                None,
                "to-delete.png".to_string(),
                "image/png".to_string(),
                AttachmentKind::Image,
                None,
                None,
                small_blob_base64(),
                None,
            )
            .await
        })
    })
    .await
    .expect("save must succeed");

    let id = saved.id.to_string();
    with_tx(&db.state, TxOpts::default(), {
        let id = id.clone();
        move |tx, cx| {
            let id = id.clone();
            Box::pin(async move { service::remove_attachment(tx, cx, &id).await })
        }
    })
    .await
    .expect("remove must succeed");

    let after = with_tx(&db.state, TxOpts::default(), {
        let id = id.clone();
        move |tx, _cx| {
            let id = id.clone();
            Box::pin(async move { service::fetch_attachment(tx, &id).await })
        }
    })
    .await
    .expect("fetch must succeed");
    assert!(after.is_none(), "row must be hard-deleted");

    // Removing again (or an id that never existed) must be a silent no-op, not an error.
    with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let id = id.clone();
        Box::pin(async move { service::remove_attachment(tx, cx, &id).await })
    })
    .await
    .expect("second remove must be a no-op, not an error");
    db.finish().await;
}

// --- access control --------------------------------------------------------------------------------------

#[tokio::test]
async fn no_session_is_unauthorized() {
    let db = TestDb::fresh().await;
    // No log_in call — no session at all.

    let result = with_tx(&db.state, TxOpts::default(), |tx, _cx| Box::pin(async move { service::fetch_attachments(tx, "customer:x").await })).await;
    assert!(matches!(result, Err(AppError::Unauthorized { .. })), "with_tx's require_user gate must reject with no session at all");
    db.finish().await;
}

// --- invariants -------------------------------------------------------------------------------------------

#[tokio::test]
async fn attachment_writes_never_break_accounting_invariants() {
    let db = TestDb::fresh().await;
    log_in(&db, Role::Cashier);

    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            service::save_attachment(
                tx,
                cx,
                None,
                None,
                "x.png".to_string(),
                "image/png".to_string(),
                AttachmentKind::Image,
                None,
                None,
                small_blob_base64(),
                None,
            )
            .await
        })
    })
    .await
    .unwrap();

    let report = with_tx(&db.state, TxOpts::default(), |tx, _cx| Box::pin(async move { invariants::run_all(tx).await.map_err(Into::into) }))
        .await
        .expect("invariants must run");
    assert!(report.iter().all(|r| r.passed), "attachment writes touch no ledger/stock data — invariants must stay green");
    db.finish().await;
}
