//! `payment_methods.rs` (02-setup.md §3.5, `setup.ts:168-189`): replaces the entire seeded
//! `payment_methods` shell with the wizard's step-7 selection. Refused once any journal entry
//! exists ("already past go-live" guard, 01.B §8).

use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, PaginatorTrait, Set};

use crate::core::error::AppError;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::journal::journal_entries::Entity as JournalEntryEntity;
use crate::entities::org::payment_methods::{ActiveModel as PaymentMethodActiveModel, Entity as PaymentMethodEntity};
use crate::entities::soft_delete::SoftDelete;
use crate::utils::id::Id;

use super::super::dto::WizardPaymentMethodInput;

/// `applyPaymentMethods` (`setup.ts:168-189`): soft-deletes every live method, inserts each of the
/// wizard's methods (`fee_pct 0`, `show_in_pos/payments true`, `sort_order = i+1`, `can_delete true`).
pub async fn apply<C: ConnectionTrait>(conn: &C, cx: &TxCtx, methods: Vec<WizardPaymentMethodInput>) -> TxResult<()> {
    let journal_count = JournalEntryEntity::find().count(conn).await.map_err(TxError::from)?;
    if journal_count > 0 {
        return Err(TxError::App(AppError::forbidden("لا يمكن تغيير طرق الدفع بعد بدء الترحيل")));
    }

    let live = PaymentMethodEntity::find_live().all(conn).await.map_err(TxError::from)?;
    for m in live {
        PaymentMethodEntity::soft_delete(conn, m.id, cx.clock.now).await.map_err(TxError::from)?;
    }

    for (i, m) in methods.into_iter().enumerate() {
        let model = PaymentMethodActiveModel {
            id: Set(Id::new()),
            name: Set(m.name),
            r#type: Set(m.kind.as_str().to_string()),
            icon: Set(None),
            account_role: Set(m.account_role.as_str().to_string()),
            fee_pct: Set(rust_decimal::Decimal::ZERO),
            requires_reference: Set(None),
            show_in_pos: Set(true),
            show_in_payments: Set(true),
            sort_order: Set((i + 1) as i16),
            branch_overrides: Set(None),
            active: Set(m.active),
            can_delete: Set(true),
            created_at: Set(cx.clock.now + chrono::Duration::milliseconds(i as i64)),
            updated_at: Set(cx.clock.now),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }

    Ok(())
}
