//! `branches.rs` — `get_branches`/`create_branch`/`update_branch`/`deactivate_branch`/
//! `reactivate_branch` (01-settings.md §3 "Branches"). `create_branch` is `pub` — `02-setup`'s
//! `applyBranches` reuses it (entry file's own note).

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::{AppError, AppResult};
use crate::core::events::ChangeCategory;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::catalog::product_branch_stock;
use crate::entities::catalog::products;
use crate::entities::org::accounts::{self, Entity as AccountEntity};
use crate::entities::org::branches::{ActiveModel, Column, Entity};
use crate::entities::org::cost_centers::{self, Entity as CostCenterEntity};
use crate::entities::platform::activity::ActivityKind;
use crate::entities::platform::audit::AuditAction;
use crate::entities::sales::shifts::{Column as ShiftColumn, Entity as ShiftEntity, ShiftStatus};
use crate::entities::soft_delete::SoftDelete;
use crate::shared::activity::{diff_fields, record, undo::UndoRegistry, AuditInput};
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::dto::{Branch, BranchInput, BranchPatch};

/// `getBranches` — list order = insertion order (branches are never soft-deleted, B-2).
pub async fn list<C: ConnectionTrait>(conn: &C) -> AppResult<Vec<Branch>> {
    let rows = Entity::find().order_by_asc(Column::CreatedAt).order_by_asc(Column::Id).all(conn).await.map_err(AppError::from)?;
    Ok(rows.into_iter().map(Branch::from_model).collect())
}

/// Finds the parent of the first live `cash`-role account with `branch_id IS NULL` (the seeded main
/// cash account's parent) — `cashParentId` (`branches.ts:43-46`).
async fn cash_parent_id<C: ConnectionTrait>(conn: &C) -> TxResult<Option<Id>> {
    let main_cash = AccountEntity::find()
        .filter(accounts::Column::SystemRole.eq("cash"))
        .filter(accounts::Column::BranchId.is_null())
        .filter(accounts::Column::DeletedAt.is_null())
        .order_by_asc(accounts::Column::CreatedAt)
        .order_by_asc(accounts::Column::Id)
        .one(conn)
        .await
        .map_err(TxError::from)?;
    Ok(main_cash.and_then(|a| a.parent_id))
}

/// `nextCashCode` (`branches.ts:49-58`): first free `111x` (x in 1..=9) under the cash parent, else
/// `1119<live account count>` (pathological fallback, still unique).
async fn next_cash_code<C: ConnectionTrait>(conn: &C, parent_id: Option<Id>) -> TxResult<String> {
    let siblings: Vec<accounts::Model> = match parent_id {
        Some(pid) => AccountEntity::find().filter(accounts::Column::ParentId.eq(pid)).all(conn).await.map_err(TxError::from)?,
        None => AccountEntity::find().filter(accounts::Column::ParentId.is_null()).all(conn).await.map_err(TxError::from)?,
    };
    let used: std::collections::HashSet<String> =
        siblings.iter().filter(|a| a.code.len() == 4 && a.code.starts_with("111") && a.code.chars().nth(3).is_some_and(|c| c.is_ascii_digit())).map(|a| a.code.clone()).collect();
    for i in 1..=9 {
        let code = format!("111{i}");
        if !used.contains(&code) {
            return Ok(code);
        }
    }
    let count = AccountEntity::find().count(conn).await.map_err(TxError::from)?;
    Ok(format!("1119{count}"))
}

/// Creates the branch's own cash-drawer account — `createBranchCashAccount` (`branches.ts:61-79`).
async fn create_branch_cash_account<C: ConnectionTrait>(conn: &C, cx: &TxCtx, branch: &Branch) -> TxResult<accounts::Model> {
    let parent_id = cash_parent_id(conn).await?;
    // Lock the parent to serialize code allocation across concurrent branch creates (§4 step 3).
    if let Some(pid) = parent_id {
        crate::core::lock::for_update_by_id(conn, "accounts", &pid.to_string()).await.map_err(TxError::from)?;
    }
    let code = next_cash_code(conn, parent_id).await?;
    let model = accounts::ActiveModel {
        id: Set(Id::new()),
        code: Set(code),
        name: Set(format!("الصندوق — {}", branch.name)),
        name_en: Set(None),
        parent_id: Set(parent_id),
        is_group: Set(false),
        kind: Set("ASSET".to_string()),
        subtype: Set("cash".to_string()),
        normal_side: Set("DEBIT".to_string()),
        system_role: Set(Some("cash".to_string())),
        currency: Set(None),
        branch_id: Set(Some(branch.id)),
        requires_party: Set(None),
        allow_manual: Set(true),
        requires_cost_center: Set(None),
        active: Set(true),
        can_delete: Set(false),
        created_at: Set(cx.clock.now),
        updated_at: Set(cx.clock.now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        code_live: Set(None),
    };
    model.insert(conn).await.map_err(TxError::from)
}

/// Creates the branch's own cost center — `createBranchCostCenter` (`branches.ts:82-94`). A live
/// code clash is refused (D-7 — the mock would create a duplicate).
async fn create_branch_cost_center<C: ConnectionTrait>(conn: &C, cx: &TxCtx, branch: &Branch) -> TxResult<cost_centers::Model> {
    let code = format!("CC-{}", branch.code);
    let clash = CostCenterEntity::find_live()
        .filter(cost_centers::Column::Code.eq(code.clone()))
        .one(conn)
        .await
        .map_err(TxError::from)?
        .is_some();
    if clash {
        return Err(TxError::App(AppError::validation("رمز مركز التكلفة مستخدم بالفعل")));
    }
    let model = cost_centers::ActiveModel {
        id: Set(Id::new()),
        code: Set(code),
        name: Set(branch.name.clone()),
        kind: Set("branch".to_string()),
        parent_id: Set(None),
        manager_user_id: Set(None),
        active: Set(true),
        can_delete: Set(false),
        branch_id: Set(Some(branch.id)),
        created_at: Set(cx.clock.now),
        updated_at: Set(cx.clock.now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        code_live: Set(None),
    };
    model.insert(conn).await.map_err(TxError::from)
}

/// `createBranch` (`branches.ts:100-129`) — `pub` (02-setup reuses it).
pub async fn create<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, input: BranchInput) -> TxResult<Branch> {
    if input.name.trim().is_empty() {
        return Err(TxError::App(AppError::validation("اسم الفرع مطلوب")));
    }
    if input.code.trim().is_empty() {
        return Err(TxError::App(AppError::validation("رمز الفرع مطلوب")));
    }
    let code_lower = input.code.trim().to_lowercase();
    let clash = Entity::find().all(conn).await.map_err(TxError::from)?.iter().any(|b| b.code.to_lowercase() == code_lower);
    if clash {
        return Err(TxError::App(AppError::validation("رمز الفرع مستخدم بالفعل")));
    }

    let model = ActiveModel {
        id: Set(Id::new()),
        name: Set(input.name.trim().to_string()),
        code: Set(input.code.trim().to_uppercase()),
        address: Set(input.address),
        // Q-3: nationalAddress is not copied, matching the mock.
        national_address: Set(None),
        phone: Set(input.phone),
        receipt_header: Set(input.receipt_header),
        cash_account_id: Set(None),
        bank_account_id: Set(input.bank_account_id),
        default_price_list_id: Set(input.default_price_list_id),
        cost_center_id: Set(None),
        active: Set(input.active),
        can_delete: Set(false),
        created_at: Set(cx.clock.now),
        updated_at: Set(cx.clock.now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    let inserted = model.insert(conn).await.map_err(TxError::from)?;
    let mut branch = Branch::from_model(inserted);

    let cash_account = create_branch_cash_account(conn, cx, &branch).await?;
    let cost_center = create_branch_cost_center(conn, cx, &branch).await?;

    let mut update: ActiveModel = Entity::find_by_id(branch.id).one(conn).await.map_err(TxError::from)?.ok_or_else(|| AppError::not_found("الفرع غير موجود"))?.into();
    update.cash_account_id = Set(Some(cash_account.id));
    update.cost_center_id = Set(Some(cost_center.id));
    update.updated_at = Set(cx.clock.now);
    let updated = update.update(conn).await.map_err(TxError::from)?;
    branch = Branch::from_model(updated);

    crate::shared::activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Settings,
        format!("إضافة فرع \"{}\" ({})", branch.name, branch.code),
        None,
        Some(RouteRef::list("settings-branches")),
    )
    .await?;
    cx.touch(ChangeCategory::Ledger);

    Ok(branch)
}

/// `updateBranch` (`branches.ts:131-168`).
pub async fn update<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, id: Id, input: BranchPatch) -> TxResult<Branch> {
    let existing = Entity::find_by_id(id).one(conn).await.map_err(TxError::from)?;
    let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("الفرع غير موجود"))) };

    if let Some(code) = &input.code {
        let code_lower = code.trim().to_lowercase();
        if code_lower != existing.code.to_lowercase() {
            let clash = Entity::find().all(conn).await.map_err(TxError::from)?.iter().any(|b| b.id != id && b.code.to_lowercase() == code_lower);
            if clash {
                return Err(TxError::App(AppError::validation("رمز الفرع مستخدم بالفعل")));
            }
        }
    }

    let before = json_seven(&existing);

    let mut model: ActiveModel = existing.clone().into();
    let mut new_name = existing.name.clone();
    if let Some(v) = &input.name {
        new_name = v.trim().to_string();
        model.name = Set(new_name.clone());
    }
    if let Some(v) = &input.code {
        model.code = Set(v.trim().to_uppercase());
    }
    if let Some(v) = input.address {
        model.address = Set(Some(v));
    }
    if let Some(v) = input.phone {
        model.phone = Set(Some(v));
    }
    if let Some(v) = input.receipt_header {
        model.receipt_header = Set(Some(v));
    }
    if let Some(v) = input.bank_account_id {
        model.bank_account_id = Set(Some(v));
    }
    if let Some(v) = input.default_price_list_id {
        model.default_price_list_id = Set(Some(v));
    }
    model.updated_at = Set(cx.clock.now);
    let updated = model.update(conn).await.map_err(TxError::from)?;

    // name truthy + cash_account_id -> rename that account.
    if input.name.is_some() && !new_name.is_empty() {
        if let Some(cash_id) = updated.cash_account_id {
            if let Some(acc) = AccountEntity::find_by_id(cash_id).one(conn).await.map_err(TxError::from)? {
                let mut acc_model: accounts::ActiveModel = acc.into();
                acc_model.name = Set(format!("الصندوق — {new_name}"));
                acc_model.updated_at = Set(cx.clock.now);
                acc_model.update(conn).await.map_err(TxError::from)?;
            }
        }
    }

    let after = json_seven(&updated);
    let (before_diff, after_diff) = diff_fields(&before, &after);

    record(
        conn,
        cx,
        registry,
        AuditInput {
            entity: "branch".to_string(),
            entity_id: updated.id,
            entity_label: Some(updated.name.clone()),
            action: AuditAction::Update,
            before: Some(serde_json::to_value(&before_diff).unwrap_or_default()),
            after: Some(serde_json::to_value(&after_diff).unwrap_or_default()),
            user_id: None,
            branch_id: None,
            at: None,
            reason: None,
            message: format!("تعديل بيانات الفرع \"{}\"", updated.name),
            link: Some(RouteRef::list("settings-branches")),
            activity_kind: Some(ActivityKind::Settings),
            undo: None,
        },
    )
    .await?;

    Ok(Branch::from_model(updated))
}

/// The 7 fields `updateBranch`'s diff tracks, in that order (`branches.ts:137,153`).
fn json_seven(b: &crate::entities::org::branches::Model) -> Vec<(&'static str, Option<serde_json::Value>)> {
    vec![
        ("name", Some(serde_json::json!(b.name))),
        ("code", Some(serde_json::json!(b.code))),
        ("address", b.address.clone().map(|v| serde_json::json!(v))),
        ("phone", b.phone.clone().map(|v| serde_json::json!(v))),
        ("receiptHeader", b.receipt_header.clone().map(|v| serde_json::json!(v))),
        ("bankAccountId", b.bank_account_id.map(|v| serde_json::json!(v.to_string()))),
        ("defaultPriceListId", b.default_price_list_id.map(|v| serde_json::json!(v.to_string()))),
    ]
}

/// `deactivateBranch` (`branches.ts:171-190`).
pub async fn deactivate<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, id: Id) -> TxResult<Branch> {
    crate::core::lock::for_update_by_id(conn, "branches", &id.to_string()).await.map_err(TxError::from)?;
    let existing = Entity::find_by_id(id).one(conn).await.map_err(TxError::from)?;
    let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("الفرع غير موجود"))) };
    if !existing.active {
        return Ok(Branch::from_model(existing));
    }

    let active_count = Entity::find().filter(Column::Active.eq(true)).count(conn).await.map_err(TxError::from)?;
    if active_count <= 1 {
        return Err(TxError::App(AppError::validation("لا يمكن إلغاء تفعيل الفرع الوحيد النشط")));
    }

    let stock_rows = product_branch_stock::Entity::find()
        .filter(product_branch_stock::Column::BranchId.eq(id))
        .all(conn)
        .await
        .map_err(TxError::from)?;
    // Only live products count (soft-deleted products' stock rows are historical, matching the
    // mock's `db.products` array, which never holds a "deleted" product either).
    let mut stock_left = rust_decimal::Decimal::ZERO;
    for row in &stock_rows {
        let is_live = products::Entity::find_by_id(row.product_id)
            .filter(products::Column::DeletedAt.is_null())
            .one(conn)
            .await
            .map_err(TxError::from)?
            .is_some();
        if is_live {
            stock_left += row.qty.abs();
        }
    }
    if round2(stock_left) > rust_decimal::Decimal::new(1, 3) {
        return Err(TxError::App(AppError::forbidden("لا يمكن إلغاء تفعيل الفرع — لا يزال يحتوي على مخزون. أنشئ تحويلاً لتفريغه أولاً")));
    }

    // Locking read (H-2): a shift can't open concurrently between this check and commit.
    let open_shift = ShiftEntity::find()
        .filter(ShiftColumn::Status.eq(ShiftStatus::Open))
        .filter(ShiftColumn::BranchId.eq(id))
        .one(conn)
        .await
        .map_err(TxError::from)?
        .is_some();
    if open_shift {
        return Err(TxError::App(AppError::forbidden("لا يمكن إلغاء تفعيل الفرع — توجد وردية مفتوحة عليه")));
    }

    let mut model: ActiveModel = existing.clone().into();
    model.active = Set(false);
    model.updated_at = Set(cx.clock.now);
    let updated = model.update(conn).await.map_err(TxError::from)?;

    if let Some(cash_id) = updated.cash_account_id {
        if let Some(acc) = AccountEntity::find_by_id(cash_id).one(conn).await.map_err(TxError::from)? {
            let mut acc_model: accounts::ActiveModel = acc.into();
            acc_model.active = Set(false);
            acc_model.updated_at = Set(cx.clock.now);
            acc_model.update(conn).await.map_err(TxError::from)?;
        }
    }

    crate::shared::activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Settings,
        format!("إلغاء تفعيل الفرع \"{}\"", updated.name),
        None,
        Some(RouteRef::list("settings-branches")),
    )
    .await?;

    Ok(Branch::from_model(updated))
}

/// `reactivateBranch` (`branches.ts:192-202`) — no guard (Q-4), logs even when already active.
pub async fn reactivate<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, id: Id) -> TxResult<Branch> {
    let existing = Entity::find_by_id(id).one(conn).await.map_err(TxError::from)?;
    let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("الفرع غير موجود"))) };

    let mut model: ActiveModel = existing.clone().into();
    model.active = Set(true);
    model.updated_at = Set(cx.clock.now);
    let updated = model.update(conn).await.map_err(TxError::from)?;

    if let Some(cash_id) = updated.cash_account_id {
        if let Some(acc) = AccountEntity::find_by_id(cash_id).one(conn).await.map_err(TxError::from)? {
            let mut acc_model: accounts::ActiveModel = acc.into();
            acc_model.active = Set(true);
            acc_model.updated_at = Set(cx.clock.now);
            acc_model.update(conn).await.map_err(TxError::from)?;
        }
    }

    crate::shared::activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Settings,
        format!("إعادة تفعيل الفرع \"{}\"", updated.name),
        None,
        Some(RouteRef::list("settings-branches")),
    )
    .await?;

    Ok(Branch::from_model(updated))
}
