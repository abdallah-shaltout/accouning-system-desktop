//! Step 3.2.8 (`03-domains/00-import.md`): builds and inserts the single `settings` row from the
//! snapshot's `data.settings`, plus the device-scoped printer/backup-folder fields the command
//! writes to `device-settings.json` after commit (cross-cutting.md §3's branch/device split).

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::entities::org::settings::ActiveModel as SettingsActiveModel;
use crate::entities::values::{AccountingPolicy, OnboardingState};
use crate::infrastructure::import::idmap::IdMap;
use crate::infrastructure::import::model::SettingsV1;
use crate::utils::id::Id;

/// The device-scoped fields carried out of `data.settings` for the command to write to
/// `device-settings.json` after commit (§3.2.8: "not written to the row... returned in
/// `ImportReport.device_fields`"). Kept as raw `Option<String>`/JSON rather than the typed
/// `core::device::DeviceSettings` shape, since only mode = legacy ever uses this and the command
/// layer (which already owns `core::device::load`/`save`) does the actual merge.
#[derive(Debug, Clone, Default)]
pub struct DeviceFields {
    pub thermal_printer_name: Option<String>,
    pub a4_printer_name: Option<String>,
    pub label_printer_name: Option<String>,
    pub backup_folder: Option<String>,
}

/// Builds the `settings` row's `ActiveModel` and inserts it. Returns `(default_branch_id,
/// device_fields)`. `resolve_branch(old_id) -> Option<Id>` and `resolve_id(old_id) -> Option<Id>`
/// are small closures over the caller's `IdMap` so this module never needs to know how branches
/// were keyed — it only asks "does this old id resolve, and if so to what."
pub async fn insert_settings<C: ConnectionTrait>(
    conn: &C,
    id_map: &IdMap,
    settings: &SettingsV1,
    tz_name: Option<&str>,
    branch_old_ids_in_order: &[String],
) -> TxResult<(Id, DeviceFields, i64)> {
    let mut rounded = 0i64;

    // P2-20/D-10: default_branch_id = remapped 'branch-main' when that id exists in the snapshot,
    // else the first branch in array order. No branch at all is a hard validation error — a
    // settings row with no resolvable branch can never satisfy the schema's NOT NULL FK.
    let default_branch_id = if let Some(id) = id_map.resolve("branch-main") {
        id
    } else if let Some(first_old) = branch_old_ids_in_order.first() {
        id_map.resolve(first_old).ok_or_else(|| {
            TxError::App(AppError::internal("تعذر تحديد الفرع الافتراضي أثناء الاستيراد", None))
        })?
    } else {
        return Err(TxError::App(AppError::validation("لا يوجد فرع في البيانات المستوردة")));
    };

    let currency = {
        let raw = settings.currency.as_deref().unwrap_or("").trim();
        if raw.is_empty() {
            "EGP".to_string()
        } else {
            raw.to_uppercase()
        }
    };

    let default_tax_id = settings.default_tax_id.as_deref().and_then(|old| id_map.resolve(old));

    let accounting = settings.accounting.as_ref().map(|a| AccountingPolicy {
        lock_date: a.lock_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        default_purchase_account_id: a.default_purchase_account_id.as_deref().and_then(|old| id_map.resolve(old)),
    });

    // `openingEntryId`/`closingEntryId` are JSON ids (no FK) to journal entries inserted *after*
    // settings in IMPORT_ORDER: `resolve_or_mint` pre-assigns the id `insert_journal_entries` then
    // reuses (plain `resolve` always missed and dropped the onboarding links).
    let onboarding = settings.onboarding.as_ref().map(|o| OnboardingState {
        business_type: o.business_type.clone(),
        go_live_date: o.go_live_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        completed_step: o.completed_step,
        skipped: o.skipped.clone(),
        done: o.done.clone(),
        finished_at: o.finished_at.as_deref().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|d| d.with_timezone(&chrono::Utc)),
        opening_entry_id: o.opening_entry_id.as_deref().filter(|s| !s.is_empty()).map(|old| id_map.resolve_or_mint(old)),
        closing_entry_id: o.closing_entry_id.as_deref().filter(|s| !s.is_empty()).map(|old| id_map.resolve_or_mint(old)),
        coa_template: o.coa_template.clone(),
    });

    let printer_settings = crate::entities::values::PrinterSettings {
        mode: match settings.printer.mode.as_str() {
            "thermal" => crate::entities::values::PrinterMode::Thermal,
            _ => crate::entities::values::PrinterMode::A4,
        },
        thermal_width_mm: settings.printer.thermal_width_mm.unwrap_or(80),
        // Device-scoped sub-fields are never written into the branch-DB row (cross-cutting §3) —
        // the row's own `printer` JSON only ever carries the branch-scoped mode/width/template
        // fields; the device-scoped ones travel out via `DeviceFields` below instead.
        thermal: None,
        a4_printer_name: None,
        label_printer_name: None,
        a4_template: settings.printer.a4_template.clone(),
        image_template: settings.printer.image_template.clone(),
    };

    let mut device_fields = DeviceFields::default();
    if let Some(thermal) = &settings.printer.thermal {
        device_fields.thermal_printer_name =
            thermal.get("printerName").and_then(|v| v.as_str()).map(|s| s.to_string());
    }
    device_fields.a4_printer_name = settings.printer.a4_printer_name.clone();
    device_fields.label_printer_name = settings.printer.label_printer_name.clone();
    if let Some(backup) = &settings.backup {
        device_fields.backup_folder = backup.get("folder").and_then(|v| v.as_str()).map(|s| s.to_string());
    }

    // Architecture rule 1 (no f32/f64 for money): converted through the shortest-round-trip-text
    // path (`model::num_to_decimal`), never `serde_json::Value::as_f64`.
    let inventory_approval_threshold = settings
        .inventory_approval_threshold
        .as_ref()
        .and_then(|v| crate::infrastructure::import::model::num_to_decimal(v).ok())
        .map(|d| {
            let r = crate::utils::money::round2(d);
            if r != d {
                rounded += 1;
            }
            r
        });

    let role_access_overrides = settings
        .role_access_overrides
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let insight_thresholds = settings.insight_thresholds.as_ref().and_then(|v| serde_json::from_value(v.clone()).ok());
    let pos = settings.pos.as_ref().and_then(|v| serde_json::from_value(v.clone()).ok());
    let sales = settings.sales.as_ref().and_then(|v| serde_json::from_value(v.clone()).ok());
    let features = settings.features.as_ref().and_then(|v| serde_json::from_value(v.clone()).ok());
    let national_address = settings.national_address.as_ref().and_then(|v| serde_json::from_value(v.clone()).ok());
    let backup_policy = settings.backup.as_ref().and_then(|v| serde_json::from_value(v.clone()).ok());

    let now = chrono::Utc::now();
    let model = SettingsActiveModel {
        id: Set(Id::new()),
        singleton: Set(1),
        store_name: Set(settings.store_name.clone()),
        logo: Set(settings.logo.clone()),
        stamp: Set(settings.stamp.clone()),
        signature: Set(settings.signature.clone()),
        currency: Set(currency),
        country: Set(settings.country.clone()),
        vat_number: Set(settings.vat_number.clone()),
        default_tax_id: Set(default_tax_id),
        invoice_number_prefix: Set(settings.invoice_number_prefix.clone().unwrap_or_else(|| "INV-".to_string())),
        printer: Set(printer_settings),
        prices_include_tax: Set(settings.prices_include_tax.unwrap_or(true)),
        address: Set(settings.address.clone()),
        national_address: Set(national_address),
        phone: Set(settings.phone.clone()),
        commercial_register: Set(settings.commercial_register.clone()),
        receipt_footer: Set(settings.receipt_footer.clone()),
        accounting: Set(accounting),
        backup: Set(backup_policy),
        inventory_approval_threshold: Set(inventory_approval_threshold),
        role_access_overrides: Set(role_access_overrides),
        insight_thresholds: Set(insight_thresholds),
        pos: Set(pos),
        sales: Set(sales),
        features: Set(features),
        onboarding: Set(onboarding),
        timezone: Set(tz_name.map(|s| s.to_string())),
        default_branch_id: Set(default_branch_id),
        created_at: Set(now),
        updated_at: Set(now),
    };

    model.insert(conn).await.map_err(TxError::from)?;

    Ok((default_branch_id, device_fields, rounded))
}
