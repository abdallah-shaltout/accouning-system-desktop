//! `payments` DTOs (03-domains/09-payments.md §2) — mirror `src/modules/payments/types/index.ts`
//! and `paymentService.ts`'s local `PaymentRow`. One args struct per command (§3.2 convention,
//! matching `domains::expenses::dto`/`domains::parties::dto`).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::entities::payments::payment_allocations::PaymentAllocationTargetKind;
use crate::entities::payments::payments::{PaymentMethodKind, PaymentTargetType, PaymentType};
use crate::shared::balances::{OpenDocument as BalancesOpenDocument, OpenDocumentKind};
use crate::utils::id::Id;
use crate::utils::money::serde_number;

// --- Enums (wire shape matches the mock's TS unions exactly) -----------------------------------

/// `PaymentType` (`types/index.ts:1`) — `#[serde(rename_all = "UPPERCASE")]` (01.B §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export_to = "payments/types/gen/")]
pub enum PaymentTypeDto {
    Received,
    Paid,
}

impl From<PaymentType> for PaymentTypeDto {
    fn from(t: PaymentType) -> Self {
        match t {
            PaymentType::Received => PaymentTypeDto::Received,
            PaymentType::Paid => PaymentTypeDto::Paid,
        }
    }
}

impl From<PaymentTypeDto> for PaymentType {
    fn from(t: PaymentTypeDto) -> Self {
        match t {
            PaymentTypeDto::Received => PaymentType::Received,
            PaymentTypeDto::Paid => PaymentType::Paid,
        }
    }
}

/// `'customer' | 'supplier'` (`Payment.targetType`) — the same `PartyKind` concept used elsewhere,
/// named/scoped to this module's DTOs per the spec (avoids importing `journal_lines::PartyKind`
/// into every response type); call sites that need a `journal_lines::PartyKind` for a `PartyRef`
/// build it directly from the entity-level `PaymentTargetType`/`PaymentType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub enum PartyKind {
    Customer,
    Supplier,
}

impl From<PaymentTargetType> for PartyKind {
    fn from(t: PaymentTargetType) -> Self {
        match t {
            PaymentTargetType::Customer => PartyKind::Customer,
            PaymentTargetType::Supplier => PartyKind::Supplier,
        }
    }
}

impl From<PartyKind> for PaymentTargetType {
    fn from(k: PartyKind) -> Self {
        match k {
            PartyKind::Customer => PaymentTargetType::Customer,
            PartyKind::Supplier => PaymentTargetType::Supplier,
        }
    }
}

/// `PaymentMethod` (`types/index.ts:2`) — named `PaymentTenderKind` on the Rust side (01.B §2) to
/// avoid a naming clash with `settings::dto::PaymentMethod` (the configurable tender record).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "payments/types/gen/", rename = "PaymentMethod")]
pub enum PaymentTenderKind {
    Cash,
    Card,
    BankTransfer,
}

impl PaymentTenderKind {
    /// The exact string `shared::ledger::accounts::settlement_account_for` compares against
    /// (`"cash"` vs. anything else -> bank).
    pub fn as_str(self) -> &'static str {
        match self {
            PaymentTenderKind::Cash => "cash",
            PaymentTenderKind::Card => "card",
            PaymentTenderKind::BankTransfer => "bank_transfer",
        }
    }
}

impl From<PaymentMethodKind> for PaymentTenderKind {
    fn from(m: PaymentMethodKind) -> Self {
        match m {
            PaymentMethodKind::Cash => PaymentTenderKind::Cash,
            PaymentMethodKind::Card => PaymentTenderKind::Card,
            PaymentMethodKind::BankTransfer => PaymentTenderKind::BankTransfer,
        }
    }
}

impl From<PaymentTenderKind> for PaymentMethodKind {
    fn from(m: PaymentTenderKind) -> Self {
        match m {
            PaymentTenderKind::Cash => PaymentMethodKind::Cash,
            PaymentTenderKind::Card => PaymentMethodKind::Card,
            PaymentTenderKind::BankTransfer => PaymentMethodKind::BankTransfer,
        }
    }
}

/// `PaymentAllocation.targetKind` (`types/index.ts:12`) — **3 stored variants** (`opening` is a
/// setup-module-only kind, never produced by this domain's own writes, 01.B §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub enum AllocationTargetKind {
    Invoice,
    PurchaseOrder,
    Opening,
}

impl From<PaymentAllocationTargetKind> for AllocationTargetKind {
    fn from(k: PaymentAllocationTargetKind) -> Self {
        match k {
            PaymentAllocationTargetKind::Invoice => AllocationTargetKind::Invoice,
            PaymentAllocationTargetKind::PurchaseOrder => AllocationTargetKind::PurchaseOrder,
            PaymentAllocationTargetKind::Opening => AllocationTargetKind::Opening,
        }
    }
}

impl From<AllocationTargetKind> for PaymentAllocationTargetKind {
    fn from(k: AllocationTargetKind) -> Self {
        match k {
            AllocationTargetKind::Invoice => PaymentAllocationTargetKind::Invoice,
            AllocationTargetKind::PurchaseOrder => PaymentAllocationTargetKind::PurchaseOrder,
            AllocationTargetKind::Opening => PaymentAllocationTargetKind::Opening,
        }
    }
}

/// `PaymentAllocationInput.targetKind` (`types/index.ts:66`) — **2 variants only**: an `opening`
/// input can never be constructed on the wire (01.B §2 — confirms `createPayment`/
/// `allocateExistingPayment` can never themselves create an `opening`-kind allocation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub enum AllocationInputTargetKind {
    Invoice,
    PurchaseOrder,
}

impl From<AllocationInputTargetKind> for AllocationTargetKind {
    fn from(k: AllocationInputTargetKind) -> Self {
        match k {
            AllocationInputTargetKind::Invoice => AllocationTargetKind::Invoice,
            AllocationInputTargetKind::PurchaseOrder => AllocationTargetKind::PurchaseOrder,
        }
    }
}

impl From<AllocationInputTargetKind> for PaymentAllocationTargetKind {
    fn from(k: AllocationInputTargetKind) -> Self {
        match k {
            AllocationInputTargetKind::Invoice => PaymentAllocationTargetKind::Invoice,
            AllocationInputTargetKind::PurchaseOrder => PaymentAllocationTargetKind::PurchaseOrder,
        }
    }
}

/// `OpenDocument.kind` (`types/index.ts:102`) — DTO-facing mirror of `shared::balances::
/// OpenDocumentKind` (2 variants only — an `OpenDocument` is never `opening`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub enum OpenDocumentKindDto {
    Invoice,
    PurchaseOrder,
}

impl From<OpenDocumentKind> for OpenDocumentKindDto {
    fn from(k: OpenDocumentKind) -> Self {
        match k {
            OpenDocumentKind::Invoice => OpenDocumentKindDto::Invoice,
            OpenDocumentKind::PurchaseOrder => OpenDocumentKindDto::PurchaseOrder,
        }
    }
}

/// `AllocationStatus` (`types/index.ts:119`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "payments/types/gen/")]
pub enum AllocationStatus {
    Full,
    Partial,
    Unallocated,
}

/// `allocationStatusFor` (`types/index.ts:121-125`): `≤ 0.005 -> unallocated`,
/// `≥ amount − 0.005 -> full`, else `partial`.
pub fn allocation_status_for(amount: Decimal, allocated: Decimal) -> AllocationStatus {
    let tolerance = Decimal::new(5, 3);
    if allocated <= tolerance {
        AllocationStatus::Unallocated
    } else if allocated >= amount - tolerance {
        AllocationStatus::Full
    } else {
        AllocationStatus::Partial
    }
}

// --- DTOs ----------------------------------------------------------------------------------------

/// `PaymentAllocation` (`types/index.ts:9-26`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct PaymentAllocation {
    #[ts(type = "string")]
    pub id: Id,
    pub target_kind: AllocationTargetKind,
    #[ts(type = "string")]
    pub target_id: Id,
    pub target_number: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    /// A business day key (`YYYY-MM-DD`) or ISO instant (`DocDate::key()`) — the payment's own date.
    pub date: String,
    #[ts(optional, type = "number")]
    #[serde(with = "serde_number::option", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_fc: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(with = "serde_number::option", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fx_gain_loss: Option<Decimal>,
}

/// `Payment` (`types/index.ts:28-63`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct Payment {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub date: String,
    pub r#type: PaymentTypeDto,
    pub target_type: PartyKind,
    #[ts(type = "string")]
    pub target_id: Id,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_ref: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_ref_number: Option<String>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    pub method: PaymentTenderKind,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub allocations: Vec<PaymentAllocation>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(with = "serde_number::option", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_fc: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(with = "serde_number::option", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(with = "serde_number::option", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fx_gain_loss: Option<Decimal>,
}

/// `PaymentRow` (`paymentService.ts:133` — `Payment` flattened + 4 computed fields).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct PaymentRow {
    #[serde(flatten)]
    pub payment: Payment,
    pub party_name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub allocated: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub unallocated: Decimal,
    pub allocation_status: AllocationStatus,
}

/// `PaymentAllocationInput` (`types/index.ts:65-69`) — request-only, 2-variant `targetKind`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct PaymentAllocationInput {
    pub target_kind: AllocationInputTargetKind,
    #[ts(type = "string")]
    pub target_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
}

/// `PaymentInput` (`types/index.ts:71-86`) — request-only.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct PaymentInput {
    pub date: String,
    pub r#type: PaymentTypeDto,
    pub target_type: PartyKind,
    #[ts(type = "string")]
    pub target_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    pub method: PaymentTenderKind,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocations: Option<Vec<PaymentAllocationInput>>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(with = "serde_number::option", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_fc: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(with = "serde_number::option", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<Decimal>,
}

/// `PaymentFilter` (`types/index.ts:88-97`). `to` is a plain date-range upper bound, NOT a route
/// (01.B §2 — the contract generator's route-hint regex false-positives on the field name `to`).
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct PaymentFilter {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<PaymentTypeDto>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<PaymentTenderKind>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /* route-ok: PaymentFilter.to is a plain date-range bound, not a route (01.B §2) */
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unallocated_only: Option<bool>,
}

/// `OpenDocument` (`types/index.ts:100-116`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct OpenDocument {
    #[ts(type = "string")]
    pub id: Id,
    pub kind: OpenDocumentKindDto,
    pub number: String,
    pub date: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub outstanding: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(with = "serde_number::option", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fc_outstanding: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(with = "serde_number::option", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<Decimal>,
}

impl From<BalancesOpenDocument> for OpenDocument {
    fn from(d: BalancesOpenDocument) -> Self {
        OpenDocument {
            id: d.id,
            kind: d.kind.into(),
            number: d.number,
            date: d.date.key(),
            due_date: d.due_date_key,
            total: d.total,
            outstanding: d.outstanding,
            currency: d.currency,
            fc_outstanding: d.fc_outstanding,
            rate: d.rate,
        }
    }
}

// --- Args structs (one per command, §3.2 convention) --------------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct PaymentsGetPaymentsArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<PaymentFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct PaymentsGetPaymentsPagedArgs {
    pub query: crate::core::dto::PagedQuery<PaymentFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct PaymentsGetPaymentArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct PaymentsCreatePaymentArgs {
    pub input: PaymentInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct PaymentsAllocateExistingPaymentArgs {
    pub payment_id: String,
    pub allocations: Vec<PaymentAllocationInput>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct PaymentsRemoveAllocationArgs {
    pub payment_id: String,
    pub allocation_id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "payments/types/gen/")]
pub struct PaymentsGetOpenDocumentsArgs {
    pub target_type: PartyKind,
    pub target_id: String,
}
