//! `payments, payment_allocations, vouchers, card_settlements, card_settlement_groups`.

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::entities::payments::card_settlement_groups::ActiveModel as CardSettlementGroupActiveModel;
use crate::entities::payments::card_settlements::ActiveModel as CardSettlementActiveModel;
use crate::entities::payments::payment_allocations::{ActiveModel as PaymentAllocationActiveModel, PaymentAllocationTargetKind};
use crate::entities::payments::payments::{ActiveModel as PaymentActiveModel, PaymentMethodKind, PaymentTargetType, PaymentType};
use crate::entities::payments::vouchers::{ActiveModel as VoucherActiveModel, OwnerDirection, VoucherKind};
use crate::entities::values::StringList;
use crate::infrastructure::import::idmap::IdMap;
use crate::infrastructure::import::model::{CardSettlementV1, PaymentV1, VoucherV1};
use crate::infrastructure::import::tables::{lenient_ref, parse_doc_date, resolve_created_at, strict_ref};
use crate::utils::id::Id;
use crate::utils::money::round2;

/// A payment allocation's polymorphic target (B-1): resolve if known (an invoice/PO row was
/// imported), else the 'onboarding'/'onboarding-close' fixed mapping, else mint a fresh id
/// consistently. `payments.target_ref` (= `allocations[0].targetId`) maps the same way.
fn allocation_target_id(id_map: &IdMap, old: &str) -> Id {
    if old == "onboarding" {
        crate::infrastructure::import::idmap::ONBOARDING_SOURCE_ID
    } else if old == "onboarding-close" {
        crate::infrastructure::import::idmap::ONBOARDING_CLOSE_SOURCE_ID
    } else {
        id_map.resolve(old).unwrap_or_else(|| id_map.resolve_or_mint(old))
    }
}

pub async fn insert_payments<C: ConnectionTrait>(
    conn: &C,
    rows: &[PaymentV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في payments"))?;

        let amount = round2(row.amount);
        if amount != row.amount {
            *rounded += 1;
        }

        let kind = if row.kind == "PAID" { PaymentType::Paid } else { PaymentType::Received };
        let target_type = if row.target_type == "supplier" { PaymentTargetType::Supplier } else { PaymentTargetType::Customer };
        let method = match row.method.as_str() {
            "card" => PaymentMethodKind::Card,
            "bank_transfer" => PaymentMethodKind::BankTransfer,
            _ => PaymentMethodKind::Cash,
        };

        // Polymorphic target (B-1): `target_id` names a party (customer/supplier). Since `parties`
        // is inserted well before `payments` in IMPORT_ORDER, a real target resolves through the
        // map directly; `resolve_or_mint` covers a legacy/edge snapshot whose target row wasn't
        // itself imported (never happens for a healthy snapshot, kept for robustness only).
        let target_id = id_map.resolve(&row.target_id).unwrap_or_else(|| id_map.resolve_or_mint(&row.target_id));

        // `targetRef` is `allocations[0].targetId` (`payments.ts:250`), an old document id: it maps
        // exactly like that allocation's `target_id` below, so the DTO's `targetRef` names the same
        // (imported) document as the allocation does.
        let target_ref = row.target_ref.as_deref().filter(|s| !s.is_empty()).map(|r| allocation_target_id(id_map, r).to_string());

        let model = PaymentActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            date_day: Set(day),
            date_instant: Set(instant),
            r#type: Set(kind),
            target_type: Set(target_type),
            target_id: Set(target_id),
            target_ref: Set(target_ref),
            target_ref_number: Set(row.target_ref_number.clone()),
            amount: Set(amount),
            method: Set(method),
            note: Set(row.note.clone()),
            branch_id: Set(strict_ref(id_map, row.branch_id.as_deref(), "payments")?),
            currency: Set(row.currency.clone().map(|c| c.to_uppercase())),
            amount_fc: Set(row.amount_fc.map(round2)),
            rate: Set(row.rate),
            fx_gain_loss: Set(row.fx_gain_loss.map(round2)),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::payments::payments::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (ai, alloc) in row.allocations.iter().enumerate() {
            let target_kind = match alloc.target_kind.as_str() {
                "purchaseOrder" => PaymentAllocationTargetKind::PurchaseOrder,
                "opening" => PaymentAllocationTargetKind::Opening,
                _ => PaymentAllocationTargetKind::Invoice,
            };
            // Polymorphic (B-1): resolve if known (an invoice/PO row was imported), else the
            // 'onboarding'/'onboarding-close' fixed mapping, else mint a fresh id consistently.
            let target_id = allocation_target_id(id_map, &alloc.target_id);
            let amount = round2(alloc.amount);
            if amount != alloc.amount {
                *rounded += 1;
            }
            let (date_day, date_instant) = alloc
                .date
                .as_deref()
                .and_then(|s| parse_doc_date(s, tz))
                .unwrap_or((day, instant));
            let alloc_model = PaymentAllocationActiveModel {
                id: Set(alloc.id.as_deref().map(|old| id_map.assign(old)).unwrap_or_else(Id::new)),
                payment_id: Set(id),
                position: Set(ai as i16),
                target_kind: Set(target_kind),
                target_id: Set(target_id),
                target_number: Set(alloc.target_number.clone()),
                amount: Set(amount),
                date_day: Set(date_day),
                date_instant: Set(date_instant),
                amount_fc: Set(alloc.amount_fc.map(round2)),
                fx_gain_loss: Set(alloc.fx_gain_loss.map(round2)),
            };
            alloc_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}

pub async fn insert_vouchers<C: ConnectionTrait>(
    conn: &C,
    rows: &[VoucherV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(created_by) = id_map.resolve(&row.created_by) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في vouchers")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في vouchers"))?;

        let amount = round2(row.amount);
        if amount != row.amount {
            *rounded += 1;
        }

        let kind = match row.kind.as_str() {
            "PAYMENT" => VoucherKind::Payment,
            "TRANSFER" => VoucherKind::Transfer,
            "OWNER" => VoucherKind::Owner,
            _ => VoucherKind::Receipt,
        };
        let direction = row.direction.as_deref().map(|s| if s == "contribution" { OwnerDirection::Contribution } else { OwnerDirection::Drawings });

        let model = VoucherActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            kind: Set(kind),
            date_day: Set(day),
            date_instant: Set(instant),
            amount: Set(amount),
            description: Set(row.description.clone()),
            note: Set(row.note.clone()),
            attachment_ids: Set(row.attachment_ids.clone().map(StringList)),
            cost_center_id: Set(lenient_ref(id_map, row.cost_center_id.as_deref())),
            created_by: Set(created_by),
            payment_method_id: Set(strict_ref(id_map, row.payment_method_id.as_deref(), "vouchers")?),
            credit_account_id: Set(strict_ref(id_map, row.credit_account_id.as_deref(), "vouchers")?),
            debit_account_id: Set(strict_ref(id_map, row.debit_account_id.as_deref(), "vouchers")?),
            source_account_id: Set(strict_ref(id_map, row.source_account_id.as_deref(), "vouchers")?),
            destination_account_id: Set(strict_ref(id_map, row.destination_account_id.as_deref(), "vouchers")?),
            fee_amount: Set(row.fee_amount.map(round2)),
            fee_account_id: Set(strict_ref(id_map, row.fee_account_id.as_deref(), "vouchers")?),
            direction: Set(direction),
            cash_account_id: Set(strict_ref(id_map, row.cash_account_id.as_deref(), "vouchers")?),
            search_normalized: sea_orm::ActiveValue::NotSet,
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::payments::vouchers::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_card_settlements<C: ConnectionTrait>(
    conn: &C,
    rows: &[CardSettlementV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(created_by) = id_map.resolve(&row.created_by) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في card_settlements")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في card_settlements"))?;

        let gross_amount = round2(row.gross_amount);
        let deposit_amount = round2(row.deposit_amount);
        let fee_amount = round2(row.fee_amount);
        if gross_amount != row.gross_amount || deposit_amount != row.deposit_amount || fee_amount != row.fee_amount {
            *rounded += 1;
        }

        let model = CardSettlementActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            date_day: Set(day),
            date_instant: Set(instant),
            gross_amount: Set(gross_amount),
            deposit_amount: Set(deposit_amount),
            fee_amount: Set(fee_amount),
            note: Set(row.note.clone()),
            created_by: Set(created_by),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::payments::card_settlements::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (gi, group) in row.groups.iter().enumerate() {
            let Some(payment_method_id) = id_map.resolve(&group.payment_method_id) else { continue };
            let group_date = chrono::NaiveDate::parse_from_str(&group.date, "%Y-%m-%d")
                .map_err(|_| AppError::validation("تاريخ غير صالح في card_settlement_groups"))?;
            let amount = round2(group.amount);
            if amount != group.amount {
                *rounded += 1;
            }
            let group_model = CardSettlementGroupActiveModel {
                id: Set(Id::new()),
                card_settlement_id: Set(id),
                position: Set(gi as i16),
                date_day: Set(group_date),
                payment_method_id: Set(payment_method_id),
                amount: Set(amount),
            };
            group_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}
