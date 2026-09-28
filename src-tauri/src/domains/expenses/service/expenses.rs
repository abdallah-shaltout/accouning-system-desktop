//! Expenses — list/detail/create (03-domains/11-expenses.md §3.4-3.7) and the shared
//! `record_expense` body used by both `create_expense` and `post_due_recurring_expense`.

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::expenses::expense_categories::{Column as CategoryColumn, Entity as CategoryEntity};
use crate::entities::expenses::expenses::{ActiveModel, Column, Entity, PaidFromKind};
use crate::entities::org::payment_methods::{Column as PaymentMethodColumn, Entity as PaymentMethodEntity};
use crate::entities::org::taxes::{Column as TaxColumn, Entity as TaxEntity};
use crate::entities::parties::parties::{Column as PartyColumn, Entity as PartyEntity};
use crate::entities::platform::activity::ActivityKind;
use crate::entities::soft_delete::SoftDelete;
use crate::shared::activity;
use crate::shared::ledger::accounts::SystemRole;
use crate::shared::ledger::period::assert_open_period;
use crate::shared::ledger::post::{self, AccountRef, PartyRef, PostJournal, PostingLine, SourceRef};
use crate::shared::numbering::{self, DocumentKind};
use crate::utils::dates::{BusinessClock, DocDate, RawDocDate};
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;
use crate::utils::text::matches_search;

use super::super::dto::{expense_row_to_dto, expense_to_dto, paid_from_from_dto, Expense, ExpenseFilter, ExpenseInput, ExpenseRow};

const CATEGORY_NOT_SELECTED: &str = "اختر تصنيف المصروف";

async fn category_name<C: ConnectionTrait>(conn: &C, category_id: Id) -> TxResult<String> {
    let category = CategoryEntity::find_by_id(category_id).one(conn).await.map_err(TxError::from)?;
    Ok(category.map(|c| c.name).unwrap_or_else(|| "—".to_string()))
}

/// **3.4 `get_expenses(filter)`** — `expenseService.ts:44-51`.
pub async fn get_expenses<C: ConnectionTrait>(conn: &C, filter: Option<ExpenseFilter>) -> TxResult<Vec<ExpenseRow>> {
    let filter = filter.unwrap_or_default();

    let mut q = Entity::find();
    if let Some(category_id) = filter.category_id {
        q = q.filter(Column::CategoryId.eq(category_id));
    }
    // `expenses.date_key` (the generated column mirroring the mock's raw date string, P2-09) has
    // no mapped SeaORM column on this entity — sorted in Rust instead, on the same `DocDate::key()`
    // string the generated column holds: `ORDER BY date_key DESC, created_at ASC, id ASC` (stable
    // sort keeps ties in insertion order, matching `b.date.localeCompare(a.date)`'s stability).
    let mut rows = q.order_by_asc(Column::CreatedAt).order_by_asc(Column::Id).all(conn).await.map_err(TxError::from)?;
    rows.sort_by(|a, b| b.date().key().cmp(&a.date().key()));

    let from = filter.from.as_deref().filter(|s| !s.is_empty());
    let to = filter.to.as_deref().filter(|s| !s.is_empty());

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        if !crate::utils::dates::in_date_range(row.date_day, from, to) {
            continue;
        }
        let name = category_name(conn, row.category_id).await?;
        let dto = expense_row_to_dto(&row, name);
        let hay = [
            Some(dto.number.as_str()),
            Some(dto.category_name.as_str()),
            dto.description.as_deref(),
            dto.supplier_invoice_no.as_deref(),
        ];
        if matches_search(&hay, filter.search.as_deref()) {
            out.push(dto);
        }
    }
    Ok(out)
}

/// **3.5 `get_expense(id)`**.
pub async fn get_expense<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<ExpenseRow> {
    let row = Entity::find_by_id(id).one(conn).await.map_err(TxError::from)?.ok_or_else(|| AppError::not_found("المصروف غير موجود"))?;
    let name = category_name(conn, row.category_id).await?;
    Ok(expense_row_to_dto(&row, name))
}

struct SplitTax {
    net: Decimal,
    vat: Decimal,
}

/// `splitTax` (`expenses.ts:44-50`).
async fn split_tax<C: ConnectionTrait>(conn: &C, amount: Decimal, is_tax_invoice: bool, tax_id: Option<Id>) -> TxResult<SplitTax> {
    if !is_tax_invoice {
        return Ok(SplitTax { net: round2(amount), vat: Decimal::ZERO });
    }
    let rate = match tax_id {
        Some(id) => TaxEntity::find()
            .filter(TaxColumn::Id.eq(id))
            .filter(TaxColumn::Active.eq(true))
            .filter(TaxColumn::DeletedAt.is_null())
            .one(conn)
            .await
            .map_err(TxError::from)?
            .map(|t| t.rate)
            .unwrap_or(Decimal::ZERO),
        None => Decimal::ZERO,
    };
    let hundred = Decimal::from(100);
    let net = round2(amount / (Decimal::ONE + rate / hundred));
    let vat = round2(amount - net);
    Ok(SplitTax { net, vat })
}

/// `validateExpenseInput` (`expenses.ts:52-63`).
async fn validate_expense_input<C: ConnectionTrait>(conn: &C, input: &ExpenseInput) -> TxResult<crate::entities::expenses::expense_categories::Model> {
    let category = CategoryEntity::find_live()
        .filter(CategoryColumn::Id.eq(input.category_id))
        .one(conn)
        .await
        .map_err(TxError::from)?
        .ok_or_else(|| AppError::validation(CATEGORY_NOT_SELECTED))?;

    if !(input.amount > Decimal::ZERO) {
        return Err(TxError::App(AppError::validation("المبلغ يجب أن يكون أكبر من صفر")));
    }

    match input.paid_from {
        super::super::dto::ExpensePaidFrom::Method { payment_method_id } => {
            let found = PaymentMethodEntity::find_live()
                .filter(PaymentMethodColumn::Id.eq(payment_method_id))
                .filter(PaymentMethodColumn::Active.eq(true))
                .one(conn)
                .await
                .map_err(TxError::from)?;
            if found.is_none() {
                return Err(TxError::App(AppError::validation("اختر طريقة الدفع")));
            }
        }
        super::super::dto::ExpensePaidFrom::Credit { supplier_id } => {
            let found = PartyEntity::find_live()
                .filter(PartyColumn::Id.eq(supplier_id))
                .filter(PartyColumn::Kind.eq("supplier"))
                .one(conn)
                .await
                .map_err(TxError::from)?;
            if found.is_none() {
                return Err(TxError::App(AppError::validation("اختر المورد")));
            }
        }
    }

    Ok(category)
}

/// Parses an `ExpenseInput.date` wire string against the transaction's `BusinessClock` (E-D5): an
/// unparsable string is refused with `VALIDATION` `التاريخ غير صالح` rather than stored verbatim
/// like the mock would.
fn parse_expense_date(raw: &str, clock: &BusinessClock) -> TxResult<DocDate> {
    let parsed = RawDocDate::parse(raw).map_err(|_| AppError::validation("التاريخ غير صالح"))?;
    Ok(parsed.resolve(clock))
}

/// **3.6 `record_expense`** — the shared body of `create_expense`/`post_due_recurring_expense`,
/// porting `recordExpense` (`expenses.ts:66-117`).
pub async fn record_expense<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::undo::UndoRegistry,
    input: ExpenseInput,
    recurring_template_id: Option<Id>,
) -> TxResult<Expense> {
    // 1. Validation (category/amount/paid-from).
    let category = validate_expense_input(conn, &input).await?;

    // 2. Tax split on the raw input amount.
    let split = split_tax(conn, input.amount, input.is_tax_invoice, input.tax_id).await?;

    // 3. Period lock, then re-verify the category still exists live under a share lock (closes
    //    the delete-vs-create race, §4).
    let date = parse_expense_date(&input.date, &cx.clock)?;
    assert_open_period(conn, &date.day, false).await?;

    lock::share_lock_by_id(conn, "expense_categories", &category.id.to_string()).await.map_err(TxError::from)?;
    let category = CategoryEntity::find_live()
        .filter(CategoryColumn::Id.eq(category.id))
        .one(conn)
        .await
        .map_err(TxError::from)?
        .ok_or_else(|| AppError::validation(CATEGORY_NOT_SELECTED))?;

    // 4. Number.
    let number = numbering::next_number(conn, DocumentKind::Expense).await?;

    // 5. Insert the expense row.
    let (paid_from_kind, paid_from_method_id, paid_from_supplier_id) = paid_from_from_dto(input.paid_from);
    let created_by = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;
    let id = Id::new();
    let model = ActiveModel {
        id: Set(id),
        number: Set(number.clone()),
        date_day: Set(date.day),
        date_instant: Set(date.instant),
        category_id: Set(category.id),
        amount: Set(round2(input.amount)),
        is_tax_invoice: Set(input.is_tax_invoice),
        tax_id: Set(input.tax_id),
        net_amount: Set(split.net),
        tax_amount: Set(split.vat),
        supplier_vat_number: Set(input.supplier_vat_number.clone()),
        supplier_invoice_no: Set(input.supplier_invoice_no.clone()),
        cost_center_id: Set(input.cost_center_id),
        paid_from_kind: Set(paid_from_kind.clone()),
        paid_from_payment_method_id: Set(paid_from_method_id),
        paid_from_supplier_id: Set(paid_from_supplier_id),
        description: Set(input.description.clone()),
        attachment_ids: Set(input.attachment_ids.clone().map(crate::entities::values::StringList)),
        repeat_monthly: Set(input.repeat_monthly.unwrap_or(false)),
        recurring_template_id: Set(recurring_template_id),
        created_by: Set(created_by),
        branch_id: Set(None),
        created_at: Set(cx.clock.now),
        updated_at: Set(cx.clock.now),
        deleted_at: Set(None),
        sync_status: Set(crate::entities::expenses::expenses::SyncStatus::Local),
    };
    let inserted = model.insert(conn).await.map_err(TxError::from)?;

    // 6. Posting lines.
    let mut lines = vec![PostingLine { cost_center_id: input.cost_center_id, ..PostingLine::debit(AccountRef::Id(category.account_id), split.net) }];
    if split.vat > Decimal::ZERO {
        lines.push(PostingLine::debit(AccountRef::Role(SystemRole::VatInput), split.vat));
    }
    match (paid_from_kind, paid_from_method_id, paid_from_supplier_id) {
        (PaidFromKind::Method, Some(method_id), _) => {
            let method = PaymentMethodEntity::find_by_id(method_id).one(conn).await.map_err(TxError::from)?.ok_or_else(|| AppError::validation("اختر طريقة الدفع"))?;
            let role = method.account_role.parse::<SystemRole>().map_err(TxError::App)?;
            lines.push(PostingLine::credit(AccountRef::Role(role), round2(input.amount)));
        }
        (PaidFromKind::Credit, _, Some(supplier_id)) => {
            let mut line = PostingLine::credit(AccountRef::Role(SystemRole::Payable), round2(input.amount));
            line.party = Some(PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Supplier, id: supplier_id });
            lines.push(line);
        }
        _ => unreachable!("paid_from is always fully populated by paid_from_from_dto"),
    }

    // 7. Post. Attachment ids are UUID strings (`Id`s) on the wire — a value that doesn't parse as
    // one is a client bug, not a business error the user can retry differently, so it fails loudly
    // rather than silently dropping the attachment (P2-23's "should be impossible" convention).
    let attachment_ids: Vec<Id> = match &input.attachment_ids {
        Some(ids) => ids
            .iter()
            .map(|s| s.parse::<Id>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::internal("معرّف مرفق غير صالح", Some(e.to_string())))?,
        None => Vec::new(),
    };
    let description = match &input.description {
        Some(d) if !d.is_empty() => format!("مصروف {number} — {}: {d}", category.name),
        _ => format!("مصروف {number} — {}", category.name),
    };
    post::post(
        conn,
        cx,
        PostJournal {
            date,
            description,
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "expense".to_string(), id, number: Some(number.clone()) }),
            lines,
            allow_closed_period: false,
            attachment_ids,
            template_id: None,
        },
    )
    .await?;

    // 8. Activity.
    activity::record::log(
        conn,
        cx,
        registry,
        ActivityKind::Expense,
        format!("مصروف {number} — {} بقيمة {:.2}", category.name, round2(input.amount)),
        Some(date),
        Some(RouteRef::detail("expense-detail", id.to_string())),
    )
    .await?;

    Ok(expense_to_dto(&inserted))
}

/// **3.7 `create_expense(input)`** = `record_expense(…, input, None)`.
pub async fn create_expense<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::undo::UndoRegistry, input: ExpenseInput) -> TxResult<Expense> {
    record_expense(conn, cx, registry, input, None).await
}
