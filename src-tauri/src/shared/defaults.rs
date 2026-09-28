//! `shared::defaults` (G-26) — three small "what does a new document default to" helpers, each a
//! behaviour-exact port of a mock function that both `06-products`/`06b-inventory` and
//! `07-purchases` (and, through them, `08-invoices`) call. Kept as ONE shared copy (G-26's whole
//! point) instead of being re-derived per domain, exactly like `shared::totals` (G-21) for the
//! totals engine.

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter};

use crate::core::error::AppError;
use crate::entities::org::branches::{self, Entity as BranchEntity};
use crate::entities::org::taxes::{self, Entity as TaxEntity};
use crate::utils::id::Id;

/// `branchPrefix` (`src/mocks/backend/branches.ts:204-208`, docs/v2/10 §1): "RYD-INV-00042" when
/// more than one branch exists in the whole company, unprefixed ("") for a single-branch company —
/// the mock checks `db.branches.length <= 1` (every row, active or not; branches aren't
/// soft-deleted, B-2's own note), then looks up the given branch's `code`. A branch id that
/// doesn't resolve (`branchById` returning `undefined`) yields `""`, exactly like the mock's `branch
/// ? ... : ''`.
pub async fn branch_prefix<C: ConnectionTrait>(conn: &C, branch_id: Option<Id>) -> Result<String, AppError> {
    let branch_count = BranchEntity::find().count(conn).await.map_err(AppError::from)?;
    if branch_count <= 1 {
        return Ok(String::new());
    }
    let Some(branch_id) = branch_id else { return Ok(String::new()) };
    let branch = BranchEntity::find_by_id(branch_id).one(conn).await.map_err(AppError::from)?;
    Ok(match branch {
        Some(b) => format!("{}-", b.code),
        None => String::new(),
    })
}

/// `defaultCostCenterFor` (`branches.ts:264-268`): explicit choice → the branch's own
/// `cost_center_id` → `None`. A branch id that doesn't resolve behaves like `branchById`
/// returning `undefined` — falls through to `None`, same as the mock's `branch?.costCenterId`.
pub async fn default_cost_center_for<C: ConnectionTrait>(conn: &C, branch_id: Option<Id>, explicit: Option<Id>) -> Result<Option<Id>, AppError> {
    if let Some(explicit) = explicit {
        return Ok(Some(explicit));
    }
    let Some(branch_id) = branch_id else { return Ok(None) };
    let branch: Option<branches::Model> = BranchEntity::find_by_id(branch_id).one(conn).await.map_err(AppError::from)?;
    Ok(branch.and_then(|b| b.cost_center_id))
}

/// `purchaseTaxRate` (`src/mocks/backend/core.ts:468-471`): the rate of the active `INPUT` tax
/// flagged `is_default` — `0` when none exists (a store with no default input tax configured
/// still posts purchases, just at 0% until one is set). `r#type`/`active` are plain string/bool
/// columns on `taxes` (never a DB enum — see the entity's own doc comment), matching the mock's
/// `t.type === 'INPUT' && t.isDefault && t.active` exactly. Soft-deleted rows are excluded — the
/// mock's `db.taxes` array never holds a tax the UI would treat as deleted (soft-delete is a
/// backend-only concept the mock doesn't have, but a deleted tax is never a valid "default"
/// candidate either way).
pub async fn purchase_tax_rate<C: ConnectionTrait>(conn: &C) -> Result<Decimal, AppError> {
    let tax = TaxEntity::find()
        .filter(taxes::Column::Type.eq("INPUT"))
        .filter(taxes::Column::IsDefault.eq(true))
        .filter(taxes::Column::Active.eq(true))
        .filter(taxes::Column::DeletedAt.is_null())
        .one(conn)
        .await
        .map_err(AppError::from)?;
    Ok(tax.map(|t| t.rate).unwrap_or(Decimal::ZERO))
}

// Every function here needs a live DB (branch/tax rows) — covered by the DB-backed
// `tests/shared_defaults.rs` suite (deferred time-boxed pass, per the entry file's testing
// convention), not pure unit tests.
