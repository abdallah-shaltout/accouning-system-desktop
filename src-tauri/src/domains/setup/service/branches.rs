//! `branches.rs` (02-setup.md §3.5, `setup.ts:106-151`): the wizard's step-5 "apply branches" —
//! renames the seeded main branch for the first entry, then calls the real `settings::branches::create`
//! (Q-2: the first branch's name/code are stored untrimmed and its cash account is never renamed).

use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::domains::settings::dto::{Branch as BranchDto, BranchInput};
use crate::domains::settings::service::branches as settings_branches;
use crate::entities::org::branches::{ActiveModel as BranchActiveModel, Column as BranchColumn, Entity as BranchEntity};
use crate::shared::activity::undo::UndoRegistry;

use super::super::dto::WizardBranchInput;
use super::shell::format_address;

/// `applyBranches` (`setup.ts:106-151`): empty list -> `VALIDATION`; renames the settings row's
/// default branch for the first entry (untrimmed, Q-2); every other entry goes through the real
/// `createBranch`. Returns every branch in list order.
pub async fn apply<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, branches: Vec<WizardBranchInput>) -> TxResult<Vec<BranchDto>> {
    if branches.is_empty() {
        return Err(TxError::App(AppError::validation("أضف فرعاً واحداً على الأقل")));
    }

    // Serialises wizard steps (§3 "Common").
    let locked = crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;
    let default_branch_id = locked.default_branch_id;

    let mut first_iter = branches.into_iter();
    let first = first_iter.next().unwrap();
    let rest: Vec<WizardBranchInput> = first_iter.collect();

    // Main = the `default_branch_id` branch, else the first branch row (P2-20).
    let main = match BranchEntity::find_by_id(default_branch_id).one(conn).await.map_err(TxError::from)? {
        Some(b) => Some(b),
        None => BranchEntity::find().order_by_asc(BranchColumn::CreatedAt).order_by_asc(BranchColumn::Id).one(conn).await.map_err(TxError::from)?,
    };

    let mut results: Vec<BranchDto> = Vec::new();

    if let Some(main) = main {
        let formatted_address = format_address(first.address.as_ref());
        let mut model: BranchActiveModel = main.into();
        // Q-2: untrimmed name, but code IS upper-cased (matches the mock's `code.toUpperCase()`).
        model.name = Set(first.name.clone());
        model.code = Set(first.code.to_uppercase());
        model.national_address = Set(first.address.clone());
        model.address = Set(formatted_address);
        model.updated_at = Set(cx.clock.now);
        let updated = model.update(conn).await.map_err(TxError::from)?;
        results.push(BranchDto::from_model(updated));
    }

    let has_rest = !rest.is_empty();
    for b in rest {
        // Case-insensitive clash check, including the just-renamed main branch — `settings_branches::create`
        // already re-checks this, but the wizard's own error text names the specific code.
        let code_lower = b.code.to_lowercase();
        let clash = BranchEntity::find().all(conn).await.map_err(TxError::from)?.iter().any(|x| x.code.to_lowercase() == code_lower);
        if clash {
            return Err(TxError::App(AppError::validation(format!("رمز الفرع \"{}\" مستخدم بالفعل", b.code))));
        }
        let input = BranchInput {
            name: b.name.clone(),
            code: b.code.clone(),
            address: format_address(b.address.as_ref()),
            national_address: b.address.clone(),
            phone: None,
            receipt_header: None,
            bank_account_id: None,
            default_price_list_id: None,
            active: true,
        };
        let created = settings_branches::create(conn, cx, registry, input).await?;
        results.push(created);
    }

    if has_rest {
        let locked2 = crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;
        let mut features = locked2.features.unwrap_or(crate::entities::values::FeatureFlags { branches: None, currencies: None, cost_centers: None });
        features.branches = Some(true);
        let mut model2: crate::entities::org::settings::ActiveModel = locked2.into();
        model2.features = Set(Some(features));
        model2.updated_at = Set(cx.clock.now);
        model2.update(conn).await.map_err(TxError::from)?;
    }

    Ok(results)
}
