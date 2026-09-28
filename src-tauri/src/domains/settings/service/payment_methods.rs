//! `payment_methods.rs` — `get_payment_methods`/`save_payment_method`/`reorder_payment_methods`/
//! `delete_payment_method` (01-settings.md §3 "Payment methods").

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::{AppError, AppResult};
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::org::payment_methods::{ActiveModel, BranchOverride, BranchOverrides, Column, Entity};
use crate::entities::platform::activity::ActivityKind;
use crate::entities::platform::audit::AuditAction;
use crate::entities::soft_delete::SoftDelete;
use crate::shared::activity::{record, undo::UndoRegistry, AuditInput};
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use super::super::dto::{PaymentMethod, PaymentMethodInput};

/// `getPaymentMethods` — `ORDER BY sort_order, created_at, id` (stable sort, `settingsService.ts:79`).
pub async fn list<C: ConnectionTrait>(conn: &C) -> AppResult<Vec<PaymentMethod>> {
    let rows = Entity::find_live()
        .order_by_asc(Column::SortOrder)
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(PaymentMethod::from_model).collect())
}

fn overrides_from_dto(input: &Option<Vec<super::super::dto::BranchAccountOverride>>) -> Option<BranchOverrides> {
    input
        .as_ref()
        .map(|list| BranchOverrides(list.iter().map(|o| BranchOverride { branch_id: o.branch_id, account_id: o.account_id }).collect()))
}

/// `savePaymentMethod` (`settingsService.ts:82-100`).
pub async fn save<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    input: PaymentMethodInput,
    id: Option<Id>,
) -> TxResult<PaymentMethod> {
    if input.name.trim().is_empty() {
        return Err(TxError::App(AppError::validation("اسم طريقة الدفع مطلوب")));
    }
    let zero = rust_decimal::Decimal::ZERO;
    let hundred = rust_decimal::Decimal::from(100);
    if !(input.fee_pct >= zero && input.fee_pct <= hundred) {
        return Err(TxError::App(AppError::validation("نسبة العمولة يجب أن تكون بين 0 و 100")));
    }

    let saved = if let Some(id) = id {
        let existing = Entity::find_by_id(id).one(conn).await.map_err(TxError::from)?;
        let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("طريقة الدفع غير موجودة"))) };
        let mut model: ActiveModel = existing.into();
        model.name = Set(input.name.clone());
        model.r#type = Set(input.kind.as_str().to_string());
        model.icon = Set(input.icon.clone());
        model.account_role = Set(input.account_role.as_str().to_string());
        model.fee_pct = Set(input.fee_pct);
        model.requires_reference = Set(input.requires_reference);
        model.show_in_pos = Set(input.show_in_pos);
        model.show_in_payments = Set(input.show_in_payments);
        model.sort_order = Set(input.sort_order as i16);
        model.branch_overrides = Set(overrides_from_dto(&input.branch_overrides));
        model.active = Set(input.active);
        model.updated_at = Set(cx.clock.now);
        model.update(conn).await.map_err(TxError::from)?
    } else {
        let model = ActiveModel {
            id: Set(Id::new()),
            name: Set(input.name.clone()),
            r#type: Set(input.kind.as_str().to_string()),
            icon: Set(input.icon.clone()),
            account_role: Set(input.account_role.as_str().to_string()),
            fee_pct: Set(input.fee_pct),
            requires_reference: Set(input.requires_reference),
            show_in_pos: Set(input.show_in_pos),
            show_in_payments: Set(input.show_in_payments),
            sort_order: Set(input.sort_order as i16),
            branch_overrides: Set(overrides_from_dto(&input.branch_overrides)),
            active: Set(input.active),
            can_delete: Set(true),
            created_at: Set(cx.clock.now),
            updated_at: Set(cx.clock.now),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        model.insert(conn).await.map_err(TxError::from)?
    };

    crate::shared::activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Settings,
        format!("حفظ طريقة الدفع \"{}\"", saved.name),
        None,
        Some(RouteRef::list("settings-payment-methods")),
    )
    .await?;

    Ok(PaymentMethod::from_model(saved))
}

/// `reorderPaymentMethods` (`settingsService.ts:102-108`): rows locked in id order (§4 concurrency),
/// `sort_order = index + 1` per listed id; unknown ids ignored; unlisted keep their order; no audit.
pub async fn reorder<C: ConnectionTrait>(conn: &C, ordered_ids: Vec<Id>) -> TxResult<()> {
    let id_strings: Vec<String> = ordered_ids.iter().map(|id| id.to_string()).collect();
    crate::core::lock::for_update_many_sorted(conn, "payment_methods", &id_strings).await.map_err(TxError::from)?;

    for (index, id) in ordered_ids.iter().enumerate() {
        Entity::update_many()
            .col_expr(Column::SortOrder, sea_orm::sea_query::Expr::value((index as i16) + 1))
            .filter(Column::Id.eq(*id))
            .exec(conn)
            .await
            .map_err(TxError::from)?;
    }
    Ok(())
}

/// `deletePaymentMethod` (`settingsService.ts:110-122`).
pub async fn delete<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, id: Id) -> TxResult<()> {
    let existing = Entity::find_by_id(id).one(conn).await.map_err(TxError::from)?;
    let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("طريقة الدفع غير موجودة"))) };
    if !existing.can_delete {
        return Err(TxError::App(AppError::forbidden("لا يمكن حذف طريقة الدفع الأساسية — عطّلها بدلاً من ذلك")));
    }

    let in_tenders = crate::entities::sales::invoice_tenders::Entity::find()
        .filter(crate::entities::sales::invoice_tenders::Column::PaymentMethodId.eq(id))
        .count(conn)
        .await
        .map_err(TxError::from)?
        > 0;
    let in_vouchers = if in_tenders {
        true
    } else {
        crate::entities::payments::vouchers::Entity::find()
            .filter(crate::entities::payments::vouchers::Column::PaymentMethodId.eq(id))
            .count(conn)
            .await
            .map_err(TxError::from)?
            > 0
    };
    let in_expenses = if in_tenders || in_vouchers {
        true
    } else {
        crate::entities::expenses::expenses::Entity::find()
            .filter(crate::entities::expenses::expenses::Column::PaidFromKind.eq(crate::entities::expenses::expenses::PaidFromKind::Method))
            .filter(crate::entities::expenses::expenses::Column::PaidFromPaymentMethodId.eq(id))
            .count(conn)
            .await
            .map_err(TxError::from)?
            > 0
    };
    if in_tenders || in_vouchers || in_expenses {
        return Err(TxError::App(AppError::forbidden("لا يمكن حذف طريقة دفع مستخدمة في مستندات مرحّلة")));
    }

    Entity::soft_delete(conn, id, cx.clock.now).await.map_err(TxError::from)?;

    record(
        conn,
        cx,
        registry,
        AuditInput {
            entity: "paymentMethod".to_string(),
            entity_id: id,
            entity_label: Some(existing.name.clone()),
            action: AuditAction::Delete,
            before: None,
            after: None,
            user_id: None,
            branch_id: None,
            at: None,
            reason: None,
            message: format!("حذف طريقة الدفع \"{}\"", existing.name),
            link: Some(RouteRef::list("settings-payment-methods")),
            activity_kind: Some(ActivityKind::Settings),
            undo: None,
        },
    )
    .await?;

    Ok(())
}
