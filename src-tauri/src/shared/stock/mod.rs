//! `shared::stock` — the ONLY code that changes `products.{stock_qty,stock_value,cost_price}`,
//! `product_branch_stock` or inserts `stock_movements`/`product_batches` rows (master plan rule 3,
//! architecture_rules D-5). A behaviour-exact port of `applyStockChange` (`core.ts:268-297`) plus
//! the FEFO batch helpers (`inventory.ts:73-133`) and the weighted-average cost-out rules the mock
//! keeps inline in its callers (`sales.ts`/`purchases.ts`/`transfers.ts` — ported to `cost.rs`).
//!
//! **Locking (D-1, "the single most important lock"):** every function that changes a product's
//! stock takes `&mut LockedProduct`, which can only be constructed by [`lock_products`] — a
//! `SELECT ... FOR UPDATE` on `products`, sorted by id, that also loads the product's
//! `product_branch_stock` rows. This makes an unlocked stock write impossible to compile: nothing
//! outside this module can build a `LockedProduct`, and every mutator below requires one.

pub mod batches;
pub mod cost;

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect, QueryTrait, Set};

use crate::core::error::AppError;
use crate::core::events::ChangeCategory;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::catalog::product_branch_stock;
use crate::entities::catalog::products;
use crate::entities::inventory::stock_movements;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::{round2, round4};

/// A polymorphic movement reference (`{ id, number }` in the mock) — the document that caused the
/// stock change (invoice, purchase order, stock adjustment, transfer, …). No FK (B-1: `ref_id` is
/// polymorphic on `stock_movements`).
#[derive(Debug, Clone)]
pub struct StockRef {
    pub id: Id,
    pub number: String,
}

/// `StockMovementReason` — the mock's `StockMovementReason` union
/// (`'sale'|'purchase'|'stock_in'|'loss'|'stocktake'|'refund'|'purchase_return'|'transfer_out'|'transfer_in'`).
/// Kept as a `&'static str` (matches the `stock_movements.reason` column, a plain string, not a DB
/// enum) rather than introducing a Rust enum the callers would have to keep in lockstep with the
/// mock's own union — every call site below passes a literal.
pub type StockMovementReason = &'static str;

/// A product row locked `FOR UPDATE` for the duration of the caller's transaction, together with
/// its `product_branch_stock` rows (also locked, since [`lock_products`] issues its `SELECT ...
/// FOR UPDATE` against both tables in the same statement set). Every stock-mutating function in
/// this module takes `&mut LockedProduct` — there is no public way to construct one except through
/// [`lock_products`], so an unlocked stock write cannot compile (D-1).
pub struct LockedProduct {
    pub model: products::Model,
    /// Keyed by `branch_id` — loaded once at lock time, mutated in place, and only ever persisted
    /// back through this module's own read-modify-write (never a bare `UPDATE`).
    branch_rows: BTreeMap<Id, product_branch_stock::Model>,
}

impl LockedProduct {
    pub fn id(&self) -> Id {
        self.model.id
    }

    pub fn stock_qty(&self) -> Decimal {
        self.model.stock_qty
    }

    pub fn stock_value(&self) -> Decimal {
        self.model.stock_value
    }

    pub fn cost_price(&self) -> Decimal {
        self.model.cost_price
    }

    pub fn track_batches(&self) -> bool {
        self.model.track_batches.unwrap_or(false)
    }

    pub fn allow_negative_stock(&self) -> bool {
        self.model.allow_negative_stock.unwrap_or(false)
    }

    /// `true` when this product is not stock-tracked at all (`applyStockChange`'s early-return
    /// condition, minus the "both changes are zero" clause, which callers check separately):
    /// `type = 'service'` or `stockMode = 'none'`. `stock_mode = NULL` means "tracked" (B-1's own
    /// note on the column) — mirroring the mock's `product.stockMode === 'none'` check, which is
    /// `false` for `undefined`.
    pub fn is_untracked(&self) -> bool {
        self.model.r#type == "service" || self.model.stock_mode.as_deref() == Some("none")
    }

    /// The qty/value pair for one branch (defaulting to zero when the branch has no row yet) —
    /// used by read paths (`branch_stock_qty`) without needing a fresh lock.
    fn branch_entry(&self, branch_id: Id) -> (Decimal, Decimal) {
        self.branch_rows.get(&branch_id).map(|r| (r.qty, r.value)).unwrap_or((Decimal::ZERO, Decimal::ZERO))
    }
}

/// `SELECT ... FOR UPDATE` on `products`, sorted by id (D-1: a fixed lock order across every
/// caller avoids a products-vs-products deadlock when two documents share more than one product).
/// Loads each locked product's `product_branch_stock` rows in the same call. Missing id -> the
/// mock's exact `productById` message: `المنتج غير موجود` / `NOT_FOUND` (`core.ts:251-254`).
pub async fn lock_products<C: ConnectionTrait>(conn: &C, ids: &[Id]) -> TxResult<BTreeMap<Id, LockedProduct>> {
    let mut sorted: Vec<Id> = ids.to_vec();
    sorted.sort();
    sorted.dedup();

    if sorted.is_empty() {
        return Ok(BTreeMap::new());
    }

    // `SELECT ... FOR UPDATE ORDER BY id` via sea-query's `lock_exclusive` (same idiom as
    // `core::settings::load_shared_locked`) rather than hand-rolled raw SQL row parsing.
    let select = products::Entity::find()
        .filter(products::Column::Id.is_in(sorted.clone()))
        .order_by_asc(products::Column::Id)
        .lock_exclusive();
    let stmt = select.build(conn.get_database_backend());
    let rows = conn.query_all(stmt).await?;

    let mut locked: BTreeMap<Id, LockedProduct> = BTreeMap::new();
    for row in &rows {
        let model = products::Model::from_query_result(row, "")?;
        locked.insert(model.id, LockedProduct { model, branch_rows: BTreeMap::new() });
    }

    for id in &sorted {
        if !locked.contains_key(id) {
            return Err(AppError::not_found("المنتج غير موجود").into());
        }
    }

    // Load branch-stock rows for every locked product (also effectively locked: only this
    // transaction can be holding the parent `products` row `FOR UPDATE`, and every writer here
    // reads/re-writes them only after acquiring that same lock).
    let branch_rows = product_branch_stock::Entity::find()
        .filter(product_branch_stock::Column::ProductId.is_in(sorted.clone()))
        .all(conn)
        .await?;
    for row in branch_rows {
        if let Some(p) = locked.get_mut(&row.product_id) {
            p.branch_rows.insert(row.branch_id, row);
        }
    }

    Ok(locked)
}

/// Convenience for a single product (the common case).
pub async fn lock_product<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<LockedProduct> {
    let mut map = lock_products(conn, &[id]).await?;
    map.remove(&id).ok_or_else(|| AppError::not_found("المنتج غير موجود").into())
}

/// Change on-hand quantity AND stock value together (docs/v2/02-accounting-review.md A1/A2 — a
/// behaviour-exact port of `applyStockChange`, `core.ts:268-297`). `valueChange` must be exactly
/// the amount posted to the inventory account for this movement, so `GL(inventory) = Σ
/// product.stockValue` holds exactly. `costPrice` is re-derived as the 4-decimal average
/// (`stockValue / stockQty`) for display; it is never the posting input. A no-op (no movement row,
/// no write) when the product is untracked ([`LockedProduct::is_untracked`]) or both changes are
/// exactly zero. `branch_id` defaults to `settings.default_branch_id` when `None`, matching the
/// mock's `DEFAULT_BRANCH_ID` fallback for every pre-branch-aware caller.
#[allow(clippy::too_many_arguments)]
pub async fn apply_change<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    p: &mut LockedProduct,
    qty_change: Decimal,
    value_change: Decimal,
    reason: StockMovementReason,
    ref_: StockRef,
    date: &DocDate,
    branch_id: Option<Id>,
) -> TxResult<()> {
    if p.is_untracked() || (qty_change.is_zero() && value_change.is_zero()) {
        return Ok(());
    }

    let new_qty = round2(p.model.stock_qty + qty_change);
    let new_value = round2(p.model.stock_value + value_change);
    let new_cost = if new_qty > Decimal::new(1, 4) {
        round4(new_value / new_qty)
    } else {
        Decimal::ZERO
    };

    p.model.stock_qty = new_qty;
    p.model.stock_value = new_value;
    p.model.cost_price = new_cost;

    let active = products::ActiveModel {
        id: Set(p.model.id),
        stock_qty: Set(new_qty),
        stock_value: Set(new_value),
        cost_price: Set(new_cost),
        updated_at: Set(cx.clock.now),
        ..Default::default()
    };
    // `stock_by_branch` (the JSON mirror column on `products`) is a read-path concern (DTO
    // assembly from the dedicated `product_branch_stock` rows below) — not double-written here,
    // matching every other JSON "mirror" column left to the read side in this schema (B-1).
    // `branch_id` defaults to `settings.default_branch_id` (the mock's `DEFAULT_BRANCH_ID`
    // fallback), never a guess from whatever branch rows happen to already exist.
    let branch = match branch_id {
        Some(id) => id,
        None => crate::core::settings::load(conn).await?.default_branch_id,
    };
    products::Entity::update(active).exec(conn).await?;

    let (cur_qty, cur_value) = p.branch_entry(branch);
    let branch_qty = round2(cur_qty + qty_change);
    let branch_value = round2(cur_value + value_change);
    match p.branch_rows.get(&branch) {
        Some(existing) => {
            let am = product_branch_stock::ActiveModel {
                id: Set(existing.id),
                qty: Set(branch_qty),
                value: Set(branch_value),
                updated_at: Set(cx.clock.now),
                ..Default::default()
            };
            product_branch_stock::Entity::update(am).exec(conn).await?;
            if let Some(row) = p.branch_rows.get_mut(&branch) {
                row.qty = branch_qty;
                row.value = branch_value;
            }
        }
        None => {
            let id = Id::new();
            let am = product_branch_stock::ActiveModel {
                id: Set(id),
                product_id: Set(p.model.id),
                branch_id: Set(branch),
                qty: Set(branch_qty),
                value: Set(branch_value),
                created_at: Set(cx.clock.now),
                updated_at: Set(cx.clock.now),
            };
            let inserted = am.clone().insert(conn).await?;
            p.branch_rows.insert(branch, inserted);
        }
    }

    let movement = stock_movements::ActiveModel {
        id: Set(Id::new()),
        date_day: Set(date.day),
        date_instant: Set(date.instant),
        date_key: Set(date.key()),
        product_id: Set(p.model.id),
        qty_change: Set(round4(qty_change)),
        value_change: Set(round2(value_change)),
        reason: Set(reason.to_string()),
        ref_id: Set(ref_.id),
        ref_number: Set(Some(ref_.number)),
        balance_after: Set(Some(new_qty)),
        batch_id: Set(None),
        created_at: Set(cx.clock.now),
    };
    movement.insert(conn).await?;

    cx.touch(ChangeCategory::Catalog);
    Ok(())
}

/// Sets the initial stock fields on a brand-new `products` `ActiveModel` (product-creation helper,
/// D-1 "so rule 3 holds" — the creation path must not poke `stock_qty`/`stock_value` itself):
/// qty 0, value 0, and the given cost price (the opening cost before any receipt).
pub fn init_product_stock(active_model: &mut products::ActiveModel, cost_price: Decimal) {
    active_model.stock_qty = Set(Decimal::ZERO);
    active_model.stock_value = Set(Decimal::ZERO);
    active_model.cost_price = Set(cost_price);
}

/// Sets `cost_price` on a locked product, but ONLY while its stock is still empty
/// (`stock_qty <= 0.0001`) — mirrors `updateProduct`'s silent discard of a manual cost-price edit
/// once stock exists (products.md §1: the average cost is derived from movements from then on,
/// never hand-edited). Returns `true` when the write was applied, `false` when it was silently
/// discarded (matching the mock's silent-no-op behaviour exactly — this is not an error).
pub async fn set_cost_when_empty<C: ConnectionTrait>(conn: &C, cx: &TxCtx, p: &mut LockedProduct, cost: Decimal) -> TxResult<bool> {
    if p.model.stock_qty > Decimal::new(1, 4) {
        return Ok(false);
    }
    p.model.cost_price = cost;
    let am = products::ActiveModel { id: Set(p.model.id), cost_price: Set(cost), updated_at: Set(cx.clock.now), ..Default::default() };
    products::Entity::update(am).exec(conn).await?;
    cx.touch(ChangeCategory::Catalog);
    Ok(true)
}

/// Read-only: a product's on-hand qty in one branch (`branchStockQty`, `inventory.ts` /
/// `transfers.ts`). Defaults to `Decimal::ZERO` when the product has no row for that branch yet.
pub async fn branch_stock_qty<C: ConnectionTrait>(conn: &C, product_id: Id, branch_id: Id) -> TxResult<Decimal> {
    let row = product_branch_stock::Entity::find()
        .filter(product_branch_stock::Column::ProductId.eq(product_id))
        .filter(product_branch_stock::Column::BranchId.eq(branch_id))
        .one(conn)
        .await?;
    Ok(row.map(|r| r.qty).unwrap_or(Decimal::ZERO))
}
