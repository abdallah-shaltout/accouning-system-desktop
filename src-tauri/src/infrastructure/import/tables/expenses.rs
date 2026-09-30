//! `expense_categories, expenses, recurring_expenses`.

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::entities::expenses::expense_categories::ActiveModel as ExpenseCategoryActiveModel;
use crate::entities::expenses::expenses::{ActiveModel as ExpenseActiveModel, PaidFromKind};
use crate::entities::expenses::recurring_expenses::{ActiveModel as RecurringExpenseActiveModel, PaidFromKind as RecurringPaidFromKind};
use crate::entities::values::StringList;
use crate::infrastructure::import::idmap::IdMap;
use crate::infrastructure::import::model::{ExpenseCategoryV1, ExpenseV1, RecurringExpenseV1};
use crate::infrastructure::import::tables::{lenient_ref, parse_doc_date, resolve_created_at, strict_ref};
use crate::utils::money::round2;

/// G-20: `expense_categories.name_live` is unique (`m0011`'s `uq_expense_categories_name_live`) —
/// the mock never enforced this, so a snapshot with two categories sharing a name (case- and
/// whitespace-insensitively, matching the column's declared collation) would otherwise fail on
/// insert. De-duplicates by appending " (٢)"-style Arabic ordinal suffixes to every name after the
/// first occurrence, deterministically by array order, so the import always succeeds rather than
/// failing on a pre-existing mock data quirk.
fn dedupe_names<'a>(names: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut seen: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut out = Vec::new();
    for name in names {
        let key = name.trim().to_lowercase();
        let count = seen.entry(key).or_insert(0);
        *count += 1;
        if *count == 1 {
            out.push(name.to_string());
        } else {
            out.push(format!("{name} ({})", *count));
        }
    }
    out
}

pub async fn insert_expense_categories<C: ConnectionTrait>(
    conn: &C,
    rows: &[ExpenseCategoryV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    let deduped_names = dedupe_names(rows.iter().map(|r| r.name.as_str()));
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(account_id) = id_map.resolve(&row.account_id) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في expense_categories")));
        };
        let model = ExpenseCategoryActiveModel {
            id: Set(id),
            name: Set(deduped_names[i].clone()),
            icon: Set(row.icon.clone()),
            account_id: Set(account_id),
            default_tax_id: Set(strict_ref(id_map, row.default_tax_id.as_deref(), "expense_categories")?),
            default_cost_center_id: Set(lenient_ref(id_map, row.default_cost_center_id.as_deref())),
            active: Set(row.active),
            can_delete: Set(row.can_delete),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::expenses::expense_categories::SyncStatus::Local),
            name_live: sea_orm::ActiveValue::NotSet,
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_expenses<C: ConnectionTrait>(
    conn: &C,
    rows: &[ExpenseV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let (Some(category_id), Some(created_by)) = (id_map.resolve(&row.category_id), id_map.resolve(&row.created_by)) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في expenses")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في expenses"))?;

        let amount = round2(row.amount);
        let net_amount = round2(row.net_amount);
        let tax_amount = round2(row.tax_amount);
        if amount != row.amount || net_amount != row.net_amount || tax_amount != row.tax_amount {
            *rounded += 1;
        }

        let paid_from_kind = if row.paid_from.kind == "credit" { PaidFromKind::Credit } else { PaidFromKind::Method };

        let model = ExpenseActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            date_day: Set(day),
            date_instant: Set(instant),
            category_id: Set(category_id),
            amount: Set(amount),
            is_tax_invoice: Set(row.is_tax_invoice),
            tax_id: Set(strict_ref(id_map, row.tax_id.as_deref(), "expenses")?),
            net_amount: Set(net_amount),
            tax_amount: Set(tax_amount),
            supplier_vat_number: Set(row.supplier_vat_number.clone()),
            supplier_invoice_no: Set(row.supplier_invoice_no.clone()),
            cost_center_id: Set(lenient_ref(id_map, row.cost_center_id.as_deref())),
            paid_from_kind: Set(paid_from_kind),
            paid_from_payment_method_id: Set(strict_ref(id_map, row.paid_from.payment_method_id.as_deref(), "expenses")?),
            paid_from_supplier_id: Set(strict_ref(id_map, row.paid_from.supplier_id.as_deref(), "expenses")?),
            description: Set(row.description.clone()),
            attachment_ids: Set(row.attachment_ids.clone().map(StringList)),
            repeat_monthly: Set(row.repeat_monthly),
            recurring_template_id: Set(None), // DEFERRED (forward ref) — set in run.rs phase B.
            created_by: Set(created_by),
            branch_id: Set(strict_ref(id_map, row.branch_id.as_deref(), "expenses")?),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::expenses::expenses::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_recurring_expenses<C: ConnectionTrait>(
    conn: &C,
    rows: &[RecurringExpenseV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(category_id) = id_map.resolve(&row.category_id) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في recurring_expenses")));
        };
        let next_date = chrono::NaiveDate::parse_from_str(&row.next_date, "%Y-%m-%d")
            .map_err(|_| AppError::validation("تاريخ غير صالح في recurring_expenses"))?;
        let amount = round2(row.amount);
        if amount != row.amount {
            *rounded += 1;
        }
        let paid_from_kind = if row.paid_from.kind == "credit" { RecurringPaidFromKind::Credit } else { RecurringPaidFromKind::Method };

        let model = RecurringExpenseActiveModel {
            id: Set(id),
            name: Set(row.name.clone()),
            category_id: Set(category_id),
            amount: Set(amount),
            is_tax_invoice: Set(row.is_tax_invoice),
            tax_id: Set(strict_ref(id_map, row.tax_id.as_deref(), "recurring_expenses")?),
            paid_from_kind: Set(paid_from_kind),
            paid_from_payment_method_id: Set(strict_ref(id_map, row.paid_from.payment_method_id.as_deref(), "recurring_expenses")?),
            paid_from_supplier_id: Set(strict_ref(id_map, row.paid_from.supplier_id.as_deref(), "recurring_expenses")?),
            description: Set(row.description.clone()),
            day: Set(row.day),
            next_date: Set(next_date),
            auto_post: Set(row.auto_post),
            active: Set(row.active),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::expenses::recurring_expenses::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}
