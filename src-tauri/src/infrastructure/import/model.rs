//! The frozen `SCHEMA_VERSION = 1` reader model (`03-domains/00-import.md` §2, D-2): one private
//! serde struct per `MockDb` table (`src/mocks/db.ts`), camelCase, `#[serde(default)]` on every
//! optional TS field, unknown fields ignored (`#[serde(default)]` at the struct level is not needed
//! since serde ignores unknown fields by default — only *missing* fields need `default`).
//!
//! Deliberately **not** the domains' DTOs (D-2): this is the frozen v1 file format and must keep
//! reading old snapshots after domain DTOs evolve. Every id field is a plain `String` (the mock's
//! own `"prefix-123"` ids) — remapping happens in `idmap`, never here. Money/qty/cost/rate fields
//! are `serde_json::Number`, converted to `Decimal` through the number's shortest text
//! (`Decimal::from_str(&n.to_string())`) by the small helper below — never through `f64`
//! (architecture rule 1: no `f32`/`f64` anywhere money touches).
//!
//! Field coverage is deliberately generous (every field the mock type can carry) rather than
//! minimal, since a frozen reader model that silently drops a field would silently lose data on
//! import — `serde_json::Value` is used for JSON sub-shapes this importer only ever copies verbatim
//! (never inspects field-by-field), which also means a mock type gaining a new nested field never
//! breaks this reader.

use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

/// Converts a `serde_json::Number` (or `null`) into a `Decimal` via its shortest round-trip text —
/// never via `f64` arithmetic (architecture rule 1). `null`/missing becomes `Decimal::ZERO` for a
/// required numeric field, or the caller wraps this in `Option` for an optional one.
pub fn num_to_decimal(v: &Value) -> Result<Decimal, String> {
    match v {
        Value::Number(n) => Decimal::from_str(&n.to_string()).map_err(|e| e.to_string()),
        Value::Null => Ok(Decimal::ZERO),
        Value::String(s) => Decimal::from_str(s).map_err(|e| e.to_string()),
        other => Err(format!("expected a JSON number, got {other:?}")),
    }
}

/// A `Decimal` deserialized from a raw JSON number via `num_to_decimal` — the one field-level
/// wrapper every money/qty/cost/rate field in this model uses instead of relying on
/// `rust_decimal::Decimal`'s own (float-based) serde impl.
pub mod decimal_field {
    use super::*;

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Decimal, D::Error> {
        let v = Value::deserialize(deserializer)?;
        num_to_decimal(&v).map_err(serde::de::Error::custom)
    }

    pub fn serialize<S: serde::Serializer>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error> {
        crate::utils::money::serde_number::serialize(value, serializer)
    }

    pub mod option {
        use super::*;

        pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<Decimal>, D::Error> {
            let v = Option::<Value>::deserialize(deserializer)?;
            match v {
                None | Some(Value::Null) => Ok(None),
                Some(v) => num_to_decimal(&v).map(Some).map_err(serde::de::Error::custom),
            }
        }

        pub fn serialize<S: serde::Serializer>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error> {
            crate::utils::money::serde_number::option::serialize(value, serializer)
        }
    }
}

// No `default_true`: every boolean flag below (`active`, `canDelete`, `allowManual`, `showInPos`,
// `showInPayments`) is read *truthily* by the mock (`if (!user.active)`, `.filter((b) => b.active)`,
// `if (!method.canDelete)` — never `!== false`), so a snapshot row that omits the key behaves as
// `false` there, and the importer must store `false` too (plan 21 Part 04 Wave 2, L1
// `import/import-edge` `login-inactive-admin`: the edge fixture's admin has no `active` key, the
// mock refuses the login, Rust used to import it as active and let it in).

/// Parses an ISO-8601 instant string (`createdAt`/`updatedAt`, always a full instant in the mock,
/// never a bare date) into a UTC `DateTime` — `None` on any parse failure, so callers can fall back
/// to the D-8 synthetic timestamp rather than failing the whole import over a cosmetic metadata
/// field.
pub fn parse_instant(raw: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(raw).ok().map(|d| d.with_timezone(&chrono::Utc))
}

// --- Envelope -------------------------------------------------------------------------------------

/// `persist.ts:20-24`'s `Snapshot { version, savedAt, data }`.
#[derive(Debug, Clone, Deserialize)]
pub struct SnapshotV1Envelope {
    pub version: i64,
    #[serde(default, rename = "savedAt")]
    pub saved_at: Option<String>,
    pub data: MockDbV1,
}

/// `MockDb` (`src/mocks/db.ts:32-124`) — every field optional/defaulted at this layer so a snapshot
/// missing a table (an old dev fixture, a hand-edited edge-case file) still parses; the importer's
/// own logic decides what an absent table means (usually: nothing to import for it).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MockDbV1 {
    pub users: Vec<UserV1>,
    pub credentials: HashMap<String, String>,
    pub accounts: Vec<AccountV1>,
    pub journal_entries: Vec<JournalEntryV1>,
    pub journal_drafts: Vec<JournalEntryV1>,
    pub journal_templates: Vec<JournalTemplateV1>,
    pub fiscal_years: Vec<FiscalYearV1>,
    pub categories: Vec<CategoryV1>,
    pub units: Vec<UnitV1>,
    pub price_lists: Vec<PriceListV1>,
    pub products: Vec<ProductV1>,
    pub stock_adjustments: Vec<StockAdjustmentV1>,
    pub stock_movements: Vec<StockMovementV1>,
    pub product_batches: Vec<ProductBatchV1>,
    pub custom_field_defs: Vec<CustomFieldDefV1>,
    pub stock_counts: Vec<StockCountV1>,
    pub debit_note_drafts: Vec<DebitNoteDraftV1>,
    pub customers: Vec<PartyV1>,
    pub suppliers: Vec<PartyV1>,
    pub party_groups: Vec<PartyGroupV1>,
    pub party_history: Vec<PartyHistoryV1>,
    pub invoices: Vec<InvoiceV1>,
    pub refunds: Vec<RefundV1>,
    pub quotations: Vec<QuotationV1>,
    pub held_sales: Vec<HeldSaleV1>,
    pub shifts: Vec<ShiftV1>,
    pub purchase_orders: Vec<PurchaseOrderV1>,
    pub purchase_returns: Vec<PurchaseReturnV1>,
    pub payments: Vec<PaymentV1>,
    pub taxes: Vec<TaxV1>,
    pub payment_methods: Vec<PaymentMethodV1>,
    pub settings: SettingsV1,
    pub activity: Vec<ActivityEntryV1>,
    pub counters: HashMap<String, i64>,
    pub expense_categories: Vec<ExpenseCategoryV1>,
    pub expenses: Vec<ExpenseV1>,
    pub recurring_expenses: Vec<RecurringExpenseV1>,
    pub vouchers: Vec<VoucherV1>,
    pub card_settlements: Vec<CardSettlementV1>,
    pub branches: Vec<BranchV1>,
    pub cost_centers: Vec<CostCenterV1>,
    pub stock_transfers: Vec<StockTransferV1>,
    pub currencies: Vec<CurrencyV1>,
    pub exchange_rates: Vec<ExchangeRateV1>,
    pub approval_requests: Vec<ApprovalRequestV1>,
    pub audit: Vec<AuditEntryV1>,
}

/// `backupArchive.ts:63-70`'s `tableCounts()` key order (array tables in `MockDb` declaration
/// order, then `attachments` last) — the one place this order is defined, so `inspect`/`import`
/// both build their `OrderedCounts` by iterating this exact list rather than re-deriving it.
pub const TABLE_COUNT_KEYS: &[&str] = &[
    "users",
    "accounts",
    "journalEntries",
    "journalDrafts",
    "journalTemplates",
    "fiscalYears",
    "categories",
    "units",
    "priceLists",
    "products",
    "stockAdjustments",
    "stockMovements",
    "productBatches",
    "customFieldDefs",
    "stockCounts",
    "debitNoteDrafts",
    "customers",
    "suppliers",
    "partyGroups",
    "partyHistory",
    "invoices",
    "refunds",
    "quotations",
    "heldSales",
    "shifts",
    "purchaseOrders",
    "purchaseReturns",
    "payments",
    "taxes",
    "paymentMethods",
    "activity",
    "expenseCategories",
    "expenses",
    "recurringExpenses",
    "vouchers",
    "cardSettlements",
    "branches",
    "costCenters",
    "stockTransfers",
    "currencies",
    "exchangeRates",
    "approvalRequests",
    "audit",
];

impl MockDbV1 {
    /// The length of the array table named `key` (one of `TABLE_COUNT_KEYS`), or `None` for an
    /// unrecognized key — used by both `inspect`'s pre-import counts and `import`'s post-import
    /// counts (the latter reads real `COUNT(*)`s instead, see `run.rs`).
    pub fn table_len(&self, key: &str) -> Option<i64> {
        let n = match key {
            "users" => self.users.len(),
            "accounts" => self.accounts.len(),
            "journalEntries" => self.journal_entries.len(),
            "journalDrafts" => self.journal_drafts.len(),
            "journalTemplates" => self.journal_templates.len(),
            "fiscalYears" => self.fiscal_years.len(),
            "categories" => self.categories.len(),
            "units" => self.units.len(),
            "priceLists" => self.price_lists.len(),
            "products" => self.products.len(),
            "stockAdjustments" => self.stock_adjustments.len(),
            "stockMovements" => self.stock_movements.len(),
            "productBatches" => self.product_batches.len(),
            "customFieldDefs" => self.custom_field_defs.len(),
            "stockCounts" => self.stock_counts.len(),
            "debitNoteDrafts" => self.debit_note_drafts.len(),
            "customers" => self.customers.len(),
            "suppliers" => self.suppliers.len(),
            "partyGroups" => self.party_groups.len(),
            "partyHistory" => self.party_history.len(),
            "invoices" => self.invoices.len(),
            "refunds" => self.refunds.len(),
            "quotations" => self.quotations.len(),
            "heldSales" => self.held_sales.len(),
            "shifts" => self.shifts.len(),
            "purchaseOrders" => self.purchase_orders.len(),
            "purchaseReturns" => self.purchase_returns.len(),
            "payments" => self.payments.len(),
            "taxes" => self.taxes.len(),
            "paymentMethods" => self.payment_methods.len(),
            "activity" => self.activity.len(),
            "expenseCategories" => self.expense_categories.len(),
            "expenses" => self.expenses.len(),
            "recurringExpenses" => self.recurring_expenses.len(),
            "vouchers" => self.vouchers.len(),
            "cardSettlements" => self.card_settlements.len(),
            "branches" => self.branches.len(),
            "costCenters" => self.cost_centers.len(),
            "stockTransfers" => self.stock_transfers.len(),
            "currencies" => self.currencies.len(),
            "exchangeRates" => self.exchange_rates.len(),
            "approvalRequests" => self.approval_requests.len(),
            "audit" => self.audit.len(),
            _ => return None,
        };
        Some(n as i64)
    }
}

// --- users / credentials --------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserV1 {
    pub id: String,
    pub username: String,
    pub name: String,
    #[serde(default)]
    pub phone: Option<String>,
    pub role: String,
    #[serde(default, with = "decimal_field")]
    pub max_discount: Decimal,
    #[serde(default)]
    pub price_list_id: Option<String>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub allowed_branches: Option<Vec<String>>,
    #[serde(default)]
    pub home_branch: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- accounts --------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountV1 {
    pub id: String,
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub name_en: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub is_group: bool,
    pub kind: String,
    pub subtype: String,
    pub normal_side: String,
    #[serde(default)]
    pub system_role: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub requires_party: Option<bool>,
    #[serde(default)]
    pub allow_manual: bool,
    #[serde(default)]
    pub requires_cost_center: Option<bool>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub can_delete: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- journal ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SourceRefV1 {
    pub kind: Option<String>,
    pub id: Option<String>,
    pub number: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalLineV1 {
    #[serde(default)]
    pub id: Option<String>,
    pub account_id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, with = "decimal_field")]
    pub debit: Decimal,
    #[serde(default, with = "decimal_field")]
    pub credit: Decimal,
    #[serde(default)]
    pub party_kind: Option<String>,
    #[serde(default)]
    pub party_id: Option<String>,
    #[serde(default)]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub cost_center_id: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub amount_fc: Option<Decimal>,
    #[serde(default, with = "decimal_field::option")]
    pub rate: Option<Decimal>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntryV1 {
    pub id: String,
    #[serde(default)]
    pub number: Option<String>,
    pub date: String,
    pub description: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub source_ref: Option<SourceRefV1>,
    #[serde(default, with = "decimal_field")]
    pub total_debit: Decimal,
    #[serde(default, with = "decimal_field")]
    pub total_credit: Decimal,
    #[serde(default)]
    pub reversed: bool,
    #[serde(default)]
    pub reversal_of_id: Option<String>,
    #[serde(default)]
    pub reversal_reason: Option<String>,
    pub created_by: String,
    #[serde(default)]
    pub posted_by: Option<String>,
    #[serde(default)]
    pub posted_at: Option<String>,
    #[serde(default)]
    pub attachment_ids: Option<Vec<String>>,
    #[serde(default)]
    pub template_id: Option<String>,
    #[serde(default)]
    pub lines: Vec<JournalLineV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalTemplateV1 {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub lines: Value,
    #[serde(default)]
    pub recurrence: Option<Value>,
    pub created_by: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiscalYearV1 {
    pub id: String,
    pub name: String,
    pub start_date: String,
    pub end_date: String,
    #[serde(default)]
    pub is_closed: bool,
    #[serde(default)]
    pub closing_entry_id: Option<String>,
    #[serde(default)]
    pub closed_at: Option<String>,
    #[serde(default)]
    pub closed_by: Option<String>,
    #[serde(default)]
    pub budgets: Vec<CostCenterBudgetV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- catalog ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryV1 {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub purchase_account_id: Option<String>,
    #[serde(default)]
    pub revenue_account_id: Option<String>,
    #[serde(default)]
    pub cogs_account_id: Option<String>,
    #[serde(default)]
    pub sale_tax_id: Option<String>,
    #[serde(default)]
    pub purchase_tax_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitV1 {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub allows_decimals: Option<bool>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceListV1 {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub values: Vec<PriceListValueV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// One entry of TS `Product.prices` (`{ priceListId, value }`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPriceV1 {
    pub price_list_id: String,
    #[serde(with = "decimal_field")]
    pub value: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceListValueV1 {
    pub product_id: String,
    #[serde(default)]
    pub unit_id: Option<String>,
    #[serde(with = "decimal_field")]
    pub value: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductUnitV1 {
    #[serde(default)]
    pub id: Option<String>,
    pub unit_id: String,
    #[serde(with = "decimal_field")]
    pub factor: Decimal,
    #[serde(default)]
    pub barcodes: Vec<String>,
    #[serde(with = "decimal_field")]
    pub price: Decimal,
    #[serde(default)]
    pub price_is_auto: bool,
    #[serde(default)]
    pub default_for_sale: bool,
    #[serde(default)]
    pub default_for_purchase: bool,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductUnitPriceV1 {
    pub price_list_id: String,
    pub unit_id: String,
    #[serde(with = "decimal_field")]
    pub value: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchStockEntryV1 {
    #[serde(with = "decimal_field")]
    pub qty: Decimal,
    #[serde(with = "decimal_field")]
    pub value: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductV1 {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub name_en: Option<String>,
    pub sku: String,
    #[serde(default)]
    pub barcode: Option<String>,
    #[serde(default)]
    pub category_id: Option<String>,
    #[serde(default)]
    pub unit_id: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub stock_mode: Option<String>,
    #[serde(default, with = "decimal_field")]
    pub cost_price: Decimal,
    #[serde(default, with = "decimal_field")]
    pub price: Decimal,
    #[serde(default, with = "decimal_field")]
    pub stock_qty: Decimal,
    #[serde(default, with = "decimal_field::option")]
    pub min_stock: Option<Decimal>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub purchase_account_id: Option<String>,
    #[serde(default, with = "decimal_field")]
    pub stock_value: Decimal,
    #[serde(default)]
    pub stock_by_branch: Option<HashMap<String, BranchStockEntryV1>>,
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub image_ids: Option<Vec<String>>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub units: Option<Vec<ProductUnitV1>>,
    #[serde(default)]
    pub unit_prices: Option<Vec<ProductUnitPriceV1>>,
    /// TS `Product.prices` — the base-unit price per price list (`products/types/index.ts`);
    /// becomes `product_prices` rows with `unit_id = NULL` (the TS `PriceList` itself carries no
    /// values).
    #[serde(default)]
    pub prices: Option<Vec<ProductPriceV1>>,
    #[serde(default, with = "decimal_field::option")]
    pub min_price: Option<Decimal>,
    #[serde(default)]
    pub sale_tax_id: Option<String>,
    #[serde(default)]
    pub purchase_tax_id: Option<String>,
    #[serde(default)]
    pub revenue_account_id: Option<String>,
    #[serde(default)]
    pub cogs_account_id: Option<String>,
    #[serde(default)]
    pub allow_negative_stock: Option<bool>,
    #[serde(default)]
    pub shelf_location: Option<String>,
    #[serde(default)]
    pub preferred_supplier_id: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub reorder_qty: Option<Decimal>,
    #[serde(default)]
    pub track_batches: Option<bool>,
    #[serde(default)]
    pub expiry_alert_days: Option<i32>,
    #[serde(default)]
    pub warranty_months: Option<i32>,
    #[serde(default)]
    pub warranty_provider: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub weight: Option<Decimal>,
    #[serde(default)]
    pub custom_fields: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFieldDefV1 {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub options: Option<Vec<String>>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub sort_order: i16,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- inventory -------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductBatchV1 {
    pub id: String,
    pub product_id: String,
    pub batch_no: String,
    #[serde(default)]
    pub expiry_date: Option<String>,
    #[serde(with = "decimal_field")]
    pub qty: Decimal,
    #[serde(with = "decimal_field")]
    pub unit_cost: Decimal,
    #[serde(default)]
    pub supplier_id: Option<String>,
    pub received_date: String,
    #[serde(default)]
    pub source_ref_id: Option<String>,
    #[serde(default)]
    pub source_ref_number: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockAdjustmentLineV1 {
    pub product_id: String,
    #[serde(default, with = "decimal_field::option")]
    pub system_qty: Option<Decimal>,
    #[serde(default, with = "decimal_field::option")]
    pub counted_qty: Option<Decimal>,
    #[serde(with = "decimal_field")]
    pub qty_change: Decimal,
    #[serde(default, with = "decimal_field::option")]
    pub unit_cost: Option<Decimal>,
    #[serde(default)]
    pub batch_no: Option<String>,
    #[serde(default)]
    pub expiry_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockAdjustmentV1 {
    pub id: String,
    pub number: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub date: String,
    pub status: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub offset_account_id: Option<String>,
    #[serde(default)]
    pub approved_by: Option<String>,
    #[serde(default)]
    pub approved_at: Option<String>,
    #[serde(default)]
    pub lines: Vec<StockAdjustmentLineV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockMovementV1 {
    pub id: String,
    pub date: String,
    pub product_id: String,
    #[serde(with = "decimal_field")]
    pub qty_change: Decimal,
    #[serde(with = "decimal_field")]
    pub value_change: Decimal,
    pub reason: String,
    pub ref_id: String,
    #[serde(default)]
    pub ref_number: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub balance_after: Option<Decimal>,
    #[serde(default)]
    pub batch_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockCountLineV1 {
    pub product_id: String,
    #[serde(with = "decimal_field")]
    pub system_qty: Decimal,
    #[serde(default, with = "decimal_field::option")]
    pub counted_qty: Option<Decimal>,
    #[serde(with = "decimal_field")]
    pub unit_cost: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockCountV1 {
    pub id: String,
    pub number: String,
    pub status: String,
    pub scope: String,
    #[serde(default)]
    pub category_id: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub blind: bool,
    pub started_at: String,
    pub started_by: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub adjustment_id: Option<String>,
    #[serde(default)]
    pub lines: Vec<StockCountLineV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebitNoteDraftLineV1 {
    pub product_id: String,
    pub batch_id: String,
    #[serde(with = "decimal_field")]
    pub qty: Decimal,
    #[serde(with = "decimal_field")]
    pub unit_cost: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebitNoteDraftV1 {
    pub id: String,
    pub number: String,
    pub supplier_id: String,
    pub date: String,
    #[serde(default)]
    pub lines: Vec<DebitNoteDraftLineV1>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockTransferLineV1 {
    pub product_id: String,
    #[serde(with = "decimal_field")]
    pub qty: Decimal,
    #[serde(default)]
    pub unit_id: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub unit_factor: Option<Decimal>,
    #[serde(default)]
    pub batch_id: Option<String>,
    #[serde(default)]
    pub batch_no: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub received_qty: Option<Decimal>,
    #[serde(default, with = "decimal_field::option")]
    pub unit_cost: Option<Decimal>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockTransferV1 {
    pub id: String,
    pub number: String,
    pub from_branch_id: String,
    pub to_branch_id: String,
    pub status: String,
    pub date: String,
    #[serde(default)]
    pub sent_at: Option<String>,
    #[serde(default)]
    pub received_at: Option<String>,
    #[serde(default)]
    pub rejected_at: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub sent_by: Option<String>,
    #[serde(default)]
    pub received_by: Option<String>,
    #[serde(default)]
    pub rejected_by: Option<String>,
    #[serde(default)]
    pub reject_reason: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub shortage_value: Option<Decimal>,
    #[serde(default)]
    pub lines: Vec<StockTransferLineV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- parties ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyPhoneV1 {
    pub label: String,
    pub number: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyOpeningBalanceV1 {
    #[serde(default, with = "decimal_field::option")]
    pub amount: Option<Decimal>,
    #[serde(default)]
    pub side: Option<String>,
    #[serde(default)]
    pub as_of_date: Option<String>,
    #[serde(default)]
    pub journal_entry_id: Option<String>,
    #[serde(default)]
    pub locked: Option<bool>,
}

/// Covers both `Customer` and `Supplier` (P2-15's single `parties` table) — fields specific to one
/// kind (`creditLimit` customer-only, `contactPerson` supplier-only) are simply absent/null on rows
/// of the other kind, exactly like the mock's own two distinct TS types sharing most fields.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyV1 {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub name_en: Option<String>,
    pub code: String,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub contacts: Option<Value>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub national_address: Option<Value>,
    #[serde(default)]
    pub structured_address: Option<Value>,
    #[serde(default)]
    pub vat_number: Option<String>,
    #[serde(default)]
    pub cr_number: Option<String>,
    #[serde(default)]
    pub national_id: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub price_list_id: Option<String>,
    #[serde(default)]
    pub payment_terms_days: Option<i32>,
    #[serde(default)]
    pub salesperson_id: Option<String>,
    #[serde(default)]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub bank: Option<Value>,
    #[serde(default)]
    pub opening_balance: Option<PartyOpeningBalanceV1>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub linked_party_id: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub credit_limit: Option<Decimal>,
    #[serde(default)]
    pub contact_person: Option<String>,
    #[serde(default)]
    pub default_expense_account_id: Option<String>,
    #[serde(default)]
    pub phones: Vec<PartyPhoneV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyGroupV1 {
    pub id: String,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub price_list_id: Option<String>,
    #[serde(default)]
    pub payment_terms_days: Option<i32>,
    #[serde(default, with = "decimal_field::option")]
    pub discount_percent: Option<Decimal>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyHistoryV1 {
    pub id: String,
    pub party_id: String,
    pub party_kind: String,
    pub date: String,
    pub message: String,
    pub user_id: String,
    #[serde(default)]
    pub created_at: Option<String>,
}

// --- sales -----------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineTaxV1 {
    #[serde(default)]
    pub tax_id: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub rate: Option<Decimal>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceLineV1 {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub product_id: Option<String>,
    pub name: String,
    #[serde(with = "decimal_field")]
    pub qty: Decimal,
    #[serde(with = "decimal_field")]
    pub price: Decimal,
    #[serde(default, with = "decimal_field")]
    pub cost_price: Decimal,
    #[serde(default, with = "decimal_field")]
    pub discount: Decimal,
    #[serde(default)]
    pub tax_id: Option<String>,
    #[serde(default)]
    pub tax_category: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub tax_rate: Option<Decimal>,
    #[serde(default, with = "decimal_field::option")]
    pub net: Option<Decimal>,
    #[serde(default, with = "decimal_field::option")]
    pub vat: Option<Decimal>,
    #[serde(default)]
    pub unit_id: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub unit_factor: Option<Decimal>,
    #[serde(default, with = "decimal_field::option")]
    pub list_price: Option<Decimal>,
    #[serde(default)]
    pub price_override_reason: Option<String>,
    #[serde(default)]
    pub batch_id: Option<String>,
    #[serde(default)]
    pub batch_no: Option<String>,
    #[serde(default)]
    pub is_free_text: bool,
    #[serde(default)]
    pub revenue_account_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceTenderV1 {
    pub payment_method_id: String,
    #[serde(with = "decimal_field")]
    pub amount: Decimal,
    #[serde(default)]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceV1 {
    pub id: String,
    pub number: String,
    pub date: String,
    #[serde(default)]
    pub customer_id: Option<String>,
    pub cashier_id: String,
    pub status: String,
    pub payment_status: String,
    #[serde(with = "decimal_field")]
    pub sub_total: Decimal,
    #[serde(with = "decimal_field")]
    pub discount_rate: Decimal,
    #[serde(with = "decimal_field")]
    pub discount_amount: Decimal,
    #[serde(with = "decimal_field")]
    pub tax_rate: Decimal,
    #[serde(with = "decimal_field")]
    pub tax_amount: Decimal,
    #[serde(with = "decimal_field")]
    pub grand_total: Decimal,
    pub payment_method: String,
    #[serde(with = "decimal_field")]
    pub paid_amount: Decimal,
    #[serde(default, with = "decimal_field")]
    pub refunded_amount: Decimal,
    #[serde(default, with = "decimal_field::option")]
    pub tendered_amount: Option<Decimal>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub shift_id: Option<String>,
    #[serde(default)]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub invoice_type: Option<String>,
    #[serde(default)]
    pub po_reference: Option<String>,
    #[serde(default)]
    pub terms: Option<String>,
    #[serde(default)]
    pub attachment_ids: Option<Vec<String>>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub exchange_rate: Option<Decimal>,
    #[serde(default)]
    pub cost_center_id: Option<String>,
    #[serde(default)]
    pub lines: Vec<InvoiceLineV1>,
    #[serde(default)]
    pub tenders: Vec<InvoiceTenderV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundLineV1 {
    pub invoice_line_id: String,
    #[serde(with = "decimal_field")]
    pub qty: Decimal,
    #[serde(default)]
    pub restock: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundV1 {
    pub id: String,
    pub number: String,
    pub invoice_id: String,
    pub date: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(with = "decimal_field")]
    pub sub_total: Decimal,
    #[serde(with = "decimal_field")]
    pub tax_amount: Decimal,
    #[serde(with = "decimal_field")]
    pub grand_total: Decimal,
    #[serde(default, with = "decimal_field")]
    pub settled_to_receivable: Decimal,
    #[serde(default, with = "decimal_field")]
    pub cash_back: Decimal,
    #[serde(default)]
    pub refund_method: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub credited_to_account: Option<Decimal>,
    #[serde(default)]
    pub lines: Vec<RefundLineV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotationV1 {
    pub id: String,
    pub number: String,
    pub date: String,
    #[serde(default)]
    pub expiry_date: Option<String>,
    #[serde(default)]
    pub customer_id: Option<String>,
    pub salesperson_id: String,
    pub status: String,
    #[serde(with = "decimal_field")]
    pub discount_rate: Decimal,
    #[serde(with = "decimal_field")]
    pub discount_amount: Decimal,
    #[serde(with = "decimal_field")]
    pub tax_amount: Decimal,
    #[serde(with = "decimal_field")]
    pub sub_total: Decimal,
    #[serde(with = "decimal_field")]
    pub grand_total: Decimal,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub terms: Option<String>,
    #[serde(default)]
    pub po_reference: Option<String>,
    #[serde(default)]
    pub attachment_ids: Option<Vec<String>>,
    #[serde(default)]
    pub converted_invoice_id: Option<String>,
    #[serde(default)]
    pub lines: Vec<InvoiceLineV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeldSaleV1 {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    pub terminal_id: String,
    pub held_at: String,
    pub held_by: String,
    #[serde(default)]
    pub customer_id: Option<String>,
    #[serde(default, with = "decimal_field")]
    pub discount_rate: Decimal,
    #[serde(default)]
    pub discount_is_pct: bool,
    #[serde(default)]
    pub note: Option<String>,
    /// The parked lines — `HeldSale.lines` in the TS type (`invoices/types/index.ts`), stored in the
    /// `held_sales.cart` JSON column (the column name, not a snapshot key). Part 04 Wave 2 (L1): this
    /// used to read a `cart` key no mock `HeldSale` ever had, so every imported held sale lost its lines.
    #[serde(default)]
    pub lines: Value,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShiftMovementV1 {
    #[serde(default)]
    pub id: Option<String>,
    pub kind: String,
    #[serde(with = "decimal_field")]
    pub amount: Decimal,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub ref_id: Option<String>,
    #[serde(default)]
    pub ref_number: Option<String>,
    pub at: String,
    #[serde(rename = "by")]
    pub by_user: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShiftV1 {
    pub id: String,
    pub number: String,
    pub terminal_id: String,
    #[serde(default)]
    pub branch_id: Option<String>,
    pub status: String,
    pub opened_by: String,
    pub opened_at: String,
    #[serde(with = "decimal_field")]
    pub opening_float: Decimal,
    #[serde(default)]
    pub opening_denominations: Option<Value>,
    #[serde(default)]
    pub closed_by: Option<String>,
    #[serde(default)]
    pub closed_at: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub counted_cash: Option<Decimal>,
    #[serde(default)]
    pub closing_denominations: Option<Value>,
    #[serde(default, with = "decimal_field::option")]
    pub expected_cash: Option<Decimal>,
    #[serde(default, with = "decimal_field::option")]
    pub variance: Option<Decimal>,
    #[serde(default)]
    pub handover_mode: Option<String>,
    #[serde(default)]
    pub force_closed_by: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub movements: Vec<ShiftMovementV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- purchases ---------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseOrderLineV1 {
    pub product_id: String,
    #[serde(with = "decimal_field")]
    pub qty: Decimal,
    #[serde(with = "decimal_field")]
    pub cost_price: Decimal,
    #[serde(default)]
    pub unit_id: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub unit_factor: Option<Decimal>,
    #[serde(default, with = "decimal_field::option")]
    pub discount: Option<Decimal>,
    #[serde(default)]
    pub discount_is_pct: Option<bool>,
    #[serde(default)]
    pub tax_id: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub received_qty: Option<Decimal>,
    #[serde(default)]
    pub batch_no: Option<String>,
    #[serde(default)]
    pub expiry_date: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub landed_cost_share: Option<Decimal>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseOrderV1 {
    pub id: String,
    pub number: String,
    pub supplier_id: String,
    pub date: String,
    pub status: String,
    #[serde(with = "decimal_field")]
    pub sub_total: Decimal,
    #[serde(with = "decimal_field")]
    pub tax_rate: Decimal,
    #[serde(with = "decimal_field")]
    pub tax_amount: Decimal,
    #[serde(with = "decimal_field")]
    pub grand_total: Decimal,
    pub payment_status: String,
    #[serde(default, with = "decimal_field")]
    pub paid_amount: Decimal,
    #[serde(default, with = "decimal_field")]
    pub returned_amount: Decimal,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub invoice_discount_pct: Option<Decimal>,
    #[serde(default, with = "decimal_field::option")]
    pub invoice_discount_amount: Option<Decimal>,
    #[serde(default)]
    pub landed_costs: Option<Value>,
    #[serde(default)]
    pub supplier_invoice_no: Option<String>,
    #[serde(default)]
    pub supplier_invoice_date: Option<String>,
    #[serde(default)]
    pub vat_not_recoverable: Option<bool>,
    #[serde(default)]
    pub sent_at: Option<String>,
    #[serde(default)]
    pub backorder_of_id: Option<String>,
    #[serde(default)]
    pub received_date: Option<String>,
    #[serde(default)]
    pub attachment_ids: Option<Vec<String>>,
    #[serde(default)]
    pub cost_center_id: Option<String>,
    #[serde(default)]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub exchange_rate: Option<Decimal>,
    #[serde(default)]
    pub lines: Vec<PurchaseOrderLineV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseReturnLineV1 {
    pub product_id: String,
    #[serde(with = "decimal_field")]
    pub qty: Decimal,
    #[serde(with = "decimal_field")]
    pub cost_price: Decimal,
    #[serde(default)]
    pub batch_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseReturnV1 {
    pub id: String,
    pub number: String,
    pub purchase_order_id: String,
    pub supplier_id: String,
    pub date: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(with = "decimal_field")]
    pub sub_total: Decimal,
    #[serde(with = "decimal_field")]
    pub tax_amount: Decimal,
    #[serde(with = "decimal_field")]
    pub grand_total: Decimal,
    #[serde(default, with = "decimal_field")]
    pub settled_to_payable: Decimal,
    #[serde(default, with = "decimal_field")]
    pub cash_back: Decimal,
    pub refund_method: String,
    #[serde(default)]
    pub from_draft_id: Option<String>,
    #[serde(default)]
    pub lines: Vec<PurchaseReturnLineV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- payments --------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentAllocationV1 {
    #[serde(default)]
    pub id: Option<String>,
    pub target_kind: String,
    pub target_id: String,
    pub target_number: String,
    #[serde(with = "decimal_field")]
    pub amount: Decimal,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub amount_fc: Option<Decimal>,
    #[serde(default, with = "decimal_field::option")]
    pub fx_gain_loss: Option<Decimal>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentV1 {
    pub id: String,
    pub number: String,
    pub date: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub target_type: String,
    pub target_id: String,
    #[serde(default)]
    pub target_ref: Option<String>,
    #[serde(default)]
    pub target_ref_number: Option<String>,
    #[serde(with = "decimal_field")]
    pub amount: Decimal,
    pub method: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub amount_fc: Option<Decimal>,
    #[serde(default, with = "decimal_field::option")]
    pub rate: Option<Decimal>,
    #[serde(default, with = "decimal_field::option")]
    pub fx_gain_loss: Option<Decimal>,
    #[serde(default)]
    pub allocations: Vec<PaymentAllocationV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- settings/taxes/payment methods ------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxV1 {
    pub id: String,
    pub name: String,
    #[serde(with = "decimal_field")]
    pub rate: Decimal,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub exemption_reason: Option<String>,
    #[serde(default)]
    pub account_role: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodV1 {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub account_role: String,
    #[serde(default, with = "decimal_field")]
    pub fee_pct: Decimal,
    #[serde(default)]
    pub requires_reference: Option<bool>,
    #[serde(default)]
    pub show_in_pos: bool,
    #[serde(default)]
    pub show_in_payments: bool,
    #[serde(default)]
    pub sort_order: i16,
    #[serde(default)]
    pub branch_overrides: Option<Value>,
    #[serde(default)]
    pub active: bool,
    /// TS `PaymentMethod.canDelete` — `false` on the seeded built-ins (cash, mada, …).
    #[serde(default)]
    pub can_delete: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PrinterSettingsV1 {
    pub mode: String,
    pub thermal_width_mm: Option<i32>,
    pub thermal: Option<Value>,
    pub a4_printer_name: Option<String>,
    pub label_printer_name: Option<String>,
    pub a4_template: Option<String>,
    pub image_template: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsV1 {
    pub store_name: String,
    pub logo: Option<String>,
    pub stamp: Option<String>,
    pub signature: Option<String>,
    pub currency: Option<String>,
    pub country: Option<String>,
    pub vat_number: Option<String>,
    pub default_tax_id: Option<String>,
    pub invoice_number_prefix: Option<String>,
    pub printer: PrinterSettingsV1,
    pub prices_include_tax: Option<bool>,
    pub address: Option<String>,
    pub national_address: Option<Value>,
    pub phone: Option<String>,
    pub commercial_register: Option<String>,
    pub receipt_footer: Option<String>,
    pub accounting: Option<AccountingPolicyV1>,
    pub backup: Option<Value>,
    pub inventory_approval_threshold: Option<Value>,
    pub role_access_overrides: Option<Value>,
    pub insight_thresholds: Option<Value>,
    pub pos: Option<Value>,
    pub sales: Option<Value>,
    pub features: Option<Value>,
    pub onboarding: Option<OnboardingStateV1>,
    pub theme: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountingPolicyV1 {
    pub lock_date: Option<String>,
    pub default_purchase_account_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OnboardingStateV1 {
    pub business_type: Option<String>,
    pub go_live_date: Option<String>,
    pub completed_step: Option<i32>,
    pub skipped: Vec<String>,
    pub done: Vec<String>,
    pub finished_at: Option<String>,
    pub opening_entry_id: Option<String>,
    pub closing_entry_id: Option<String>,
    pub coa_template: Option<String>,
}

// --- expenses --------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseCategoryV1 {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    pub account_id: String,
    #[serde(default)]
    pub default_tax_id: Option<String>,
    #[serde(default)]
    pub default_cost_center_id: Option<String>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub can_delete: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PaidFromV1 {
    pub kind: String,
    pub payment_method_id: Option<String>,
    pub supplier_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpenseV1 {
    pub id: String,
    pub number: String,
    pub date: String,
    pub category_id: String,
    #[serde(with = "decimal_field")]
    pub amount: Decimal,
    #[serde(default)]
    pub is_tax_invoice: bool,
    #[serde(default)]
    pub tax_id: Option<String>,
    #[serde(default, with = "decimal_field")]
    pub net_amount: Decimal,
    #[serde(default, with = "decimal_field")]
    pub tax_amount: Decimal,
    #[serde(default)]
    pub supplier_vat_number: Option<String>,
    #[serde(default)]
    pub supplier_invoice_no: Option<String>,
    #[serde(default)]
    pub cost_center_id: Option<String>,
    pub paid_from: PaidFromV1,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub attachment_ids: Option<Vec<String>>,
    #[serde(default)]
    pub repeat_monthly: bool,
    #[serde(default)]
    pub recurring_template_id: Option<String>,
    pub created_by: String,
    #[serde(default)]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurringExpenseV1 {
    pub id: String,
    pub name: String,
    pub category_id: String,
    #[serde(with = "decimal_field")]
    pub amount: Decimal,
    #[serde(default)]
    pub is_tax_invoice: bool,
    #[serde(default)]
    pub tax_id: Option<String>,
    pub paid_from: PaidFromV1,
    #[serde(default)]
    pub description: Option<String>,
    pub day: i8,
    pub next_date: String,
    #[serde(default)]
    pub auto_post: bool,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- vouchers --------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoucherV1 {
    pub id: String,
    pub number: String,
    pub kind: String,
    pub date: String,
    #[serde(with = "decimal_field")]
    pub amount: Decimal,
    pub description: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub attachment_ids: Option<Vec<String>>,
    #[serde(default)]
    pub cost_center_id: Option<String>,
    pub created_by: String,
    #[serde(default)]
    pub payment_method_id: Option<String>,
    #[serde(default)]
    pub credit_account_id: Option<String>,
    #[serde(default)]
    pub debit_account_id: Option<String>,
    #[serde(default)]
    pub source_account_id: Option<String>,
    #[serde(default)]
    pub destination_account_id: Option<String>,
    #[serde(default, with = "decimal_field::option")]
    pub fee_amount: Option<Decimal>,
    #[serde(default)]
    pub fee_account_id: Option<String>,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub cash_account_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardSettlementGroupV1 {
    pub date: String,
    pub payment_method_id: String,
    #[serde(with = "decimal_field")]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardSettlementV1 {
    pub id: String,
    pub number: String,
    pub date: String,
    #[serde(with = "decimal_field")]
    pub gross_amount: Decimal,
    #[serde(with = "decimal_field")]
    pub deposit_amount: Decimal,
    #[serde(with = "decimal_field")]
    pub fee_amount: Decimal,
    #[serde(default)]
    pub note: Option<String>,
    pub created_by: String,
    #[serde(default)]
    pub groups: Vec<CardSettlementGroupV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- branches / dimensions --------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchV1 {
    pub id: String,
    pub name: String,
    pub code: String,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub national_address: Option<Value>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub receipt_header: Option<String>,
    #[serde(default)]
    pub cash_account_id: Option<String>,
    #[serde(default)]
    pub bank_account_id: Option<String>,
    #[serde(default)]
    pub default_price_list_id: Option<String>,
    #[serde(default)]
    pub cost_center_id: Option<String>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub can_delete: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostCenterBudgetV1 {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub cost_center_id: Option<String>,
    pub fiscal_year_id: String,
    #[serde(with = "decimal_field")]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostCenterV1 {
    pub id: String,
    pub code: String,
    pub name: String,
    /// TS `CostCenter.type` (`settings/types/dimensions.ts`).
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub manager_user_id: Option<String>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub can_delete: bool,
    #[serde(default)]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub budgets: Vec<CostCenterBudgetV1>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyV1 {
    pub code: String,
    pub name_ar: String,
    pub symbol: String,
    pub decimals: i32,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub fixed: Option<bool>,
    #[serde(default, with = "decimal_field::option")]
    pub fixed_rate: Option<Decimal>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRateV1 {
    #[serde(default)]
    pub id: Option<String>,
    pub currency: String,
    pub date: String,
    #[serde(with = "decimal_field")]
    pub rate: Decimal,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- approvals -------------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequestV1 {
    pub id: String,
    pub kind: String,
    pub summary: String,
    #[serde(with = "decimal_field")]
    pub value: Decimal,
    #[serde(default)]
    pub request_note: Option<String>,
    pub requested_by: String,
    pub requested_by_name: String,
    pub requested_at: String,
    pub status: String,
    #[serde(default)]
    pub decided_by: Option<String>,
    #[serde(default)]
    pub decided_by_name: Option<String>,
    #[serde(default)]
    pub decided_at: Option<String>,
    #[serde(default)]
    pub decision_comment: Option<String>,
    #[serde(default)]
    pub link: Option<Value>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

// --- activity / audit ------------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntryV1 {
    pub id: String,
    pub date: String,
    pub user_id: String,
    pub kind: String,
    pub message: String,
    #[serde(default)]
    pub link: Option<Value>,
    #[serde(default)]
    pub audit_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntryV1 {
    pub id: String,
    pub entity: String,
    pub entity_id: String,
    #[serde(default)]
    pub entity_label: Option<String>,
    pub action: String,
    #[serde(default)]
    pub before: Option<Value>,
    #[serde(default)]
    pub after: Option<Value>,
    pub user_id: String,
    #[serde(default)]
    pub branch_id: Option<String>,
    pub at: String,
    #[serde(default)]
    pub reason: Option<String>,
    pub message: String,
    #[serde(default)]
    pub link: Option<Value>,
    #[serde(default)]
    pub created_at: Option<String>,
}
