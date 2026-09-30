//! `domains::parties::service::link` — §3 `link`/`unlink`/`get_linked_net_balance`.

use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::parties::parties::{self as parties_entity, ActiveModel as PartyActiveModel};
use crate::entities::platform::activity::ActivityKind;
use crate::shared::activity::{self, UndoRegistry};
use crate::shared::balances;
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use super::super::dto::PartyKind;

/// **`link`** (`partyService.ts:213-224`): both rows locked (sorted ids), both must exist (customer
/// kind / supplier kind respectively) else `NOT_FOUND` `الطرف غير موجود`; sets `linked_party_id`
/// both ways (previous partners untouched — Q-3); logs once, keyed off the customer's detail route.
pub async fn link<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, customer_id: Id, supplier_id: Id) -> TxResult<()> {
    let mut ids = vec![customer_id.to_string(), supplier_id.to_string()];
    ids.sort();
    lock::for_update_many_sorted(conn, "parties", &ids).await?;

    let customer = parties_entity::Entity::find_by_id(customer_id)
        .one(conn)
        .await
        .map_err(AppError::from)?
        .filter(|p| p.kind == "customer")
        .ok_or_else(|| AppError::not_found("الطرف غير موجود"))?;
    let supplier = parties_entity::Entity::find_by_id(supplier_id)
        .one(conn)
        .await
        .map_err(AppError::from)?
        .filter(|p| p.kind == "supplier")
        .ok_or_else(|| AppError::not_found("الطرف غير موجود"))?;

    let mut customer_am: PartyActiveModel = customer.clone().into();
    customer_am.linked_party_id = Set(Some(supplier.id));
    // The mock's link never touches `updatedAt`; keep the column as is so `ON UPDATE
    // CURRENT_TIMESTAMP(3)` can't bump it (the DTO shows `updatedAt` only when it differs from
    // `createdAt`, 05 §DTO).
    customer_am.updated_at = Set(customer.updated_at);
    customer_am.update(conn).await.map_err(AppError::from)?;

    let mut supplier_am: PartyActiveModel = supplier.clone().into();
    supplier_am.linked_party_id = Set(Some(customer.id));
    supplier_am.updated_at = Set(supplier.updated_at);
    supplier_am.update(conn).await.map_err(AppError::from)?;

    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Party,
        format!("ربط بطاقتي \"{}\" (عميل) و\"{}\" (مورد)", customer.name, supplier.name),
        None,
        Some(RouteRef::detail("customer", customer.id.to_string())),
    )
    .await?;

    cx.touch(crate::core::events::ChangeCategory::Parties);
    Ok(())
}

/// **`unlink`** (`partyService.ts:226-245`): plain read first (no lock) to learn whether there's a
/// link at all — missing party or no `linked_party_id` is a silent `Ok(())` (no activity, no
/// touch). Otherwise locks both ids (sorted), re-reads, clears the party's link and, if the
/// counterpart row of the other kind still exists, clears its link unconditionally (§3 — the code's
/// behavior, not parties.md §1's wording).
pub async fn unlink<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, party_id: Id, kind: PartyKind) -> TxResult<()> {
    let kind_str = kind.as_str();
    let counterpart_kind_str = match kind {
        PartyKind::Customer => "supplier",
        PartyKind::Supplier => "customer",
    };

    let initial = parties_entity::Entity::find_by_id(party_id).one(conn).await.map_err(AppError::from)?.filter(|p| p.kind == kind_str);
    let Some(initial) = initial else { return Ok(()) };
    let Some(linked_id) = initial.linked_party_id else { return Ok(()) };

    let mut ids = vec![party_id.to_string(), linked_id.to_string()];
    ids.sort();
    lock::for_update_many_sorted(conn, "parties", &ids).await?;

    let p = parties_entity::Entity::find_by_id(party_id).one(conn).await.map_err(AppError::from)?.filter(|p| p.kind == kind_str);
    let Some(p) = p else { return Ok(()) };
    let counterpart = parties_entity::Entity::find_by_id(linked_id).one(conn).await.map_err(AppError::from)?.filter(|c| c.kind == counterpart_kind_str);

    let mut p_am: PartyActiveModel = p.clone().into();
    p_am.linked_party_id = Set(None);
    p_am.updated_at = Set(p.updated_at); // unlink never touches `updatedAt` either (see `link`).
    p_am.update(conn).await.map_err(AppError::from)?;

    if let Some(counterpart) = &counterpart {
        let mut c_am: PartyActiveModel = counterpart.clone().into();
        c_am.linked_party_id = Set(None);
        c_am.updated_at = Set(counterpart.updated_at);
        c_am.update(conn).await.map_err(AppError::from)?;
    }

    let route_name = match kind {
        PartyKind::Customer => "customer",
        PartyKind::Supplier => "supplier",
    };
    let message = match &counterpart {
        Some(c) => format!("إلغاء ربط بطاقة \"{}\" و\"{}\"", p.name, c.name),
        None => format!("إلغاء ربط بطاقة \"{}\"", p.name),
    };
    activity::log(conn, cx, registry, ActivityKind::Party, message, None, Some(RouteRef::detail(route_name, p.id.to_string()))).await?;

    cx.touch(crate::core::events::ChangeCategory::Parties);
    Ok(())
}

/// **`get_linked_net_balance`** (`partyService.ts:248-252`): either id absent -> `None`; else
/// `customer_balance − supplier_balance` (no extra rounding — both are already `round2`'d).
pub async fn get_linked_net_balance<C: ConnectionTrait>(conn: &C, customer_id: Option<Id>, supplier_id: Option<Id>) -> TxResult<Option<rust_decimal::Decimal>> {
    let (Some(customer_id), Some(supplier_id)) = (customer_id, supplier_id) else { return Ok(None) };
    let customer_balance = balances::customer_balance(conn, customer_id).await?;
    let supplier_balance = balances::supplier_balance(conn, supplier_id).await?;
    Ok(Some(customer_balance - supplier_balance))
}
