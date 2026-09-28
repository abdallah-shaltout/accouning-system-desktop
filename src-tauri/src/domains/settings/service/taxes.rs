//! `taxes.rs` — `get_taxes`/`save_tax`/`delete_tax` (01-settings.md §3 "Taxes").

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::{AppError, AppResult};
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::org::taxes::{ActiveModel, Column, Entity};
use crate::entities::platform::activity::ActivityKind;
use crate::entities::platform::audit::AuditAction;
use crate::entities::soft_delete::SoftDelete;
use crate::shared::activity::{record, undo::UndoRegistry, AuditInput};
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use super::super::dto::{Tax, TaxAccountRole, TaxCategory, TaxDirection, TaxInput, TaxLegacyType};

/// `getTaxes` — list order = insertion order (`ORDER BY created_at, id`, entry §3.3).
pub async fn list<C: ConnectionTrait>(conn: &C) -> AppResult<Vec<Tax>> {
    let rows = Entity::find_live().order_by_asc(Column::CreatedAt).order_by_asc(Column::Id).all(conn).await.map_err(AppError::from)?;
    Ok(rows.into_iter().map(Tax::from_model).collect())
}

fn category_str(c: TaxCategory) -> &'static str {
    match c {
        TaxCategory::S => "S",
        TaxCategory::Z => "Z",
        TaxCategory::E => "E",
        TaxCategory::O => "O",
    }
}

fn type_str(t: TaxLegacyType) -> &'static str {
    match t {
        TaxLegacyType::Output => "OUTPUT",
        TaxLegacyType::Input => "INPUT",
    }
}

/// `saveTax` (`settingsService.ts:33-60`), in this exact validation order (Q-1 kept: the
/// clear-other-defaults side effect happens before the `NOT_FOUND` check on update, same as the
/// mock — the surrounding transaction rolls it back on error, so only the error path differs).
pub async fn save<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    input: TaxInput,
    id: Option<Id>,
) -> TxResult<Tax> {
    if input.name.trim().is_empty() {
        return Err(TxError::App(AppError::validation("اسم الضريبة مطلوب")));
    }
    let zero = rust_decimal::Decimal::ZERO;
    let hundred = rust_decimal::Decimal::from(100);
    if !(input.rate >= zero && input.rate <= hundred) {
        return Err(TxError::App(AppError::validation("النسبة يجب أن تكون بين 0 و 100")));
    }
    if matches!(input.category, TaxCategory::E) && input.exemption_reason.as_deref().map(str::trim).unwrap_or("").is_empty() {
        return Err(TxError::App(AppError::validation("سبب الإعفاء مطلوب للضرائب المعفاة")));
    }

    // direction/accountRole are server-derived from `type`, never trusted from the client.
    let (direction, account_role) = match input.kind {
        TaxLegacyType::Output => (TaxDirection::Sales, TaxAccountRole::VatOutput),
        TaxLegacyType::Input => (TaxDirection::Purchase, TaxAccountRole::VatInput),
    };

    if input.is_default {
        Entity::update_many()
            .col_expr(Column::IsDefault, sea_orm::sea_query::Expr::value(false))
            .filter(Column::Type.eq(type_str(input.kind)))
            .filter(Column::DeletedAt.is_null())
            .exec(conn)
            .await
            .map_err(TxError::from)?;
    }

    let exemption_reason = if matches!(input.category, TaxCategory::E) { input.exemption_reason.as_deref().map(|s| s.trim().to_string()) } else { None };

    let saved = if let Some(id) = id {
        let existing = Entity::find_by_id(id).one(conn).await.map_err(TxError::from)?;
        let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("الضريبة غير موجودة"))) };
        let mut model: ActiveModel = existing.into();
        model.name = Set(input.name.clone());
        model.rate = Set(input.rate);
        model.r#type = Set(type_str(input.kind).to_string());
        model.is_default = Set(input.is_default);
        model.active = Set(input.active);
        model.category = Set(category_str(input.category).to_string());
        model.direction = Set(match direction {
            TaxDirection::Sales => "sales".to_string(),
            TaxDirection::Purchase => "purchase".to_string(),
        });
        model.exemption_reason = Set(exemption_reason);
        model.account_role = Set(Some(match account_role {
            TaxAccountRole::VatOutput => "vatOutput".to_string(),
            TaxAccountRole::VatInput => "vatInput".to_string(),
        }));
        model.updated_at = Set(cx.clock.now);
        model.update(conn).await.map_err(TxError::from)?
    } else {
        let model = ActiveModel {
            id: Set(Id::new()),
            name: Set(input.name.clone()),
            rate: Set(input.rate),
            r#type: Set(type_str(input.kind).to_string()),
            is_default: Set(input.is_default),
            active: Set(input.active),
            category: Set(category_str(input.category).to_string()),
            direction: Set(match direction {
                TaxDirection::Sales => "sales".to_string(),
                TaxDirection::Purchase => "purchase".to_string(),
            }),
            exemption_reason: Set(exemption_reason),
            account_role: Set(Some(match account_role {
                TaxAccountRole::VatOutput => "vatOutput".to_string(),
                TaxAccountRole::VatInput => "vatInput".to_string(),
            })),
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
        format!("حفظ الضريبة \"{}\"", saved.name),
        None,
        Some(RouteRef::list("settings-taxes")),
    )
    .await?;

    Ok(Tax::from_model(saved))
}

/// `deleteTax` (`settingsService.ts:62-71`).
pub async fn delete<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, id: Id) -> TxResult<()> {
    let existing = Entity::find_by_id(id).one(conn).await.map_err(TxError::from)?;
    let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("الضريبة غير موجودة"))) };
    if existing.is_default {
        return Err(TxError::App(AppError::forbidden("لا يمكن حذف الضريبة الافتراضية — عيّن ضريبة أخرى افتراضية أولاً")));
    }

    let in_use_invoice = crate::entities::sales::invoice_lines::Entity::find()
        .filter(crate::entities::sales::invoice_lines::Column::TaxId.eq(id))
        .count(conn)
        .await
        .map_err(TxError::from)?
        > 0;
    let in_use_purchase = if in_use_invoice {
        true
    } else {
        crate::entities::purchases::purchase_order_lines::Entity::find()
            .filter(crate::entities::purchases::purchase_order_lines::Column::TaxId.eq(id))
            .count(conn)
            .await
            .map_err(TxError::from)?
            > 0
    };
    if in_use_invoice || in_use_purchase {
        return Err(TxError::App(AppError::forbidden("لا يمكن حذف ضريبة مستخدمة في مستندات مرحّلة")));
    }

    Entity::soft_delete(conn, id, cx.clock.now).await.map_err(TxError::from)?;

    record(
        conn,
        cx,
        registry,
        AuditInput {
            entity: "tax".to_string(),
            entity_id: id,
            entity_label: Some(existing.name.clone()),
            action: AuditAction::Delete,
            before: None,
            after: None,
            user_id: None,
            branch_id: None,
            at: None,
            reason: None,
            message: format!("حذف الضريبة \"{}\"", existing.name),
            link: Some(RouteRef::list("settings-taxes")),
            activity_kind: Some(ActivityKind::Settings),
            undo: None,
        },
    )
    .await?;

    Ok(())
}
