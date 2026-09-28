//! `shared::ledger::accounts` (C-1): the ONLY role → account lookup in the backend, ported 1:1
//! from `src/mocks/backend/accounts.ts`. Nothing outside `shared::ledger` should ever resolve an
//! account by system role or hard-coded code.

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use ts_rs::TS;

use crate::core::error::AppError;
use crate::entities::org::accounts::{Column, Entity, Model as Account};
use crate::utils::id::Id;

/// `SystemRole` — the 36 system roles a leaf account may be tagged with (`accounts.ts:11-48`).
/// The entity's `system_role` column is a plain nullable `String` (several roles can share the
/// role in principle — branch cash drawers, per-currency banks — see `Account.systemRole`'s own
/// doc comment), so this enum exists purely as the typed, exhaustive Rust surface `resolve_account`
/// and its callers use; `as_str()`/`ROLE_LABEL` are its only bridge to the stored string.
///
/// G-12: also the DTO-facing type (`serde`, `TS`) other domains' response shapes reference (an
/// account's `systemRole` field) — `#[serde(rename_all = "camelCase")]` matches the mock's own
/// `SystemRole` string union member spelling (`as_str()`), and `FromStr` is the inverse of
/// `as_str()` for reading the DB's plain `String` column back into this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub enum SystemRole {
    Cash,
    Bank,
    CardClearing,
    WalletClearing,
    Receivable,
    Inventory,
    InventoryInTransit,
    VatInput,
    Payable,
    VatOutput,
    VatPayable,
    CustomerAdvances,
    Capital,
    OwnerCurrent,
    Drawings,
    RetainedEarnings,
    CurrentEarnings,
    OpeningBalanceEquity,
    Sales,
    ServiceRevenue,
    SalesReturns,
    OtherIncome,
    FxGain,
    CashOver,
    PurchaseDiscounts,
    Cogs,
    InventoryVariance,
    InventoryWriteOff,
    FreightIn,
    CardFees,
    BankFees,
    FxLoss,
    CashShort,
    BadDebt,
    Depreciation,
    Zakat,
}

impl SystemRole {
    /// The exact string stored in `accounts.system_role` — matches the mock's `SystemRole` union
    /// member spelling (`accounts.ts`/`accounting/types/index.ts`) byte-for-byte.
    pub fn as_str(self) -> &'static str {
        match self {
            SystemRole::Cash => "cash",
            SystemRole::Bank => "bank",
            SystemRole::CardClearing => "cardClearing",
            SystemRole::WalletClearing => "walletClearing",
            SystemRole::Receivable => "receivable",
            SystemRole::Inventory => "inventory",
            SystemRole::InventoryInTransit => "inventoryInTransit",
            SystemRole::VatInput => "vatInput",
            SystemRole::Payable => "payable",
            SystemRole::VatOutput => "vatOutput",
            SystemRole::VatPayable => "vatPayable",
            SystemRole::CustomerAdvances => "customerAdvances",
            SystemRole::Capital => "capital",
            SystemRole::OwnerCurrent => "ownerCurrent",
            SystemRole::Drawings => "drawings",
            SystemRole::RetainedEarnings => "retainedEarnings",
            SystemRole::CurrentEarnings => "currentEarnings",
            SystemRole::OpeningBalanceEquity => "openingBalanceEquity",
            SystemRole::Sales => "sales",
            SystemRole::ServiceRevenue => "serviceRevenue",
            SystemRole::SalesReturns => "salesReturns",
            SystemRole::OtherIncome => "otherIncome",
            SystemRole::FxGain => "fxGain",
            SystemRole::CashOver => "cashOver",
            SystemRole::PurchaseDiscounts => "purchaseDiscounts",
            SystemRole::Cogs => "cogs",
            SystemRole::InventoryVariance => "inventoryVariance",
            SystemRole::InventoryWriteOff => "inventoryWriteOff",
            SystemRole::FreightIn => "freightIn",
            SystemRole::CardFees => "cardFees",
            SystemRole::BankFees => "bankFees",
            SystemRole::FxLoss => "fxLoss",
            SystemRole::CashShort => "cashShort",
            SystemRole::BadDebt => "badDebt",
            SystemRole::Depreciation => "depreciation",
            SystemRole::Zakat => "zakat",
        }
    }

    /// `ROLE_LABEL` (`accounts.ts:11-48`) — the Arabic label used in the "no account for this
    /// role" error message.
    pub fn label(self) -> &'static str {
        match self {
            SystemRole::Cash => "الصندوق",
            SystemRole::Bank => "البنك",
            SystemRole::CardClearing => "تسوية البطاقات",
            SystemRole::WalletClearing => "تسوية المحافظ الإلكترونية",
            SystemRole::Receivable => "العملاء",
            SystemRole::Inventory => "المخزون",
            SystemRole::InventoryInTransit => "بضاعة بالطريق بين الفروع",
            SystemRole::VatInput => "ضريبة القيمة المضافة — مدخلات",
            SystemRole::Payable => "الموردين",
            SystemRole::VatOutput => "ضريبة القيمة المضافة — مخرجات",
            SystemRole::VatPayable => "ضريبة القيمة المضافة — صافي مستحق",
            SystemRole::CustomerAdvances => "دفعات مقدمة من العملاء",
            SystemRole::Capital => "رأس المال",
            SystemRole::OwnerCurrent => "جاري المالك",
            SystemRole::Drawings => "المسحوبات الشخصية",
            SystemRole::RetainedEarnings => "الأرباح المحتجزة",
            SystemRole::CurrentEarnings => "صافي ربح الفترة",
            SystemRole::OpeningBalanceEquity => "أرصدة افتتاحية",
            SystemRole::Sales => "مبيعات البضائع",
            SystemRole::ServiceRevenue => "إيرادات الخدمات",
            SystemRole::SalesReturns => "مرتجعات المبيعات",
            SystemRole::OtherIncome => "إيرادات أخرى",
            SystemRole::FxGain => "أرباح فروق العملة",
            SystemRole::CashOver => "زيادة الصندوق",
            SystemRole::PurchaseDiscounts => "خصم مكتسب من الموردين",
            SystemRole::Cogs => "تكلفة البضاعة المباعة",
            SystemRole::InventoryVariance => "فروقات جرد المخزون",
            SystemRole::InventoryWriteOff => "بضاعة تالفة ومنتهية الصلاحية",
            SystemRole::FreightIn => "شحن وتخليص المشتريات",
            SystemRole::CardFees => "عمولات البطاقات",
            SystemRole::BankFees => "رسوم بنكية",
            SystemRole::FxLoss => "خسائر فروق العملة",
            SystemRole::CashShort => "عجز الصندوق",
            SystemRole::BadDebt => "ديون معدومة",
            SystemRole::Depreciation => "مصروف الإهلاك",
            SystemRole::Zakat => "الزكاة",
        }
    }

    /// Every variant, in declaration order — used by `FromStr`'s exhaustive match and by callers
    /// that need to enumerate all 36 roles (e.g. a picker, or a seed/import validity check).
    pub const ALL: [SystemRole; 36] = [
        SystemRole::Cash,
        SystemRole::Bank,
        SystemRole::CardClearing,
        SystemRole::WalletClearing,
        SystemRole::Receivable,
        SystemRole::Inventory,
        SystemRole::InventoryInTransit,
        SystemRole::VatInput,
        SystemRole::Payable,
        SystemRole::VatOutput,
        SystemRole::VatPayable,
        SystemRole::CustomerAdvances,
        SystemRole::Capital,
        SystemRole::OwnerCurrent,
        SystemRole::Drawings,
        SystemRole::RetainedEarnings,
        SystemRole::CurrentEarnings,
        SystemRole::OpeningBalanceEquity,
        SystemRole::Sales,
        SystemRole::ServiceRevenue,
        SystemRole::SalesReturns,
        SystemRole::OtherIncome,
        SystemRole::FxGain,
        SystemRole::CashOver,
        SystemRole::PurchaseDiscounts,
        SystemRole::Cogs,
        SystemRole::InventoryVariance,
        SystemRole::InventoryWriteOff,
        SystemRole::FreightIn,
        SystemRole::CardFees,
        SystemRole::BankFees,
        SystemRole::FxLoss,
        SystemRole::CashShort,
        SystemRole::BadDebt,
        SystemRole::Depreciation,
        SystemRole::Zakat,
    ];
}

/// G-12: the inverse of `as_str()`, for reading `accounts.system_role` (a plain nullable
/// `String` column, never a DB enum — see the struct doc comment) back into the typed enum. Used
/// wherever a DTO assembler needs to surface an account's system role as `SystemRole` rather than
/// a raw string (e.g. an accounts-list DTO in Part 03). Unknown strings are not a panic — a
/// future/unrecognized role string in the DB is possible after a partial migration or manual SQL,
/// so this is a normal `Result`, not an `unwrap`-only path.
impl FromStr for SystemRole {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SystemRole::ALL
            .into_iter()
            .find(|role| role.as_str() == s)
            .ok_or_else(|| AppError::internal("دور حساب غير معروف في شجرة الحسابات", Some(format!("unknown SystemRole string: {s:?}"))))
    }
}

/// `AccountCtx` (`accounts.ts`'s `AccountCtx`).
#[derive(Debug, Clone, Default)]
pub struct AccountCtx {
    pub branch_id: Option<Id>,
    pub currency: Option<String>,
}

/// Resolves a system role to its account (`accountFor`, `accounts.ts:61-69`). Candidates are live,
/// active rows with this `system_role`, in `(created_at, id)` order (the mock's array insertion
/// order). Picks the branch match, else the currency match, else a row with neither, else the
/// first. `NOT_FOUND` naming the role's Arabic label when none exists.
pub async fn resolve_account<C: ConnectionTrait>(conn: &C, role: SystemRole, ctx: &AccountCtx) -> Result<Account, AppError> {
    let candidates: Vec<Account> = Entity::find()
        .filter(Column::SystemRole.eq(role.as_str()))
        .filter(Column::Active.eq(true))
        .filter(Column::DeletedAt.is_null())
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let by_branch = ctx.branch_id.and_then(|bid| candidates.iter().find(|a| a.branch_id == Some(bid)));
    let by_currency = ctx.currency.as_ref().and_then(|cur| candidates.iter().find(|a| a.currency.as_deref() == Some(cur.as_str())));
    let neutral = candidates.iter().find(|a| a.branch_id.is_none() && a.currency.is_none());
    let first = candidates.first();

    let account = by_branch.or(by_currency).or(neutral).or(first);

    match account {
        Some(a) => Ok(a.clone()),
        None => Err(AppError::not_found(format!(
            "لا يوجد حساب في شجرة الحسابات لدور \"{}\" — أضف حساباً بهذا الدور أولاً",
            role.label()
        ))),
    }
}

/// `accountById` (`accounts.ts:73-77`). Looks up regardless of `active`/soft-delete state — the
/// mock's `accountById` never filters (P2-36: an inactive explicit id is allowed).
pub async fn account_by_id<C: ConnectionTrait>(conn: &C, id: Id) -> Result<Account, AppError> {
    Entity::find_by_id(id)
        .one(conn)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("الحساب غير موجود في شجرة الحسابات"))
}

/// `settlementAccountFor` (`accounts.ts:82-88`): cash for cash payments, bank for anything else
/// (card / transfer / credit settlement — credit itself never settles here).
pub async fn settlement_account_for<C: ConnectionTrait>(conn: &C, method: &str, ctx: &AccountCtx) -> Result<Account, AppError> {
    if method == "cash" {
        resolve_account(conn, SystemRole::Cash, ctx).await
    } else {
        resolve_account(conn, SystemRole::Bank, ctx).await
    }
}

/// A product/category's account override chain — the exact shape `revenueAccountFor`/
/// `cogsAccountFor`/`purchaseAccountFor` (`accounts.ts:110-153`) need without depending on the
/// `products` domain's DTOs (Part 03 owns those). Callers pass whichever ids the resolved
/// product/category actually carry.
pub struct ProductAccountOverrides {
    pub product_account_id: Option<Id>,
    pub category_account_id: Option<Id>,
}

/// `revenueAccountFor`: product override → category override → `sales`/`serviceRevenue` role
/// default (`is_service` picks which role default applies).
pub async fn revenue_account_for<C: ConnectionTrait>(
    conn: &C,
    overrides: &ProductAccountOverrides,
    is_service: bool,
) -> Result<Account, AppError> {
    if let Some(id) = overrides.product_account_id {
        return account_by_id(conn, id).await;
    }
    if let Some(id) = overrides.category_account_id {
        return account_by_id(conn, id).await;
    }
    let role = if is_service { SystemRole::ServiceRevenue } else { SystemRole::Sales };
    resolve_account(conn, role, &AccountCtx::default()).await
}

/// `cogsAccountFor`: product override → category override → `cogs` role default.
pub async fn cogs_account_for<C: ConnectionTrait>(conn: &C, overrides: &ProductAccountOverrides) -> Result<Account, AppError> {
    if let Some(id) = overrides.product_account_id {
        return account_by_id(conn, id).await;
    }
    if let Some(id) = overrides.category_account_id {
        return account_by_id(conn, id).await;
    }
    resolve_account(conn, SystemRole::Cogs, &AccountCtx::default()).await
}

/// `purchaseAccountFor`: product → category → settings default → `freightIn` fallback.
pub async fn purchase_account_for<C: ConnectionTrait>(
    conn: &C,
    overrides: &ProductAccountOverrides,
    default_purchase_account_id: Option<Id>,
) -> Result<Account, AppError> {
    if let Some(id) = overrides.product_account_id {
        return account_by_id(conn, id).await;
    }
    if let Some(id) = overrides.category_account_id {
        return account_by_id(conn, id).await;
    }
    if let Some(id) = default_purchase_account_id {
        return account_by_id(conn, id).await;
    }
    resolve_account(conn, SystemRole::FreightIn, &AccountCtx::default()).await
}

/// `saleTaxIdFor`: product override → category override → `None` (caller falls back to the store
/// default tax).
pub fn sale_tax_id_for(product_tax_id: Option<Id>, category_tax_id: Option<Id>) -> Option<Id> {
    product_tax_id.or(category_tax_id)
}

/// `purchaseTaxIdFor`: same shape as `sale_tax_id_for`.
pub fn purchase_tax_id_for(product_tax_id: Option<Id>, category_tax_id: Option<Id>) -> Option<Id> {
    product_tax_id.or(category_tax_id)
}

#[cfg(test)]
mod system_role_tests {
    use super::*;

    #[test]
    fn from_str_is_the_exact_inverse_of_as_str_for_every_role() {
        for role in SystemRole::ALL {
            assert_eq!(SystemRole::from_str(role.as_str()).unwrap(), role);
        }
    }

    #[test]
    fn from_str_rejects_unknown_strings() {
        assert!(SystemRole::from_str("notARealRole").is_err());
        assert!(SystemRole::from_str("").is_err());
        // case-sensitive: the DB column stores the exact `as_str()` spelling.
        assert!(SystemRole::from_str("Cash").is_err());
    }

    #[test]
    fn serde_round_trips_as_camel_case() {
        let json = serde_json::to_string(&SystemRole::VatOutput).unwrap();
        assert_eq!(json, "\"vatOutput\"");
        let back: SystemRole = serde_json::from_str(&json).unwrap();
        assert_eq!(back, SystemRole::VatOutput);
    }
}
