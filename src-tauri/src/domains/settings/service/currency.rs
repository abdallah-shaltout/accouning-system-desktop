//! `currency.rs` — `get_currencies`/`get_exchange_rates`/`create_currency`/`update_currency`/
//! `save_exchange_rate`/`is_base_currency_locked`/`set_base_currency` (01-settings.md §3
//! "Currency"). `is_base_currency_locked`/`set_base_currency`/`create_currency` are `pub`
//! (02-setup reuses them, per the entry file's own note).

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::{AppError, AppResult};
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::journal::journal_entries::Entity as JournalEntryEntity;
use crate::entities::org::currencies::{ActiveModel as CurrencyActiveModel, Column as CurrencyColumn, Entity as CurrencyEntity};
use crate::entities::org::exchange_rates::{ActiveModel as RateActiveModel, Column as RateColumn, Entity as RateEntity};
use crate::entities::platform::activity::ActivityKind;
use crate::shared::activity::undo::UndoRegistry;
use crate::shared::currency;
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use super::super::dto::{Currency, CurrencyPatch, ExchangeRate, ExchangeRateInput};

pub async fn list_currencies<C: ConnectionTrait>(conn: &C) -> AppResult<Vec<Currency>> {
    let rows = CurrencyEntity::find().order_by_asc(CurrencyColumn::CreatedAt).order_by_asc(CurrencyColumn::Code).all(conn).await.map_err(AppError::from)?;
    Ok(rows.into_iter().map(Currency::from_model).collect())
}

pub async fn list_exchange_rates<C: ConnectionTrait>(conn: &C, currency_filter: Option<String>) -> AppResult<Vec<ExchangeRate>> {
    let mut q = RateEntity::find().order_by_asc(RateColumn::CreatedAt).order_by_asc(RateColumn::Id);
    if let Some(c) = currency_filter {
        q = q.filter(RateColumn::Currency.eq(c));
    }
    let rows = q.all(conn).await.map_err(AppError::from)?;
    Ok(rows.into_iter().map(ExchangeRate::from_model).collect())
}

/// `isBaseCurrencyLocked` (`currency.ts:50-52`) — `pub` (02-setup reuses it).
pub async fn is_base_currency_locked<C: ConnectionTrait>(conn: &C) -> TxResult<bool> {
    let count = JournalEntryEntity::find().count(conn).await.map_err(TxError::from)?;
    Ok(count > 0)
}

/// `setBaseCurrency` (`currency.ts:54-59`) — `pub` (02-setup reuses it).
pub async fn set_base_currency<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, code: String) -> TxResult<()> {
    if is_base_currency_locked(conn).await? {
        return Err(TxError::App(AppError::forbidden("لا يمكن تغيير العملة الأساسية بعد بدء الترحيل")));
    }
    let upper = code.to_uppercase();
    let locked = crate::core::settings::load_shared_locked(conn).await?;
    let mut model: crate::entities::org::settings::ActiveModel = locked.into();
    model.currency = Set(upper.clone());
    model.updated_at = Set(cx.clock.now);
    model.update(conn).await.map_err(TxError::from)?;

    crate::shared::activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Settings,
        format!("تغيير العملة الأساسية إلى {upper}"),
        None,
        Some(RouteRef::list("settings-general")),
    )
    .await?;
    Ok(())
}

/// `createCurrency` (`currency.ts:30-37`) — `pub` (02-setup reuses it). Case-sensitive comparison
/// against the base currency (Q-6, before uppercasing) — the mock has no audit for this write.
pub async fn create<C: ConnectionTrait>(conn: &C, cx: &TxCtx, input: Currency) -> TxResult<Currency> {
    let base = currency::base_currency(conn).await?;
    if input.code == base {
        return Err(TxError::App(AppError::validation("هذه هي العملة الأساسية بالفعل")));
    }
    let exists = CurrencyEntity::find_by_id(input.code.clone()).one(conn).await.map_err(TxError::from)?.is_some();
    if exists {
        return Err(TxError::App(AppError::validation("هذه العملة مضافة بالفعل")));
    }
    let upper = input.code.to_uppercase();
    // P4-4/B-3: the transaction's business clock (the DB's `UTC_TIMESTAMP`), not the OS clock.
    let now = cx.clock.now;
    let model = CurrencyActiveModel {
        code: Set(upper),
        name_ar: Set(input.name_ar),
        symbol: Set(input.symbol),
        decimals: Set(input.decimals),
        active: Set(input.active),
        fixed: Set(input.fixed),
        fixed_rate: Set(input.fixed_rate),
        created_at: Set(now),
        updated_at: Set(now),
    };
    // Q-6: a PK clash from the upper-casing races the pre-check above (two concurrent creates of
    // the same code in different letter-casing) — same "already added" text as the pre-check.
    let inserted = match model.insert(conn).await {
        Ok(m) => m,
        Err(e) if crate::core::error::mysql_errno(&e) == Some(crate::core::error::MYSQL_ERRNO_DUPLICATE_KEY) => {
            return Err(TxError::App(AppError::validation("هذه العملة مضافة بالفعل")));
        }
        Err(e) => return Err(TxError::from(e)),
    };
    Ok(Currency::from_model(inserted))
}

/// `updateCurrency` (`currency.ts:39-44`) — plain merge, `code` is never changed (D-8).
pub async fn update<C: ConnectionTrait>(conn: &C, cx: &TxCtx, code: String, patch: CurrencyPatch) -> TxResult<Currency> {
    let existing = CurrencyEntity::find_by_id(code).one(conn).await.map_err(TxError::from)?;
    let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("العملة غير موجودة"))) };

    let mut model: CurrencyActiveModel = existing.into();
    if let Some(v) = patch.name_ar {
        model.name_ar = Set(v);
    }
    if let Some(v) = patch.symbol {
        model.symbol = Set(v);
    }
    if let Some(v) = patch.decimals {
        model.decimals = Set(v);
    }
    if let Some(v) = patch.active {
        model.active = Set(v);
    }
    if let Some(v) = patch.fixed {
        model.fixed = Set(Some(v));
    }
    if let Some(v) = patch.fixed_rate {
        model.fixed_rate = Set(Some(v));
    }
    model.updated_at = Set(cx.clock.now);
    let updated = model.update(conn).await.map_err(TxError::from)?;
    Ok(Currency::from_model(updated))
}

/// `saveExchangeRate` (`currency.ts:62-74`): delete-then-insert for the same `(currency, date)` —
/// the mock moves the row to the end of the array, so list order stays equal.
pub async fn save_exchange_rate<C: ConnectionTrait>(conn: &C, cx: &TxCtx, input: ExchangeRateInput) -> TxResult<ExchangeRate> {
    // Currency row locked FOR UPDATE (serialises same-currency saves, §4) — one locking read that
    // is also the existence check. `currencies` is keyed by `code`, not `id` (so the generic
    // `core::lock::for_update_by_id`, which selects `id`, cannot be used here).
    let lock = sea_orm::Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT `code` FROM `currencies` WHERE `code` = ? FOR UPDATE",
        [input.currency.clone().into()],
    );
    if conn.query_one(lock).await.map_err(TxError::from)?.is_none() {
        return Err(TxError::App(AppError::not_found("العملة غير مفعّلة")));
    }

    let rate = match input.rate {
        Some(r) if !r.is_zero() => Some(r),
        _ => input.inverse_rate.filter(|r| !r.is_zero()).map(|inv| crate::utils::money::round2(Decimal::ONE / inv)),
    };
    let rate = match rate {
        Some(r) if r > Decimal::ZERO => r,
        _ => return Err(TxError::App(AppError::validation("أدخل سعر الصرف"))),
    };

    let date = chrono::NaiveDate::parse_from_str(&input.date, "%Y-%m-%d")
        .map_err(|_| AppError::validation("تاريخ غير صالح"))?;
    let currency_upper = input.currency.to_uppercase();

    RateEntity::delete_many()
        .filter(RateColumn::Currency.eq(currency_upper.clone()))
        .filter(RateColumn::Date.eq(date))
        .exec(conn)
        .await
        .map_err(TxError::from)?;

    let model = RateActiveModel {
        id: Set(Id::new()),
        currency: Set(currency_upper),
        date: Set(date),
        rate: Set(rate),
        created_at: Set(cx.clock.now),
        updated_at: Set(cx.clock.now),
    };
    let inserted = model.insert(conn).await.map_err(TxError::from)?;
    Ok(ExchangeRate::from_model(inserted))
}
