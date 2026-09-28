//! Rules 4 (overdue-customers), 5 (credit-limit) (14b-insights.md §3.1).

use indexmap::IndexMap;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::parties::parties::Entity as PartyEntity;
use crate::entities::sales::invoices::{Column as InvoiceColumn, Entity as InvoiceEntity, InvoiceStatus};
use crate::shared::balances::customer_balance;
use crate::shared::totals::document_outstanding;
use crate::utils::dates::format_iso_ms;
use crate::utils::route::RouteRef;

use super::super::common::{days_between, fmt_money, js_num, InsightCtx};
use super::super::super::super::dto::{InsightDto, InsightIcon, InsightSeverity};

/// `isOverdue` (`invoiceService.ts:56-59`): status ≠ REFUNDED (drafts NOT excluded, quirk Q-3), due
/// date set, outstanding > 0, due key < ISO(now).
fn is_overdue(inv: &crate::entities::sales::invoices::Model, now_iso: &str) -> bool {
    if inv.status == InvoiceStatus::Refunded {
        return false;
    }
    let Some(due) = inv.due_date() else { return false };
    let outstanding = document_outstanding(inv.grand_total - inv.refunded_amount, inv.paid_amount);
    outstanding > Decimal::ZERO && due.key().as_str() < now_iso
}

/// 4 — overdue customers (`ir:146-177`).
pub async fn overdue_customers<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let now_iso = format_iso_ms(ctx.now);
    let all = InvoiceEntity::find().filter(InvoiceColumn::DeletedAt.is_null()).all(conn).await.map_err(AppError::from)?;
    let overdue: Vec<_> = all.iter().filter(|i| is_overdue(i, &now_iso)).collect();
    if overdue.is_empty() {
        return Ok(Vec::new());
    }

    struct Agg {
        total: Decimal,
        max_days: i64,
    }
    let mut by_customer: IndexMap<crate::utils::id::Id, Agg> = IndexMap::new();
    for inv in &overdue {
        let Some(cid) = inv.customer_id else { continue };
        let due_day = inv.due_date().expect("is_overdue guarantees a due date").day;
        let days = days_between(ctx.today, due_day).max(0);
        let outstanding = document_outstanding(inv.grand_total - inv.refunded_amount, inv.paid_amount);
        let entry = by_customer.entry(cid).or_insert(Agg { total: Decimal::ZERO, max_days: 0 });
        entry.total = round2(entry.total + outstanding);
        entry.max_days = entry.max_days.max(days);
    }

    let mut out = Vec::new();
    for (customer_id, agg) in by_customer {
        if agg.total <= Decimal::ZERO {
            continue;
        }
        let Some(customer) = PartyEntity::find_by_id(customer_id).one(conn).await.map_err(AppError::from)? else { continue };
        out.push(InsightDto {
            id: format!("overdue-customer:{customer_id}"),
            rule_key: "overdue-customers".to_string(),
            severity: InsightSeverity::Critical,
            message: format!("\"{}\" متأخر {} يوماً بمبلغ {}", customer.name, js_num(Decimal::from(agg.max_days)), fmt_money(agg.total, ctx.numerals)),
            metric: Some(fmt_money(agg.total, ctx.numerals)),
            action_label: "كشف الحساب".to_string(),
            action_to: RouteRef::detail("customer", customer_id.to_string()),
            icon: InsightIcon::UserX,
            roles: vec![crate::core::auth::Role::Accountant, crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
            value: agg.total,
            created_at: ctx.today.format("%Y-%m-%d").to_string(),
        });
    }
    Ok(out)
}

fn round2(d: Decimal) -> Decimal {
    crate::utils::money::round2(d)
}

/// 5 — credit-limit near/over (`ir:180-204`).
pub async fn credit_limit<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    use crate::entities::parties::parties::Column as PartyColumn;

    let customers = PartyEntity::find()
        .filter(PartyColumn::Kind.eq("customer"))
        .filter(PartyColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let mut out = Vec::new();
    for c in &customers {
        let Some(limit) = c.credit_limit else { continue };
        if limit <= Decimal::ZERO {
            continue;
        }
        let balance = customer_balance(conn, c.id).await?;
        if balance < limit * Decimal::new(9, 1) {
            continue;
        }
        let over = balance > limit;
        out.push(InsightDto {
            id: format!("credit-limit:{}", c.id),
            rule_key: "credit-limit".to_string(),
            severity: if over { InsightSeverity::Critical } else { InsightSeverity::Warning },
            message: if over {
                format!("\"{}\" تجاوز الحد الائتماني: الرصيد {} من أصل {}", c.name, fmt_money(balance, ctx.numerals), fmt_money(limit, ctx.numerals))
            } else {
                format!("\"{}\" قريب من الحد الائتماني: {} من أصل {}", c.name, fmt_money(balance, ctx.numerals), fmt_money(limit, ctx.numerals))
            },
            metric: Some(fmt_money(balance, ctx.numerals)),
            action_label: "عرض العميل".to_string(),
            action_to: RouteRef::detail("customer", c.id.to_string()),
            icon: InsightIcon::CreditCard,
            roles: vec![crate::core::auth::Role::Cashier, crate::core::auth::Role::Accountant, crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
            value: balance,
            created_at: ctx.today.format("%Y-%m-%d").to_string(),
        });
    }
    Ok(out)
}
