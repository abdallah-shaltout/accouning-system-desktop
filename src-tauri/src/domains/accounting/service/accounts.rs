//! Chart-of-accounts CRUD (12-accounting.md §3.2), porting `accountingService.ts:40-153`.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, Set, Statement};

use crate::core::error::{map_unique_violation, AppError};
use crate::core::lock;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity};
use crate::entities::org::accounts::{ActiveModel, Column, Entity, Model as AccountModel};
use crate::entities::soft_delete::SoftDelete;
use crate::shared::activity;
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::dto::{Account, AccountInput, AccountWithBalance, DateRange, NormalSide};

const CODE_LIVE_CONSTRAINT: &str = "uq_accounts_code_live";

/// **`get_accounts(range)`** (:40-58): live accounts `ORDER BY code`, with per-account posted
/// debit/credit totals over `range` and a signed balance.
pub async fn get_accounts<C: ConnectionTrait>(conn: &C, range: Option<DateRange>) -> TxResult<Vec<AccountWithBalance>> {
    let accounts = Entity::find_live().all(conn).await.map_err(TxError::from)?;

    let range = range.unwrap_or_default();
    let from = range.from.as_deref();
    let to = range.to.as_deref();

    // One aggregate over posted lines, filtered by the business day (`inDateRange`) — each bound
    // only applied when non-empty.
    let mut sql = "SELECT jl.account_id AS account_id, SUM(jl.debit) AS d, SUM(jl.credit) AS c \
                    FROM journal_lines jl JOIN journal_entries je ON je.id = jl.journal_entry_id \
                    WHERE 1 = 1"
        .to_string();
    let mut values: Vec<sea_orm::Value> = Vec::new();
    if let Some(from) = from {
        sql.push_str(" AND je.date_day >= ?");
        values.push(from.into());
    }
    if let Some(to) = to {
        sql.push_str(" AND je.date_day <= ?");
        values.push(to.into());
    }
    sql.push_str(" GROUP BY jl.account_id");

    let stmt = Statement::from_sql_and_values(conn.get_database_backend(), &sql, values);
    let rows = conn.query_all(stmt).await.map_err(TxError::from)?;

    let mut totals: BTreeMap<Id, (Decimal, Decimal)> = BTreeMap::new();
    for row in rows {
        let account_id: Id = row.try_get("", "account_id").map_err(TxError::from)?;
        let d: Option<Decimal> = row.try_get("", "d").ok();
        let c: Option<Decimal> = row.try_get("", "c").ok();
        totals.insert(account_id, (d.unwrap_or(Decimal::ZERO), c.unwrap_or(Decimal::ZERO)));
    }

    let mut result: Vec<AccountWithBalance> = accounts
        .into_iter()
        .map(|a| {
            let (d, c) = totals.get(&a.id).copied().unwrap_or((Decimal::ZERO, Decimal::ZERO));
            let debit_total = round2(d);
            let credit_total = round2(c);
            let normal_side = NormalSide::from_str_opt(&a.normal_side).unwrap_or(NormalSide::Debit);
            let balance = round2(match normal_side {
                NormalSide::Debit => d - c,
                NormalSide::Credit => c - d,
            });
            let has_postings = totals.contains_key(&a.id);
            AccountWithBalance::from_account(Account::from_model(a), debit_total, credit_total, balance, has_postings)
        })
        .collect();

    // `ORDER BY code` — MariaDB `utf8mb4_bin` matches the mock's `localeCompare` on plain digit
    // strings; Rust `str` ordering (byte order) is the same for ASCII digits.
    result.sort_by(|a, b| a.code.cmp(&b.code));

    Ok(result)
}

fn validate_account(input: &AccountInput, account_by_id: &BTreeMap<Id, AccountModel>, except_id: Option<Id>) -> Result<(), AppError> {
    if !input.code.chars().all(|c| c.is_ascii_digit()) || input.code.is_empty() || input.code.len() > 8 {
        return Err(AppError::validation("رمز الحساب أرقام فقط (حتى 8 أرقام)"));
    }
    if input.name.trim().is_empty() {
        return Err(AppError::validation("اسم الحساب مطلوب"));
    }
    // Code clash comes before the parent checks (`validateAccount`'s order — the first rule wins).
    if account_by_id.values().any(|a| Some(a.id) != except_id && a.code == input.code) {
        return Err(AppError::conflict("رمز الحساب مستخدم من قبل"));
    }
    if let Some(parent_id) = input.parent_id {
        if Some(parent_id) == except_id {
            return Err(AppError::validation("لا يمكن أن يكون الحساب أباً لنفسه"));
        }
        let parent = account_by_id.get(&parent_id).ok_or_else(|| AppError::validation("الحساب الأب غير موجود"))?;
        if !parent.is_group {
            return Err(AppError::validation("الحساب الأب يجب أن يكون حساباً رئيسياً (تجميعياً)"));
        }
        if parent.kind != input.kind.as_str() {
            return Err(AppError::validation(format!("الحساب الأب من نوع مختلف ({})", parent.kind)));
        }
        if !input.code.starts_with(&parent.code) {
            return Err(AppError::validation(format!("رمز الحساب يجب أن يبدأ برمز الحساب الأب ({})", parent.code)));
        }
    }
    if input.is_group && input.allow_manual {
        return Err(AppError::validation("الحسابات الرئيسية (التجميعية) لا تقبل الترحيل المباشر"));
    }
    Ok(())
}

/// **`save_account(input, id)`** (:81-119).
pub async fn save_account<C: ConnectionTrait>(conn: &C, cx: &TxCtx, input: AccountInput, id: Option<Id>) -> TxResult<Account> {
    // Load every live account once (small table) to validate the code/parent/kind rules the same
    // way the mock's `db.accounts.find(...)` in-memory scan does.
    let all = Entity::find_live().all(conn).await.map_err(TxError::from)?;
    let by_id: BTreeMap<Id, AccountModel> = all.iter().map(|a| (a.id, a.clone())).collect();
    // Includes the code-clash pre-check (defensive; the DB's `uq_accounts_code_live` is the real gate, §4).
    validate_account(&input, &by_id, id)?;

    let name = input.name.trim().to_string();

    let saved = if let Some(id) = id {
        lock::for_update_by_id(conn, "accounts", &id.to_string()).await.map_err(TxError::from)?;
        let existing = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
        let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("الحساب غير موجود"))) };

        if !existing.can_delete && (input.code != existing.code || input.kind.as_str() != existing.kind || input.is_group != existing.is_group) {
            return Err(TxError::App(AppError::validation("لا يمكن تغيير رمز أو نوع أو شكل (رئيسي/فرعي) حساب أساسي في النظام")));
        }

        if existing.is_group != input.is_group {
            let has_children = Entity::find_live().filter(Column::ParentId.eq(existing.id)).count(conn).await.map_err(TxError::from)? > 0;
            if has_children {
                return Err(TxError::App(AppError::validation("للحساب حسابات فرعية — لا يمكن جعله فرعياً (postable)")));
            }
            let has_postings = LineEntity::find().filter(LineColumn::AccountId.eq(existing.id)).count(conn).await.map_err(TxError::from)? > 0;
            if has_postings {
                return Err(TxError::App(AppError::validation("للحساب قيود مسجلة — لا يمكن جعله رئيسياً (تجميعياً)")));
            }
        }

        let mut model: ActiveModel = existing.into();
        model.code = Set(input.code.clone());
        model.name = Set(name.clone());
        model.parent_id = Set(input.parent_id);
        model.is_group = Set(input.is_group);
        model.kind = Set(input.kind.as_str().to_string());
        model.subtype = Set(input.subtype.as_str().to_string());
        model.normal_side = Set(input.normal_side.as_str().to_string());
        model.allow_manual = Set(input.allow_manual);
        model.active = Set(input.active);
        if input.name_en.is_some() {
            model.name_en = Set(input.name_en.clone());
        }
        if input.requires_party.is_some() {
            model.requires_party = Set(input.requires_party);
        }
        model
            .update(conn)
            .await
            .map_err(|e| map_unique_violation(e, CODE_LIVE_CONSTRAINT, || "رمز الحساب مستخدم من قبل".to_string()))?
    } else {
        let now = cx.clock.now;
        let model = ActiveModel {
            id: Set(Id::new()),
            code: Set(input.code.clone()),
            name: Set(name.clone()),
            name_en: Set(input.name_en.clone()),
            parent_id: Set(input.parent_id),
            is_group: Set(input.is_group),
            kind: Set(input.kind.as_str().to_string()),
            subtype: Set(input.subtype.as_str().to_string()),
            normal_side: Set(input.normal_side.as_str().to_string()),
            system_role: Set(None),
            currency: Set(None),
            branch_id: Set(None),
            requires_party: Set(input.requires_party),
            allow_manual: Set(input.allow_manual),
            requires_cost_center: Set(None),
            active: Set(input.active),
            can_delete: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
            code_live: sea_orm::ActiveValue::NotSet,
        };
        model
            .insert(conn)
            .await
            .map_err(|e| map_unique_violation(e, CODE_LIVE_CONSTRAINT, || "رمز الحساب مستخدم من قبل".to_string()))?
    };

    let verb = if id.is_some() { "تعديل" } else { "إضافة" };
    activity::log(
        conn,
        cx,
        &activity::UndoRegistry::new(),
        crate::entities::platform::activity::ActivityKind::Journal,
        format!("{verb} الحساب {} — {}", saved.code, saved.name),
        None,
        Some(RouteRef::list("accounts")),
    )
    .await?;

    Ok(Account::from_model(saved))
}

/// **`delete_account(id)`** (:121-129).
pub async fn delete_account<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: Id) -> TxResult<()> {
    lock::for_update_by_id(conn, "accounts", &id.to_string()).await.map_err(TxError::from)?;
    let existing = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
    let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("الحساب غير موجود"))) };

    if !existing.can_delete {
        return Err(TxError::App(AppError::validation("حساب أساسي في النظام ولا يمكن حذفه")));
    }
    let has_children = Entity::find_live().filter(Column::ParentId.eq(id)).count(conn).await.map_err(TxError::from)? > 0;
    if has_children {
        return Err(TxError::App(AppError::conflict("للحساب حسابات فرعية — احذفها أولاً")));
    }
    let has_postings = LineEntity::find().filter(LineColumn::AccountId.eq(id)).count(conn).await.map_err(TxError::from)? > 0;
    if has_postings {
        return Err(TxError::App(AppError::conflict("للحساب قيود مسجلة — يمكنك إيقافه بدلاً من حذفه")));
    }

    Entity::soft_delete(conn, id, cx.clock.now).await.map_err(TxError::from)?;
    Ok(())
}

/// **`reparent_account(id, new_parent_id)`** (:136-153).
pub async fn reparent_account<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: Id, new_parent_id: Option<Id>) -> TxResult<Account> {
    let existing = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
    let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("الحساب غير موجود"))) };

    if new_parent_id == Some(id) {
        return Err(TxError::App(AppError::validation("لا يمكن أن يكون الحساب أباً لنفسه")));
    }

    let all = Entity::find_live().all(conn).await.map_err(TxError::from)?;
    let by_id: BTreeMap<Id, AccountModel> = all.iter().map(|a| (a.id, a.clone())).collect();

    if let Some(new_parent_id) = new_parent_id {
        let parent = by_id.get(&new_parent_id).ok_or_else(|| AppError::validation("الحساب الأب غير موجود"))?;
        if !parent.is_group {
            return Err(TxError::App(AppError::validation("الحساب الأب يجب أن يكون حساباً رئيسياً (تجميعياً)")));
        }
        if parent.kind != existing.kind {
            return Err(TxError::App(AppError::validation(format!("لا يمكن نقل الحساب إلى مجموعة من نوع مختلف ({})", parent.kind))));
        }
        // Cycle check: walk the new parent's ancestor chain looking for `id`.
        let mut cursor = Some(new_parent_id);
        while let Some(cursor_id) = cursor {
            if cursor_id == id {
                return Err(TxError::App(AppError::validation("لا يمكن نقل الحساب إلى أحد فروعه")));
            }
            cursor = by_id.get(&cursor_id).and_then(|a| a.parent_id);
        }

        // Lock the account plus the new parent and its ancestor chain, sorted, then re-verify the
        // chain against fresh reads (§3.2's concurrency note).
        let mut lock_ids: Vec<String> = vec![id.to_string(), new_parent_id.to_string()];
        let mut cursor = by_id.get(&new_parent_id).and_then(|a| a.parent_id);
        while let Some(cursor_id) = cursor {
            lock_ids.push(cursor_id.to_string());
            cursor = by_id.get(&cursor_id).and_then(|a| a.parent_id);
        }
        lock::for_update_many_sorted(conn, "accounts", &lock_ids).await.map_err(TxError::from)?;
    } else {
        lock::for_update_by_id(conn, "accounts", &id.to_string()).await.map_err(TxError::from)?;
    }

    let mut model: ActiveModel = existing.clone().into();
    model.parent_id = Set(new_parent_id);
    let saved = model.update(conn).await.map_err(TxError::from)?;

    activity::log(
        conn,
        cx,
        &activity::UndoRegistry::new(),
        crate::entities::platform::activity::ActivityKind::Journal,
        format!("نقل الحساب {} — {}", saved.code, saved.name),
        None,
        Some(RouteRef::list("accounts")),
    )
    .await?;

    Ok(Account::from_model(saved))
}
