//! `currencies, exchange_rates, branches, accounts, cost_centers, fiscal_years,
//! cost_center_budgets, taxes, payment_methods, users, credentials` — the first block of
//! `order::IMPORT_ORDER` (no FKs, or FKs only within this block).

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::entities::org::accounts::ActiveModel as AccountActiveModel;
use crate::entities::org::branches::ActiveModel as BranchActiveModel;
use crate::entities::org::cost_center_budgets::ActiveModel as CostCenterBudgetActiveModel;
use crate::entities::org::cost_centers::ActiveModel as CostCenterActiveModel;
use crate::entities::org::credentials::ActiveModel as CredentialActiveModel;
use crate::entities::org::currencies::ActiveModel as CurrencyActiveModel;
use crate::entities::org::exchange_rates::ActiveModel as ExchangeRateActiveModel;
use crate::entities::org::fiscal_years::ActiveModel as FiscalYearActiveModel;
use crate::entities::org::payment_methods::{ActiveModel as PaymentMethodActiveModel, BranchOverride, BranchOverrides};
use crate::entities::org::taxes::ActiveModel as TaxActiveModel;
use crate::entities::org::users::ActiveModel as UserActiveModel;
use crate::entities::values::StringList;
use crate::infrastructure::import::idmap::IdMap;
use crate::infrastructure::import::model::{AccountV1, BranchV1, CostCenterV1, CurrencyV1, ExchangeRateV1, FiscalYearV1, PaymentMethodV1, TaxV1, UserV1};
use crate::infrastructure::import::tables::{resolve_created_at, strict_ref};
use crate::utils::id::Id;
use crate::utils::money::{round2, round4};

pub async fn insert_currencies<C: ConnectionTrait>(conn: &C, rows: &[CurrencyV1], import_base: chrono::DateTime<chrono::Utc>) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let created_at = resolve_created_at(row.created_at.as_deref(), import_base, i);
        let model = CurrencyActiveModel {
            code: Set(row.code.to_uppercase()),
            name_ar: Set(row.name_ar.clone()),
            symbol: Set(row.symbol.clone()),
            decimals: Set(row.decimals),
            active: Set(row.active),
            fixed: Set(row.fixed),
            fixed_rate: Set(row.fixed_rate.map(round4)),
            created_at: Set(created_at),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_exchange_rates<C: ConnectionTrait>(
    conn: &C,
    rows: &[ExchangeRateV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = row.id.as_deref().map(|old| id_map.assign(old)).unwrap_or_else(Id::new);
        let date = chrono::NaiveDate::parse_from_str(&row.date, "%Y-%m-%d")
            .map_err(|_| AppError::validation("تاريخ غير صالح في exchange_rates"))?;
        let rate = round4(row.rate);
        if rate != row.rate {
            *rounded += 1;
        }
        let model = ExchangeRateActiveModel {
            id: Set(id),
            currency: Set(row.currency.to_uppercase()),
            date: Set(date),
            rate: Set(rate),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_branches<C: ConnectionTrait>(
    conn: &C,
    rows: &[BranchV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let national_address = row.national_address.as_ref().and_then(|v| serde_json::from_value(v.clone()).ok());
        let model = BranchActiveModel {
            id: Set(id),
            name: Set(row.name.clone()),
            code: Set(row.code.clone()),
            address: Set(row.address.clone()),
            national_address: Set(national_address),
            phone: Set(row.phone.clone()),
            receipt_header: Set(row.receipt_header.clone()),
            // Deferred (order::DEFERRED covers only accounts/categories/cost_centers self-refs and
            // audit/fiscal_years/users/journal_entries forward refs — branches' own forward refs
            // into accounts/price_lists/cost_centers are resolved inline here instead, since
            // accounts/cost_centers/price_lists are NOT all guaranteed to precede branches in
            // IMPORT_ORDER... they ARE: order.rs puts branches before accounts/cost_centers/
            // price_lists, so these must be deferred too. Written NULL here; phase B sets them.
            cash_account_id: Set(None),
            bank_account_id: Set(None),
            default_price_list_id: Set(None),
            cost_center_id: Set(None),
            active: Set(row.active),
            can_delete: Set(row.can_delete),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

/// Phase B for `branches`' forward references (see the note above — `branches` needed its own
/// deferred set added to `order::DEFERRED` for `cash_account_id`/`bank_account_id`/
/// `default_price_list_id`/`cost_center_id`; done inline here rather than in `run.rs`'s generic
/// phase B loop since these four columns are branch-specific and the generic loop only needs to
/// know "which (table, column) pairs exist", which `order::DEFERRED` must list — the manager should
/// add these four pairs to `order::DEFERRED` if this file's own list above is kept in sync
/// separately; see the "Needs from manager" note in the final report).
pub async fn set_branch_forward_refs<C: ConnectionTrait>(conn: &C, id: Id, row: &BranchV1, id_map: &IdMap) -> TxResult<()> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let mut am = crate::entities::org::branches::ActiveModel { id: Set(id), ..Default::default() };
    let mut touched = false;
    if let Some(old) = &row.cash_account_id {
        if let Some(new_id) = id_map.resolve(old) {
            am.cash_account_id = Set(Some(new_id));
            touched = true;
        }
    }
    if let Some(old) = &row.bank_account_id {
        if let Some(new_id) = id_map.resolve(old) {
            am.bank_account_id = Set(Some(new_id));
            touched = true;
        }
    }
    if let Some(old) = &row.default_price_list_id {
        if let Some(new_id) = id_map.resolve(old) {
            am.default_price_list_id = Set(Some(new_id));
            touched = true;
        }
    }
    if let Some(old) = &row.cost_center_id {
        if let Some(new_id) = id_map.resolve(old) {
            am.cost_center_id = Set(Some(new_id));
            touched = true;
        }
    }
    if touched {
        crate::entities::org::branches::Entity::update_many()
            .set(am)
            .filter(crate::entities::org::branches::Column::Id.eq(id))
            .exec(conn)
            .await
            .map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_accounts<C: ConnectionTrait>(
    conn: &C,
    rows: &[AccountV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let model = AccountActiveModel {
            id: Set(id),
            code: Set(row.code.clone()),
            name: Set(row.name.clone()),
            name_en: Set(row.name_en.clone()),
            // Deferred (self-reference) — phase B fills this in.
            parent_id: Set(None),
            is_group: Set(row.is_group),
            kind: Set(row.kind.clone()),
            subtype: Set(row.subtype.clone()),
            normal_side: Set(row.normal_side.clone()),
            system_role: Set(row.system_role.clone()),
            currency: Set(row.currency.clone().map(|c| c.to_uppercase())),
            branch_id: Set(strict_ref(id_map, row.branch_id.as_deref(), "accounts")?),
            requires_party: Set(row.requires_party),
            allow_manual: Set(row.allow_manual),
            requires_cost_center: Set(row.requires_cost_center),
            active: Set(row.active),
            can_delete: Set(row.can_delete),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
            code_live: sea_orm::ActiveValue::NotSet,
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_cost_centers<C: ConnectionTrait>(
    conn: &C,
    rows: &[CostCenterV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let model = CostCenterActiveModel {
            id: Set(id),
            code: Set(row.code.clone()),
            name: Set(row.name.clone()),
            kind: Set(row.kind.clone()),
            // Deferred (self-reference).
            parent_id: Set(None),
            // Deferred (forward ref into users, inserted later in IMPORT_ORDER).
            manager_user_id: Set(None),
            active: Set(row.active),
            can_delete: Set(row.can_delete),
            branch_id: Set(strict_ref(id_map, row.branch_id.as_deref(), "cost_centers")?),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
            code_live: sea_orm::ActiveValue::NotSet,
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_fiscal_years<C: ConnectionTrait>(
    conn: &C,
    rows: &[FiscalYearV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let start_date = chrono::NaiveDate::parse_from_str(&row.start_date, "%Y-%m-%d")
            .map_err(|_| AppError::validation("تاريخ غير صالح في fiscal_years"))?;
        let end_date = chrono::NaiveDate::parse_from_str(&row.end_date, "%Y-%m-%d")
            .map_err(|_| AppError::validation("تاريخ غير صالح في fiscal_years"))?;
        let model = FiscalYearActiveModel {
            id: Set(id),
            name: Set(row.name.clone()),
            start_date: Set(start_date),
            end_date: Set(end_date),
            is_closed: Set(row.is_closed),
            // Deferred (forward ref into journal_entries).
            closing_entry_id: Set(None),
            closed_at: Set(row.closed_at.as_deref().and_then(crate::infrastructure::import::model::parse_instant)),
            // Deferred (forward ref into users, inserted later) — run.rs phase B.
            closed_by: Set(None),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (bi, budget) in row.budgets.iter().enumerate() {
            let budget_id = budget.id.as_deref().map(|old| id_map.assign(old)).unwrap_or_else(Id::new);
            let cc_old = budget.cost_center_id.as_deref();
            let Some(cost_center_id) = cc_old.and_then(|old| id_map.resolve(old)) else { continue };
            let amount = round2(budget.amount);
            let budget_model = CostCenterBudgetActiveModel {
                id: Set(budget_id),
                cost_center_id: Set(cost_center_id),
                fiscal_year_id: Set(id),
                amount: Set(amount),
                created_at: Set(resolve_created_at(None, import_base, bi)),
                updated_at: Set(resolve_created_at(None, import_base, bi)),
            };
            budget_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}

pub async fn insert_taxes<C: ConnectionTrait>(conn: &C, rows: &[TaxV1], id_map: &IdMap, import_base: chrono::DateTime<chrono::Utc>) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let model = TaxActiveModel {
            id: Set(id),
            name: Set(row.name.clone()),
            rate: Set(round4_pct(row.rate)),
            r#type: Set(row.kind.clone()),
            is_default: Set(row.is_default),
            active: Set(row.active),
            category: Set(row.category.clone().unwrap_or_else(|| "S".to_string())),
            direction: Set(row.direction.clone().unwrap_or_else(|| "sales".to_string())),
            exemption_reason: Set(row.exemption_reason.clone()),
            account_role: Set(row.account_role.clone()),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

/// `taxes.rate` is `(9,4)` (a percentage stored as e.g. `15.0000`), never rounded per §3.2.5 ("rate
/// (19,6) and % (9,4) are never rounded — scale fits JS doubles already written by round2/round4").
/// This is a pass-through, named so a caller reads it as an intentional no-op, not an oversight.
fn round4_pct(v: Decimal) -> Decimal {
    v
}

pub async fn insert_payment_methods<C: ConnectionTrait>(
    conn: &C,
    rows: &[PaymentMethodV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let branch_overrides: Option<BranchOverrides> = row.branch_overrides.as_ref().and_then(|v| {
            let raw: Vec<serde_json::Value> = serde_json::from_value(v.clone()).ok()?;
            let mut out = Vec::new();
            for item in raw {
                let branch_old = item.get("branchId")?.as_str()?;
                let account_old = item.get("accountId")?.as_str()?;
                let branch_id = id_map.resolve(branch_old)?;
                let account_id = id_map.resolve(account_old)?;
                out.push(BranchOverride { branch_id, account_id });
            }
            Some(BranchOverrides(out))
        });
        let model = PaymentMethodActiveModel {
            id: Set(id),
            name: Set(row.name.clone()),
            r#type: Set(row.kind.clone()),
            icon: Set(row.icon.clone()),
            account_role: Set(row.account_role.clone()),
            fee_pct: Set(row.fee_pct),
            requires_reference: Set(row.requires_reference),
            show_in_pos: Set(row.show_in_pos),
            show_in_payments: Set(row.show_in_payments),
            sort_order: Set(row.sort_order),
            branch_overrides: Set(branch_overrides),
            active: Set(row.active),
            can_delete: Set(row.can_delete),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_users<C: ConnectionTrait>(conn: &C, rows: &[UserV1], id_map: &IdMap, import_base: chrono::DateTime<chrono::Utc>) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let allowed_branches: Option<StringList> = row.allowed_branches.as_ref().map(|list| {
            StringList(list.iter().filter_map(|old| id_map.resolve(old)).map(|id| id.to_string()).collect())
        });
        let model = UserActiveModel {
            id: Set(id),
            username: Set(row.username.clone()),
            name: Set(row.name.clone()),
            phone: Set(row.phone.clone()),
            role: Set(row.role.clone()),
            max_discount: Set(row.max_discount),
            // Deferred (forward ref into price_lists, inserted later).
            price_list_id: Set(None),
            active: Set(row.active),
            avatar: Set(row.avatar.clone()),
            allowed_branches: Set(allowed_branches),
            home_branch: Set(strict_ref(id_map, row.home_branch.as_deref(), "users")?),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

/// Step 7 (cross-cutting §1/§9): for each `(username, password)` in `data.credentials`, find the
/// **imported** user with `username ===` (exact, case-sensitive) and insert
/// `credentials(user_id, password_hash)`. A credential with no matching user is skipped and counted
/// in `skipped` (for the report log line); a user with no credential gets no row. Hashing runs in
/// `tokio::task::spawn_blocking` per the spec (argon2 is CPU-bound).
pub async fn insert_credentials<C: ConnectionTrait>(
    conn: &C,
    credentials: &std::collections::HashMap<String, String>,
    users: &[UserV1],
    id_map: &IdMap,
    now: chrono::DateTime<chrono::Utc>,
) -> TxResult<i64> {
    let mut skipped = 0i64;
    for (username, password) in credentials.iter() {
        let Some(user) = users.iter().find(|u| &u.username == username) else {
            skipped += 1;
            continue;
        };
        let Some(user_id) = id_map.resolve(&user.id) else {
            skipped += 1;
            continue;
        };
        let password = password.clone();
        let hash = tokio::task::spawn_blocking(move || crate::core::auth::hash_password(&password))
            .await
            .map_err(|e| TxError::App(AppError::internal("فشل تجزئة كلمة المرور أثناء الاستيراد", Some(e.to_string()))))?
            .map_err(|e| TxError::App(AppError::internal("فشل تجزئة كلمة المرور أثناء الاستيراد", Some(e.to_string()))))?;

        let model = CredentialActiveModel { user_id: Set(user_id), password_hash: Set(hash), created_at: Set(now), updated_at: Set(now) };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(skipped)
}
