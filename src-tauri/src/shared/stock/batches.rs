//! FEFO batches (D-3) — a behaviour-exact port of `activeBatchesFor`/`receiveBatch`/`consumeFefo`/
//! `isBatchExpired`/`isBatchNearExpiry` (`inventory.ts:73-133`).

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::core::tx::{TxCtx, TxResult};
use crate::entities::catalog::product_batches;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::StockRef;

/// `0.0001` — the same "close enough to zero" epsilon the mock uses (`remaining <= 0.0001`,
/// `batch.qty > 0.0001`), built at call time via `Decimal::new(1, 4)` rather than a `const` (a
/// `Decimal` value isn't itself `const`-constructible from `from_parts` in a `const` context here).
fn qty_epsilon() -> Decimal {
    Decimal::new(1, 4)
}

/// All batches for a product with remaining qty > 0.0001, earliest expiry first — undated batches
/// sort last (`inventory.ts:74-77`: `(a.expiryDate ?? '9999-99-99').localeCompare(...)`), ties
/// broken by `received_date` then `id` (stable insertion order, matching the mock's stable
/// `Array.prototype.sort` over insertion-ordered `db.productBatches`).
pub async fn active_batches<C: ConnectionTrait>(conn: &C, product_id: Id) -> TxResult<Vec<product_batches::Model>> {
    let mut rows = product_batches::Entity::find()
        .filter(product_batches::Column::ProductId.eq(product_id))
        .order_by_asc(product_batches::Column::ReceivedDate)
        .order_by_asc(product_batches::Column::Id)
        .all(conn)
        .await?;
    rows.retain(|b| b.qty > qty_epsilon());
    rows.sort_by(|a, b| {
        let ak = a.expiry_date.unwrap_or(far_future());
        let bk = b.expiry_date.unwrap_or(far_future());
        ak.cmp(&bk).then_with(|| a.received_date.cmp(&b.received_date)).then_with(|| a.id.cmp(&b.id))
    });
    Ok(rows)
}

fn far_future() -> NaiveDate {
    // Sorts after any real date, mirroring the mock's `'9999-99-99'` sentinel string (an invalid
    // calendar date used purely for `localeCompare` ordering — the equivalent "after everything"
    // sentinel here is the latest representable `NaiveDate`).
    NaiveDate::MAX
}

pub fn is_batch_expired(batch: &product_batches::Model, today: NaiveDate) -> bool {
    matches!(batch.expiry_date, Some(exp) if exp < today)
}

/// `inventory.ts:83-87`: not expired, has an expiry date, and expires within `alert_days` from
/// today (inclusive).
pub fn is_batch_near_expiry(batch: &product_batches::Model, alert_days: i64, today: NaiveDate) -> bool {
    let Some(exp) = batch.expiry_date else { return false };
    if is_batch_expired(batch, today) {
        return false;
    }
    let days = (exp - today).num_days();
    days <= alert_days
}

/// Receives a batch (STOCK_IN of a tracked product, or a purchase receipt — `inventory.ts:95-109`):
/// inserts a new lot row with the given values. Does NOT change `products.stock_qty/value` —
/// callers pair this with [`super::apply_change`] on the same product, same as the mock.
#[allow(clippy::too_many_arguments)]
pub async fn receive_batch<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    product_id: Id,
    qty: Decimal,
    unit_cost: Decimal,
    batch_no: String,
    expiry: Option<NaiveDate>,
    date: &DocDate,
    ref_: &StockRef,
) -> TxResult<product_batches::Model> {
    let am = product_batches::ActiveModel {
        id: Set(Id::new()),
        product_id: Set(product_id),
        batch_no: Set(batch_no),
        expiry_date: Set(expiry),
        qty: Set(qty),
        unit_cost: Set(unit_cost),
        supplier_id: Set(None),
        received_date: Set(date.day),
        source_ref_id: Set(Some(ref_.id)),
        source_ref_number: Set(Some(ref_.number.clone())),
        created_at: Set(cx.clock.now),
        updated_at: Set(cx.clock.now),
    };
    Ok(am.insert(conn).await?)
}

/// One batch drawn from, for callers that want to record which batch a sale line came from.
#[derive(Debug, Clone, Copy)]
pub struct Draw {
    pub batch_id: Id,
    pub qty: Decimal,
}

/// FEFO consumption (`inventory.ts:113-129`): draws `qty` from the earliest-expiring non-expired
/// batches first. **Does not error on a shortfall** — this matches the mock's actual code, even
/// though its own doc comment at `inventory.ts:113-117` claims it throws; the real implementation
/// just walks batches and returns whatever it managed to draw (recorded here and in the phase
/// status note per the spec's D-3 instruction). Callers that need "enough stock existed" already
/// checked `product.stockQty` beforehand.
pub async fn consume_fefo<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    product_id: Id,
    qty: Decimal,
    allow_expired: bool,
    today: NaiveDate,
) -> TxResult<Vec<Draw>> {
    let mut remaining = round2(qty);
    let mut draws = Vec::new();
    let batches = active_batches(conn, product_id).await?;
    for batch in batches {
        if remaining <= qty_epsilon() {
            break;
        }
        if !allow_expired && is_batch_expired(&batch, today) {
            continue;
        }
        let take = batch.qty.min(remaining);
        if take <= Decimal::ZERO {
            continue;
        }
        let new_qty = round2(batch.qty - take);
        let am = product_batches::ActiveModel { id: Set(batch.id), qty: Set(new_qty), updated_at: Set(cx.clock.now), ..Default::default() };
        product_batches::Entity::update(am).exec(conn).await?;
        draws.push(Draw { batch_id: batch.id, qty: take });
        remaining = round2(remaining - take);
    }
    Ok(draws)
}

/// Draws exactly `qty` (or whatever remains, if less) from ONE named batch, scoped to the given
/// product — the strict, caller-picked-batch variant (`sales.ts:348-356`, `purchases.ts:474-476`),
/// as distinct from FEFO's "walk every batch" behaviour. Returns the qty actually taken (`0` if the
/// batch doesn't exist or belongs to a different product).
pub async fn draw_batch<C: ConnectionTrait>(conn: &C, cx: &TxCtx, product_id: Id, batch_id: Id, qty: Decimal) -> TxResult<Decimal> {
    let batch = product_batches::Entity::find()
        .filter(product_batches::Column::Id.eq(batch_id))
        .filter(product_batches::Column::ProductId.eq(product_id))
        .one(conn)
        .await?;
    let Some(batch) = batch else { return Ok(Decimal::ZERO) };
    let take = batch.qty.min(qty).max(Decimal::ZERO);
    if take <= Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }
    let new_qty = round2(batch.qty - take);
    let am = product_batches::ActiveModel { id: Set(batch.id), qty: Set(new_qty), updated_at: Set(cx.clock.now), ..Default::default() };
    product_batches::Entity::update(am).exec(conn).await?;
    Ok(take)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(id: Id, expiry: Option<&str>, received: &str, qty: &str) -> product_batches::Model {
        product_batches::Model {
            id,
            product_id: Id::new(),
            batch_no: "B".to_string(),
            expiry_date: expiry.map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()),
            qty: qty.parse().unwrap(),
            unit_cost: Decimal::ZERO,
            supplier_id: None,
            received_date: NaiveDate::parse_from_str(received, "%Y-%m-%d").unwrap(),
            source_ref_id: None,
            source_ref_number: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn is_batch_expired_cases() {
        let today = NaiveDate::parse_from_str("2026-09-27", "%Y-%m-%d").unwrap();
        let expired = batch(Id::new(), Some("2026-09-26"), "2026-01-01", "1");
        let not_expired = batch(Id::new(), Some("2026-09-27"), "2026-01-01", "1");
        let undated = batch(Id::new(), None, "2026-01-01", "1");
        assert!(is_batch_expired(&expired, today));
        assert!(!is_batch_expired(&not_expired, today));
        assert!(!is_batch_expired(&undated, today));
    }

    #[test]
    fn is_batch_near_expiry_cases() {
        let today = NaiveDate::parse_from_str("2026-09-27", "%Y-%m-%d").unwrap();
        let near = batch(Id::new(), Some("2026-09-30"), "2026-01-01", "1");
        let far = batch(Id::new(), Some("2026-12-31"), "2026-01-01", "1");
        let expired = batch(Id::new(), Some("2026-09-01"), "2026-01-01", "1");
        assert!(is_batch_near_expiry(&near, 5, today));
        assert!(!is_batch_near_expiry(&far, 5, today));
        assert!(!is_batch_near_expiry(&expired, 5, today)); // already expired -> false, not "near".
    }

    #[test]
    fn far_future_sorts_after_any_real_date() {
        let real = NaiveDate::parse_from_str("9999-12-31", "%Y-%m-%d").unwrap();
        assert!(far_future() >= real);
    }
}
