//! Shared JSON value objects (21.02-B, owner B1, task B-5). Every struct here derives
//! `FromJsonQueryResult` (SeaORM's blanket impl for `serde`-round-trippable JSON column values) plus
//! `serde` with `rename_all = "camelCase"`, so a `JSON` column on any entity can hold one of these
//! directly as its Rust field type and round-trip identically to the TS shape it mirrors.
//!
//! **How B2 (and any later phase) should use this file:** add a `#[sea_orm(column_type = "Json")]`
//! field of one of these types on an entity (e.g. `pub address: Option<Address>` on `parties`), and
//! SeaORM/serde does the rest — no manual `serde_json::to_string`/`from_str` at the call site.
//! Don't add a new ad-hoc JSON struct elsewhere; if a column needs a JSON shape this file doesn't
//! have yet, add it here so it stays one inventory.

use std::collections::BTreeMap;

use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

use crate::utils::route::RouteRef;

// --- Address (doc 18.E core/types/address.ts `Address`) -----------------------------------------

/// Structured, country-aware address — mirrors `core/types/address.ts`'s `Address` field-for-field.
/// Used by `parties.structured_address`, `branches.national_address`, `settings.national_address`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub country: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub district_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub district_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_free_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city_free_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub district_free_text: Option<String>,

    // EG fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apartment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landmark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,

    // SA fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sa_building_no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sa_additional_no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sa_postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sa_unit_no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sa_short_address: Option<String>,
}

/// The deprecated `parties/types` `NationalAddress` shape — kept only so legacy/seeded rows that
/// still carry this exact shape (rather than the newer `Address`) round-trip unchanged. New writes
/// should prefer `Address`/`structured_address`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct NationalAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub district: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_no: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_address: Option<String>,
}

// --- RouteRef as a JSON column value -------------------------------------------------------------

/// `utils::route::RouteRef` wrapped so it can be used directly as a `JSON` column's Rust type
/// (`activity.link`, `audit.link`, `approval_requests.link`) — `RouteRef` itself stays in `utils`
/// (it's also used outside JSON columns, e.g. DTOs), this newtype is only the SeaORM JSON-column
/// bridge (`FromJsonQueryResult` can't be derived on a foreign type per orphan rules).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(transparent)]
pub struct RouteRefValue(pub RouteRef);

impl From<RouteRef> for RouteRefValue {
    fn from(r: RouteRef) -> Self {
        RouteRefValue(r)
    }
}

impl From<RouteRefValue> for RouteRef {
    fn from(v: RouteRefValue) -> Self {
        v.0
    }
}

// --- Settings sub-policies (settings/types/index.ts `StoreSettings`) -----------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrinterMode {
    A4,
    Thermal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrinterConnectionType {
    Windows,
    Network,
}

/// Device-scoped in the mock's own field, but the *shape* is shared with the frontend's
/// `ThermalPrinterSettings` — kept here for any JSON column that still needs to carry it
/// (cross-cutting.md §3 notes the live values move to `device-settings.json`, not the DB row; this
/// type exists so a settings JSON blob that embeds a last-known snapshot, or a test fixture, has a
/// single shape to agree on).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct ThermalPrinterSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub printer_name: Option<String>,
    pub connection: PrinterConnectionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    pub dpi: i32,
    pub cut: bool,
    pub open_drawer: bool,
    pub copies: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct PrinterSettings {
    pub mode: PrinterMode,
    pub thermal_width_mm: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal: Option<ThermalPrinterSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a4_printer_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_printer_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a4_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_template: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct AccountingPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_date: Option<chrono::NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_purchase_account_id: Option<crate::utils::id::Id>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct PosPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_price: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sell_below_cost: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_open_shift: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_cash_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_currency_rate: Option<rust_decimal::Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct SalesPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_without_receipt: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlags {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branches: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currencies: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_centers: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub go_live_date: Option<chrono::NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_step: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub done: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_entry_id: Option<crate::utils::id::Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closing_entry_id: Option<crate::utils::id::Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coa_template: Option<String>,
}

/// Sparse per-role area-access overrides — mirrors `users/helpers/permissions.ts`'s
/// `RoleAccessOverrides` (`Partial<Record<Role, Partial<Record<Area, Access>>>>`). Used directly by
/// `core::auth::RoleAccessOverrides` after B-8 loads `settings.role_access_overrides` (this JSON
/// shape stores plain lowercase string keys; `core::settings::load` converts them into the typed
/// `HashMap<Role, HashMap<Area, Access>>` the auth module expects, tolerating an unknown key by
/// skipping it rather than failing the whole settings load).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(transparent)]
pub struct RoleAccessOverrides(pub BTreeMap<String, BTreeMap<String, String>>);

/// Sparse dashboard-insight threshold overrides — stored as a JSON object of `snake/camelCase`
/// threshold name → numeric value; the insight engine (frontend-owned) already tolerates unknown/
/// missing keys, so this is deliberately an open map rather than a fixed struct. Values are
/// `Decimal` (serialized as JSON numbers) — master plan rule 5 allows no f32/f64 in entities, and
/// `architecture_rules` enforces it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(transparent)]
pub struct InsightThresholds(pub BTreeMap<String, JsonDecimal>);

/// A `Decimal` that crosses JSON as a plain number (P2-33), for use inside JSON value objects
/// where a field-level `#[serde(with = …)]` can't reach (map values, list items).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonDecimal(#[serde(with = "crate::utils::money::serde_number")] pub rust_decimal::Decimal);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct BackupPolicy {
    pub auto_enabled: bool,
    /// "HH:mm", 24h.
    pub auto_time: String,
    pub retention: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_auto_run_date: Option<chrono::NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup_failed_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup_error: Option<String>,
}

// --- Audit field diffs (diagnostics/types `AuditEntry.before`/`after`) ---------------------------

/// One changed field in an audit row's before/after — `audit.before`/`audit.after` each hold a
/// `Vec<AuditFieldDiff>` (or the whole snapshot may be a single opaque JSON object; both
/// `before`/`after` columns are typed `Option<serde_json::Value>` on the entity itself since the
/// mock's `AuditEntry.before`/`after` are `Record<string, unknown>` snapshots, not a pre-computed
/// diff list — this struct is provided for call sites that *do* want to store/compute a diff list
/// rather than two full snapshots).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct AuditFieldDiff {
    pub field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(transparent)]
pub struct AuditFieldDiffs(pub Vec<AuditFieldDiff>);

// --- Held-sale cart payload (invoices/types `HeldSale`) ------------------------------------------

/// The parked POS cart: stored as opaque JSON (P2-18) because its shape mirrors the POS page's own
/// working cart state 1:1 and has no ledger/report reader that needs individual fields indexed —
/// unlike invoice lines, nothing ever queries "held sales with a line for product X." Kept as
/// `serde_json::Value` rather than a fully-typed struct so a POS-side cart shape change never needs
/// a migration; `held_sales.cart` is `JSON NOT NULL` on the entity.
pub type HeldSaleCart = serde_json::Value;

// --- Attachment id lists (product.imageIds, journalEntry.attachmentIds, …) -----------------------

/// An ordered list of attachment ids (`AttachmentField`'s `ownerRef`-keyed store) — used by any
/// column that is a TS `string[]` of ids in display order (`products.image_ids`,
/// `journal_entries.attachment_ids`, `products.tags`, `party.tags`, etc. all share this same
/// "ordered list of strings" JSON shape). A plain `Vec<String>` newtype rather than reusing
/// `Vec<String>` directly, since a bare `Vec<String>` has no single blanket `FromJsonQueryResult`
/// impl to hang the JSON column type off in every entity file without repeating the derive.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(transparent)]
pub struct StringList(pub Vec<String>);

impl From<Vec<String>> for StringList {
    fn from(v: Vec<String>) -> Self {
        StringList(v)
    }
}

impl From<StringList> for Vec<String> {
    fn from(v: StringList) -> Self {
        v.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_round_trips_camel_case() {
        let addr = Address {
            country: "EG".to_string(),
            region_id: Some("cairo".to_string()),
            region_name: None,
            city_id: None,
            city_name: None,
            district_id: None,
            district_name: None,
            region_free_text: None,
            city_free_text: None,
            district_free_text: None,
            street: Some("شارع النصر".to_string()),
            building_no: Some("12".to_string()),
            floor: None,
            apartment: None,
            landmark: None,
            postal_code: None,
            sa_building_no: None,
            sa_additional_no: None,
            sa_postal_code: None,
            sa_unit_no: None,
            sa_short_address: None,
        };
        let json = serde_json::to_string(&addr).unwrap();
        assert!(json.contains("\"regionId\":\"cairo\""));
        assert!(json.contains("\"buildingNo\":\"12\""));
        let back: Address = serde_json::from_str(&json).unwrap();
        assert_eq!(back, addr);
    }

    #[test]
    fn role_access_overrides_round_trip() {
        let mut inner = BTreeMap::new();
        inner.insert("reports".to_string(), "read".to_string());
        let mut outer = BTreeMap::new();
        outer.insert("cashier".to_string(), inner);
        let overrides = RoleAccessOverrides(outer);
        let json = serde_json::to_string(&overrides).unwrap();
        let back: RoleAccessOverrides = serde_json::from_str(&json).unwrap();
        assert_eq!(back, overrides);
    }

    #[test]
    fn string_list_round_trips() {
        let list = StringList(vec!["a".to_string(), "b".to_string()]);
        let json = serde_json::to_string(&list).unwrap();
        assert_eq!(json, r#"["a","b"]"#);
        let back: StringList = serde_json::from_str(&json).unwrap();
        assert_eq!(back, list);
    }
}
