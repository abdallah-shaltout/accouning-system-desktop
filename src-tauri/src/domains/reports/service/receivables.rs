//! `reports::service::receivables` (13b §3.15, 3.16): AR/AP aging, overdue documents.

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::parties::parties;
use crate::entities::soft_delete::SoftDelete;
use crate::shared::balances::{self, OpenDocumentKind};
use crate::utils::dates::BusinessClock;
use crate::utils::money::round2;

use super::super::dto::{AgingReportRow, OverdueDocKind, OverdueRow, PartyKindArg};
use super::common;

async fn live_parties<C: ConnectionTrait>(conn: &C, kind: PartyKindArg) -> TxResult<Vec<parties::Model>> {
    let kind_str = match kind {
        PartyKindArg::Customer => "customer",
        PartyKindArg::Supplier => "supplier",
    };
    Ok(parties::Entity::find_live()
        .filter(parties::Column::Kind.eq(kind_str))
        .order_by_asc(parties::Column::CreatedAt)
        .order_by_asc(parties::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?)
}

/// **`reports_get_aging_report`** (13b §3.15).
pub async fn aging_report<C: ConnectionTrait>(conn: &C, kind: PartyKindArg, clock: &BusinessClock) -> TxResult<Vec<AgingReportRow>> {
    let today = clock.today();
    let parties = live_parties(conn, kind).await?;

    let mut rows = Vec::new();
    for party in parties {
        let docs = match kind {
            PartyKindArg::Customer => balances::open_invoices_for(conn, party.id).await.map_err(WrapErr)?,
            PartyKindArg::Supplier => balances::open_purchase_orders_for(conn, party.id).await.map_err(WrapErr)?,
        };
        if docs.is_empty() {
            continue;
        }
        let mut row = AgingReportRow { party_id: party.id, name: party.name.clone(), current: Decimal::ZERO, b30: Decimal::ZERO, b60: Decimal::ZERO, b90plus: Decimal::ZERO, total: Decimal::ZERO };
        for doc in &docs {
            let ref_day = doc.due_date.unwrap_or(doc.date.day);
            let d = common::days_between(today, ref_day);
            if d <= 0 {
                row.current = round2(row.current + doc.outstanding);
            } else if d <= 30 {
                row.b30 = round2(row.b30 + doc.outstanding);
            } else if d <= 60 {
                row.b60 = round2(row.b60 + doc.outstanding);
            } else {
                row.b90plus = round2(row.b90plus + doc.outstanding);
            }
        }
        row.total = round2(row.current + row.b30 + row.b60 + row.b90plus);
        if row.total > Decimal::new(1, 3) {
            rows.push(row);
        }
    }
    rows.sort_by(|a, b| b.total.cmp(&a.total));
    Ok(rows)
}

/// **`reports_get_overdue_report`** (13b §3.16).
pub async fn overdue_report<C: ConnectionTrait>(conn: &C, kind: PartyKindArg, clock: &BusinessClock) -> TxResult<Vec<OverdueRow>> {
    let today = clock.today();
    let parties = live_parties(conn, kind).await?;

    let mut rows = Vec::new();
    for party in parties {
        let docs = match kind {
            PartyKindArg::Customer => balances::open_invoices_for(conn, party.id).await.map_err(WrapErr)?,
            PartyKindArg::Supplier => balances::open_purchase_orders_for(conn, party.id).await.map_err(WrapErr)?,
        };
        for doc in docs {
            let Some(due) = doc.due_date else { continue };
            let d = common::days_between(today, due);
            if d <= 0 {
                continue;
            }
            rows.push(OverdueRow {
                id: doc.id,
                kind: match doc.kind {
                    OpenDocumentKind::Invoice => OverdueDocKind::Invoice,
                    OpenDocumentKind::PurchaseOrder => OverdueDocKind::PurchaseOrder,
                },
                number: doc.number,
                party_id: party.id,
                party_name: party.name.clone(),
                phone: party.phone.clone(),
                date: doc.date.key(),
                due_date: Some(due.format("%Y-%m-%d").to_string()),
                days_overdue: d,
                outstanding: doc.outstanding,
            });
        }
    }
    rows.sort_by(|a, b| b.days_overdue.cmp(&a.days_overdue));
    Ok(rows)
}

/// Bridges `shared::balances`'s plain `AppError` into `TxResult`'s `TxError`.
struct WrapErr(crate::core::error::AppError);
impl From<crate::core::error::AppError> for WrapErr {
    fn from(e: crate::core::error::AppError) -> Self {
        WrapErr(e)
    }
}
impl From<WrapErr> for crate::core::tx::TxError {
    fn from(e: WrapErr) -> Self {
        crate::core::tx::TxError::App(e.0)
    }
}
