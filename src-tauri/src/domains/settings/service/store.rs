//! `store.rs` — `get_settings`/`update_settings` (01-settings.md §3 "get_settings"/"update_settings").
//! Merges the branch-DB `settings` row with this process's own `device-settings.json` (cross-cutting
//! §3), exactly as `settingsService.getSettings/updateSettings` appear to the frontend as one
//! `StoreSettings` object.

use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use crate::core::device::{DeviceSettings, ThermalPrinterConfig};
use crate::core::error::{AppError, AppResult};
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::org::settings::ActiveModel as SettingsActiveModel;
use crate::entities::platform::activity::ActivityKind;
use crate::shared::activity::undo::UndoRegistry;
use crate::utils::route::RouteRef;

use super::country::country_timezone;
use super::super::dto::*;

fn printer_dto(row_printer: &crate::entities::values::PrinterSettings, device: &DeviceSettings) -> PrinterSettings {
    // D-1/D-4: device-owned keys always come from the device file, never the row's own JSON
    // (even if present there — a stale value from before D-1 was applied is ignored on read).
    let thermal = device.printer.thermal.as_ref().map(|t| ThermalPrinterSettings {
        printer_name: t.printer_name.clone(),
        connection: match t.connection.as_deref() {
            Some("network") => PrinterConnectionType::Network,
            _ => PrinterConnectionType::Windows,
        },
        host: t.host.clone(),
        dpi: t.dpi.unwrap_or(203) as i32,
        cut: t.cut.unwrap_or(true),
        open_drawer: t.open_drawer.unwrap_or(false),
        copies: t.copies.unwrap_or(1) as i32,
    });

    PrinterSettings {
        mode: match row_printer.mode {
            crate::entities::values::PrinterMode::A4 => PrinterMode::A4,
            crate::entities::values::PrinterMode::Thermal => PrinterMode::Thermal,
        },
        thermal_width_mm: row_printer.thermal_width_mm,
        thermal,
        a4_printer_name: device.printer.a4_printer_name.clone(),
        label_printer_name: device.printer.label_printer_name.clone(),
        a4_template: row_printer.a4_template.clone(),
        image_template: row_printer.image_template.clone(),
    }
}

fn onboarding_dto(o: crate::entities::values::OnboardingState) -> OnboardingState {
    OnboardingState {
        business_type: o.business_type,
        go_live_date: o.go_live_date.map(|d| d.format("%Y-%m-%d").to_string()),
        completed_step: o.completed_step,
        skipped: if o.skipped.is_empty() { None } else { Some(o.skipped) },
        done: if o.done.is_empty() { None } else { Some(o.done) },
        finished_at: o.finished_at.map(crate::utils::dates::format_iso_ms),
        opening_entry_id: o.opening_entry_id,
        closing_entry_id: o.closing_entry_id,
        coa_template: o.coa_template,
    }
}

fn pos_policy_dto(p: crate::entities::values::PosPolicy) -> PosPolicy {
    PosPolicy {
        override_price: p.override_price,
        sell_below_cost: p.sell_below_cost,
        require_open_shift: p.require_open_shift,
        foreign_cash_enabled: p.foreign_cash_enabled,
        foreign_currency: p.foreign_currency,
        foreign_currency_rate: p.foreign_currency_rate,
    }
}

fn accounting_policy_dto(a: crate::entities::values::AccountingPolicy) -> AccountingPolicy {
    AccountingPolicy { lock_date: a.lock_date.map(|d| d.format("%Y-%m-%d").to_string()), default_purchase_account_id: a.default_purchase_account_id }
}

fn feature_flags_dto(f: crate::entities::values::FeatureFlags) -> FeatureFlags {
    FeatureFlags { branches: f.branches, currencies: f.currencies, cost_centers: f.cost_centers }
}

fn insight_thresholds_dto(t: crate::entities::values::InsightThresholds) -> std::collections::BTreeMap<String, crate::entities::values::JsonDecimal> {
    t.0
}

/// `get_settings(conn, device)` (01-settings.md §3): the branch row's own fields plus the device
/// file's printer/backup-folder fields merged in.
pub async fn get_settings<C: ConnectionTrait>(conn: &C, device: &DeviceSettings) -> AppResult<StoreSettings> {
    let row = crate::core::settings::load(conn).await?;

    let backup = match row.backup {
        Some(b) => {
            let mut value = serde_json::to_value(&b).map_err(|e| AppError::internal("تعذرت قراءة إعدادات النسخ الاحتياطي", Some(e.to_string())))?;
            if let (Some(folder), Some(obj)) = (&device.backup_folder, value.as_object_mut()) {
                obj.insert("folder".to_string(), serde_json::Value::String(folder.clone()));
            }
            Some(value)
        }
        None => device.backup_folder.as_ref().map(|folder| {
            serde_json::json!({
                "autoEnabled": false,
                "autoTime": "02:00",
                "retention": 30,
                "folder": folder,
            })
        }),
    };

    Ok(StoreSettings {
        store_name: row.store_name,
        logo: row.logo,
        stamp: row.stamp,
        signature: row.signature,
        currency: row.currency,
        country: row.country,
        vat_number: row.vat_number,
        default_tax_id: row.default_tax_id,
        invoice_number_prefix: row.invoice_number_prefix,
        printer: printer_dto(&row.printer, device),
        prices_include_tax: Some(row.prices_include_tax),
        address: row.address,
        national_address: row.national_address,
        phone: row.phone,
        commercial_register: row.commercial_register,
        receipt_footer: row.receipt_footer,
        accounting: row.accounting.map(accounting_policy_dto),
        backup,
        inventory_approval_threshold: row.inventory_approval_threshold,
        role_access_overrides: row.role_access_overrides,
        insight_thresholds: row.insight_thresholds.map(insight_thresholds_dto),
        pos: row.pos.map(pos_policy_dto),
        sales: row.sales.map(|s| SalesPolicy { refund_without_receipt: s.refund_without_receipt }),
        features: row.features.map(feature_flags_dto),
        onboarding: row.onboarding.map(onboarding_dto),
    })
}

/// The device-file delta `update_settings` computes from a patch — applied before the transaction
/// (D-6), saved, then only kept if the transaction commits.
#[derive(Debug, Clone, Default)]
pub struct DeviceDelta {
    pub thermal: Option<Option<ThermalPrinterConfig>>,
    pub a4_printer_name: Option<Option<String>>,
    pub label_printer_name: Option<Option<String>>,
    pub backup_folder: Option<Option<String>>,
}

impl DeviceDelta {
    pub fn is_empty(&self) -> bool {
        self.thermal.is_none() && self.a4_printer_name.is_none() && self.label_printer_name.is_none() && self.backup_folder.is_none()
    }

    pub fn apply(&self, device: &mut DeviceSettings) {
        if let Some(t) = &self.thermal {
            device.printer.thermal = t.clone();
        }
        if let Some(a) = &self.a4_printer_name {
            device.printer.a4_printer_name = a.clone();
        }
        if let Some(l) = &self.label_printer_name {
            device.printer.label_printer_name = l.clone();
        }
        if let Some(f) = &self.backup_folder {
            device.backup_folder = f.clone();
        }
    }
}

fn thermal_config_from_dto(t: &ThermalPrinterSettings) -> ThermalPrinterConfig {
    ThermalPrinterConfig {
        printer_name: t.printer_name.clone(),
        connection: Some(match t.connection {
            PrinterConnectionType::Windows => "windows".to_string(),
            PrinterConnectionType::Network => "network".to_string(),
        }),
        host: t.host.clone(),
        dpi: Some(t.dpi as u32),
        cut: Some(t.cut),
        open_drawer: Some(t.open_drawer),
        copies: Some(t.copies as u32),
    }
}

/// `update_settings` (01-settings.md §3), steps 1-6. Returns the committed row plus the
/// `DeviceDelta` that must be applied to `AppState.device`/`device-settings.json` (the command
/// layer does that around this call, D-6).
pub async fn update_settings<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    patch: StoreSettingsPatch,
) -> TxResult<(crate::entities::org::settings::Model, DeviceDelta)> {
    // Step 1: storeName blank check.
    if let Some(name) = &patch.store_name {
        if name.trim().is_empty() {
            return Err(TxError::App(AppError::validation("اسم المتجر مطلوب")));
        }
    }

    // Read the current row (unlocked) first to resolve the country profile for VAT validation,
    // matching the mock's `patch.country ?? db.settings.country` read-before-lock order.
    let current = crate::core::settings::load(conn).await?;

    // Step 2: vatNumber validation against the (possibly patched) country.
    if let Some(vat) = &patch.vat_number {
        if !vat.is_empty() {
            let country = patch.country.as_deref().or(current.country.as_deref());
            let profile = super::country::country_profile(country);
            if !(profile.tax_id_valid)(vat) {
                return Err(TxError::App(AppError::validation(format!("{} يجب أن يكون {}", profile.tax_id_label, profile.tax_id_hint))));
            }
        }
    }

    // Step 3: lock the row for the read-modify-write.
    let locked = crate::core::settings::load_shared_locked(conn).await?;
    let mut model: SettingsActiveModel = locked.clone().into();

    let mut device_delta = DeviceDelta::default();

    if let Some(v) = patch.store_name {
        model.store_name = Set(v);
    }
    if let Some(v) = patch.logo {
        model.logo = Set(Some(v));
    }
    if let Some(v) = patch.stamp {
        model.stamp = Set(Some(v));
    }
    if let Some(v) = patch.signature {
        model.signature = Set(Some(v));
    }
    if let Some(v) = patch.currency {
        model.currency = Set(v);
    }
    let mut new_country: Option<String> = None;
    if let Some(v) = patch.country {
        model.country = Set(Some(v.clone()));
        new_country = Some(v);
    }
    if let Some(v) = patch.vat_number {
        model.vat_number = Set(Some(v));
    }
    if let Some(v) = patch.default_tax_id {
        model.default_tax_id = Set(Some(v));
    }
    if let Some(v) = patch.invoice_number_prefix {
        model.invoice_number_prefix = Set(v);
    }

    // `printer` merges one level deep (`{ ...current.printer, ...patch.printer }`) — device keys
    // route to `device_delta` instead of the row (D-1/D-4).
    if let Some(p) = patch.printer {
        let mut printer = locked.printer.clone();
        if let Some(mode) = p.mode {
            printer.mode = match mode {
                PrinterMode::A4 => crate::entities::values::PrinterMode::A4,
                PrinterMode::Thermal => crate::entities::values::PrinterMode::Thermal,
            };
        }
        if let Some(w) = p.thermal_width_mm {
            printer.thermal_width_mm = w;
        }
        if let Some(t) = p.thermal {
            device_delta.thermal = Some(Some(thermal_config_from_dto(&t)));
        }
        if let Some(a) = p.a4_printer_name {
            device_delta.a4_printer_name = Some(Some(a));
        }
        if let Some(l) = p.label_printer_name {
            device_delta.label_printer_name = Some(Some(l));
        }
        if let Some(t) = p.a4_template {
            printer.a4_template = Some(t);
        }
        if let Some(t) = p.image_template {
            printer.image_template = Some(t);
        }
        model.printer = Set(printer);
    }

    if let Some(v) = patch.prices_include_tax {
        model.prices_include_tax = Set(v);
    }
    if let Some(v) = patch.address {
        model.address = Set(Some(v));
    }
    if let Some(v) = patch.national_address {
        model.national_address = Set(Some(v));
    }
    if let Some(v) = patch.phone {
        model.phone = Set(Some(v));
    }
    if let Some(v) = patch.commercial_register {
        model.commercial_register = Set(Some(v));
    }
    if let Some(v) = patch.receipt_footer {
        model.receipt_footer = Set(Some(v));
    }
    if let Some(v) = patch.accounting {
        model.accounting = Set(Some(crate::entities::values::AccountingPolicy {
            lock_date: v
                .lock_date
                .as_deref()
                .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            default_purchase_account_id: v.default_purchase_account_id,
        }));
    }

    // `backup` (whole object) → row `backup` without `folder`; `folder` (present or absent in the
    // new object) → `device.backup_folder` (D-1: the mock replaces the object wholesale, so an
    // absent `folder` clears the device field).
    if let Some(backup_value) = patch.backup {
        let folder = backup_value.get("folder").and_then(|v| v.as_str()).map(|s| s.to_string());
        device_delta.backup_folder = Some(folder);
        let mut row_backup = backup_value.clone();
        if let Some(obj) = row_backup.as_object_mut() {
            obj.remove("folder");
        }
        let parsed: crate::entities::values::BackupPolicy =
            serde_json::from_value(row_backup).map_err(|e| AppError::internal("تعذر حفظ إعدادات النسخ الاحتياطي", Some(e.to_string())))?;
        model.backup = Set(Some(parsed));
    }

    if let Some(v) = patch.inventory_approval_threshold {
        model.inventory_approval_threshold = Set(Some(v));
    }
    if let Some(v) = patch.role_access_overrides {
        model.role_access_overrides = Set(Some(v));
    }
    if let Some(v) = patch.insight_thresholds {
        model.insight_thresholds = Set(Some(crate::entities::values::InsightThresholds(v)));
    }
    if let Some(v) = patch.pos {
        model.pos = Set(Some(crate::entities::values::PosPolicy {
            override_price: v.override_price,
            sell_below_cost: v.sell_below_cost,
            require_open_shift: v.require_open_shift,
            foreign_cash_enabled: v.foreign_cash_enabled,
            foreign_currency: v.foreign_currency,
            foreign_currency_rate: v.foreign_currency_rate,
        }));
    }
    if let Some(v) = patch.sales {
        model.sales = Set(Some(crate::entities::values::SalesPolicy { refund_without_receipt: v.refund_without_receipt }));
    }
    if let Some(v) = patch.features {
        model.features = Set(Some(crate::entities::values::FeatureFlags { branches: v.branches, currencies: v.currencies, cost_centers: v.cost_centers }));
    }
    if let Some(v) = patch.onboarding {
        model.onboarding = Set(Some(crate::entities::values::OnboardingState {
            business_type: v.business_type,
            go_live_date: v.go_live_date.as_deref().and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
            completed_step: v.completed_step,
            skipped: v.skipped.unwrap_or_default(),
            done: v.done.unwrap_or_default(),
            finished_at: v.finished_at.as_deref().and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok()).map(|d| d.with_timezone(&chrono::Utc)),
            opening_entry_id: v.opening_entry_id,
            closing_entry_id: v.closing_entry_id,
            coa_template: v.coa_template,
        }));
    }

    // D-5: country changed -> also set timezone.
    if let Some(new_country) = &new_country {
        let tz = country_timezone(Some(new_country.as_str()));
        model.timezone = Set(tz.map(|t| t.to_string()));
    }

    model.updated_at = Set(cx.clock.now);

    let updated = model.update(conn).await.map_err(TxError::from)?;

    crate::shared::activity::log(conn, cx, registry, ActivityKind::Settings, "تحديث إعدادات المتجر", None, Some(RouteRef::list("settings-general")))
        .await?;

    Ok((updated, device_delta))
}
