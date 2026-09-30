//! `shell.rs` (02-setup.md §3.3 "`seed_company_shell`"): the Rust `ensureEmptyCompanyShell` — a
//! behaviour-exact port of `seedEmptyCompany('EG')` (`src/mocks/seed/index.ts:80-121`), in FK order,
//! writing no audit rows (the mock writes none either). Also `format_address` (`format.ts:205-223`,
//! ported here since it's only needed by `apply_branches`).

use chrono::Datelike;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, PaginatorTrait, Set};

use crate::core::auth;
use crate::core::error::AppError;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::org::accounts::ActiveModel as AccountActiveModel;
use crate::entities::org::branches::{ActiveModel as BranchActiveModel, Entity as BranchEntity};
use crate::entities::org::cost_centers::ActiveModel as CostCenterActiveModel;
use crate::entities::org::credentials;
use crate::entities::org::fiscal_years::ActiveModel as FiscalYearActiveModel;
use crate::entities::org::payment_methods::ActiveModel as PaymentMethodActiveModel;
use crate::entities::org::settings::{ActiveModel as SettingsActiveModel, Entity as SettingsEntity};
use crate::entities::org::taxes::ActiveModel as TaxActiveModel;
use crate::entities::org::users::{ActiveModel as UserActiveModel, Entity as UserEntity};
use crate::entities::values::{Address, PrinterMode, PrinterSettings};
use crate::utils::id::Id;

use crate::domains::settings::service::country::country_profile;

/// `formatAddress` (`core/helpers/format.ts:205-223`) — one printable line, most-specific first,
/// comma-separated. Only used by `apply_branches` here (02-setup.md §3.5).
pub fn format_address(addr: Option<&Address>) -> Option<String> {
    let addr = addr?;
    let region = addr.region_name.clone().or_else(|| addr.region_free_text.clone());
    let city = addr.city_name.clone().or_else(|| addr.city_free_text.clone());
    let district = addr.district_name.clone().or_else(|| addr.district_free_text.clone());
    let building_no = if addr.country == "SA" { addr.sa_building_no.clone() } else { addr.building_no.clone() };

    let mut parts: Vec<String> = Vec::new();
    if let Some(s) = &addr.street {
        if !s.trim().is_empty() {
            parts.push(s.clone());
        }
    }
    if let Some(b) = &building_no {
        if !b.trim().is_empty() {
            parts.push(format!("مبنى {b}"));
        }
    }
    if addr.country == "EG" {
        if let Some(f) = &addr.floor {
            if !f.trim().is_empty() {
                parts.push(format!("الدور {f}"));
            }
        }
        if let Some(a) = &addr.apartment {
            if !a.trim().is_empty() {
                parts.push(format!("شقة {a}"));
            }
        }
    }
    if let Some(l) = &addr.landmark {
        if !l.trim().is_empty() {
            parts.push(l.clone());
        }
    }
    if let Some(d) = &district {
        if !d.trim().is_empty() {
            parts.push(d.clone());
        }
    }
    if let Some(c) = &city {
        if !c.trim().is_empty() {
            parts.push(c.clone());
        }
    }
    if let Some(r) = &region {
        if !r.trim().is_empty() {
            parts.push(r.clone());
        }
    }
    let postal = if addr.country == "SA" { addr.sa_postal_code.clone() } else { addr.postal_code.clone() };
    if let Some(p) = &postal {
        if !p.trim().is_empty() {
            parts.push(p.clone());
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("، "))
    }
}

/// True once `users` and `settings` are both empty (the shell has not been seeded yet).
async fn is_unseeded<C: ConnectionTrait>(conn: &C) -> TxResult<bool> {
    let users = UserEntity::find().count(conn).await.map_err(TxError::from)?;
    let settings = SettingsEntity::find().count(conn).await.map_err(TxError::from)?;
    if users == 0 && settings == 0 {
        return Ok(true);
    }
    if users == 0 && settings > 0 {
        return Err(TxError::App(AppError::internal("حالة قاعدة بيانات غير متسقة — لا يوجد مستخدمون رغم وجود صف إعدادات", None)));
    }
    Ok(false)
}

/// `seedEmptyCompany('EG')` (`seed/index.ts:80-121`): idempotent — a no-op once `users` is
/// non-empty (checked by the caller, `get_onboarding_progress`, via [`is_unseeded`]).
pub async fn seed_company_shell<C: ConnectionTrait>(conn: &C, cx: &TxCtx) -> TxResult<()> {
    if !is_unseeded(conn).await? {
        return Ok(());
    }

    let now = cx.clock.now;
    let country = "EG";
    let profile = country_profile(Some(country));

    // 1. Accounts (standard template, EG, no business-type add-on).
    let rows = super::coa::build_accounts(super::coa::AccountTemplateKind::Standard, Some(country), None);
    let mut code_to_id: std::collections::HashMap<String, Id> = std::collections::HashMap::new();
    for r in &rows {
        code_to_id.insert(r.code.clone(), Id::new());
    }
    let mut main_cash_id: Option<Id> = None;
    for (i, r) in rows.iter().enumerate() {
        let id = code_to_id[&r.code];
        if r.code == "1110" {
            main_cash_id = Some(id);
        }
        let parent_id = r.parent_code.as_ref().map(|pc| code_to_id[pc]);
        let model = AccountActiveModel {
            id: Set(id),
            code: Set(r.code.clone()),
            name: Set(r.name.clone()),
            name_en: Set(r.name_en.clone()),
            parent_id: Set(parent_id),
            is_group: Set(r.is_group),
            kind: Set(r.kind.as_str().to_string()),
            subtype: Set(r.subtype.as_str().to_string()),
            normal_side: Set(r.normal_side.as_str().to_string()),
            system_role: Set(r.role.map(|role| role.as_str().to_string())),
            currency: Set(None),
            branch_id: Set(None),
            requires_party: Set(r.requires_party.then_some(true)) /* mock: `requiresParty` is set only when true, else absent */,
            allow_manual: Set(r.allow_manual),
            requires_cost_center: Set(None),
            active: Set(true),
            can_delete: Set(r.can_delete),
            // Template order preserved via `created_at = now + i ms` (mirrors `apply_coa_template`'s
            // D-7 ordering trick — the shell also wants `ORDER BY created_at, id` == template order).
            created_at: Set(now + chrono::Duration::milliseconds(i as i64)),
            updated_at: Set(now),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
            code_live: sea_orm::ActiveValue::NotSet,
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    let main_cash_id = main_cash_id.ok_or_else(|| AppError::internal("لم يتم إنشاء حساب الصندوق الرئيسي أثناء التهيئة", None))?;

    // 2. Fiscal years: [y-1 (closed), y (open)].
    let year = cx.clock.today().year();
    for (offset, is_closed) in [(-1i32, true), (0i32, false)] {
        let y = year + offset;
        let start = chrono::NaiveDate::from_ymd_opt(y, 1, 1).unwrap();
        let end = chrono::NaiveDate::from_ymd_opt(y, 12, 31).unwrap();
        let model = FiscalYearActiveModel {
            id: Set(Id::new()),
            name: Set(y.to_string()),
            start_date: Set(start),
            end_date: Set(end),
            is_closed: Set(is_closed),
            closing_entry_id: Set(None),
            closed_at: Set(None),
            closed_by: Set(None),
            created_at: Set(now + chrono::Duration::milliseconds(offset as i64)),
            updated_at: Set(now),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }

    // 3. Branch `الفرع الرئيسي`/`MAIN`, cash_account_id = 1110, can_delete = false, cost_center_id
    //    NULL at insert (set after the cost center is created, matching the mock's two-step wiring).
    let branch_id = Id::new();
    let branch_model = BranchActiveModel {
        id: Set(branch_id),
        name: Set("الفرع الرئيسي".to_string()),
        code: Set("MAIN".to_string()),
        address: Set(None),
        national_address: Set(None),
        phone: Set(None),
        receipt_header: Set(None),
        cash_account_id: Set(Some(main_cash_id)),
        bank_account_id: Set(None),
        default_price_list_id: Set(None),
        cost_center_id: Set(None),
        active: Set(true),
        can_delete: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    branch_model.insert(conn).await.map_err(TxError::from)?;

    // 4. Cost center `CC-MAIN`/`الفرع الرئيسي`/branch/can_delete false/branch_id -> branch, then set
    //    the branch's own `cost_center_id` back to it.
    let cost_center_id = Id::new();
    let cc_model = CostCenterActiveModel {
        id: Set(cost_center_id),
        code: Set("CC-MAIN".to_string()),
        name: Set("الفرع الرئيسي".to_string()),
        kind: Set("branch".to_string()),
        parent_id: Set(None),
        manager_user_id: Set(None),
        active: Set(true),
        can_delete: Set(false),
        branch_id: Set(Some(branch_id)),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        code_live: sea_orm::ActiveValue::NotSet,
    };
    cc_model.insert(conn).await.map_err(TxError::from)?;

    let branch_row = BranchEntity::find_by_id(branch_id).one(conn).await.map_err(TxError::from)?.ok_or_else(|| AppError::internal("الفرع غير موجود بعد إنشائه", None))?;
    let mut branch_update: BranchActiveModel = branch_row.into();
    branch_update.cost_center_id = Set(Some(cost_center_id));
    branch_update.update(conn).await.map_err(TxError::from)?;

    // 5. Taxes from the country profile: sales (OUTPUT/vatOutput/default) + purchase (INPUT/vatInput/default).
    let vat_rate = rust_decimal::Decimal::from(profile.vat_rate);
    let sales_tax_id = Id::new();
    let sales_tax = TaxActiveModel {
        id: Set(sales_tax_id),
        name: Set(format!("{} (مبيعات)", profile.vat_label)),
        rate: Set(vat_rate),
        r#type: Set("OUTPUT".to_string()),
        is_default: Set(true),
        active: Set(true),
        category: Set("S".to_string()),
        direction: Set("sales".to_string()),
        exemption_reason: Set(None),
        account_role: Set(Some("vatOutput".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    sales_tax.insert(conn).await.map_err(TxError::from)?;

    let purchase_tax = TaxActiveModel {
        id: Set(Id::new()),
        name: Set(format!("{} (مشتريات)", profile.vat_label)),
        rate: Set(vat_rate),
        r#type: Set("INPUT".to_string()),
        is_default: Set(true),
        active: Set(true),
        category: Set("S".to_string()),
        direction: Set("purchase".to_string()),
        exemption_reason: Set(None),
        account_role: Set(Some("vatInput".to_string())),
        created_at: Set(now + chrono::Duration::milliseconds(1)),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    purchase_tax.insert(conn).await.map_err(TxError::from)?;

    // 6. Payment methods: نقداً (cash/cash, pos+payments, sort 1), آجل (credit/receivable, pos only, sort 2).
    let cash_pm = PaymentMethodActiveModel {
        id: Set(Id::new()),
        name: Set("نقداً".to_string()),
        r#type: Set("cash".to_string()),
        icon: Set(None),
        account_role: Set("cash".to_string()),
        fee_pct: Set(rust_decimal::Decimal::ZERO),
        requires_reference: Set(None),
        show_in_pos: Set(true),
        show_in_payments: Set(true),
        sort_order: Set(1),
        branch_overrides: Set(None),
        active: Set(true),
        can_delete: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    cash_pm.insert(conn).await.map_err(TxError::from)?;

    let credit_pm = PaymentMethodActiveModel {
        id: Set(Id::new()),
        name: Set("آجل".to_string()),
        r#type: Set("credit".to_string()),
        icon: Set(None),
        account_role: Set("receivable".to_string()),
        fee_pct: Set(rust_decimal::Decimal::ZERO),
        requires_reference: Set(None),
        show_in_pos: Set(true),
        show_in_payments: Set(false),
        sort_order: Set(2),
        branch_overrides: Set(None),
        active: Set(true),
        can_delete: Set(false),
        created_at: Set(now + chrono::Duration::milliseconds(1)),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    credit_pm.insert(conn).await.map_err(TxError::from)?;

    // 7. Settings row: شركتي / EGP / EG / default_tax_id = sales VAT / INV- / printer a4+80mm /
    //    prices_include_tax true / default_branch_id = MAIN / timezone = Africa/Cairo.
    let settings_model = SettingsActiveModel {
        id: Set(Id::new()),
        singleton: Set(1),
        store_name: Set("شركتي".to_string()),
        logo: Set(None),
        stamp: Set(None),
        signature: Set(None),
        currency: Set(profile.currency_code.to_string()),
        country: Set(Some(profile.code.to_string())),
        vat_number: Set(None),
        default_tax_id: Set(Some(sales_tax_id)),
        invoice_number_prefix: Set("INV-".to_string()),
        printer: Set(PrinterSettings {
            mode: PrinterMode::A4,
            thermal_width_mm: 80,
            thermal: None,
            a4_printer_name: None,
            label_printer_name: None,
            a4_template: None,
            image_template: None,
        }),
        prices_include_tax: Set(true),
        address: Set(None),
        national_address: Set(None),
        phone: Set(None),
        commercial_register: Set(None),
        receipt_footer: Set(None),
        accounting: Set(None),
        backup: Set(None),
        inventory_approval_threshold: Set(None),
        role_access_overrides: Set(None),
        insight_thresholds: Set(None),
        pos: Set(None),
        sales: Set(None),
        features: Set(None),
        onboarding: Set(None),
        timezone: Set(Some(profile.timezone.to_string())),
        default_branch_id: Set(branch_id),
        created_at: Set(now),
        updated_at: Set(now),
    };
    settings_model.insert(conn).await.map_err(TxError::from)?;

    // 8. User `admin`/`المدير`/admin/max_discount 100/active + credentials `hash_password("admin123")`.
    let admin_id = Id::new();
    let admin_model = UserActiveModel {
        id: Set(admin_id),
        username: Set("admin".to_string()),
        name: Set("المدير".to_string()),
        phone: Set(None),
        role: Set("admin".to_string()),
        max_discount: Set(rust_decimal::Decimal::from(100)),
        price_list_id: Set(None),
        active: Set(true),
        avatar: Set(None),
        allowed_branches: Set(None),
        home_branch: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    admin_model.insert(conn).await.map_err(TxError::from)?;

    let password_hash = tokio::task::spawn_blocking(|| auth::hash_password("admin123"))
        .await
        .map_err(|e| AppError::internal("تعذر تشفير كلمة المرور", Some(e.to_string())))?
        .map_err(|e| AppError::internal("تعذر تشفير كلمة المرور", Some(e.to_string())))?;
    let creds_model = credentials::ActiveModel { user_id: Set(admin_id), password_hash: Set(password_hash), created_at: Set(now), updated_at: Set(now) };
    creds_model.insert(conn).await.map_err(TxError::from)?;

    Ok(())
}

// Bootstrap session (D-1, §3.2) lives in `session.rs` — `ensure_bootstrap_session`/
// `clear_bootstrap_session`, used by `commands.rs`.
