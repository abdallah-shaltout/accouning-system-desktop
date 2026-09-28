//! `country_tax.rs` (02-setup.md §3.5, `setupService.ts:104-136`): applies a country/currency/VAT
//! choice — composes `settings::currency::{is_base_currency_locked, set_base_currency, create}`.

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::core::error::AppError;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::domains::settings::dto::Currency as CurrencyDto;
use crate::domains::settings::service::currency;
use crate::entities::org::taxes::{ActiveModel as TaxActiveModel, Column as TaxColumn, Entity as TaxEntity};
use crate::shared::activity::undo::UndoRegistry;

use super::super::dto::CountryTaxInput;
use crate::domains::settings::service::country::country_profile;

/// `applyCountryTax` (`setupService.ts:104-136`).
pub async fn apply<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, input: CountryTaxInput) -> TxResult<()> {
    // 1. Refused once base currency is locked (any journal entry posted).
    if currency::is_base_currency_locked(conn).await? {
        return Err(TxError::App(AppError::forbidden("لا يمكن تغيير الدولة أو العملة الأساسية بعد أول ترحيل")));
    }

    // Serialises wizard steps — locked before any of the writes below (§3 "Common").
    let locked = crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;

    let profile = country_profile(Some(&input.country));

    // 3. Currency change (only if it actually changed) — `set_base_currency` logs its own activity.
    if locked.currency != input.currency {
        currency::set_base_currency(conn, cx, registry, input.currency.clone()).await?;
    }

    // 4. prices_include_tax / country / timezone (D-4).
    let mut model: crate::entities::org::settings::ActiveModel = locked.into();
    model.prices_include_tax = Set(input.prices_include_tax);
    model.country = Set(Some(profile.code.to_string()));
    model.timezone = Set(Some(profile.timezone.to_string()));
    model.updated_at = Set(cx.clock.now);
    model.update(conn).await.map_err(TxError::from)?;

    // Live taxes with account_role = vatOutput/vatInput get their rate/name rewritten.
    let vat_rate = rust_decimal::Decimal::from(profile.vat_rate);
    let live_taxes = TaxEntity::find().filter(TaxColumn::DeletedAt.is_null()).all(conn).await.map_err(TxError::from)?;
    for t in live_taxes {
        let new_name = match t.account_role.as_deref() {
            Some("vatOutput") => Some(format!("{} (مبيعات)", profile.vat_label)),
            Some("vatInput") => Some(format!("{} (مشتريات)", profile.vat_label)),
            _ => None,
        };
        if let Some(name) = new_name {
            let mut tax_model: TaxActiveModel = t.into();
            tax_model.rate = Set(vat_rate);
            tax_model.name = Set(name);
            tax_model.updated_at = Set(cx.clock.now);
            tax_model.update(conn).await.map_err(TxError::from)?;
        }
    }

    // 5. Extra currencies: non-empty code, not already present (exact match) -> createCurrency.
    let mut any_created = false;
    for extra in &input.extra_currencies {
        let code = extra.code.trim();
        if code.is_empty() {
            continue;
        }
        let exists = crate::entities::org::currencies::Entity::find_by_id(code.to_string())
            .one(conn)
            .await
            .map_err(TxError::from)?
            .is_some();
        if exists {
            continue;
        }
        let dto = CurrencyDto {
            code: code.to_string(),
            name_ar: code.to_string(),
            symbol: code.to_string(),
            decimals: 2,
            active: true,
            fixed: Some(false),
            fixed_rate: Some(extra.rate),
        };
        currency::create(conn, dto).await?;
        any_created = true;
    }

    // 6. features.currencies = true once at least one extra currency was added.
    if any_created {
        let locked2 = crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;
        let mut features = locked2.features.unwrap_or(crate::entities::values::FeatureFlags { branches: None, currencies: None, cost_centers: None });
        features.currencies = Some(true);
        let mut model2: crate::entities::org::settings::ActiveModel = locked2.into();
        model2.features = Set(Some(features));
        model2.updated_at = Set(cx.clock.now);
        model2.update(conn).await.map_err(TxError::from)?;
    }

    // `vatRegistered` is ignored (`:135`) — accepted on the wire but never used.
    let _ = input.vat_registered;

    Ok(())
}
